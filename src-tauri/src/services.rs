use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering as AtomicOrdering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use tauri::{AppHandle, Manager};

use crate::models::{
    default_columns, AppSettings, BeatmapEntry, BeatmapGroup, BeatmapPage, BeatmapQuery,
    CollectionSummary, CoverPayload, ExtractedData, LoadResult,
};
use crate::realm_parser;

static SETTINGS_WRITE_LOCK: Mutex<()> = Mutex::new(()); // 串行提交设置，避免快速切换模式时并发覆盖临时文件。
static FILE_REPLACE_LOCK: Mutex<()> = Mutex::new(()); // 串行提交设置和封面文件，避免同一目标被并发替换。
static DATABASE_LOAD_LOCK: Mutex<()> = Mutex::new(()); // 避免多个数据库读取任务同时复制 Realm 快照。
static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0); // 同一纳秒内继续使用递增序号保证临时路径唯一。

/**
 * 校验 Realm 路径，并使用 Rust 解析器加载标准化后的数据库数据。
 */
pub fn load_database(_app: &AppHandle, realm_path: &str) -> Result<(ExtractedData, LoadResult), String> {
    let _load_guard = DATABASE_LOAD_LOCK
        .lock()
        .map_err(|_| "数据库加载锁已损坏。".to_string())?;
    let realm_path = PathBuf::from(realm_path);
    validate_realm_path(&realm_path)?;

    let mut data = realm_parser::parse_realm_file(&realm_path)?; // 直接读取 Realm 快照，不再启动固定 SchemaVersion 的外部提取器。
    normalize_extracted_data(&mut data);

    let result = LoadResult {
        source_path: data.source_path.clone(),
        generated_at: data.generated_at.clone(),
        collection_count: data.collections.len(),
    };
    Ok((data, result))
}

/**
 * 按当前模式生成收藏夹汇总，并隐藏该模式下完全没有条目的收藏夹。
 */
pub fn collection_summaries(data: &ExtractedData, mode: &str) -> Vec<CollectionSummary> {
    let mode = normalize_mode(mode); // 命令边界统一兼容 All、fruits、catch 和 ctb 等历史名称。
    data.collections
        .iter()
        .filter_map(|collection| {
            let current_mode_count = collection
                .items
                .iter()
                .filter(|item| item.matches_mode(&mode))
                .count();
            if current_mode_count == 0 {
                return None;
            }

            Some(CollectionSummary {
                id: collection.id.clone(),
                name: collection.name.clone(),
                last_modified: collection.last_modified.clone(),
                total_count: collection.items.len(),
                current_mode_count,
                missing_count: collection.items.iter().filter(|item| item.missing).count(),
            })
        })
        .collect()
}

/**
 * 从 Rust 内存状态中筛选、排序、分组并按组分页返回谱面。
 */
pub fn query_beatmaps(data: &ExtractedData, query: &BeatmapQuery) -> Result<BeatmapPage, String> {
    let collection = data
        .collections
        .iter()
        .find(|item| item.id == query.collection_id)
        .ok_or_else(|| "未找到选中的收藏夹。".to_string())?;

    let mode = normalize_mode(&query.mode); // 非法模式按设置契约回退到 osu，避免返回难以解释的空列表。
    let mut items: Vec<&BeatmapEntry> = collection
        .items
        .iter()
        .filter(|item| item.matches_mode(&mode))
        .collect();
    sort_beatmap_refs(&mut items, query.sort_column.as_deref(), query.descending);

    let beatmap_total = items.len();
    let groups = group_beatmap_refs(items); // 排序完成后再分组，使组位置和组内顺序都遵循当前排序。
    let group_total = groups.len();
    let page_size = normalize_page_size(query.page_size);
    let page = query.page.max(1);
    let start = page
        .saturating_sub(1)
        .saturating_mul(page_size)
        .min(group_total);
    let end = start.saturating_add(page_size).min(group_total);
    Ok(BeatmapPage {
        beatmap_total,
        group_total,
        groups: groups[start..end].to_vec(),
    })
}

/**
 * 按名称（原语言）、艺术家（原语言）和谱师的去首尾空格文本建立稳定分组。
 */
fn group_beatmap_refs(items: Vec<&BeatmapEntry>) -> Vec<BeatmapGroup> {
    let mut group_indexes = std::collections::HashMap::<String, usize>::new();
    let mut groups = Vec::<BeatmapGroup>::new();
    for item in items {
        let key = beatmap_group_key(item);
        if let Some(index) = group_indexes.get(&key).copied() {
            groups[index].items.push((*item).clone()); // 分组结果持有独立谱面数据，避免误克隆借用引用。
            continue;
        }

        group_indexes.insert(key.clone(), groups.len());
        groups.push(BeatmapGroup {
            key,
            items: vec![(*item).clone()], // 首个成员同样复制实体，保持分组结构不依赖源集合借用。
        });
    }
    groups
}

/**
 * 使用长度前缀拼接分组字段，避免字段内容包含分隔符时产生键冲突。
 */
fn beatmap_group_key(item: &BeatmapEntry) -> String {
    let name = item.display_name_original();
    let artist = item.field_text("artistOriginal");
    let fields = [name.trim(), artist.trim(), item.mapper.trim()]; // 分组区分大小写，仅忽略字段首尾空格。
    fields
        .iter()
        .map(|field| format!("{}:{field}", field.len()))
        .collect::<Vec<_>>()
        .join("")
}

/**
 * 将分页大小限制在界面公开的五个选项，非法输入统一回退为二十。
 */
fn normalize_page_size(page_size: usize) -> usize {
    match page_size {
        10 | 20 | 50 | 100 | 200 => page_size,
        _ => 20,
    }
}

/**
 * 返回应用设置；文件缺失或内容损坏时使用安全默认值。
 */
pub fn load_settings(app: &AppHandle) -> AppSettings {
    let mut settings = settings_path(app)
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|json| serde_json::from_str::<AppSettings>(&json).ok())
        .map(normalize_settings)
        .unwrap_or_default();
    let stored_path = PathBuf::from(settings.realm_path.trim());
    if validate_realm_path(&stored_path).is_err() {
        settings.realm_path = find_default_realm_path().unwrap_or_default(); // 已保存路径失效时回退到程序旁的 client.realm。
    };
    settings
}

/**
 * 将列配置、数据库路径和当前模式写入应用数据目录。
 */
pub fn save_settings(app: &AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    let _write_guard = SETTINGS_WRITE_LOCK
        .lock()
        .map_err(|_| "设置文件写入锁已损坏。".to_string())?;
    let settings = normalize_settings(settings);
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(error_text("无法创建设置目录"))?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(error_text("无法序列化应用设置"))?;
    let temporary = unique_temporary_path(&path); // 每次保存使用独立临时文件，防止并发请求互相截断。
    if let Err(error) = fs::write(&temporary, json) {
        let _ = fs::remove_file(&temporary); // 写入失败时清理可能残留的半成品设置文件。
        return Err(format!("无法写入临时设置文件：{error}"));
    }
    replace_file(&temporary, &path)?;
    Ok(settings)
}

/**
 * 从缓存或 osu! CDN 获取封面，并转换为前端可直接使用的数据地址。
 */
pub async fn get_cover(app: &AppHandle, beatmap_set_id: i64) -> Result<Option<CoverPayload>, String> {
    if beatmap_set_id <= 0 {
        return Ok(None);
    }

    let cover_dir = app_runtime_dir(app)?.join("covers");
    fs::create_dir_all(&cover_dir).map_err(error_text("无法创建封面缓存目录"))?;
    let target = cover_dir.join(format!("{beatmap_set_id}.jpg"));
    if let Ok(bytes) = fs::read(&target) {
        if let Some(mime) = detect_image_mime(&bytes) {
            return Ok(Some(cover_payload(&bytes, mime)));
        }
        let _ = fs::remove_file(&target); // 删除损坏缓存，允许本次重新下载。
    }

    let client = reqwest::Client::builder()
        .user_agent("osulazer-collection-view")
        .build()
        .map_err(error_text("无法创建封面下载客户端"))?;
    let urls = [
        format!("https://assets.ppy.sh/beatmaps/{beatmap_set_id}/covers/raw.jpg"),
        format!("https://assets.ppy.sh/beatmaps/{beatmap_set_id}/covers/cover@2x.jpg"),
        format!("https://assets.ppy.sh/beatmaps/{beatmap_set_id}/covers/cover.jpg"),
    ];

    for url in urls {
        let Ok(response) = client.get(url).timeout(std::time::Duration::from_secs(12)).send().await else {
            continue;
        };
        if !response.status().is_success() {
            continue;
        }
        let Ok(bytes) = response.bytes().await else {
            continue;
        };
        let Some(mime) = detect_image_mime(&bytes) else {
            continue;
        };

        let temporary = unique_temporary_path(&target); // 同一 SID 并发下载时使用独立临时文件，避免互相覆盖。
        if let Err(error) = fs::write(&temporary, &bytes) {
            let _ = fs::remove_file(&temporary); // 下载内容写入失败时删除可能残留的半成品缓存。
            return Err(format!("无法写入封面临时文件：{error}"));
        }
        if let Ok(existing) = fs::read(&target) {
            if let Some(existing_mime) = detect_image_mime(&existing) {
                let _ = fs::remove_file(&temporary); // 其他请求已提交有效缓存时直接复用该结果。
                return Ok(Some(cover_payload(&existing, existing_mime)));
            }
            let _ = fs::remove_file(&target); // 并发请求留下损坏目标时，本次下载负责重新提交。
        }
        replace_file(&temporary, &target)?;
        return Ok(Some(cover_payload(&bytes, mime)));
    }

    Ok(None)
}

/**
 * 对 Realm 解析结果执行模式归一化，确保所有下游模块共享同一数据契约。
 */
fn normalize_extracted_data(data: &mut ExtractedData) {
    for collection in &mut data.collections {
        for item in &mut collection.items {
            item.normalize();
        }
    }
}

/**
 * 归一化设置内容，补齐新增列并过滤已经删除的无效列。
 */
fn normalize_settings(mut settings: AppSettings) -> AppSettings {
    let defaults = default_columns();
    let defaults_by_key: std::collections::HashMap<String, crate::models::ColumnConfig> = defaults
        .iter()
        .cloned()
        .map(|item| (item.key.clone(), item))
        .collect(); // 默认列同时作为合法键和固定标签的可信来源。
    let mut seen = std::collections::HashSet::new();
    settings.columns.retain_mut(|item| {
        let Some(default) = defaults_by_key.get(&item.key) else {
            return false;
        };
        if !seen.insert(item.key.clone()) {
            return false;
        }
        item.label = default.label.clone(); // 忽略持久化文件中可能被篡改的列标签。
        true
    });
    for default in defaults {
        if !seen.contains(&default.key) {
            settings.columns.push(default);
        }
    }
    if !settings.columns.iter().any(|item| item.visible) {
        if let Some(first) = settings.columns.first_mut() {
            first.visible = true;
        }
    }

    settings.realm_path = settings.realm_path.trim().to_string(); // 去除复制路径时可能带入的首尾空白。
    settings.selected_mode = normalize_mode(&settings.selected_mode);
    settings.page_size = normalize_page_size(settings.page_size); // 旧设置中的合法分页值继续保留。
    settings
}

/**
 * 将历史模式名统一为当前应用使用的筛选键。
 */
pub fn normalize_mode(mode: &str) -> String {
    match mode.trim().to_ascii_lowercase().as_str() {
        "all" => "all",
        "osu" => "osu",
        "taiko" => "taiko",
        "fruits" | "catch" | "ctb" => "ctb",
        "mania" => "mania",
        "missing" => "missing",
        _ => "osu",
    }
    .to_string()
}

/**
 * 对谱面引用执行默认反序或指定列排序，并始终把空值放在末尾。
 */
pub fn sort_beatmap_refs(
    items: &mut Vec<&BeatmapEntry>,
    sort_column: Option<&str>,
    descending: bool,
) {
    let Some(column) = sort_column
        .map(str::trim)
        .filter(|value| !value.is_empty() && is_supported_sort_column(value))
    else {
        items.reverse(); // 保持 Python 版默认将收藏夹原始顺序反向显示的行为。
        return;
    };

    items.sort_by(|left, right| compare_items(left, right, column, descending));
}

/**
 * 判断排序字段是否属于 Rust 已实现且前端允许展示的数据列。
 */
fn is_supported_sort_column(column: &str) -> bool {
    matches!(
        column,
        "nameOriginal"
            | "name"
            | "title"
            | "titleUnicode"
            | "artist"
            | "artistUnicode"
            | "artistOriginal"
            | "beatmapId"
            | "beatmapSetId"
            | "starRating"
            | "circleSize"
            | "overallDifficulty"
            | "approachRate"
            | "drainRate"
            | "totalObjectCount"
            | "lengthMs"
            | "bpm"
            | "statusInt"
            | "difficultyName"
            | "mapper"
            | "mode"
            | "rulesetShortName"
            | "rulesetName"
            | "backgroundUrl"
            | "md5"
            | "missing"
    )
}

/**
 * 比较两个谱面在指定列上的值，数字列使用数值比较，文本列忽略大小写。
 */
fn compare_items(
    left: &BeatmapEntry,
    right: &BeatmapEntry,
    column: &str,
    descending: bool,
) -> Ordering {
    let left_value = sort_value(left, column);
    let right_value = sort_value(right, column);
    match (left_value, right_value) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(left), Some(right)) => {
            let ordering = match (left, right) {
                (SortValue::Number(left), SortValue::Number(right)) => {
                    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
                }
                (SortValue::Text(left), SortValue::Text(right)) => left.cmp(&right),
                _ => Ordering::Equal,
            };
            if descending { ordering.reverse() } else { ordering }
        }
    }
}

/**
 * 提取排序值；无效数字和空文本返回 None。
 */
fn sort_value(item: &BeatmapEntry, column: &str) -> Option<SortValue> {
    let number = match column {
        "starRating" => item.star_rating,
        "beatmapId" => item.beatmap_id.map(|value| value as f64),
        "beatmapSetId" => item.beatmap_set_id.map(|value| value as f64),
        "circleSize" => item.circle_size,
        "overallDifficulty" => item.overall_difficulty,
        "approachRate" => item.approach_rate,
        "drainRate" => item.drain_rate,
        "totalObjectCount" => item.total_object_count.map(|value| value as f64),
        "lengthMs" => item.length_ms,
        "bpm" => item.bpm,
        "statusInt" => item.status_int.map(|value| value as f64),
        _ => None,
    };
    if number.is_some() {
        return number
            .filter(|value| value.is_finite())
            .map(SortValue::Number);
    }

    let text = match column {
        "nameOriginal" => item.display_name_original(),
        "name" => item.display_name(),
        "title" => item.title.clone(),
        "titleUnicode" => item.title_unicode.clone(),
        "artist" => item.artist.clone(),
        "artistUnicode" | "artistOriginal" => item.field_text("artistOriginal"),
        "difficultyName" => item.difficulty_name.clone(),
        "mapper" => item.mapper.clone(),
        "mode" | "rulesetShortName" => item.mode.clone(),
        "rulesetName" => item.ruleset_name.clone(),
        "backgroundUrl" => item.background_url.clone(),
        "md5" => item.md5.clone(),
        "missing" => item.missing.to_string(),
        _ => String::new(),
    };
    let normalized = text.trim().to_lowercase();
    (!normalized.is_empty()).then_some(SortValue::Text(normalized))
}

enum SortValue {
    Number(f64),
    Text(String),
}

/**
 * 校验数据库文件存在、是普通文件且扩展名为 realm。
 */
fn validate_realm_path(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("Realm 文件不存在：{}", path.display()));
    }
    let is_realm = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("realm"));
    if !is_realm {
        return Err("请选择扩展名为 .realm 的数据库文件。".to_string());
    }
    Ok(())
}

/**
 * 返回应用运行时数据目录并确保目录存在。
 */
pub fn app_runtime_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(error_text("无法定位应用数据目录"))?
        .join("runtime");
    fs::create_dir_all(&path).map_err(error_text("无法创建应用运行时目录"))?;
    Ok(path)
}

/**
 * 返回设置文件的固定路径。
 */
fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_runtime_dir(app)?.join("ui_settings.json"))
}

/**
 * 查找与 Python 版一致的默认 client.realm，优先使用应用可执行文件所在目录。
 */
fn find_default_realm_path() -> Option<String> {
    let mut candidates = Vec::new();
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            candidates.push(parent.join("client.realm"));
        }
    }
    #[cfg(debug_assertions)]
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../client.realm"),
    ); // 仅开发版兼容把 client.realm 放在前端项目根目录。
    candidates
        .into_iter()
        .find(|path| validate_realm_path(path).is_ok())
        .map(|path| path.to_string_lossy().into_owned())
}

/**
 * 为并发写缓存和导出生成当前进程内唯一的同目录临时路径。
 */
fn unique_temporary_path(target: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let sequence = TEMPORARY_FILE_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed);
    let extension = target
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("tmp");
    target.with_extension(format!(
        "{extension}.{}.{}.{}.tmp",
        std::process::id(),
        timestamp,
        sequence
    ))
}

/**
 * 使用临时文件替换目标文件，避免设置和图片出现半写入状态。
 */
fn replace_file(temporary: &Path, target: &Path) -> Result<(), String> {
    let _replace_guard = match FILE_REPLACE_LOCK.lock() {
        Ok(guard) => guard,
        Err(_) => {
            let _ = fs::remove_file(temporary); // 替换锁异常时清理调用方已经写好的临时文件。
            return Err("文件替换锁已损坏。".to_string());
        }
    };
    let result = (|| -> Result<(), String> {
        if target.exists() {
            fs::remove_file(target).map_err(error_text("无法替换已有文件"))?;
        }
        fs::rename(temporary, target).map_err(error_text("无法提交临时文件"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary); // 提交失败时清理未使用的唯一临时文件。
    }
    result
}

/**
 * 根据常见图片文件头判断缓存内容是否可用。
 */
fn detect_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(&b"WEBP"[..]) {
        Some("image/webp")
    } else {
        None
    }
}

/**
 * 将图片字节编码成 WebView 可直接渲染的数据地址。
 */
fn cover_payload(bytes: &[u8], mime: &str) -> CoverPayload {
    CoverPayload {
        data_url: format!("data:{mime};base64,{}", STANDARD.encode(bytes)),
    }
}

/**
 * 将标准错误转换为带中文上下文的字符串错误。
 */
fn error_text<E: std::fmt::Display>(context: &'static str) -> impl FnOnce(E) -> String {
    move |error| format!("{context}：{error}")
}
