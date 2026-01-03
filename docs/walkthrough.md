# 验证指南 - Audio Sorter 增强功能

本指南将指导您测试新的扫描管理器、资源监控和重复检测功能。

## 前置条件
- 已安装 Rust。
- 已安装 `fpcalc` (Chromaprint) 并添加到 PATH 环境变量中。
- 存在 `data/` 目录和 `index.json`（可选，如果不存在将自动创建）。
- 存在 `test_samples/` 或任何包含音频文件以供扫描的目录。

## 1. 启动服务器
运行服务器并配置输入目录，以启用 Web 端扫描功能。

```powershell
# 在终端 1 中运行
cargo run -- serve --index-dir ./data --input-dir "f:\code\rust_test\audio-sorter\test_samples"
```
*注意：请将路径替换为您的实际音频文件夹路径。*

## 2. Web 仪表盘
在浏览器中打开 [http://127.0.0.1:3000](http://127.0.0.1:3000)。

### 检查组件：
1.  **扫描按钮**：验证头部是否可见“Scan Library”（扫描库）按钮。
2.  **标签页**：验证是否存在“Library”（库）和“Duplicates”（重复项）标签页。

## 3. 测试扫描
1.  点击 **"Scan Library"**。
2.  **观察**：
    -   “Scan Progress”（扫描进度）面板出现。
    -   进度条开始增长。
    -   **资源监控**：CPU使用率 (%) 和内存使用量实时更新。
    -   “Processed”（已处理）计数增加。
3.  等待扫描完成。

## 4. 测试重复检测
1.  **创建重复文件**：
    -   将您的一个音频文件（例如 `test.mp3`）复制为同一文件夹下的 `test_copy.mp3`。
2.  **重新扫描**：步骤同上。
3.  **检查重复项标签页**：
    -   点击 “Duplicates”。
    -   验证是否出现 `Duplicate Group #1`，其中包含 `test.mp3` 和 `test_copy.mp3`。
    -   验证元数据（艺术家、标题）是否匹配。

## 5. 离线模式
如果未设置 `ACOUSTID_CLIENT_ID`，您可以强制使用离线模式。
-   除非缺少环境变量，否则 Web 扫描目前默认尝试在线模式。
-   检查控制台日志以查看 “Mode: OFFLINE” 或 “ONLINE”（如果后端有打印）。
