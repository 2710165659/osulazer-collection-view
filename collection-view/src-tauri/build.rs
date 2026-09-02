/**
 * 运行 Tauri 构建脚本并生成平台资源与配置代码。
 */
fn main() {
    // 让 Tauri 在构建期读取配置并嵌入桌面图标资源。
    tauri_build::build()
}
