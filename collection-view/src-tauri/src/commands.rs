use std::sync::{
    atomic::Ordering,
    Arc,
};

use tauri::{AppHandle, State};

use crate::{
    exporter,
    models::{
        AppSettings, BeatmapPage, BeatmapQuery, CollectionSummary, ColumnConfig, CoverPayload,
        ExportAllRequest, ExportCurrentRequest, LoadResult,
    },
    services,
    state::AppState,
};

/**
 * 调用 Rust Realm 解析器并把标准化数据保存到应用状态。
 */
#[tauri::command]
pub async fn load_database(
    app: AppHandle,
    state: State<'_, AppState>,
    realm_path: String,
) -> Result<LoadResult, String> {
    let generation = state.load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    {
        let mut guard = state
            .data
            .write()
            .map_err(|_| "数据库状态锁已损坏。".to_string())?;
        *guard = None; // 新加载开始后立即撤销旧数据库，失败时前后端都保持未加载状态。
    }
    let worker_app = app.clone();
    let loaded = tauri::async_runtime::spawn_blocking(move || {
        services::load_database(&worker_app, &realm_path)
    })
    .await
    .map_err(|error| format!("Realm 加载任务异常结束：{error}"))??;

    let (data, result) = loaded;
    if state.load_generation.load(Ordering::SeqCst) != generation {
        return Err("本次数据库加载已被更新的请求替代。".to_string());
    }
    let mut guard = state
        .data
        .write()
        .map_err(|_| "数据库状态锁已损坏。".to_string())?;
    *guard = Some(Arc::new(data)); // 大型数据库由所有查询和导出任务共享只读引用。
    Ok(result)
}

/**
 * 清空当前 Rust 数据库状态，并使仍在执行的旧加载请求失去提交资格。
 */
#[tauri::command]
pub fn clear_database(state: State<'_, AppState>) -> Result<(), String> {
    state.load_generation.fetch_add(1, Ordering::SeqCst);
    let mut guard = state
        .data
        .write()
        .map_err(|_| "数据库状态锁已损坏。".to_string())?;
    *guard = None;
    Ok(())
}

/**
 * 返回当前模式下存在条目的收藏夹汇总。
 */
#[tauri::command]
pub fn get_collection_summaries(
    state: State<'_, AppState>,
    mode: String,
) -> Result<Vec<CollectionSummary>, String> {
    let guard = state
        .data
        .read()
        .map_err(|_| "数据库状态锁已损坏。".to_string())?;
    let data = guard
        .as_ref()
        .ok_or_else(|| "请先加载 Realm 数据库。".to_string())?;
    Ok(services::collection_summaries(data.as_ref(), &mode))
}

/**
 * 返回服务端筛选、排序和分页后的谱面列表。
 */
#[tauri::command]
pub fn query_beatmaps(
    state: State<'_, AppState>,
    query: BeatmapQuery,
) -> Result<BeatmapPage, String> {
    let guard = state
        .data
        .read()
        .map_err(|_| "数据库状态锁已损坏。".to_string())?;
    let data = guard
        .as_ref()
        .ok_or_else(|| "请先加载 Realm 数据库。".to_string())?;
    services::query_beatmaps(data.as_ref(), &query)
}

/**
 * 从 Rust 管理的 JSON 文件中读取应用设置。
 */
#[tauri::command]
pub fn load_settings(app: AppHandle) -> AppSettings {
    services::load_settings(&app)
}

/**
 * 将完整应用设置写入 Rust 管理的 JSON 文件。
 */
#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    services::save_settings(&app, settings)
}

/**
 * 返回 Rust 定义的默认列配置，供设置弹窗执行恢复默认操作。
 */
#[tauri::command]
pub fn get_default_columns() -> Vec<ColumnConfig> {
    crate::models::default_columns()
}

/**
 * 从 Rust 磁盘缓存或网络加载谱面封面。
 */
#[tauri::command]
pub async fn get_cover(
    app: AppHandle,
    beatmap_set_id: i64,
) -> Result<Option<CoverPayload>, String> {
    services::get_cover(&app, beatmap_set_id).await
}

/**
 * 在阻塞线程中生成并保存当前列表 Excel。
 */
#[tauri::command]
pub async fn export_current(
    state: State<'_, AppState>,
    request: ExportCurrentRequest,
) -> Result<(), String> {
    let data = {
        let guard = state
            .data
            .read()
            .map_err(|_| "数据库状态锁已损坏。".to_string())?;
        guard
            .clone()
            .ok_or_else(|| "请先加载 Realm 数据库。".to_string())?
    };
    tauri::async_runtime::spawn_blocking(move || exporter::export_current(data.as_ref(), &request))
        .await
        .map_err(|error| format!("当前列表导出任务异常结束：{error}"))?
}

/**
 * 在阻塞线程中生成四模式汇总和收藏夹明细 Excel ZIP。
 */
#[tauri::command]
pub async fn export_all_modes(
    state: State<'_, AppState>,
    request: ExportAllRequest,
) -> Result<(), String> {
    let data = {
        let guard = state
            .data
            .read()
            .map_err(|_| "数据库状态锁已损坏。".to_string())?;
        guard
            .clone()
            .ok_or_else(|| "请先加载 Realm 数据库。".to_string())?
    };
    tauri::async_runtime::spawn_blocking(move || exporter::export_all_modes(data.as_ref(), &request))
        .await
        .map_err(|error| format!("全部模式导出任务异常结束：{error}"))?
}
