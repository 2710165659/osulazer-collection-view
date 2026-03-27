# 项目介绍

一个osu!lazer收藏夹查看程序。（不是Vibe Coding）

## 主要模块
1. extractor：c#提取realm数据库内容，并输出为json格式`CollectionRealmExtractor.exe <realm路径> <输出json路径>`
2. collection-view：Tauri + Vue 的可视化桌面应用

## 环境
1. nodejs
2. rust
3. c#（仅构建extractor需要）
