# osu!lazer 收藏夹管理工具

这是 Python/Tkinter 版本迁移后的桌面应用。当前运行技术栈统一为 **Tauri 2 + Rust + Vue 3 + TypeScript**，用于读取 osu!lazer 的 Realm 数据库并查看、筛选和导出收藏夹内容。

Python 版本只作为功能基准，不参与本项目的运行、开发或打包。Realm 数据库由 Rust 通过 `realm-db-reader` 直接读取；解析、筛选、排序、分页、设置、封面缓存和导出均由 Rust 负责。

## 功能

- 选择并加载任意 `.realm` 数据库文件。
- 浏览全部、osu!、Taiko、Catch、Mania 和缺失项。
- 按收藏夹查看总数、当前模式数量、缺失数量和更新时间。
- 由 Rust 完成谱面排序和分页，WebView 只保存当前页数据。
- 单击谱面预览封面，双击打开完整详情。
- 自定义显示列、列顺序和每页数量，并持久化用户设置。
- 将当前收藏夹和模式导出为单个 Excel 文件。
- 将 osu!、Taiko、Catch、Mania 四个模式分别导出为 Excel，并打包成 ZIP。
- 在 Rust 侧下载、校验和缓存 osu! 封面。

## 技术栈

| 层级 | 技术 | 职责 |
| --- | --- | --- |
| 桌面容器 | Tauri 2 | 窗口、IPC、系统对话框和应用打包 |
| 核心后端 | Rust | 数据状态、业务查询、文件操作、缓存和导出 |
| 前端 | Vue 3 + TypeScript | 界面展示和用户交互 |
| 状态管理 | Pinia | 保存轻量界面状态和当前分页结果 |
| UI | Element Plus | 表格、分页、弹窗和表单控件 |
| Realm 兼容层 | realm-db-reader | 只读打开旧版 Realm 9.9 与新版 Realm 24 数据库 |

Rust 主要依赖：

- `realm-db-reader`：直接读取 osu!lazer Realm 表、列表和关联对象。
- `serde` / `serde_json`：序列化数据契约和设置文件。
- `reqwest`：下载谱面封面。
- `base64`：将缓存封面转换为 WebView 可展示的数据地址。
- `zip`：生成 XLSX OOXML 文件及四模式 ZIP。
- `tauri-plugin-dialog`：打开系统文件、保存文件和消息对话框。

前端不再使用 Tauri Shell、FS、Opener 插件，也不再使用 JavaScript `xlsx`。WebView 没有任意读写文件的权限。

## 目录结构

```text
osulazer-collection-view/
|-- collection-view/
|   |-- src/                    # Vue 3 / TypeScript 前端
|   |   |-- components/         # 收藏夹、谱面、封面和详情界面
|   |   |-- composables/        # Rust 封面命令封装
|   |   |-- entities/           # 前后端 TypeScript 数据契约
|   |   |-- store/              # 统一 Pinia 状态仓库
|   |   `-- utils/              # 模式、列和显示值格式化
|   `-- src-tauri/
|       |-- src/commands.rs     # 暴露给前端的 Tauri 命令
|       |-- src/services.rs     # 数据加载、查询、设置和封面缓存
|       |-- src/realm_parser.rs # Rust Realm 24/9.9 数据库解析
|       |-- src/exporter.rs     # Rust XLSX/ZIP 导出
|       |-- src/models.rs       # Realm、设置和命令数据模型
|       |-- src/state.rs        # Rust 内存数据状态
|       `-- tauri.conf.json      # 窗口、CSP 和打包配置
```

## 工作原理

1. Vue 通过系统对话框取得用户选择的 `.realm` 路径。
2. 前端调用 Rust `load_database` 命令。
3. Rust 校验路径，并复制一次 `client.realm` 临时快照，避免和正在运行的 osu!lazer 争抢数据库状态。
4. `realm-db-reader` 直接读取收藏夹、谱面及 Realm Link 关联的元数据、难度、模式、谱师和谱面集。
5. Rust 统一 `fruits/catch/ctb` 为 `ctb`，把找不到本地谱面的收藏夹 MD5 标记为 `missing`，完成后删除数据库快照。
6. 完整数据保存在 Rust 的共享只读内存状态中；前端通过命令按模式请求收藏夹汇总，以及经过排序和分页的当前谱面页。
7. 设置、封面文件和导出文件均由 Rust 直接读写，WebView 不接触任意本地文件内容。

谱面状态码按 osu!lazer 数据契约显示：

```text
-2 graveyard
-1 wip
 0 pending
 1 ranked
 2 approved
 3 qualified
 4 loved
```

## 运行时文件

程序在 Tauri 的应用数据目录下创建 `runtime/`：

```text
runtime/
|-- ui_settings.json            # Realm 路径、模式、列配置和分页大小
`-- covers/                     # 按 SID 缓存的已校验封面
```

Realm 快照只会短暂写入系统临时目录，成功进入 Rust 内存后立即删除，不作为长期运行时文件保留。

删除这些运行时文件只会清除缓存和本地设置，不会修改用户选择的 Realm 数据库。

## 开发

环境要求：

- Node.js 与 npm。
- Rust stable 工具链。
- Tauri 对应平台的系统构建环境。
- 不需要 Python、C#/.NET 或额外的 Realm 原生 DLL。

```powershell
cd .\collection-view
npm install
npm run tauri dev
```

## 打包

执行：

```powershell
cd .\collection-view
npm install
npm run tauri build
```

Realm 解析器会作为 Rust 依赖直接编译进 Tauri 应用，目标机器不需要 Python、.NET 或单独的提取器。

## 安全边界

- Realm 路径和扩展名由 Rust 再次校验。
- Realm 只在 Rust 后端以只读快照方式解析，前端未获得 Shell 权限。
- 前端只拥有 Tauri 核心能力和系统 Dialog 权限。
- CSP 仅允许应用资源、Tauri IPC 和 `data:` / `blob:` 封面。
- 设置、封面和导出使用临时文件提交，降低中途退出造成文件损坏的概率。
