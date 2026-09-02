//! 使用纯 Rust 读取 osu!lazer Realm 数据库，并转换为应用使用的收藏夹模型。

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Utc;
use realm_db_reader::{Group, Link, Realm, Row, Value};

use crate::models::{BeatmapEntry, CollectionInfo, ExtractedData};

// 小表直接缓存全部行，大表仅按链接访问并缓存已读取行。
const BULK_ROW_LIMIT: usize = 50_000;
static SNAPSHOT_SEQUENCE: AtomicU64 = AtomicU64::new(0); // 防止多个加载请求使用相同快照名。

/**
 * 读取 Realm 文件快照并生成当前应用的数据结构。
 */
pub fn parse_realm_file(realm_path: &Path) -> Result<ExtractedData, String> {
    let snapshot = snapshot_realm(realm_path)?;
    let result = parse_snapshot(&snapshot, realm_path);
    let _ = fs::remove_file(&snapshot); // 解析完成后删除快照，避免临时目录堆积。
    result
}

/**
 * 复制一份数据库快照，避免 osu!lazer 正在运行时锁文件或写事务影响读取。
 */
fn snapshot_realm(realm_path: &Path) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    let sequence = SNAPSHOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let snapshot = std::env::temp_dir().join(format!(
        "collection-view-client-realm-{}-{stamp}-{sequence}.realm",
        std::process::id()
    ));
    fs::copy(realm_path, &snapshot).map_err(error_text("复制 client.realm 快照失败"))?;
    Ok(snapshot)
}

/**
 * 打开快照，读取收藏夹、谱面及其关联对象。
 */
fn parse_snapshot(snapshot: &Path, source_path: &Path) -> Result<ExtractedData, String> {
    let realm = Realm::open(snapshot).map_err(error_text("打开 client.realm 失败"))?;
    let group = realm.into_group().map_err(error_text("读取 Realm 组失败"))?;
    let mut store = RowStore::new(&group);

    let requested_hashes = collect_requested_hashes(&mut store)?;
    let beatmaps = build_beatmap_index(&mut store, &requested_hashes)?;
    let collections = build_collections(&mut store, &beatmaps)?;

    Ok(ExtractedData {
        source_path: source_path.to_string_lossy().into_owned(),
        generated_at: Utc::now().to_rfc3339(),
        collections,
    })
}

/**
 * 收集所有收藏夹引用的 MD5，避免为未被收藏的谱面解引用完整元数据。
 */
fn collect_requested_hashes(store: &mut RowStore<'_>) -> Result<std::collections::HashSet<String>, String> {
    let mut requested = std::collections::HashSet::new();
    for row in store.bulk_rows("class_BeatmapCollection")? {
        for hash in string_list(row.get("BeatmapMD5Hashes")) {
            let key = hash.trim().to_ascii_lowercase();
            if !key.is_empty() {
                requested.insert(key);
            }
        }
    }
    Ok(requested)
}

/**
 * 扫描谱面表，并只为收藏夹实际引用的 MD5 解析完整关联数据。
 */
fn build_beatmap_index(
    store: &mut RowStore<'_>,
    requested: &std::collections::HashSet<String>,
) -> Result<HashMap<String, BeatmapEntry>, String> {
    let mut index = HashMap::new();
    for row in store.all_rows("class_Beatmap")? {
        let md5 = string_value(row.get("MD5Hash")).trim().to_ascii_lowercase();
        if !requested.contains(&md5) || index.contains_key(&md5) {
            continue;
        }
        let entry = build_beatmap_entry(store, &row, &md5)?;
        index.insert(md5, entry);
        if index.len() == requested.len() {
            break; // 已匹配全部收藏夹 MD5 时无需继续扫描剩余谱面。
        }
    }
    Ok(index)
}

/**
 * 将一行 Beatmap 及其 Realm 关联对象转换为前端谱面字段。
 */
fn build_beatmap_entry(
    store: &mut RowStore<'_>,
    row: &Row<'static>,
    md5: &str,
) -> Result<BeatmapEntry, String> {
    let metadata = linked_row(store, row.get("Metadata"))?;
    let difficulty = linked_row(store, row.get("Difficulty"))?;
    let ruleset = linked_row(store, row.get("Ruleset"))?;
    let beatmap_set = linked_row(store, row.get("BeatmapSet"))?;

    let artist = metadata
        .as_ref()
        .map(|item| string_value(item.get("Artist")))
        .unwrap_or_default();
    let title = metadata
        .as_ref()
        .map(|item| string_value(item.get("Title")))
        .unwrap_or_default();
    let mapper = metadata
        .as_ref()
        .map(|item| linked_row(store, item.get("Author")))
        .transpose()?
        .flatten()
        .map(|item| string_value(item.get("Username")))
        .unwrap_or_default();

    let beatmap_set_id = beatmap_set
        .as_ref()
        .map(|item| int_value(item.get("OnlineID"), -1))
        .filter(|value| *value > 0);
    let ruleset_short_name = ruleset
        .as_ref()
        .map(|item| string_value(item.get("ShortName")))
        .unwrap_or_default();

    let title_unicode = metadata
        .as_ref()
        .map(|item| {
            non_empty_or(
                string_value(item.get("TitleUnicode")),
                title.clone(),
            )
        })
        .unwrap_or_else(|| title.clone());
    let artist_unicode = metadata
        .as_ref()
        .map(|item| {
            non_empty_or(
                string_value(item.get("ArtistUnicode")),
                artist.clone(),
            )
        })
        .unwrap_or_else(|| artist.clone());

    Ok(BeatmapEntry {
        md5: md5.to_string(),
        title, // 标题、艺术家等文本直接来自 Beatmap.Metadata 关联对象。
        title_unicode,
        artist,
        artist_unicode,
        beatmap_id: positive_int(row.get("OnlineID")), // 非正数是 osu!lazer 表示未关联在线谱面的哨兵值。
        beatmap_set_id,
        star_rating: non_negative_float(row.get("StarRating")),
        circle_size: difficulty.as_ref().and_then(|item| float_value(item.get("CircleSize"))),
        overall_difficulty: difficulty
            .as_ref()
            .and_then(|item| float_value(item.get("OverallDifficulty"))),
        approach_rate: difficulty
            .as_ref()
            .and_then(|item| float_value(item.get("ApproachRate"))),
        drain_rate: difficulty
            .as_ref()
            .and_then(|item| float_value(item.get("DrainRate"))),
        total_object_count: positive_or_zero_int(row.get("TotalObjectCount")),
        length_ms: non_negative_float(row.get("Length")),
        bpm: positive_float(row.get("BPM")),
        status_int: optional_int(first_value(row, &["Status", "StatusInt"]))
            .and_then(|value| value.try_into().ok()), // Realm 实际列名为 Status，兼容旧提取器使用的 StatusInt 别名。
        difficulty_name: string_value(row.get("DifficultyName")),
        mapper,
        ruleset_short_name,
        ruleset_name: ruleset
            .as_ref()
            .map(|item| string_value(item.get("Name")))
            .unwrap_or_default(),
        background_url: beatmap_set_id
            .filter(|value| *value > 0)
            .map(|value| format!("https://assets.ppy.sh/beatmaps/{value}/covers/cover.jpg"))
            .unwrap_or_default(),
        missing: false, // 能从 class_Beatmap 命中 MD5 的条目均视为本地存在。
        mode: String::new(),
    })
}

/**
 * 读取 class_BeatmapCollection，并保留找不到本地谱面的 MD5 条目。
 */
fn build_collections(
    store: &mut RowStore<'_>,
    beatmaps: &HashMap<String, BeatmapEntry>,
) -> Result<Vec<CollectionInfo>, String> {
    let mut collections = Vec::new();
    for row in store.bulk_rows("class_BeatmapCollection")? {
        let hashes = string_list(row.get("BeatmapMD5Hashes"));
        let mut items = Vec::with_capacity(hashes.len());
        for hash in hashes {
            let key = hash.trim().to_ascii_lowercase();
            if key.is_empty() {
                continue;
            }
            items.push(beatmaps.get(&key).cloned().unwrap_or_else(|| BeatmapEntry {
                md5: hash,
                missing: true, // 收藏夹允许保留已经删除或尚未导入的谱面 MD5。
                ..BeatmapEntry::default()
            }));
        }
        collections.push(CollectionInfo {
            id: uuid_string(row.get("ID")),
            name: string_value(row.get("Name")),
            last_modified: timestamp_string(row.get("LastModified")),
            items,
        });
    }
    collections.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(collections)
}

// 表示表内容的缓存策略，避免对数万行 Beatmap 关联表重复整表复制。
enum TableData {
    Bulk(Vec<Row<'static>>),
    Lazy { table: realm_db_reader::Table, rows: HashMap<usize, Row<'static>> },
}

// 按表名管理 Realm 行，并统一处理 Link 解引用。
struct RowStore<'a> {
    group: &'a Group,
    numbers: HashMap<String, usize>,
    tables: HashMap<usize, TableData>,
}

impl<'a> RowStore<'a> {
    /**
     * 创建空的 Realm 行缓存。
     */
    fn new(group: &'a Group) -> Self {
        Self { group, numbers: HashMap::new(), tables: HashMap::new() }
    }

    /**
     * 查找表名对应的表编号，并给出可读的中文错误。
     */
    fn table_number(&mut self, name: &str) -> Result<usize, String> {
        if let Some(number) = self.numbers.get(name) { return Ok(*number); }
        let number = self.group.get_table_names().iter().position(|item| item == name)
            .ok_or_else(|| format!("数据库中没有 {name} 表，请确认 osu!lazer 数据库版本"))?;
        self.numbers.insert(name.to_string(), number);
        Ok(number)
    }

    /**
     * 按表大小决定全量读取或懒加载。
     */
    fn prepare(&mut self, name: &str) -> Result<usize, String> {
        let number = self.table_number(name)?;
        if self.tables.contains_key(&number) { return Ok(number); }
        let table = self.group.get_table(number).map_err(error_text("打开 Realm 表失败"))?;
        let data = if table.row_count().map_err(error_text("读取 Realm 表行数失败"))? <= BULK_ROW_LIMIT {
            TableData::Bulk(table.get_rows().map_err(error_text("读取 Realm 表失败"))?.into_iter().map(Row::into_owned).collect())
        } else {
            TableData::Lazy { table, rows: HashMap::new() } // 大表只在 Link 实际指向时读取目标行。
        };
        self.tables.insert(number, data);
        Ok(number)
    }

    /**
     * 读取整张小表；收藏夹等控制表超过阈值时拒绝异常数据。
     */
    fn bulk_rows(&mut self, name: &str) -> Result<Vec<Row<'static>>, String> {
        let number = self.prepare(name)?;
        match self.tables.get(&number).expect("表已插入缓存") {
            TableData::Bulk(rows) => Ok(rows.clone()),
            TableData::Lazy { .. } => Err(format!("{name} 表行数超过 {BULK_ROW_LIMIT}，无法建立安全索引")),
        }
    }

    /**
     * 顺序读取任意大小的表，用于必须建立 MD5 全量索引的 Beatmap 表。
     */
    fn all_rows(&mut self, name: &str) -> Result<Vec<Row<'static>>, String> {
        let number = self.table_number(name)?;
        let table = self.group.get_table(number).map_err(error_text("打开 Realm 表失败"))?;
        let row_count = table.row_count().map_err(error_text("读取 Realm 表行数失败"))?;
        let mut rows = Vec::with_capacity(row_count);
        for row_number in 0..row_count {
            rows.push(table.get_row(row_number).map_err(error_text("读取 Realm 表行失败"))?.into_owned());
        }
        Ok(rows)
    }

    /**
     * 根据 Link 读取目标行，并缓存懒加载表中的结果。
     */
    fn row(&mut self, link: &Link) -> Result<Option<Row<'static>>, String> {
        if !self.tables.contains_key(&link.target_table_number) {
            let table = self.group.get_table(link.target_table_number).map_err(error_text("打开 Realm 关联表失败"))?;
            let data = if table.row_count().map_err(error_text("读取 Realm 关联表行数失败"))? <= BULK_ROW_LIMIT {
                TableData::Bulk(table.get_rows().map_err(error_text("读取 Realm 关联表失败"))?.into_iter().map(Row::into_owned).collect())
            } else {
                TableData::Lazy { table, rows: HashMap::new() } // 复用 OPP 的大表懒加载策略，控制内存占用。
            };
            self.tables.insert(link.target_table_number, data);
        }
        match self.tables.get_mut(&link.target_table_number).expect("关联表已插入缓存") {
            TableData::Bulk(rows) => Ok(rows.get(link.row_number).cloned()),
            TableData::Lazy { table, rows } => {
                if let Some(row) = rows.get(&link.row_number) { return Ok(Some(row.clone())); }
                let row = table.get_row(link.row_number).map_err(error_text("读取 Realm 关联行失败"))?.into_owned();
                rows.insert(link.row_number, row.clone());
                Ok(Some(row))
            }
        }
    }
}

/**
 * 从可选 Link 值中解引用目标行；缺少链接时视为空值。
 */
fn linked_row(store: &mut RowStore<'_>, value: Option<&Value>) -> Result<Option<Row<'static>>, String> {
    match value {
        Some(Value::Link(link)) => store.row(link),
        _ => Ok(None),
    }
}

/**
 * 读取 Realm 字符串列表，兼容现代格式的 List 和旧格式的子表表示。
 */
fn string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::List(values)) => values.iter().filter_map(|item| match item {
            Value::String(text) => Some(text.clone()),
            Value::Binary(bytes) => String::from_utf8(bytes.clone()).ok(), // 兼容少数版本把 MD5 列表编码为二进制值。
            _ => None,
        }).collect(),
        Some(Value::Table(rows)) => rows.iter().filter_map(|row| {
            let text = row.values().find_map(|value| match value { Value::String(text) => Some(text.clone()), _ => None })?;
            (!text.is_empty()).then_some(text)
        }).collect(),
        _ => Vec::new(),
    }
}

/**
 * 将 Realm 字符串值转换为空字符串，屏蔽 null 和不兼容类型。
 */
fn string_value(value: Option<&Value>) -> String {
    match value { Some(Value::String(text)) => text.clone(), _ => String::new() }
}

/**
 * 按顺序返回首个存在的 Realm 字段，兼容数据库列名与旧模型别名。
 */
fn first_value<'a>(row: &'a Row<'_>, names: &[&str]) -> Option<&'a Value> {
    names.iter().find_map(|name| row.get(name))
}

/**
 * 将 Realm 整数值转换为带默认值的 i64。
 */
fn int_value(value: Option<&Value>, fallback: i64) -> i64 {
    match value { Some(Value::Int(number)) => *number, _ => fallback }
}

/**
 * 读取可选整数，字段缺失或 null 时返回 None。
 */
fn optional_int(value: Option<&Value>) -> Option<i64> {
    match value { Some(Value::Int(number)) => Some(*number), _ => None }
}

/**
 * 读取浮点和双精度值，统一为 f64。
 */
fn float_value(value: Option<&Value>) -> Option<f64> {
    match value { Some(Value::Float(number)) => Some(*number as f64), Some(Value::Double(number)) => Some(*number), _ => None }
}

/**
 * 仅保留大于零的整数 ID。
 */
fn positive_int(value: Option<&Value>) -> Option<i64> { Some(int_value(value, -1)).filter(|number| *number > 0) }

/**
 * 保留非负整数计数。
 */
fn positive_or_zero_int(value: Option<&Value>) -> Option<i64> { Some(int_value(value, -1)).filter(|number| *number >= 0) }

/**
 * 仅保留非负浮点值。
 */
fn non_negative_float(value: Option<&Value>) -> Option<f64> { float_value(value).filter(|number| number.is_finite() && *number >= 0.0) }

/**
 * 仅保留正浮点值。
 */
fn positive_float(value: Option<&Value>) -> Option<f64> { float_value(value).filter(|number| number.is_finite() && *number > 0.0) }

/**
 * 当 Realm 的 Unicode 文本为空时回退到普通文本，保持前端名称稳定。
 */
fn non_empty_or(value: String, fallback: String) -> String {
    if value.is_empty() { fallback } else { value }
}

/**
 * 将 Realm UUID 按小写十六进制输出，与旧提取器字符串格式一致。
 */
fn uuid_string(value: Option<&Value>) -> String {
    match value { Some(Value::Uuid(bytes)) => bytes.iter().map(|byte| format!("{byte:02x}")).collect(), _ => String::new() }
}

/**
 * 将 Realm 时间戳转换为 RFC3339，未知类型时返回空字符串。
 */
fn timestamp_string(value: Option<&Value>) -> String {
    match value { Some(Value::Timestamp(timestamp)) => timestamp.to_rfc3339(), _ => String::new() }
}

/**
 * 将标准错误转换为带上下文的中文错误。
 */
fn error_text<E: std::fmt::Display>(context: &'static str) -> impl FnOnce(E) -> String {
    move |error| format!("{context}：{error}")
}
