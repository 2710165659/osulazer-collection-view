# extractor（C# 提取器）

 `client.realm` 是 `osu!lazer` 使用的 Realm 数据库，直接读取这套结构并不方便。当前项目通过 Realm 的 .NET 生态做只读提取，把结果输出位json文件。

## 环境
- 操作系统：Windows 10 / 11（x64）
- .NET SDK：.NET 8.0 SDK 或 .NET 7.0 SDK
- 目标运行时：win-x64
- 依赖包：
  - Realms (.NET Realm 数据库客户端)
  - System.Text.Json (.NET 内置)


## 构建命令

```bash
dotnet publish extractor\CollectionRealmExtractor.csproj `
    -c Release `
    -r win-x64 `
    --self-contained true `
    -p:PublishSingleFile=true `
    -p:PublishTrimmed=false `
    -o "extractor\publish"
```
## 使用

进入构建后目录，运行`.\extractor\publish\CollectionRealmExtractor.exe <realm路径> <输出json路径>`
