use std::{
    collections::HashSet,
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::{
    models::{
        default_columns, BeatmapEntry, CollectionInfo, ColumnConfig, ExportAllRequest,
        ExportCurrentRequest, ExtractedData,
    },
    services::{normalize_mode, sort_beatmap_refs},
};

static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0); // 为同一纳秒内的并发导出补充进程内唯一序号。
static EXPORT_WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(()); // 串行替换最终导出文件，避免同一路径并发提交互相删除。

const MODE_EXPORT_ORDER: [&str; 4] = ["osu", "taiko", "ctb", "mania"];

/**
 * 将当前收藏夹、当前模式和当前排序结果导出为单个 Excel 文件。
 */
pub fn export_current(data: &ExtractedData, request: &ExportCurrentRequest) -> Result<(), String> {
    let columns = visible_columns(&request.columns)?;
    let collection = data
        .collections
        .iter()
        .find(|item| item.id == request.collection_id)
        .ok_or_else(|| "未找到要导出的收藏夹。".to_string())?;
    let mode = normalize_mode(&request.mode); // 导出与列表查询共用同一模式名归一化契约。
    let mut items: Vec<&BeatmapEntry> = collection
        .items
        .iter()
        .filter(|item| item.matches_mode(&mode))
        .collect();
    sort_beatmap_refs(
        &mut items,
        request.sort_column.as_deref(),
        request.descending,
    );
    if items.is_empty() {
        return Err("当前列表没有可导出的谱面。".to_string());
    }

    let sheet = build_beatmap_sheet(collection.name.as_str(), &columns, &items);
    let workbook = build_xlsx(vec![sheet])?;
    write_atomic(Path::new(&request.output_path), &workbook)
}

/**
 * 将四个游戏模式汇总表和各收藏夹明细表打包为一个 ZIP 文件。
 */
pub fn export_all_modes(data: &ExtractedData, request: &ExportAllRequest) -> Result<(), String> {
    let columns = visible_columns(&request.columns)?;
    let cursor = Cursor::new(Vec::new());
    let mut archive = ZipWriter::new(cursor);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut used_entry_names = HashSet::new(); // 记录 ZIP 内文件名，避免清洗后重名导致解压覆盖。

    for mode in MODE_EXPORT_ORDER {
        let workbook = build_mode_workbook(data, mode, &columns)?; // 保留原有模式汇总工作簿的行顺序。
        archive
            .start_file(unique_archive_filename(
                &format!("collections_{mode}"),
                "",
                &mut used_entry_names,
            ), options)
            .map_err(error_text("无法创建模式 Excel 压缩项"))?;
        archive
            .write_all(&workbook)
            .map_err(error_text("无法写入模式 Excel 压缩项"))?;

        for collection in &data.collections {
            let items = sorted_collection_items(
                collection,
                mode,
                request.sort_column.as_deref(),
                request.descending,
            );
            if items.is_empty() {
                continue; // 与“导出当前列表”一致，当前模式没有谱面的收藏夹不生成空文件。
            }

            let collection_workbook = build_collection_workbook(collection, &columns, &items)?;
            let entry_name = unique_archive_filename(
                mode,
                &collection.name,
                &mut used_entry_names,
            );
            archive
                .start_file(entry_name, options)
                .map_err(error_text("无法创建收藏夹 Excel 压缩项"))?;
            archive
                .write_all(&collection_workbook)
                .map_err(error_text("无法写入收藏夹 Excel 压缩项"))?;
        }
    }

    let bytes = archive
        .finish()
        .map_err(error_text("无法完成导出压缩包"))?
        .into_inner();
    write_atomic(Path::new(&request.output_path), &bytes)
}

/**
 * 创建单个模式的工作簿，包含 Summary 和每个非空收藏夹 Sheet。
 */
fn build_mode_workbook(
    data: &ExtractedData,
    mode: &str,
    columns: &[ColumnConfig],
) -> Result<Vec<u8>, String> {
    let mut sheets = Vec::new();
    let mut summary_rows = vec![vec![
        "Collection".to_string(),
        "Mode".to_string(),
        "Visible Items".to_string(),
        "Missing Items".to_string(),
        "Last Modified".to_string(),
    ]];

    for collection in &data.collections {
        let items: Vec<&BeatmapEntry> = collection
            .items
            .iter()
            .filter(|item| item.matches_mode(mode))
            .collect();
        if items.is_empty() {
            continue;
        }

        summary_rows.push(vec![
            collection.name.clone(),
            mode.to_string(),
            items.len().to_string(),
            items.iter().filter(|item| item.missing).count().to_string(),
            format_datetime_text(&collection.last_modified),
        ]);
        sheets.push(build_beatmap_sheet(&collection.name, columns, &items));
    }

    sheets.insert(
        0,
        SheetData {
            name: "Summary".to_string(),
            rows: summary_rows,
        },
    );
    build_xlsx(sheets)
}

/**
 * 按当前列表导出规则生成单个收藏夹的独立工作簿。
 */
fn build_collection_workbook(
    collection: &CollectionInfo,
    columns: &[ColumnConfig],
    items: &[&BeatmapEntry],
) -> Result<Vec<u8>, String> {
    let sheet = build_beatmap_sheet(collection.name.as_str(), columns, items);
    build_xlsx(vec![sheet])
}

/**
 * 筛选并排序一个收藏夹在指定模式下的谱面，复用当前列表和单收藏夹导出的顺序。
 */
fn sorted_collection_items<'a>(
    collection: &'a CollectionInfo,
    mode: &str,
    sort_column: Option<&str>,
    descending: bool,
) -> Vec<&'a BeatmapEntry> {
    let mut items: Vec<&BeatmapEntry> = collection
        .items
        .iter()
        .filter(|item| item.matches_mode(mode))
        .collect();
    sort_beatmap_refs(&mut items, sort_column, descending); // 无指定排序时保持当前列表的默认反向顺序。
    items
}

/**
 * 生成 ZIP 内独立收藏夹工作簿的唯一文件名。
 */
fn unique_archive_filename(
    mode: &str,
    collection_name: &str,
    used_names: &mut HashSet<String>,
) -> String {
    let base = if collection_name.is_empty() {
        mode.to_string()
    } else {
        format!("{mode}_{}", sanitize_archive_component(collection_name))
    };
    let mut candidate = format!("{base}.xlsx");
    let mut suffix_index = 1usize;
    while used_names.contains(&candidate.to_lowercase()) {
        let suffix = format!("_{suffix_index}");
        let truncated = truncate_chars(&base, 180usize.saturating_sub(suffix.len()));
        candidate = format!("{truncated}{suffix}.xlsx");
        suffix_index += 1;
    }
    used_names.insert(candidate.to_lowercase()); // ZIP 文件名按不区分大小写的方式去重，兼容 Windows 解压器。
    candidate
}

/**
 * 清洗收藏夹名称，使其适合作为 ZIP 内的 Windows 文件名片段。
 */
fn sanitize_archive_component(value: &str) -> String {
    let cleaned = sanitize_xml_text(value)
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect::<String>();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "collection".to_string()
    } else {
        truncate_chars(trimmed, 180)
    }
}

/**
 * 根据列配置生成一个收藏夹的表头和谱面数据行。
 */
fn build_beatmap_sheet(
    name: &str,
    columns: &[ColumnConfig],
    items: &[&BeatmapEntry],
) -> SheetData {
    let mut rows = vec![columns.iter().map(|column| column.label.clone()).collect()];
    rows.extend(items.iter().map(|item| {
        columns
            .iter()
            .map(|column| {
                let value = item.field_text(&column.key);
                if value.is_empty() { "-".to_string() } else { value } // 导出与界面一致地用短横线表示空字段。
            })
            .collect()
    }));
    SheetData {
        name: name.to_string(),
        rows,
    }
}

/**
 * 过滤可见列，并防止生成没有任何字段的无效工作簿。
 */
fn visible_columns(columns: &[ColumnConfig]) -> Result<Vec<ColumnConfig>, String> {
    let defaults = default_columns();
    let default_by_key = defaults
        .into_iter()
        .map(|column| (column.key.clone(), column))
        .collect::<std::collections::HashMap<_, _>>();
    let mut seen = HashSet::new();
    let visible: Vec<ColumnConfig> = columns
        .iter()
        .filter(|column| column.visible && seen.insert(column.key.clone()))
        .filter_map(|column| default_by_key.get(&column.key).cloned()) // 仅导出 Rust 白名单中的字段和固定标签。
        .collect();
    if visible.is_empty() {
        return Err("至少选择一列后才能导出。".to_string());
    }
    Ok(visible)
}

/**
 * 使用最小 Office Open XML 结构生成标准 XLSX 字节。
 */
fn build_xlsx(mut sheets: Vec<SheetData>) -> Result<Vec<u8>, String> {
    ensure_unique_sheet_names(&mut sheets);
    let cursor = Cursor::new(Vec::new());
    let mut archive = ZipWriter::new(cursor);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

    write_zip_text(&mut archive, "[Content_Types].xml", &content_types_xml(sheets.len()), options)?;
    write_zip_text(&mut archive, "_rels/.rels", package_relationships_xml(), options)?;
    write_zip_text(&mut archive, "xl/workbook.xml", &workbook_xml(&sheets), options)?;
    write_zip_text(
        &mut archive,
        "xl/_rels/workbook.xml.rels",
        &workbook_relationships_xml(sheets.len()),
        options,
    )?;
    write_zip_text(&mut archive, "xl/styles.xml", styles_xml(), options)?;

    for (index, sheet) in sheets.iter().enumerate() {
        write_zip_text(
            &mut archive,
            &format!("xl/worksheets/sheet{}.xml", index + 1),
            &worksheet_xml(sheet),
            options,
        )?;
    }

    Ok(archive
        .finish()
        .map_err(error_text("无法完成 XLSX 文件"))?
        .into_inner())
}

/**
 * 写入 ZIP 内的文本文件，并统一补充错误上下文。
 */
fn write_zip_text(
    archive: &mut ZipWriter<Cursor<Vec<u8>>>,
    name: &str,
    content: &str,
    options: FileOptions,
) -> Result<(), String> {
    archive
        .start_file(name, options)
        .map_err(error_text("无法创建 XLSX 内部文件"))?;
    archive
        .write_all(content.as_bytes())
        .map_err(error_text("无法写入 XLSX 内部文件"))
}

/**
 * 生成工作簿内容类型声明。
 */
fn content_types_xml(sheet_count: usize) -> String {
    let worksheets = (1..=sheet_count)
        .map(|index| format!(
            r#"<Override PartName="/xl/worksheets/sheet{index}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#
        ))
        .collect::<String>();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>{worksheets}</Types>"#
    )
}

/**
 * 生成 XLSX 包根关系文件。
 */
fn package_relationships_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#
}

/**
 * 生成工作簿及 Sheet 名称声明。
 */
fn workbook_xml(sheets: &[SheetData]) -> String {
    let sheet_nodes = sheets
        .iter()
        .enumerate()
        .map(|(index, sheet)| format!(
            r#"<sheet name="{}" sheetId="{}" r:id="rId{}"/>"#,
            escape_xml_attribute(&sheet.name),
            index + 1,
            index + 1
        ))
        .collect::<String>();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>{sheet_nodes}</sheets></workbook>"#
    )
}

/**
 * 生成工作簿到各工作表和样式文件的关系声明。
 */
fn workbook_relationships_xml(sheet_count: usize) -> String {
    let sheet_relations = (1..=sheet_count)
        .map(|index| format!(
            r#"<Relationship Id="rId{index}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{index}.xml"/>"#
        ))
        .collect::<String>();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{sheet_relations}<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#,
        sheet_count + 1
    )
}

/**
 * 生成默认单元格样式和加粗居中的表头样式。
 */
fn styles_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyAlignment="1"><alignment horizontal="center"/></xf></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>"#
}

/**
 * 将二维文本数据转换为工作表 XML，并按内容估算列宽。
 */
fn worksheet_xml(sheet: &SheetData) -> String {
    let column_count = sheet.rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths = (0..column_count)
        .map(|column| {
            sheet
                .rows
                .iter()
                .map(|row| row.get(column).map(|value| value.chars().count()).unwrap_or(0))
                .max()
                .unwrap_or(0)
                .saturating_add(2)
                .clamp(10, 40)
        })
        .collect::<Vec<_>>();
    let column_xml = widths
        .iter()
        .enumerate()
        .map(|(index, width)| format!(
            r#"<col min="{}" max="{}" width="{}" customWidth="1"/>"#,
            index + 1,
            index + 1,
            width
        ))
        .collect::<String>();

    let rows_xml = sheet
        .rows
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            let cells = row
                .iter()
                .enumerate()
                .map(|(column_index, value)| {
                    let reference = format!("{}{}", column_name(column_index + 1), row_index + 1);
                    let style = if row_index == 0 { r#" s="1""# } else { "" };
                    let sanitized = sanitize_xml_text(value); // 清除 XML 非法字符并限制 Excel 单元格最大长度。
                    format!(
                        r#"<c r="{reference}" t="inlineStr"{style}><is><t xml:space="preserve">{}</t></is></c>"#,
                        escape_xml_text(&sanitized)
                    )
                })
                .collect::<String>();
            format!(r#"<row r="{}">{cells}</row>"#, row_index + 1)
        })
        .collect::<String>();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cols>{column_xml}</cols><sheetData>{rows_xml}</sheetData></worksheet>"#
    )
}

/**
 * 将从一开始的列序号转换为 Excel 列名。
 */
fn column_name(mut index: usize) -> String {
    let mut name = String::new();
    while index > 0 {
        let remainder = (index - 1) % 26;
        name.insert(0, (b'A' + remainder as u8) as char);
        index = (index - 1) / 26;
    }
    name
}

/**
 * 清洗所有 Sheet 名并保证工作簿内不重名。
 */
fn ensure_unique_sheet_names(sheets: &mut [SheetData]) {
    let mut used = HashSet::new();
    for (index, sheet) in sheets.iter_mut().enumerate() {
        let base = sanitize_sheet_name(&sheet.name, &format!("Collection{}", index + 1));
        let mut candidate = base.clone();
        let mut suffix_index = 1;
        while used.contains(&candidate.to_lowercase()) {
            let suffix = format!("_{suffix_index}");
            let prefix_length = 31usize.saturating_sub(suffix.len());
            candidate = format!("{}{}", truncate_chars(&base, prefix_length), suffix);
            suffix_index += 1;
        }
        used.insert(candidate.to_lowercase());
        sheet.name = candidate;
    }
}

/**
 * 按 Excel 规则移除 Sheet 名非法字符并限制为 31 个字符。
 */
fn sanitize_sheet_name(name: &str, fallback: &str) -> String {
    let xml_safe = sanitize_xml_text(name); // Sheet 名同样必须满足 XML 1.0 字符约束。
    let cleaned = xml_safe
        .chars()
        .map(|character| match character {
            '/' | '\\' | '*' | '?' | '[' | ']' | ':' => ' ',
            value => value,
        })
        .collect::<String>();
    let cleaned = cleaned.trim().trim_matches('\'').trim(); // Excel 不允许 Sheet 名以单引号开头或结尾。
    let value = if cleaned.is_empty() { fallback } else { cleaned };
    truncate_chars(value, 31)
}

/**
 * 按 Unicode 字符数量安全截断字符串。
 */
fn truncate_chars(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}

/**
 * 将 Realm ISO 时间压缩为导出使用的“日期 时间”文本。
 */
fn format_datetime_text(value: &str) -> String {
    let value = value.trim();
    let bytes = value.as_bytes();
    let digit_positions = [0usize, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18];
    if value.len() >= 19
        && digit_positions
            .iter()
            .all(|index| bytes.get(*index).is_some_and(|byte| byte.is_ascii_digit()))
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && matches!(bytes.get(10), Some(b'T' | b' '))
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':')
    {
        return format!("{} {}", &value[..10], &value[11..19]);
    }
    value.to_string()
}

/**
 * 移除 XML 1.0 不允许的控制字符，并按 Excel 单元格上限截断文本。
 */
fn sanitize_xml_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            matches!(
                *character,
                '\u{0009}' | '\u{000A}' | '\u{000D}'
                    | '\u{0020}'..='\u{D7FF}'
                    | '\u{E000}'..='\u{FFFD}'
                    | '\u{10000}'..='\u{10FFFF}'
            )
        })
        .take(32_767) // Excel 单个单元格最多保存 32767 个文本字符。
        .collect()
}

/**
 * 转义 XML 文本节点中的特殊字符。
 */
fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/**
 * 转义 XML 属性中的特殊字符。
 */
fn escape_xml_attribute(value: &str) -> String {
    escape_xml_text(value)
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/**
 * 使用同目录临时文件完成原子写入，降低导出中断导致文件损坏的概率。
 */
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("导出路径不能为空。".to_string());
    }
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(error_text("无法创建导出目录"))?;
    }
    let temporary = unique_temporary_path(path); // 相同目标的并发导出各自使用独立临时文件。
    if let Err(error) = fs::write(&temporary, bytes) {
        let _ = fs::remove_file(&temporary); // 写入失败时清理可能残留的半成品导出文件。
        return Err(format!("无法写入导出临时文件：{error}"));
    }
    let _write_guard = match EXPORT_WRITE_LOCK.lock() {
        Ok(guard) => guard,
        Err(_) => {
            let _ = fs::remove_file(&temporary); // 写锁异常时也不能遗留已经生成的导出临时文件。
            return Err("导出文件写入锁已损坏。".to_string());
        }
    };
    let result = (|| -> Result<(), String> {
        if path.exists() {
            fs::remove_file(path).map_err(error_text("无法覆盖已有导出文件"))?;
        }
        fs::rename(&temporary, path).map_err(error_text("无法完成导出文件写入"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary); // 导出提交失败时清理已写入但未采用的临时文件。
    }
    result
}

/**
 * 为每次导出生成位于目标目录内且当前进程中唯一的临时文件路径。
 */
fn unique_temporary_path(target: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let sequence = TEMPORARY_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let extension = target
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("export");
    target.with_extension(format!(
        "{extension}.{}.{}.{}.tmp",
        std::process::id(),
        timestamp,
        sequence
    ))
}

/**
 * 将标准错误转换为带中文上下文的字符串错误。
 */
fn error_text<E: std::fmt::Display>(context: &'static str) -> impl FnOnce(E) -> String {
    move |error| format!("{context}：{error}")
}

struct SheetData {
    name: String,
    rows: Vec<Vec<String>>,
}
