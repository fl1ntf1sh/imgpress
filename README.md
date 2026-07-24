# imgpress

imgpress 是一个桌面批量压缩工具，支持普通图片和扫描件 PDF。默认启动 Slint GUI，也提供 CLI 模式。

## 功能

- 批量压缩图片到目标大小以内
- 输出格式支持 JPEG / WebP
- 支持递归扫描输入目录
- 支持保留源目录结构或扁平化输出
- 支持 PDF 按页渲染后压缩，输出为 `name_page1.jpg` 这类文件
- 支持跳过已经小于目标大小的普通图片
- 支持运行日志写入 `log.txt`
- 支持全部成功后删除源文件，并在删除前弹窗确认
- GUI 会保存上次路径和参数

## 快速开始

默认无子命令时启动 GUI：

```powershell
cargo run
```

CLI 模式：

```powershell
cargo run -- cli -i input -o output --max-size 500kb --format jpeg
```

常用参数：

```text
-i, --input <PATH>          源文件或源目录
-o, --output <PATH>         输出目录
--max-size <SIZE>           目标大小，支持 b / kb / mb，默认 500kb
--format <jpeg|jpg|webp>    输出格式，默认 jpeg
--min-quality <1-100>       最低质量，默认 20
--max-quality <1-100>       最高质量，默认 95
--scale-step <0.10-0.99>    缩放回退比例，默认 0.85
--max-scales <0-32>         最大缩放轮数，默认 8
--no-preserve-structure     不保留源目录结构
--no-skip-if-smaller        不跳过小文件
--no-recursive              不递归扫描子目录
--log-file                  写入运行日志
--delete-source --yes       全部成功后删除源文件，CLI 必须显式确认
```

## 构建

```powershell
$env:MUPDF_MSVC_PLATFORM_TOOLSET='v145'
cargo build
cargo build --release
```

测试和静态检查：

```powershell
$env:MUPDF_MSVC_PLATFORM_TOOLSET='v145'
cargo test
cargo clippy --all-targets -- -D warnings
```

Windows 可执行文件图标来自：

```text
assets/icon.ico
assets/icon.rc
```

`build.rs` 会编译 Windows 资源，并同时编译 Slint UI。

## 支持格式

输入：

- 图片：`png`、`jpg`、`jpeg`、`webp`、`bmp`、`tiff`、`tif`、`gif`、`ico`、`ppm`、`pgm`、`pbm`
- PDF：`pdf`

输出：

- JPEG：扩展名 `.jpg`
- WebP：扩展名 `.webp`

## GUI 行为

GUI 文件位于：

```text
src/ui/app.slint
src/gui/
```

主要交互：

- 左侧选择源目录、输出目录和处理选项
- 右侧设置压缩参数和查看运行状态
- 运行状态只保留一个总任务进度条
- 日志框会实时追加最近的处理日志
- “打开输出目录”和“打开日志目录”可以直接打开对应目录
- 勾选“全部成功后删除源文件”时，压缩完成后会弹窗确认
- 删除确认弹窗倒计时 15 秒，未选择时默认确认删除
- 点击“取消删除”或主界面“取消”会取消删除源文件

GUI 设置保存到应用数据目录下的 `settings.json`。日志写入同目录下的 `log.txt`。

## 日志文件

启用日志后，`log.txt` 会追加每次运行报告，包含：

- 运行时间
- 源路径和输出路径
- 压缩参数
- 成功/失败数量
- 输入/输出字节数和压缩比例
- 源文件删除状态
- 失败文件列表和原因

## 代码结构

```text
src/
  main.rs              # CLI/GUI 入口选择
  lib.rs               # crate 对外导出
  cli.rs               # clap 参数定义
  config.rs            # 压缩配置和统一参数校验
  compressor.rs        # 质量二分 + 缩放回退压缩逻辑
  discovery.rs         # 扫描输入文件，生成 FileTask
  error.rs             # 统一错误类型
  log.rs               # log.txt 写入
  progress.rs          # CLI/GUI 共用进度回调 trait
  settings.rs          # GUI 设置持久化
  source.rs            # 删除源文件
  codec/
    mod.rs             # Codec trait
    jpeg.rs            # JPEG 编码
    webp.rs            # WebP 编码
  input/
    mod.rs             # 输入类型分发
    types.rs           # ExtractedImage / ImageLabel / InputKind
    image.rs           # 普通图片读取
    pdf.rs             # PDF 页面提取入口
  output/
    mod.rs             # 输出模块入口
    naming.rs          # 输出路径命名规则
  pdf/
    mod.rs             # PDF 对外入口
    renderer.rs        # PdfRenderer trait
    mupdf.rs           # MuPDF 实现
  pipeline/
    mod.rs             # pipeline 对外入口
    report.rs          # CompressReport
    runner.rs          # 批量任务调度和删除源文件确认
    task.rs            # 单文件处理
  gui/
    mod.rs             # GUI 对外入口
    slint_app.rs       # Slint 窗口创建和回调绑定
    worker.rs          # 后台压缩任务和 ProgressReporter 实现
    delete_confirm.rs  # 删除源文件确认弹窗通道和倒计时
    run_log.rs         # GUI 运行日志行缓存
    settings_binding.rs# 设置加载/保存到 UI
    ui_options.rs      # UI 参数解析为 CompressOptions
  ui/
    app.slint          # Slint 界面
```

## 核心流程

```text
collect_files
  -> FileTask(input, output)
  -> input::extract_images
      -> image::extract 普通图片
      -> pdf::extract PDF 每页
  -> output::path_for
  -> Compressor::compress_to_size
  -> 写入输出文件
  -> 汇总 CompressReport
  -> 如请求删除源文件，删除前确认
```

压缩策略：

1. 在 `min_quality..=max_quality` 范围内二分搜索。
2. 找到不超过目标大小的最高质量版本。
3. 如果最低质量仍超限，按 `scale_step` 缩小分辨率后重试。
4. 超过 `max_scales` 后仍失败则记录失败。

## PDF 说明

PDF 通过本地 `mupdf` 依赖渲染为图片。当前项目使用本地 patch：

```toml
[patch.crates-io]
mupdf = { path = "patch/mupdf" }
```

该 patch 用于规避当前 Rust/MSVC 环境下上游 `mupdf` 的编译兼容问题。后续如果上游修复，可以移除 `[patch.crates-io]` 并删除 `patch/mupdf`，然后重新验证构建。

PDF 渲染抽象已经隔离到：

```text
src/pdf/renderer.rs
src/pdf/mupdf.rs
```

如果未来要替换为 `pdfium-render`，优先新增一个 renderer 实现，而不是直接改 pipeline。

## Word 文档说明

当前版本不处理 Word 文档。

如果以后要支持 Word 文档，有两种不同方向：

- 提取 DOCX 内嵌图片：读取 `word/media/*`，不需要 LibreOffice，但不是按页渲染。
- Word 按页渲染：通常需要 LibreOffice 或 Microsoft Word 这类排版引擎先转 PDF，再复用现有 PDF 渲染流程。

## 安全行为

- 默认不删除源文件
- CLI 删除源文件必须传 `--delete-source --yes`
- GUI 删除源文件会在压缩完成后弹窗确认
- 如果有任何失败、取消或没有成功处理文件，源文件不会被删除
- 如果输出目录位于输入目录内部，删除源目录内容时会跳过输出目录

## 已知限制

- PDF 渲染比例当前固定为 `2.0`
- GUI 日志框保留最近 12 行，不是完整滚动日志视图
- 当前版本不处理 Word 文档；如需按页渲染，需另接 LibreOffice / Word 转 PDF 流程
- 运行日志使用 UTC 时间
- 当前 `patch/mupdf` 是本地依赖补丁，发布/升级依赖时需要特别注意

## 开发建议

- 新增输入格式时，优先在 `src/input/` 下新增模块，并扩展 `InputKind`
- 新增输出命名规则时，优先改 `src/output/naming.rs`
- 替换 PDF 引擎时，优先新增 `PdfRenderer` 实现
- 调整参数规则时，只改 `config::validate_options()`，CLI 和 GUI 会共用
- UI 视觉调整集中改 `src/ui/app.slint`，业务绑定尽量放在 `src/gui/` 子模块中
