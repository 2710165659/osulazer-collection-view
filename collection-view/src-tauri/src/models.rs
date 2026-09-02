use serde::{Deserialize, Serialize};

/**
 * 定义 Rust Realm 解析器输出的顶层数据结构。
 */
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExtractedData {
    pub source_path: String,
    pub generated_at: String,
    pub collections: Vec<CollectionInfo>,
}

/**
 * 定义单个收藏夹及其包含的谱面。
 */
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CollectionInfo {
    pub id: String,
    pub name: String,
    pub last_modified: String,
    pub items: Vec<BeatmapEntry>,
}

/**
 * 定义前后端共用的谱面数据，并为可能为空的 Realm 字段使用 Option。
 */
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BeatmapEntry {
    pub md5: String,
    pub title: String,
    pub title_unicode: String,
    pub artist: String,
    pub artist_unicode: String,
    pub beatmap_id: Option<i64>,
    pub beatmap_set_id: Option<i64>,
    pub star_rating: Option<f64>,
    pub circle_size: Option<f64>,
    pub overall_difficulty: Option<f64>,
    pub approach_rate: Option<f64>,
    pub drain_rate: Option<f64>,
    pub total_object_count: Option<i64>,
    pub length_ms: Option<f64>,
    pub bpm: Option<f64>,
    pub status_int: Option<i32>,
    pub difficulty_name: String,
    pub mapper: String,
    pub ruleset_short_name: String,
    pub ruleset_name: String,
    pub background_url: String,
    pub missing: bool,
    pub mode: String,
}

impl BeatmapEntry {
    /**
     * 将数据库的 fruits 模式统一归一为界面使用的 ctb，并标记缺失项模式。
     */
    pub fn normalize(&mut self) {
        self.mode = if self.missing {
            "missing".to_string()
        } else {
            match self.ruleset_short_name.trim().to_ascii_lowercase().as_str() {
                "fruits" | "catch" | "ctb" => "ctb".to_string(), // osu!lazer 数据库存储名称为 fruits。
                "osu" => "osu".to_string(),
                "taiko" => "taiko".to_string(),
                "mania" => "mania".to_string(),
                value if value.is_empty() => "unknown".to_string(),
                value => value.to_string(),
            }
        };

        if !self.missing && self.mode != "unknown" {
            self.ruleset_short_name = self.mode.clone(); // 前端和导出统一使用归一后的短名称。
        }
    }

    /**
     * 判断谱面是否属于指定筛选模式，all 与 missing 使用独立语义。
     */
    pub fn matches_mode(&self, mode: &str) -> bool {
        match mode {
            "all" => true,
            "missing" => self.missing,
            value => !self.missing && self.mode == value,
        }
    }

    /**
     * 生成英文标题与艺术家组合后的普通名称。
     */
    pub fn display_name(&self) -> String {
        if self.missing {
            return format!("[Missing] {}", self.md5);
        }

        join_non_empty(&self.artist, &self.title).unwrap_or_else(|| self.md5.clone())
    }

    /**
     * 优先使用 Unicode 标题和艺术家生成界面主名称。
     */
    pub fn display_name_original(&self) -> String {
        if self.missing {
            return format!("[Missing] {}", self.md5);
        }

        let artist = if self.artist_unicode.is_empty() {
            &self.artist
        } else {
            &self.artist_unicode
        };
        let title = if self.title_unicode.is_empty() {
            &self.title
        } else {
            &self.title_unicode
        };
        join_non_empty(artist, title).unwrap_or_else(|| self.display_name())
    }

    /**
     * 按表格列键生成统一的展示文本，供界面详情和 Rust 导出共同使用。
     */
    pub fn field_text(&self, key: &str) -> String {
        match key {
            "nameOriginal" => self.display_name_original(),
            "name" => self.display_name(),
            "title" => self.title.clone(),
            "titleUnicode" => self.title_unicode.clone(),
            "artist" => self.artist.clone(),
            "artistUnicode" | "artistOriginal" => {
                if self.artist_unicode.is_empty() {
                    self.artist.clone()
                } else {
                    self.artist_unicode.clone()
                }
            }
            "beatmapId" => option_to_string(self.beatmap_id),
            "beatmapSetId" => option_to_string(self.beatmap_set_id),
            "starRating" => self
                .star_rating
                .filter(|value| value.is_finite() && *value >= 0.0)
                .map(|value| format!("{value:.2}"))
                .unwrap_or_default(),
            "circleSize" => format_float(self.circle_size),
            "overallDifficulty" => format_float(self.overall_difficulty),
            "approachRate" => format_float(self.approach_rate),
            "drainRate" => format_float(self.drain_rate),
            "totalObjectCount" => self
                .total_object_count
                .filter(|value| *value >= 0)
                .map(|value| value.to_string())
                .unwrap_or_default(),
            "lengthMs" => format_length(self.length_ms),
            "bpm" => format_float(self.bpm),
            "statusInt" => status_text(self.status_int),
            "difficultyName" => self.difficulty_name.clone(),
            "mapper" => self.mapper.clone(),
            "mode" => self.mode.clone(),
            "rulesetShortName" => self.ruleset_short_name.clone(),
            "rulesetName" => self.ruleset_name.clone(),
            "backgroundUrl" => self.background_url.clone(),
            "md5" => self.md5.clone(),
            "missing" => if self.missing { "是" } else { "否" }.to_string(),
            _ => String::new(),
        }
    }
}

/**
 * 定义左侧收藏夹表格使用的轻量汇总数据。
 */
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionSummary {
    pub id: String,
    pub name: String,
    pub last_modified: String,
    pub total_count: usize,
    pub current_mode_count: usize,
    pub missing_count: usize,
}

/**
 * 定义谱面分页、模式筛选和排序请求。
 */
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BeatmapQuery {
    pub collection_id: String,
    pub mode: String,
    pub sort_column: Option<String>,
    pub descending: bool,
    pub page: usize,
    pub page_size: usize,
}

/**
 * 定义一个按名称、原语言艺术家和谱师聚合的谱面组。
 */
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatmapGroup {
    pub key: String,
    pub items: Vec<BeatmapEntry>,
}

/**
 * 定义按谱面组分页后的查询结果，并同时保留原始谱面总数。
 */
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatmapPage {
    pub beatmap_total: usize,
    pub group_total: usize,
    pub groups: Vec<BeatmapGroup>,
}

/**
 * 定义数据库加载后的基础信息。
 */
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadResult {
    pub source_path: String,
    pub generated_at: String,
    pub collection_count: usize,
}

/**
 * 定义可持久化的谱面列配置。
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnConfig {
    pub key: String,
    pub label: String,
    pub visible: bool,
}

/**
 * 定义 Rust 负责落盘的应用设置。
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub realm_path: String,
    pub selected_mode: String,
    pub columns: Vec<ColumnConfig>,
    pub page_size: usize,
}

impl Default for AppSettings {
    /**
     * 返回与 Python 版本主要显示列一致的默认设置。
     */
    fn default() -> Self {
        Self {
            realm_path: String::new(),
            selected_mode: "osu".to_string(),
            columns: default_columns(),
            page_size: 20, // 新用户默认每页展示二十个谱面组。
        }
    }
}

/**
 * 定义 Rust 封面缓存返回给 WebView 的数据地址。
 */
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverPayload {
    pub data_url: String,
}

/**
 * 定义导出当前收藏夹的请求参数。
 */
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCurrentRequest {
    pub output_path: String,
    pub collection_id: String,
    pub mode: String,
    pub columns: Vec<ColumnConfig>,
    pub sort_column: Option<String>,
    pub descending: bool,
}

/**
 * 定义导出全部模式压缩包的请求参数。
 */
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportAllRequest {
    pub output_path: String,
    pub columns: Vec<ColumnConfig>,
    pub sort_column: Option<String>, // 全模式导出的明细表沿用当前谱面列表排序列。
    pub descending: bool, // 全模式导出的明细表沿用当前谱面列表升降序。
}

/**
 * 创建应用默认列，隐藏列仍保留顺序以便用户随时恢复。
 */
pub fn default_columns() -> Vec<ColumnConfig> {
    vec![
        column("nameOriginal", "名称（原语言）", true),
        column("starRating", "难度", true),
        column("beatmapId", "BID", true),
        column("artistOriginal", "艺术家（原语言）", true),
        column("difficultyName", "难度名", true),
        column("mapper", "谱师", true),
        column("mode", "模式", true),
        column("beatmapSetId", "SID", false),
        column("circleSize", "CS", false),
        column("overallDifficulty", "OD", false),
        column("approachRate", "AR", false),
        column("drainRate", "HP", false),
        column("totalObjectCount", "Note数", false),
        column("lengthMs", "长度", false),
        column("bpm", "BPM", false),
        column("statusInt", "状态", false),
        column("name", "名称", false),
        column("title", "歌曲名称", false),
        column("titleUnicode", "歌曲名称（Unicode）", false),
        column("artist", "艺术家", false),
        column("artistUnicode", "艺术家（Unicode）", false),
        column("rulesetShortName", "模式简称", false),
        column("rulesetName", "模式名称", false),
        column("backgroundUrl", "封面图片 URL", false),
        column("md5", "MD5", false),
        column("missing", "是否缺失", false),
    ]
}

/**
 * 创建单个默认列配置，减少重复初始化代码。
 */
fn column(key: &str, label: &str, visible: bool) -> ColumnConfig {
    ColumnConfig {
        key: key.to_string(),
        label: label.to_string(),
        visible,
    }
}

/**
 * 将两个非空文本组合为“艺术家 - 标题”形式。
 */
fn join_non_empty(left: &str, right: &str) -> Option<String> {
    match (left.is_empty(), right.is_empty()) {
        (false, false) => Some(format!("{left} - {right}")),
        (false, true) => Some(left.to_string()),
        (true, false) => Some(right.to_string()),
        (true, true) => None,
    }
}

/**
 * 将可选整数转换为空值安全的展示文本。
 */
fn option_to_string<T: ToString>(value: Option<T>) -> String {
    value.map(|item| item.to_string()).unwrap_or_default()
}

/**
 * 使用 Python 版本相同的紧凑规则格式化浮点数。
 */
fn format_float(value: Option<f64>) -> String {
    let Some(value) = value.filter(|item| item.is_finite()) else {
        return String::new();
    };
    let formatted = format!("{value:.1}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/**
 * 将毫秒长度转换为分秒文本。
 */
fn format_length(value: Option<f64>) -> String {
    let Some(value) = value.filter(|item| item.is_finite() && *item >= 0.0) else {
        return String::new();
    };
    let total_seconds = (value / 1000.0).round() as i64;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

/**
 * 按 osu!lazer Realm 中的真实状态码映射生成状态文本。
 */
fn status_text(value: Option<i32>) -> String {
    match value {
        Some(-2) => "graveyard".to_string(),
        Some(-1) => "wip".to_string(),
        Some(0) => "pending".to_string(),
        Some(1) => "ranked".to_string(),
        Some(2) => "approved".to_string(),
        Some(3) => "qualified".to_string(),
        Some(4) => "loved".to_string(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}
