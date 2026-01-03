# Audio Sorter

轻量的本地音频库整理与指纹识别工具，支持批量扫描、AcoustID/MusicBrainz 联网元数据获取、旋律特征分析、重复检测，以及基于 Web 的仪表盘。

## 功能
- 扫描目录，生成/更新 `index.json` 与旋律分析缓存 `analysis.bin`
- 调用 `fpcalc` 生成 Chromaprint 指纹并查询 AcoustID，再联动 MusicBrainz 获取原唱/元数据
- 离线模式：仅使用本地标签（lofty）整理
- Web 仪表盘：触发扫描、查看进度、资源占用、重复文件、相似歌曲推荐
- 旋律相似度：`bliss-audio` 提取 40 维向量，欧氏距离排序返回前 20 条推荐

## 快速开始
1. 安装 Rust 与 `cargo`。
2. 安装 `fpcalc`（Chromaprint 官方二进制，或自行编译）并加入 PATH。
3. 准备输入目录与输出目录，例如 `test_samples/` 与 `data/`。
4. CLI 扫描：
   ```powershell
   cargo run -- scan -i ./test_samples -o ./data --offline  # 无 AcoustID 密钥时
   # 或在线模式（需环境变量 ACOUSTID_CLIENT_ID）：
   # cargo run -- scan -i ./test_samples -o ./data
   ```
5. 启动 Web 仪表盘：
   ```powershell
   cargo run -- serve --index-dir ./data --input-dir ./test_samples
   # 打开 http://127.0.0.1:3000
   ```

## 主要模块
- `src/main.rs`：CLI 入口（`scan` / `serve`）。
- `src/scanner.rs`：遍历音频文件。
- `src/fingerprint.rs`：调用 `fpcalc` 生成 Chromaprint 指纹。
- `src/acoustid.rs` + `src/musicbrainz.rs`：联网查询元数据。
- `src/organizer.rs`：读取本地标签，合成统一元数据结构。
- `src/analysis_store.rs` + `src/worker.rs`：旋律向量提取与二进制缓存。
- `src/server.rs` + `src/scan_manager.rs`：Axum Web API、进度/资源监控、重复/推荐接口。

## 数据文件
- `index.json`：文件路径、标签、指纹、时间戳等索引。
- `analysis.bin`：旋律向量的 `bincode` 序列化缓存。
- `out_lib/fpcalc.exe`：可选，随源码放置的 fpcalc 二进制，`build.rs` 会在构建时复制到目标目录。

## API 速览（端口默认 3000）
- `GET /api/tracks`：全部索引。
- `POST /api/scan/start`：触发扫描（需启动时配置 `--input-dir`）。
- `GET /api/scan/status`：扫描进度与资源占用。
- `GET /api/duplicates`：重复文件分组。
- `GET /api/recommend?path=<abs-path>`：基于旋律向量的相似歌曲。

## 许可证
- 本项目采用 **MIT License**（见 `LICENSE`）。
- 第三方：`fpcalc`/Chromaprint 官方代码基于 **LGPL-2.1 或更高版本**；若使用的 FFmpeg/FFT 组件为 GPL 版本，生成的 `fpcalc` 二进制将转为 GPL。
- 在 MIT 项目中使用的建议：
  - 将 `fpcalc` 视为外部运行时依赖，避免静态链接到可执行文件。
  - 随发行物附带 Chromaprint 许可证文本和来源说明，并允许用户替换 `fpcalc` 二进制。
  - 若分发自编译的 `fpcalc`，确保所用依赖（如 FFmpeg）为 LGPL 版本，否则需要遵守 GPL 传播条款。

## 致谢
- Chromaprint / AcoustID
- MusicBrainz
- bliss-audio
