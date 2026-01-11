# 音频整理应用任务列表

## 规划与设计

- [ ] 探索工作区和现有依赖项 <!-- id: 0 -->
- [/] 调研 Rust 音频指纹库 (Chromaprint/AcoustID) <!-- id: 1 -->
- [/] 调研元数据 API 客户端 (MusicBrainz) <!-- id: 2 -->
- [/] 调研翻唱歌曲识别策略 <!-- id: 3 -->
- [x] 创建可行性文档 (`feasibility_study.md`) <!-- id: 4 -->
- [x] 创建实施计划 (`implementation_plan.md`) <!-- id: 5 -->

## 实施 - 核心

- [x] 初始化 Rust 项目 <!-- id: 6 -->
- [x] 实现文件系统扫描和遍历 <!-- id: 7 -->
- [x] 实现音频指纹生成 <!-- id: 8 -->
- [x] 实现 AcoustID/MusicBrainz 集成 <!-- id: 9 -->

## 实施 - 索引与状态管理 (新工作流)

- [x] 设计索引数据结构 (Path, Metadata, Mtime) <!-- id: 17 -->
- [x] 实现持久化存储 (JSON/SQLite) <!-- id: 18 -->
- [x] 实现增量扫描逻辑 (检测文件变化) <!-- id: 19 -->
- [x] 移除文件移动/重命名逻辑 <!-- id: 20 -->
- [x] 整合 "离线模式" 选项 <!-- id: 16 -->

## 验证

- [x] 验证增量索引 (修改文件后是否触发更新) <!-- id: 21 -->
- [x] 验证只读操作 (确保原文件未被移动) <!-- id: 22 -->
- [x] 创建操作指南 (`walkthrough.md`) <!-- id: 15 -->

## 实现 - 风格分类器

- [x] **规划**: 确定模型输入规格 (Melspectrogram 96 bands) 和依赖项 <!-- id: 23 -->
- [x] **环境**: 配置 `ort` 依赖和 Runtime DLL (Windows/Linux) <!-- id: 24 -->
- [x] **DSP 实现**: 音频重采样 (16kHz) + Mel Spectrogram 计算 (RustFFT/ndarray) <!-- id: 25 -->
- [x] **推理逻辑**: 实现 `Audio -> Embedding -> Genre` 流水线 <!-- id: 26 -->
- [x] **集成**: 实现 CLI `classify` 命令和 `ScanManager` 逻辑 <!-- id: 27 -->

## Web UI 集成

- [x] API 端点 (`/api/classify/start`)
- [x] 前端 "分类" 按钮和状态显示
- [x] 数据表格显示 "流派" 列
