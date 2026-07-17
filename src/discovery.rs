use crate::config::Format;
use crate::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileTask {
    pub input: PathBuf,
    pub output: PathBuf,
}

const SUPPORTED_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "bmp", "tiff", "tif", "gif", "ico",
    "ppm", "pgm", "pbm", "pdf",
];

pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| SUPPORTED_EXTS.iter().any(|s| s.eq_ignore_ascii_case(ext)))
}

pub fn collect_files(
    input: &Path,
    output: &Path,
    recursive: bool,
    preserve_structure: bool,
    format: Format,
) -> Result<Vec<FileTask>> {
    let mut tasks = Vec::new();
    if input.is_file() {
        if is_supported(input) {
            let out = unique_output(output, input, preserve_structure, format);
            tasks.push(FileTask {
                input: input.to_path_buf(),
                output: out,
            });
        }
        return Ok(tasks);
    }
    walk(input, input, output, recursive, preserve_structure, format, &mut tasks)?;
    Ok(tasks)
}

fn walk(
    input_root: &Path,
    dir: &Path,
    output_root: &Path,
    recursive: bool,
    preserve_structure: bool,
    format: Format,
    tasks: &mut Vec<FileTask>,
) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("read_dir({}) failed: {}", dir.display(), e);
            return Ok(());
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            if recursive {
                walk(input_root, &path, output_root, recursive, preserve_structure, format, tasks)?;
            }
            continue;
        }

        if !is_supported(&path) {
            continue;
        }

        let rel = path.strip_prefix(input_root).unwrap_or(&path);
        let (out_dir, target_name) = build_output_path(rel, output_root, preserve_structure, format);
        let out_path = unique_in_dir(&out_dir, &target_name);
        tasks.push(FileTask {
            input: path,
            output: out_path,
        });
    }
    Ok(())
}

fn build_output_path(
    input: &Path,
    output_root: &Path,
    preserve_structure: bool,
    format: Format,
) -> (PathBuf, String) {
    let ext = output_extension(format);
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    if preserve_structure {
        let parent = input.parent().unwrap_or(Path::new(""));
        let d = if parent.as_os_str().is_empty() {
            output_root.to_path_buf()
        } else {
            output_root.join(parent)
        };
        (d, format!("{}.{}", stem, ext))
    } else {
        let prefix = flatten_prefix(input);
        let name = if prefix.is_empty() {
            format!("{}.{}", stem, ext)
        } else {
            format!("{}_{}.{}", prefix, stem, ext)
        };
        (output_root.to_path_buf(), name)
    }
}

pub fn output_extension(format: Format) -> &'static str {
    format.extension()
}

fn flatten_prefix(rel: &Path) -> String {
    rel.parent()
        .unwrap_or(Path::new(""))
        .iter()
        .filter_map(|c| c.to_str())
        .collect::<Vec<_>>()
        .join("_")
}

pub fn unique_in_dir(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let stem = Path::new(name).file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = Path::new(name).extension().and_then(|s| s.to_str()).unwrap_or("jpg");
    for i in 1..u16::MAX {
        let new_name = format!("{}_{}.{}", stem, i, ext);
        let p = dir.join(&new_name);
        if !p.exists() {
            return p;
        }
    }
    let mut fallback = candidate;
    let mut counter = 0u64;
    while fallback.exists() {
        counter += 1;
        fallback = dir.join(format!("{}_{}.{}", stem, counter, ext));
    }
    log::warn!("unique_in_dir exhausted, fallback to {}", fallback.display());
    fallback
}

fn unique_output(
    output_root: &Path,
    input: &Path,
    preserve_structure: bool,
    format: Format,
) -> PathBuf {
    let (out_dir, name) = build_output_path(input, output_root, preserve_structure, format);
    unique_in_dir(&out_dir, &name)
}
