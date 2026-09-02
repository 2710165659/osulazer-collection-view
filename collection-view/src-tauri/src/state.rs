use std::sync::{
    atomic::AtomicU64,
    Arc, RwLock,
};

use crate::models::ExtractedData;

/**
 * 保存当前已加载的数据，避免在 WebView 与 Rust 之间反复传输整份 JSON。
 */
#[derive(Default)]
pub struct AppState {
    pub data: RwLock<Option<Arc<ExtractedData>>>,
    pub load_generation: AtomicU64, // 仅允许最后发起的数据库加载请求提交 Rust 内存状态。
}
