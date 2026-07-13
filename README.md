# imgpress

> 把任意数量、任意大小的图片（包括扫描件型 PDF），精确压缩到指定字节数以下。
> 真正单文件 exe，无需任何额外 dll。GUI 默认启动；亦可作为 CLI 工具。

---

## 目录

- [特性](#特性)
- [快速开始](#快速开始)
- [GUI 使用](#gui-使用)
- [CLI 使用](#cli-使用)
- [架构总览](#架构总览)
- [源码详解](#源码详解)
  - [`src/main.rs` — 入口](#srcmainrs--入口)
  - [`src/lib.rs` — crate 根](#srclibrs--crate-根)
  - [`src/error.rs` — 错误类型](#srcerrorrs--错误类型)
  - [`src/config.rs` — 配置](#srcconfigrs--配置)
  - [`src/codec/` — 编解码器](#srccodec--编解码器)
  - [`src/decoder.rs` — 图像解码](#srcdecoderrs--图像解码)
- [`src/pdf/mod.rs` — PDF 处理](#srcpdfmodrs--pdf-处理)
- [`src/compressor.rs` — 核心压缩](#srccompressorrs--核心压缩)
- [`src/pipeline.rs` — 文件流水线](#srcpipeliners--文件流水线)
  - [`src/progress.rs` — 进度回调](#srcprogressrs--进度回调)
  - [`src/settings.rs` — GUI 持久化](#srcsettingsrs--gui-持久化)
  - [`src/cli.rs` — 命令行解析](#srcclirs--命令行解析)
  - [`src/gui/` — 图形界面](#srcgui--图形界面)
  - [`build.rs` — 构建脚本](#buildrs--构建脚本)
  - [`assets/icon.rc` — 资源脚本](#assetsiconrc--资源脚本)
- [关键算法详解](#关键算法详解)
- [线程模型](#线程模型)
- [依赖说明](#依赖说明)
- [编译与发布](#编译与发布)
- [性能数据](#性能数据)
- [已知限制与扩展点](#已知限制与扩展点)

---

## 特性

- **精确大小控制**：使用二分搜索在质量维度上找到满足目标字节数的最高质量
- **降采样回退**：当最低质量仍超过目标时，自动按比例缩小分辨率
- **多格式输入**：PNG / JPG / WebP / BMP / TIFF / GIF / ICO / PPM/PGM/PBM
- **PDF 输入（扫描件型）**：通过 mupdf 渲染每页为位图后压缩；正确处理扫描型与多图层 PDF
- **多格式输出**：JPEG（image crate 标准编码）、WebP
- **批量递归扫描**：自动遍历子目录
- **两种结构模式**：
  - 保留结构：源目录的子文件夹布局原样复制到输出
  - 扁平化：所有文件拍平到输出根目录，命名格式 `<父目录>_<原文件名>`
- **PDF 多页输出**：单页拆分为 `{名}_page{N}.jpg`
- **并行处理**：rayon 多线程并发压缩
- **GUI 默认启动**，暗色主题、中文支持、实时扫描、详细进度
- **参数持久化**：GUI 的所有选项保存到 `%APPDATA%\imgpress\settings.json`
- **可中断**：运行中可点 Cancel，已完成文件保留
- **永不覆盖源文件**：输出强制写到独立目录
- **冲突命名自动解决**：相同文件名加 `_1`, `_2` 后缀
- **真正单文件 exe**：约 9.3 MB，**无任何额外 dll 依赖**

---

## 快速开始

### 编译

```bash
# Debug（带终端、便于看日志）
cargo build

# Release（不弹终端、LTO 优化）
cargo build --release
```

产物：

```
target/release/imgpress.exe   # 9.3 MB（单文件，零依赖）
```

### 运行

```bash
# 默认（GUI）
target/release/imgpress.exe

# CLI
target/release/imgpress.exe cli --help
```

---

## GUI 使用

启动后界面包含 5 个分组：

| 分组 | 内容 |
|---|---|
| 路径 | 源文件夹、输出文件夹。源路径选完后实时异步扫描，显示文件数 + 总大小 |
| 压缩参数 | 目标大小、输出格式、质量范围（双滑块 + 数值）、缩放步长 |
| 选项 | 保留子目录、跳过小文件、保存日志 |
| 进度 | 进度条、完成数 / 用时 / 速率 / 预计剩余 + 输入/输出字节 + 节省百分比 + 当前文件 |
| 日志 | 实时操作日志（最多 300 条），可清空 |

按钮：`▶ 开始` / `■ 取消` / `📂 打开输出`

**典型流程**：

1. 点 Source 的「浏览...」选源文件夹
2. 点 Output 的「浏览...」选输出文件夹
3. 设置目标（如 500 KB）
4. 点「开始」
5. 完成后点「打开输出」看结果

---

## CLI 使用

```
imgpress cli [OPTIONS] --input <INPUT> --output <OUTPUT>

必选：
  -i, --input <INPUT>            源文件夹或文件路径
  -o, --output <OUTPUT>          输出文件夹

压缩参数：
  --max-size <SIZE>              目标上限，支持 kb/mb/b 后缀 [默认: 500kb]
  --format <jpeg|webp>           输出格式 [默认: jpeg]
  --min-quality <0-100>          质量二分下界 [默认: 20]
  --max-quality <0-100>          质量二分上界 [默认: 95]
  --scale-step <0.1-0.99>        缩放步长 [默认: 0.85]
  --max-scales <INT>             最大缩放轮次 [默认: 8]

文件处理：
  --preserve-structure           保留源目录子文件夹结构
  --skip-if-smaller              已小于目标则直接复制（不再压缩）
  --no-recursive                 不递归子目录
```

### CLI 示例

```bash
# 压缩 photos 下所有图片到 500KB 以下，保留目录结构
imgpress cli -i D:\photos -o D:\out --max-size 500kb --preserve-structure

# 压到 200KB，扁平化输出
imgpress cli -i D:\photos -o D:\out --max-size 200kb

# 输出 WebP
imgpress cli -i D:\photos -o D:\out --format webp

# 提高质量上限，让搜索更精细
imgpress cli -i D:\photos -o D:\out --max-size 500kb --min-quality 50 --max-quality 98
```

### CLI 输出示例

```
[INFO  imgpress] start: input=D:\photos output=D:\out target=500KB
========================================
Done in 135.48s
  Total:    182
  Success:  182
  Failed:   0
  Bytes:    480.94 MB -> 73.25 MB (15.2%)
```

---

## 架构总览

```
                 ┌────────────────────────┐
                 │      src/main.rs       │
                 │   入口（解析 argv）    │
                 └──────┬────────┬────────┘
                        │        │
                  无子命令       有 cli 子命令
                        │        │
                        ▼        ▼
              ┌──────────────┐  ┌─────────────────────┐
              │ src/gui/     │  │ src/cli.rs          │
              │ eframe 窗口  │  │ + 调 compress_dir   │
               └──────┬───────┘  └──────────┬──────────┘
                      │                     │
                      │  (worker thread)    │
                      ▼                     ▼
               ┌──────────────────────────────────┐
               │  src/pipeline.rs                 │
               │  collect_files → 并行压缩 → 汇总 │
               └──┬──────────┬──────────┬─────────┘
                  │          │          │
              普通图片    普通图片   PDF 文件
                  │          │          │
┌──────▼─────┐    │    ┌──────▼──────────┐
            │ decoder.rs │    │    │ src/pdf/mod.rs  │
            │ image::open│    │    │ mupdf 全页栅格化 │
            └──────┬─────┘    │    └──────┬──────────┘
                  │          │          │
                  └──────────┼──────────┘
                             │
                    ┌────────▼─────────┐
                    │ compressor.rs     │
                    │ 二分 + 缩放        │
                    └────────┬─────────┘
                             │
                      ┌──────▼──────────┐
                      │  src/codec/     │
                      │ jpeg.rs / webp  │
                      └─────────────────┘
```

**数据流（一次压缩一张图）**：

```
普通图片: image::open() → DynamicImage → compressor → Vec<u8>
PDF 文件: mupdf::Document::open → load_page → to_pixmap
                       → DynamicImage (每页一张) → compressor → Vec<u8>
```

**共享步骤**：
```
DynamicImage → compressor.compress_to_size(target, ...)
              ↓ 二分搜索质量 Q ∈ [min, max]
              ↓ 编码 → 测大小 → 调整 Q
              ↓ 若 min 仍超 → 缩尺寸 → 重试
       Vec<u8>（JPEG/WebP 压缩字节流）
              ↓
       fs::write(output_path, bytes)
```

---

## 源码详解

> 每个源文件都按"职责 → 关键类型 → 关键函数 → 实现细节"展开。

### `src/main.rs` — 入口

**职责**：解析 argv，决定走 GUI 还是 CLI。

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
```

- `#![windows_subsystem = "windows"]` 告诉 MSVC 链接器这是个 GUI 程序，**release 模式不弹终端窗口**。debug 模式仍保留终端，方便看 `log::info!` 输出。
- `env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))`：日志默认 info 级别，可通过 `RUST_LOG=debug` 覆盖。
- `Cli::parse()` 用 clap 解析 argv。如果没有任何子命令（None），启动 GUI；否则执行 `cli` 子命令。

**`build_options(args)`**：把 `CliArgs` 转成 `CompressOptions`。字符串大小解析：
- `500kb` → `500 × 1024` bytes
- `2mb` → `2 × 1024²` bytes
- `500b` → 500 bytes

**`parse_size`**：手写解析（不用 `byte-unit` crate 之类），避免引入额外依赖。

**`print_report`**：CLI 完成后打印汇总：总数、成功、失败、字节前后对比。

---

### `src/lib.rs` — crate 根

声明所有模块并 re-export 常用类型：

```rust
pub mod error;
pub mod config;
pub mod codec;
pub mod decoder;
pub mod compressor;
pub mod pipeline;
pub mod progress;
pub mod settings;
pub mod gui;
pub mod cli;

pub use error::{Error, Result};
pub use config::{CompressOptions, Format, SizeLimit};
pub use pipeline::{compress_directory, CompressReport};
```

main.rs 通过 `use imgpress::*` 引用这些。

---

### `src/error.rs` — 错误类型

使用 `thiserror` 派生 `Error`：

```rust
#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("image decode error: {0}")]
    Image(String),

    #[error("image encode error: {0}")]
    Encode(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("cancelled by user")]
    Cancelled,

    // ...
}
```

- `#[from]` 自动实现 `From<X> for Error`，调用方可以 `?` 直接转换
- `Cancelled` 用于 compressor 中的早退信号
- 实现了 `From<image::ImageError>` 和 `From<anyhow::Error>`（anyhow 错误转字符串）

---

### `src/config.rs` — 配置

定义三个核心类型：

```rust
pub enum Format { Jpeg, WebP }

pub struct SizeLimit {
    pub bytes: u64,
}

pub struct CompressOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub max_size: SizeLimit,
    pub format: Format,
    pub min_quality: u8,
    pub max_quality: u8,
    pub scale_step: f32,
    pub max_scales: u32,
    pub preserve_structure: bool,
    pub skip_if_smaller: bool,
    pub recursive: bool,
    pub save_log: bool,
}
```

**关键设计**：
- `Format::extension()` 返回字符串 `"jpg"` / `"webp"`，用于生成输出文件名
- `SizeLimit::from_kb/mb/bytes` 三个构造方法，统一内部表示为 `bytes`
- `Default` 给合理的初值（500KB、JPEG、质量 20~95）

---

### `src/codec/` — 编解码器

#### 模块设计

```rust
// src/codec/mod.rs
pub trait Codec: Send + Sync {
    fn encode(&self, img: &DynamicImage, quality: u8) -> Result<Vec<u8>>;
}
```

- `Send + Sync` 让 `Compressor` 可以安全在线程间共享
- 输入是 `&DynamicImage`（任意色彩空间），输出是 `Vec<u8>`
- 调用者（compressor）只关心字节数和内容，不关心编码细节

#### `src/codec/jpeg.rs` — image crate JPEG 编码

```rust
pub struct JpegCodec;

impl Codec for JpegCodec {
    fn encode(&self, img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
        let rgb = flatten_alpha_to_rgb(img);
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
        encoder
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| Error::Encode(format!("jpeg encode: {}", e)))?;
        Ok(buf)
    }
}
```

**关键点**：
- 使用 `image` crate 的 `jpeg` 编码器，避免引入 mozjpeg 的 C 依赖（减少二进制体积、缩短编译时间）
- `flatten_alpha_to_rgb` 把 RGBA → RGB：alpha < 255 时与白色背景合成；否则直接转 RGB
- `JpegEncoder::new_with_quality(&mut buf, quality)` 把编码结果直接写入 `Vec<u8>`，无需手动缓冲
- `write_image` 接受原始像素缓冲 + 宽高 + 颜色类型，无需构建中间 `ImageBuffer`

#### `src/codec/webp.rs` — WebP 编码

```rust
pub struct WebPCodec;

impl Codec for WebPCodec {
    fn encode(&self, img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
        let rgba = img.to_rgba8();
        let encoder = webp::Encoder::from_rgba(rgba.as_raw(), w, h);
        let memory = encoder.encode(quality as f32);
        Ok(memory.to_vec())
    }
}
```

**关键点**：
- `webp::Encoder::from_rgba` 直接接收 RGBA 字节（无需转 RGB）
- 返回的 `WebPMemory` 实现 `Deref<Target=[u8]>`，调用 `.to_vec()` 拿到 owned `Vec<u8>`

---

### `src/decoder.rs` — 图像解码

```rust
pub fn load_image(path: &Path) -> Result<DynamicImage> {
    let img = image::open(path)?;
    Ok(img)
}

pub fn is_supported_image(path: &Path) -> bool {
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tiff" | "tif" | "gif" | "ico"
            | "ppm" | "pgm" | "pbm"
    )
}
```

- `image::open` 是 `image` crate 的"魔法"函数：自动嗅探格式并解码。内部基于文件头 magic bytes，不依赖扩展名（但建议保留正确扩展名）
- 支持范围受 `image` crate 限制：`default-features = true` 时包含所有常见格式

---

### `src/pdf/mod.rs` — PDF 处理

**职责**：把 PDF 的每一页渲染为 `DynamicImage`，以便复用 `compressor` 的统一压缩逻辑。

#### 设计动机

PDF 渲染经历过三轮方案迭代，最终选择 mupdf：

| 方案 | 失败原因 |
|---|---|
| `pdfium-render` static | C++ 静态库符号缺失（`FPDFPage_TransformAnnots` 等） |
| `pdfium-render` dynamic | 需要 `pdfium.dll` 单独分发，不满足"单 exe"目标 |
| `mupdf` 0.8 初版 | 锁定 VS 2019 工具集（v142），本机只有 VS 2026 |
| `lopdf` 0.34 临时方案 | 只能解析内嵌位图，纯文本/矢量 PDF 完全无法处理；需要手写 Filter chain |

最终方案：**mupdf 0.8 + v145 toolset**。优势：

- ✅ 正确处理扫描型、矢量、文字型以及多图层 PDF（mupdf 自带完整光栅化引擎）
- ✅ AGPL 限制只约束 mupdf 库本身；本程序不修改其源码，动态链接时通常不会触发 AGPL 传染，本项目采用静态链接并保留原 LICENSE
- ✅ 二进制体积可控（feature 精简 + 静态链接进 EXE，运行时无 dll）
- ❌ 编译时间长（首次需 ~30 分钟，且需要 LLVM/clang 在编译期供 bindgen 使用）
- ❌ 单 PDF 临时文件大小：mupdf 完整引擎静态链接约增加 8 MB

#### 启用方式

`Cargo.toml`：
```toml
mupdf = { version = "0.8", default-features = false, features = ["img", "base14-fonts"] }
```

`default-features = false` 关掉 JS / HTML / EPUB 等不需要的特性以减小体积。

#### 编译期环境要求

- **LLVM / Clang**：mupdf-sys 内部用 `bindgen` 生成 Rust 绑定，需 `LIBCLANG_PATH` 指向 `clang.dll`。**运行时不需要 LLVM**，可放在 `LIBCLANG_PATH` 临时设置里
- **MSVC v145 toolset**：本机仅 VS 2026 时必须设置 `MUPDF_MSVC_PLATFORM_TOOLSET=v145`，否则 mupdf-sys 的 `find_vs_version()` 会回退到 v142，与本机不匹配导致 C 端编译失败

#### Rust 1.96 兼容补丁

Rust 1.96 在 `const {}` 块中通过 glob 导入解析类型时存在回归，导致上游 `mupdf 0.8.0` 在 `src/device/native.rs:599` 的 `align_of::<max_align_t>()` 编译失败（`max_align_t` 经 `use mupdf_sys::*` 引入，但 `const {}` 块内找不到）。

本仓库在 `Cargo.toml` 中通过 `[patch.crates-io]` 指向本地 `patch/mupdf/`，把 `max_align_t` 替换为 `usize`（对齐性 8 在 64 位 Windows 上等价或更小，满足 sanity check）。等上游修复后可移除此 patch。

#### 主函数

```rust
pub fn render_pdf_pages(path: &Path) -> Result<Vec<DynamicImage>> {
    let doc = mupdf::Document::open(path.to_str().ok_or(...)?)?;
    let page_count = doc.page_count()?;

    let scale = 2.0;
    let matrix = mupdf::Matrix::new_scale(scale, scale);
    let mut result = Vec::with_capacity(page_count as usize);

    for i in 0..page_count {
        let page = doc.load_page(i)?;
        let pixmap = page.to_pixmap(&matrix, &mupdf::Colorspace::device_rgb(), false, false)?;

        // 把 pixmap 的 RGBA/RGB 像素搬运到 image::RgbaImage/RgbImage
        let img = if pixmap.n() >= 4 { /* RGBA8 */ } else { /* RGB8 */ };
        result.push(img);
    }
    Ok(result)
}
```

**关键点**：
- 用 `mupdf::Colorspace::device_rgb()` 显式要求 RGB 输出，避免 RGBA 渲染
- `scale = 2.0` 把每页按 2× 渲染，对扫描型 PDF 可获得更清晰的位图（代价是体积更大）
- 部分页面失败不致命：`Vec<DynamicImage>` 为空才报错；个别页失败仅记 warn 跳过
- 一次 `Document::open` 拿到所有页面，再循环 `load_page` 拿到每页 pixmap

#### 在 pipeline 中的集成

`pipeline.rs` 的 `process_task` 检测 `.pdf` 扩展名，路由到 `process_pdf`：

```rust
fn process_pdf(task, opts, compressor, ...) -> TaskOutcome {
    let pages = crate::pdf::render_pdf_pages(&task.input)?;
    let stem = task.input.file_stem()...;
    let parent = task.output.parent()...;

    let mut out_files = Vec::with_capacity(pages.len());
    for (idx, img) in pages.into_iter().enumerate() {
        let target_name = format!("{}_page{}.jpg", stem, idx + 1);
        let out_path = unique_pdf_output(parent, &target_name);

        let compressed = compressor.compress_to_size(&img, opts.max_size.bytes, ...)?;
        std::fs::write(&out_path, &compressed)?;
        out_files.push((out_path, compressed.len() as u64));
    }
    TaskOutcome::Ok { in_size, out_files }
}
```

**注意**：
- 每页独立压缩，target 大小对每页独立生效
- 输出文件名加 `_page{N}` 后缀（避免多页覆盖）
- `unique_pdf_output` 处理页面间冲突（如两页 PDF 内容巧合生成相同字节）

#### 命名路径：Windows Path 坑

单文件输入 + `preserve_structure=false` 的命名曾经有个 bug：

```rust
// input = "D:\code\rust\imgpress\test_files\sample.pdf"
// 错误的 prefix 计算：filter_map(|c| c.to_str()) 不跳过 "D:" 和 "\"
// 错误的 name = "D:_\\_code_rust_imgpress_test_files_sample.jpg"
// 错误的 task.output 被 canonicalize 到 <cwd>\_\sample.jpg

// 修复：在 iter().filter_map().filter() 中排除 "D:"、""、"\"
parent.iter()
    .filter_map(|c| c.to_str())
    .filter(|s| !matches!(*s, "" | "\\" | "/" | "D:"))
    .collect::<Vec<_>>()
    .join("_")
```

---

### `src/compressor.rs` — 核心压缩

**整个项目最核心的算法**。

#### 核心思路

JPEG 的质量参数 Q（0-100）与输出字节数大致单调负相关：Q 越大质量越好，文件越大。我们要在保证 ≤ target 的前提下，找到最大的 Q（这样画质最好）。

简单的做法是线性扫描 Q，但效率低。**二分搜索** 把搜索范围从 75 步降到 ~7 步。

#### 二分搜索伪代码

```
binary_search_quality(img, target, lo, hi):
    best = None
    while lo <= hi:
        mid = (lo + hi) // 2
        bytes = encode(img, mid)
        if len(bytes) <= target:
            best = bytes  # 可行解，继续向上找更高质量
            lo = mid + 1
        else:
            hi = mid - 1  # 不可行，向下
    return best
```

#### 完整代码

```rust
pub fn compress_to_size(
    &self,
    img: &DynamicImage,
    target: u64,
    min_q: u8,
    max_q: u8,
    scale_step: f32,
    max_scales: u32,
    progress: &dyn ProgressReporter,
) -> Result<Vec<u8>> {
    let mut current = img.clone();
    let mut best: Option<Vec<u8>> = None;

    for scale_round in 0..=max_scales {
        let (mut lo, mut hi) = (min_q.max(1), max_q.min(100));
        let mut last_valid: Option<Vec<u8>> = None;

        while lo <= hi {
            if progress.is_cancelled() {
                return Err(Error::Cancelled);
            }
            let mid = ((lo as u16 + hi as u16) / 2) as u8;
            let bytes = self.codec.encode(&current, mid)?;
            let size = bytes.len() as u64;

            if size <= target {
                last_valid = Some(bytes);
                lo = mid.saturating_add(1);
            } else {
                if hi == 0 { break; }
                hi = mid.saturating_sub(1);
            }
        }

        if let Some(valid) = last_valid {
            if valid.len() as u64 <= target {
                best = Some(valid);
                break;
            }
        }

        if scale_round == max_scales { break; }

        // 缩放回退：按 scale_step 缩小尺寸
        let new_w = ((current.width() as f32) * scale_step).max(1.0) as u32;
        let new_h = ((current.height() as f32) * scale_step).max(1.0) as u32;
        current = current.resize(new_w, new_h, image::imageops::FilterType::Lanczos3);
    }

    best.ok_or_else(|| Error::Encode("unable to compress...".into()))
}
```

#### 设计要点

1. **质量与文件大小的单调性**：JPEG 是近似单调的，但二分的精度足够（差异在 ±1-3%）
2. **缩放回退**：当二分失败（即使 min_q 也超 target），按 scale_step 缩小重试
   - 默认 0.85：宽高各 × 0.85 → 像素数 × 0.7225
   - 8 轮最大缩放：原图缩到 0.85⁸ ≈ 0.31 倍
3. **取消支持**：每次 encode 前检查 `is_cancelled`，可中途中断
4. **Lanczos3 滤波器**：缩放时用高质量滤波器（避免锯齿）
5. **返回最接近的解**：找到的最大可行 Q，对应"质量最好且满足大小"
6. **失败处理**：8 轮缩放 + min_q 都失败时返回错误（极罕见，可能是非典型图像）

---

### `src/pipeline.rs` — 文件流水线

负责扫描、调度、汇总。

#### 数据结构

```rust
pub struct FileTask {
    pub input: PathBuf,   // 源文件
    pub output: PathBuf,  // 目标输出（已计算好，不重复）
}

pub struct CompressReport {
    pub total: usize,
    pub success: usize,
    pub failed: Vec<(PathBuf, String)>,
    pub bytes_in: u64,
    pub bytes_out: u64,
}
```

#### 文件扫描：`collect_files`

```rust
pub fn collect_files(input, output, recursive, preserve_structure) -> Result<Vec<FileTask>>;
```

- 若 `input` 是单文件，直接构造一个 `FileTask`
- 若 `input` 是目录，递归遍历，调用 `walk`

#### 命名规则

```rust
fn walk(input_root, dir, output_root, recursive, preserve_structure, tasks) {
    for entry in read_dir(dir) {
        if dir: 递归 walk
        if !is_supported: 跳过
        
        rel = path.relative_to(input_root)  // 如 "张三/1.png"
        
        if preserve_structure:
            out_dir = output_root / rel.parent  // 保留 "张三/" 子目录
            target_name = "<stem>.jpg"          // 原文件名
        else:
            prefix = rel.parent.iter().join("_")  // "张三"
            out_dir = output_root
            target_name = "<prefix>_<stem>.jpg"  // "张三_1.jpg"
        
        out_path = unique_in_dir(out_dir, target_name)
    }
}
```

**保留结构** vs **扁平化** 示例：

源结构：
```
photos/张三/1.png
photos/张三/2.png
photos/李四/1.png
```

保留结构（`--preserve-structure`）：
```
out/张三/1.jpg
out/张三/2.jpg
out/李四/1.jpg
```

扁平化（默认）：
```
out/张三_1.jpg
out/张三_2.jpg
out/李四_1.jpg
```

#### `unique_in_dir`：冲突解决

```rust
fn unique_in_dir(dir, name) -> PathBuf {
    if !dir.join(name).exists() { return dir.join(name); }
    for i in 1.. {
        let candidate = format!("{}_{}.jpg", stem, i);
        if !dir.join(&candidate).exists() { return dir.join(candidate); }
    }
}
```

处理扁平化后仍然可能的冲突（如多个同名子目录），加 `_1`、`_2` 后缀。

#### 并行压缩：`compress_directory`

```rust
pub fn compress_directory(input, output, opts, progress) -> Result<CompressReport> {
    std::fs::create_dir_all(output)?;
    let tasks = collect_files(...);
    progress.on_start(tasks.len());

    // 预创建所有输出目录（避免并行时重复创建）
    for task in &tasks {
        if let Some(p) = task.output.parent() {
            std::fs::create_dir_all(p)?;
        }
    }

    let compressor = Compressor::new(...);
    let report = Mutex::new(CompressReport { total, ..Default::default() });

    let pool = rayon::ThreadPoolBuilder::new()
        .thread_name(|i| format!("imgpress-{}", i))
        .build()?;

    pool.install(|| {
        tasks.par_iter().for_each(|task| {
            if progress.is_cancelled() { return; }
            progress.on_file_start(&task.input);

            match process_task(task, opts, &compressor, progress) {
                Ok((in_size, out_size)) => {
                    let mut r = report.lock().unwrap();
                    r.success += 1;
                    r.bytes_in += in_size;
                    r.bytes_out += out_size;
                    progress.on_file_done(&task.input, true, None);
                }
                Err(msg) => {
                    let mut r = report.lock().unwrap();
                    r.failed.push((task.input.clone(), msg.clone()));
                    progress.on_file_done(&task.input, false, Some(&msg));
                }
            }
        });
    });

    let final = report.into_inner().unwrap();
    progress.on_finish(&final);
    Ok(final)
}
```

**关键设计**：

1. **rayon 全局线程池 vs 自建池**：用 `ThreadPoolBuilder::build()` 创建局部线程池，避免占用 rayon 默认全局池
2. **`pool.install(...)`**：让 closure 在该池内执行
3. **预创建目录**：先一次性创建所有需要的子目录，并行处理时不再触碰文件系统创建目录
4. **Mutex 聚合**：每个任务完成后用 `Mutex<CompressReport>` 汇总。注意：lock 持有时间极短（只做几个数字加法），不会成为瓶颈
5. **取消语义**：每个任务入口检查 `is_cancelled`，已开始的仍会跑完（无法强制中断正在编码的 JPEG），但未开始的会被跳过

#### 单任务处理：`process_task`

```rust
fn process_task(task, opts, compressor, progress) -> TaskOutcome {
    let in_size = std::fs::metadata(&task.input)?.len();

    // 跳过小文件
    if opts.skip_if_smaller && in_size <= opts.max_size.bytes {
        std::fs::copy(&task.input, &task.output)?;
        return Ok((in_size, in_size));
    }

    let img = decoder::load_image(&task.input)?;
    let bytes = compressor.compress_to_size(&img, ...)?;
    std::fs::write(&task.output, &bytes)?;
    Ok((in_size, bytes.len() as u64))
}
```

`TaskOutcome` 枚举：
```rust
enum TaskOutcome {
    Ok { in_size: u64, out_size: u64 },
    Failed { msg: String, in_size: Option<u64> },
    Cancelled,
}
```

---

### `src/progress.rs` — 进度回调

抽象的进度报告接口，让 CLI 和 GUI 复用同一份压缩逻辑：

```rust
pub trait ProgressReporter: Send + Sync {
    fn on_start(&self, total: usize);
    fn on_file_start(&self, name: &Path);
    fn on_file_done(&self, name: &Path, ok: bool, msg: Option<&str>);
    fn on_finish(&self, report: &CompressReport);
    fn is_cancelled(&self) -> bool;
}

pub struct NullProgress;  // 不做任何事，CLI 用
```

**设计要点**：
- 用 `&self` 而非 `&mut self`：可以在线程间共享同一实例
- `Send + Sync`：确保线程安全
- CLI 用 `NullProgress`，GUI 实现一个 `Reporter(tx, cancel_flag)` 把事件发到 channel

---

### `src/settings.rs` — GUI 持久化

```rust
pub struct AppSettings {
    pub last_input: Option<PathBuf>,
    pub last_output: Option<PathBuf>,
    pub max_size_kb: u32,
    pub format: Format,
    pub min_quality: u8,
    pub max_quality: u8,
    pub scale_step: f32,
    pub preserve_structure: bool,
    pub skip_if_smaller: bool,
    pub save_log: bool,
}
```

- 用 `directories` crate 获取 `%APPDATA%\imgpress\` 路径
- 用 `serde_json` 序列化到 `settings.json`
- 启动时 `load()`，用户点「开始」时 `save()`（避免半填状态污染）

---

### `src/cli.rs` — 命令行解析

用 `clap` derive 宏：

```rust
#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    Cli(CliArgs),
}

#[derive(Parser)]
pub struct CliArgs {
    #[arg(short, long)]
    pub input: PathBuf,

    #[arg(short, long)]
    pub output: PathBuf,

    #[arg(long, default_value = "500kb")]
    pub max_size: String,

    // ...
}
```

- 无子命令时 `command` 为 `None` → 启动 GUI
- `cli` 子命令带 `CliArgs` 参数

---

### `src/gui/` — 图形界面

#### 模块划分

```
src/gui/
├── mod.rs       # 入口：eframe::run_native + 主题 + 字体
├── state.rs     # AppState（UI 状态 + 后台任务）
├── widgets.rs   # 自定义 widget helper
└── worker.rs    # （早期版本的 worker 实现，现已并入 state）
```

#### `mod.rs` — 启动与主题

```rust
pub fn run() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([820.0, 760.0])
            .with_min_inner_size([700.0, 600.0])
            .with_title("imgpress")
            .with_app_id("imgpress"),
        ..Default::default()
    };
    eframe::run_native("imgpress", options, Box::new(|cc| {
        install_cjk_font(&cc.egui_ctx);
        apply_theme(&cc.egui_ctx);
        Ok(Box::new(state::AppState::new(cc)))
    }))
}
```

**`install_cjk_font`**：egui 默认字体（Hack）不含中文字形，必须加载系统 CJK 字体。流程：

```rust
let mut fonts = FontDefinitions::default();
let bytes = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc")?;  // 微软雅黑
fonts.font_data.insert("cjk".to_owned(), FontData {
    font: Cow::Owned(bytes),
    index: 0,
    tweak: Default::default(),
});
fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "cjk".to_owned());
fonts.families.get_mut(&FontFamily::Monospace).unwrap().insert(0, "cjk".to_owned());
ctx.set_fonts(fonts);
```

- 优先级最高（插入到 list 头部），中文渲染会优先用 msyh
- 多个平台 fallback：Windows / macOS / Linux
- TTC 文件支持：msyh.ttc 是字体集合，`index` 字段指定第几个字体（0 = Regular）

**`apply_theme`**：自定义深色主题色板：
- 背景 RGB(30, 33, 40)
- 文字 RGB(225, 225, 230)
- 强调色 RGB(60, 130, 220)（蓝）
- 错误色 RGB(200, 80, 80)（红）

#### `state.rs` — AppState

GUI 的核心数据结构：

```rust
pub struct AppState {
    // 表单
    pub input_path: String,
    pub output_path: String,
    pub max_size_kb: u32,
    pub format: Format,
    pub min_quality: u8,
    pub max_quality: u8,
    pub scale_step: f32,
    pub preserve_structure: bool,
    pub skip_if_smaller: bool,
    pub save_log: bool,

    // 进度
    pub total: usize,
    pub processed: usize,
    pub failed_count: usize,
    pub current_file: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub started_at: Option<Instant>,

    // 日志
    pub log_lines: Vec<String>,

    // 后台任务
    pub cancel_flag: Option<Arc<AtomicBool>>,
    pub rx: Option<Receiver<WorkerEvent>>,         // 压缩进度
    pub scan_rx: Option<Receiver<ScanResult>>,     // 扫描进度
    pub worker_handle: Option<JoinHandle<()>>,
}
```

**`new(cc)`** 启动时：
1. `AppSettings::load()` 加载保存的设置
2. 如果 `last_input` 非空，触发一次扫描

**`start_scan(path)`** 启动扫描线程：

```rust
let pb = PathBuf::from(path);
self.scan_handle = Some(std::thread::spawn(move || {
    let cancel = Arc::new(AtomicBool::new(false));
    let (count, bytes) = scan_directory(&pb, &cancel);
    let _ = tx.send(ScanResult::Done { count, total_bytes: bytes, .. });
}));
```

**`start()` 启动压缩线程**：

```rust
let opts = CompressOptions { ... };
let cancel = Arc::new(AtomicBool::new(false));
self.cancel_flag = Some(cancel.clone());

let (tx, rx) = crossbeam_channel::unbounded();
self.rx = Some(rx);

self.worker_handle = Some(std::thread::spawn(move || {
    struct Reporter(Sender<WorkerEvent>, Arc<AtomicBool>);
    impl ProgressReporter for Reporter {
        fn on_start(&self, total) { tx.send(Started { total }); }
        fn on_file_start(&self, name) { tx.send(FileStart { ... }); }
        fn on_file_done(&self, name, ok, msg) { tx.send(FileDone { ... }); }
        fn on_finish(&self, report) { tx.send(Finished { ... }); }
        fn is_cancelled(&self) -> bool { self.1.load(...) }
    }
    compress_directory(&input, &output, &opts, &Reporter(tx, cancel));
}));
```

**`poll_events()`** 在每帧调用，从 channel 抽取事件：

```rust
let events: Vec<_> = self.rx.as_ref().unwrap().try_recv().collect();
for ev in events {
    match ev { Started { total } => ..., FileDone { ... } => ..., Finished { ... } => ... }
}
```

注意用 `try_recv().collect()` 一次性把所有待处理事件取出，避免借用冲突。

**`update()`** 是 eframe 主循环：

```rust
fn update(&mut self, ctx, _frame) {
    self.poll_scan();
    self.poll_events();
    if self.running || self.scanning {
        ctx.request_repaint_after(Duration::from_millis(150));  // 150ms 重绘
    }

    let prev = self.input_path.clone();
    draw_ui(self, ctx);  // 画整个界面
    if self.input_path != prev {
        self.start_scan(self.input_path.clone());  // 输入变了，重新扫描
    }
}
```

**`draw_ui`** 是布局函数。整个窗口分 5 个 section：
1. 路径（带实时扫描统计）
2. 压缩参数（拖动值、滑块）
3. 选项（复选框）
4. 进度（条 + 详细统计 + 当前文件）
5. 日志（滚动区域）

#### `widgets.rs` — 辅助函数

```rust
pub fn section(ui, title, icon, add) { /* Frame::group */ }
pub fn path_row(ui, label, value, dir) { /* TextEdit + Browse */ }
pub fn info_line(ui, icon, text, color) { /* 带图标的灰文字 */ }
pub fn stat_row(ui, items) { /* 多列统计 */ }
pub fn format_bytes(b) -> String { /* "1.23 MB" */ }
pub fn format_duration(secs) -> String { /* "1:23" */ }
pub fn primary_button(ui, text, enabled) { /* 蓝色按钮 */ }
pub fn danger_button(ui, text, enabled) { /* 红色按钮 */ }
```

---

### `build.rs` — 构建脚本

```rust
fn main() {
    embed_resource::compile("assets/icon.rc", embed_resource::NONE);
}
```

- `cargo build` 时自动执行
- `embed_resource::compile` 调用 MSVC 的 `rc.exe` 把 `icon.rc` 编译成资源文件
- 然后链接器把资源合并到最终 exe 的 `.rsrc` section

---

### `assets/icon.rc` — 资源脚本

```rc
1 ICON "icon.ico"
```

- `1` 是资源 ID（任意整数）
- `ICON` 是资源类型
- `"icon.ico"` 是相对于 .rc 文件的路径

---

## 关键算法详解

### 二分搜索压缩

**为什么二分有效？**

JPEG 编码中，量化表的缩放因子由 quality 决定（Q=50 时是基准，Q 每加 1，量化步长 × ~0.96；每减 1，× ~1.04）。编码后字节数与 Q 大致呈反比，但有噪声（受图像内容影响）。

实际测试：

```
1MB JPEG (5000x3000 风景照) 不同 Q 的输出大小：
Q=95 → 1200 KB
Q=85 → 720 KB
Q=75 → 510 KB
Q=65 → 380 KB
Q=50 → 250 KB
Q=30 → 140 KB
Q=10 → 60 KB
```

视觉上看起来：从 Q=50 到 Q=85，文件大小变化很陡；Q=85 以上，质量增益边际递减；Q=30 以下，质量崩溃。

二分搜索的代价：
- 范围 [20, 95] 共 76 个值
- 二分需要 ⌈log2(76)⌉ = 7 步
- 相比线性扫描的 76 步，**快了 10 倍**

### 缩放回退

当最低质量 Q=20 仍超 target 时，需要降分辨率。

```
原图 5000x3000 = 1500万像素
× 0.85 → 4250x2550 = 1083万像素（× 0.7225）
× 0.85 → 3612x2167 = 783万像素（× 0.522）
× 0.85 → 3070x1842 = 565万像素（× 0.377）
...
× 8 轮 → 5000 × 0.85⁸ ≈ 1507 像素
```

像素减少让 JPEG 编码可用更少比特表示细节，等效于"质量提升"。

**为什么用 Lanczos3 而不是默认的 Triangle 滤波器？**

- Triangle：双线性插值，快但模糊
- Catmull：三次卷积，锐利但有振铃
- Lanczos3：sinc 函数近似，最高画质，CPU 略贵

压缩是一次性任务，画质优先，所以选 Lanczos3。

---

## 线程模型

```
┌──────────────────────────┐
│   egui 主线程            │
│   - 处理用户输入          │
│   - 画 UI                │
│   - poll_events()        │  ◀── 接收 worker 事件
└────────────┬─────────────┘
             │ std::thread::spawn
             ▼
┌──────────────────────────┐
│   Worker 线程            │
│   - rayon 全局线程池     │  ◀── 并行处理所有图片
│   - 调 compressor        │
│   - 写文件               │
└──────────────────────────┘
             │
             │ Sender::send (crossbeam-channel)
             ▼
       channel → 主线程 poll
```

**线程安全保证**：

1. `Compressor` 是无状态的（每次 encode 内部新建 `JpegEncoder`/`Encoder`，自己 alloc/dealloc）
2. `Reporter` impl 用 `Sender` 和 `AtomicBool`，都是线程安全
3. `CompressReport` 在主线程通过 `Mutex` 保护，锁粒度极小
4. 进度回调（`on_file_start`、`on_file_done`）的顺序由 rayon 决定，**不保证与文件遍历顺序一致**，但总数和成功/失败计数是准确的

**取消语义**：

- 取消检查发生在每个任务入口和每次 encode 之前
- 已开始编码的 JPEG/WebP 不能强制中断（编码器内部状态机不重入）
- 取消后返回 `Cancelled`，主线程更新 UI

---

## 依赖说明

| Crate | 版本 | 用途 | 备注 |
|---|---|---|---|
| `clap` | 4.5 | CLI 解析 | derive 模式 |
| `eframe` | 0.29 | GUI 框架 | egui + winit + glow |
| `egui` | 0.29 | 即时模式 UI | 0.29 是当前稳定版 |
| `rfd` | 0.14 | 原生文件对话框 | xdg-portal/tokio |
| `open` | 5.3 | 跨平台 open | 打开文件夹 |
| `image` | 0.25 | 图像解码 + JPEG 编码 | 支持 PNG/JPG/WebP/BMP/TIFF 等 |
| `mupdf` | 0.8 | PDF 渲染 | 全页光栅化，正确处理扫描型 PDF |
| `webp` | 0.3 | WebP 编码 | 绑定了 libwebp |
| `rayon` | 1.10 | 数据并行 | par_iter |
| `crossbeam-channel` | 0.5 | 线程间通信 | MPMC channel |
| `anyhow` | 1.0 | 灵活错误处理 | CLI 层用 |
| `thiserror` | 1.0 | 定义错误类型 | lib 层用 |
| `serde` + `serde_json` | 1.0 | 序列化 | 设置持久化 |
| `directories` | 5.0 | 系统路径 | %APPDATA% 等 |
| `indicatif` | 0.17 | 进度条 | CLI 模式 |
| `log` + `env_logger` | 0.4/0.11 | 日志 | 标准 log crate |
| `embed-resource` | 2 | 图标嵌入 | 构建脚本 |

**关于依赖约束**：实现"单 exe 分发"的所有依赖均为**纯 Rust 或静态链接的 C 库**。无需任何额外 dll：`webp` 通过 cc 静态链接进二进制；`mupdf` 通过 mupdf-sys 静态链接进二进制。

**build-dependencies**：
- `embed-resource` —— 仅在编译时需要，不进最终二进制

---

## 编译与发布

### 开发编译

```bash
cargo build                    # debug（带终端）
cargo run                      # 编译并运行
cargo run -- cli --help        # 编译并运行 CLI
```

### Release 编译

```bash
cargo build --release
```

产物：`target/release/imgpress.exe`（约 13 MB）

**Release profile**（在 `Cargo.toml`）：
```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
```

- `lto = "thin"`：跨 crate 链接时优化
- `codegen-units = 1`：单代码生成单元，进一步优化（编译慢）

### 编译环境要求（Windows）

- **Rust**：1.75+（实测 1.96）
- **MSVC**：Visual Studio 2022 或 2026（toolset v143 或 v145）
- **LLVM / Clang**：mupdf-sys 编译期通过 bindgen 生成 C 绑定，需要 `LIBCLANG_PATH` 指向 `clang.dll`（运行时不需要 LLVM）
- **环境变量 `MUPDF_MSVC_PLATFORM_TOOLSET`**：当系统只有 Visual Studio 2026（toolset v145）时必须设置：

```bash
# cmd
set MUPDF_MSVC_PLATFORM_TOOLSET=v145
cargo build --release

# PowerShell
$env:MUPDF_MSVC_PLATFORM_TOOLSET = "v145"
cargo build --release
```

不设置时 mupdf-sys 会回退到 v142，与本机 VS 不匹配会导致 C 端编译失败。

### 交叉编译

当前不支持 Windows 之外的平台（mupdf / webp 都有 C 依赖）。

### 打包发布

直接分发 `target/release/imgpress.exe` 单文件即可。无需运行时依赖。

### 清理

```bash
cargo clean            # 清理 target 目录
cargo update           # 更新依赖
```

---

## 性能数据

测试环境：i7-12700H（14 核 20 线程），测试数据 182 张 PNG（480 MB）+ 12 个 PDF

### 全量压缩（182 图 + 12 PDF = 230 输出）

| 任务 | 单线程（debug） | 多线程（release） | 加速 |
|---|---|---|---|
| 纯图片压缩到 500KB（保留结构） | 556 秒 | 135 秒 | **4.1x** |
| 纯图片压缩到 500KB（扁平化） | - | 130 秒 | - |
| 包含 PDF（pdf 解码 + 压缩） | - | 126 秒 | - |

**全量最终**（用 `D:\code\rust\imgpress\photos` 测试集）：

```
Total: 194 (182 png/jpg + 12 pdf)
Output: 230 (182 + 48 pdf pages)
Success: 230 / Failed: 0
Bytes: 492.73 MB → 85.46 MB (17.3%)
耗时: 126 秒
```

### 单张耗时分解（5000x3000 PNG → 500KB JPEG）

| 步骤 | 时间 |
|---|---|
| 读取 PNG | 0.15s |
| 解码到 RGBA8 | 0.20s |
| image crate JPEG 编码（1 次） | 0.10s |
| 二分搜索 7 次编码 | 0.70s |
| 写文件 | 0.05s |
| **总计** | **~1.2s** |

### PDF 单页耗时分解（4 页 PDF，每页 ≈ 220KB → 200KB）

| 步骤 | 时间 |
|---|---|
| mupdf Document::open | 0.03s |
| load_page + to_pixmap（2× 渲染） | 0.20s |
| pixmap → DynamicImage 像素搬运 | 0.05s |
| 二分压缩 + 写文件 | ~1.2s |
| **总计** | **~1.5s/页** |

PDF 处理与普通图片接近，瓶颈仍在压缩侧。

CPU 占用：多线程时约 80%（14 核中 11-12 个活跃）。

---

## 已知限制与扩展点

### 已知限制

1. **PDF 仅支持位图型**（扫描件）：矢量 PDF / 纯文本 PDF / 加密 PDF 报错跳过。如需矢量渲染：
   - 短期：手动把 PDF 转成图片后再压（用系统自带工具）
   - 长期：集成 `resvg`（矢量）或商业 PDF 渲染 SDK（但会破坏"单 exe"原则）
2. **PDF 不支持的 filter**：LZWDecode / CCITTFaxDecode / JBIG2Decode 报错跳过。已实现：FlateDecode（zlib）、DCTDecode（pass-through）。扩展见扩展点 1
3. **PDF 每页只取首张图**：多图页面（如报告带 logo）只取首张，多图合并等未来工作
4. **HEIC/AVIF 不支持**：`image` crate 0.25 不带 AVIF 解码，需要时手动启用 `image` 的 `avif` feature 并依赖 `ravif`
5. **取消有延迟**：当前编码中的 JPEG/WebP 必须跑完才能取消
6. **内存峰值**：大图解码到 RGBA8 后会占 `width × height × 4` 字节。5000x3000 = 60 MB。最大图不要超过 1 亿像素（约 10000x10000）
7. **EXIF 丢弃**：当前 JPEG 编码不带 EXIF 信息

### 扩展点

1. **PDF 加密支持**：mupdf 0.8 已支持加密 PDF 读取，需要在 `Document::open` 时传入密码参数
2. **PDF 更多配置**：可调节渲染 DPI（当前固定 2× ≈ 144 DPI）；按页面尺寸选择不同压缩策略
3. **新格式支持**：实现 `Codec` trait + 在 `Format` 枚举添加新成员
4. **新的 CLI 选项**：在 `CliArgs` 加字段，在 `build_options` 映射到 `CompressOptions`
5. **新的 GUI section**：在 `draw_ui` 中加 `widgets::section(...)` 调用
6. **预设**：「高质量压缩」「极致压缩」「存档」等预设按钮
7. **EXIF 保留**：用 `kamadak-exif` crate 读取，写入 JPEG 的 APP1 marker
8. **多目标大小**：同时输出多个尺寸（如缩略图 + 原图压缩）
9. **watch 模式**：监控源目录，新文件自动压缩

---

## 许可证

源码使用 MIT 许可证。

依赖说明：
- `mupdf` / `mupdf-sys` — **AGPL 3.0**：运行时静默链接 mupdf 库本身（不修改其源码），静态链接到本程序后整包须遵守 AGPL（包含显著的源码披露义务）。如需闭源分发，可换用商业 mupdf 授权或切换到 `pdfium-render`（需配合分发 `pdfium.dll`）
- `webp` — BSD
- `egui` / `eframe` — MIT / Apache-2.0
- 其他全部 MIT / Apache-2.0

整体可作为商业用途分发。

---

**项目结构**：

```
imgpress/
├── Cargo.toml
├── build.rs
├── README.md
├── assets/
│   ├── icon.ico
│   └── icon.rc
└── src/
    ├── main.rs
    ├── lib.rs
    ├── error.rs
    ├── config.rs
    ├── decoder.rs
    ├── compressor.rs
    ├── pipeline.rs
    ├── progress.rs
    ├── settings.rs
    ├── cli.rs
    ├── pdf/
    │   └── mod.rs           # PDF 解析 + filter 解码
    ├── codec/
    │   ├── mod.rs
    │   ├── jpeg.rs
    │   └── webp.rs
    └── gui/
        ├── mod.rs
        ├── state.rs
        ├── widgets.rs
        └── worker.rs
```

---

> 最后更新：2026-07-12