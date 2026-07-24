use crate::config::Format;
use crate::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileTask {
    pub input: PathBuf,
    pub output: PathBuf,
}

struct CollectContext<'a> {
    input_root: &'a Path,
    output_root: &'a Path,
    recursive: bool,
    preserve_structure: bool,
    format: Format,
    reserved_outputs: HashSet<PathBuf>,
}

const SUPPORTED_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "bmp", "tiff", "tif", "gif", "ico", "ppm", "pgm", "pbm", "pdf",
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
    let mut ctx = CollectContext {
        input_root: input,
        output_root: output,
        recursive,
        preserve_structure,
        format,
        reserved_outputs: HashSet::new(),
    };
    if input.is_file() {
        if is_supported(input) {
            let out = unique_output(
                output,
                input,
                preserve_structure,
                format,
                &mut ctx.reserved_outputs,
            );
            tasks.push(FileTask {
                input: input.to_path_buf(),
                output: out,
            });
        }
        return Ok(tasks);
    }
    walk(input, &mut tasks, &mut ctx)?;
    Ok(tasks)
}

fn walk(dir: &Path, tasks: &mut Vec<FileTask>, ctx: &mut CollectContext<'_>) -> Result<()> {
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
            if ctx.recursive {
                walk(&path, tasks, ctx)?;
            }
            continue;
        }

        if !is_supported(&path) {
            continue;
        }

        let rel = path.strip_prefix(ctx.input_root).unwrap_or(&path);
        let (out_dir, target_name) =
            build_output_path(rel, ctx.output_root, ctx.preserve_structure, ctx.format);
        let out_path = unique_in_dir_reserved(&out_dir, &target_name, &mut ctx.reserved_outputs);
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
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
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

pub(crate) fn output_extension(format: Format) -> &'static str {
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

pub(crate) fn unique_in_dir(dir: &Path, name: &str) -> PathBuf {
    let mut reserved = HashSet::new();
    unique_in_dir_reserved(dir, name, &mut reserved)
}

fn unique_in_dir_reserved(dir: &Path, name: &str, reserved: &mut HashSet<PathBuf>) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() && !reserved.contains(&candidate) {
        reserved.insert(candidate.clone());
        return candidate;
    }
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jpg");
    for i in 1..u16::MAX {
        let new_name = format!("{}_{}.{}", stem, i, ext);
        let p = dir.join(&new_name);
        if !p.exists() && !reserved.contains(&p) {
            reserved.insert(p.clone());
            return p;
        }
    }
    let mut fallback = candidate;
    let mut counter = 0u64;
    while fallback.exists() || reserved.contains(&fallback) {
        counter += 1;
        fallback = dir.join(format!("{}_{}.{}", stem, counter, ext));
    }
    reserved.insert(fallback.clone());
    log::warn!(
        "unique_in_dir exhausted, fallback to {}",
        fallback.display()
    );
    fallback
}

fn unique_output(
    output_root: &Path,
    input: &Path,
    preserve_structure: bool,
    format: Format,
    reserved_outputs: &mut HashSet<PathBuf>,
) -> PathBuf {
    let (out_dir, name) = build_output_path(input, output_root, preserve_structure, format);
    unique_in_dir_reserved(&out_dir, &name, reserved_outputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let unique = format!(
            "imgpress_{}_{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    #[test]
    fn collect_files_reserves_unique_outputs_with_same_stem() {
        let input = temp_path("same_stem_input");
        let output = temp_path("same_stem_output");
        std::fs::create_dir_all(&input).unwrap();
        std::fs::write(input.join("a.png"), []).unwrap();
        std::fs::write(input.join("a.jpg"), []).unwrap();

        let tasks = collect_files(&input, &output, false, true, Format::Jpeg).unwrap();
        let mut outputs = tasks
            .iter()
            .map(|task| task.output.clone())
            .collect::<Vec<_>>();
        outputs.sort();
        outputs.dedup();

        assert_eq!(tasks.len(), 2);
        assert_eq!(outputs.len(), 2);

        let _ = std::fs::remove_dir_all(&input);
    }

    #[test]
    fn collect_files_avoids_existing_output_file() {
        let input = temp_path("existing_output_input");
        let output = temp_path("existing_output_output");
        std::fs::create_dir_all(&input).unwrap();
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(input.join("a.png"), []).unwrap();
        std::fs::write(output.join("a.jpg"), []).unwrap();

        let tasks = collect_files(&input, &output, false, true, Format::Jpeg).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].output.file_name().unwrap(), "a_1.jpg");

        let _ = std::fs::remove_dir_all(&input);
        let _ = std::fs::remove_dir_all(&output);
    }
}
