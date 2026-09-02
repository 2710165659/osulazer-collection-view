// 发布版隐藏额外的 Windows 控制台窗口。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/**
 * 启动桌面应用并进入 Tauri 主事件循环。
 */
fn main() {
    collection_view_lib::run()
}
