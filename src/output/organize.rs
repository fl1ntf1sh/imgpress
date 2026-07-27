use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrganizeResult {
    pub moved: usize,
    pub skipped: usize,
}

pub fn organize_output_root_by_name(root: &Path) -> std::io::Result<OrganizeResult> {
    let mut result = OrganizeResult {
        moved: 0,
        skipped: 0,
    };

    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = person_name_from_path(&path) else {
            result.skipped += 1;
            continue;
        };

        let target_dir = root.join(name);
        std::fs::create_dir_all(&target_dir)?;
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".to_string());
        let target = unique_target(&target_dir, &file_name);
        std::fs::rename(&path, target)?;
        result.moved += 1;
    }

    Ok(result)
}

fn person_name_from_path(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?.trim();
    let name = stem
        .char_indices()
        .find(|(_, c)| is_name_separator(*c))
        .map(|(index, _)| &stem[..index])
        .unwrap_or(stem)
        .trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn is_name_separator(c: char) -> bool {
    matches!(
        c,
        ' ' | '_' | '-' | '－' | '—' | '–' | '+' | '＋' | ',' | '，' | '、'
    )
}

fn unique_target(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }

    let path = Path::new(file_name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path.extension().and_then(|s| s.to_str());
    for i in 1..u16::MAX {
        let name = match ext {
            Some(ext) => format!("{}_{}.{}", stem, i, ext),
            None => format!("{}_{}", stem, i),
        };
        let target = dir.join(name);
        if !target.exists() {
            return target;
        }
    }

    dir.join(file_name)
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
    fn organizes_only_root_files_by_name_prefix() {
        let root = temp_path("organize_output");
        std::fs::create_dir_all(root.join("张三")).unwrap();
        std::fs::write(root.join("张三_身份证.jpg"), []).unwrap();
        std::fs::write(root.join("李四-照片.jpg"), []).unwrap();
        std::fs::write(root.join("无分隔符.jpg"), []).unwrap();
        std::fs::write(root.join("张三").join("已有.jpg"), []).unwrap();

        let result = organize_output_root_by_name(&root).unwrap();

        assert_eq!(result.moved, 3);
        assert_eq!(result.skipped, 0);
        assert!(root.join("张三").join("张三_身份证.jpg").exists());
        assert!(root.join("李四").join("李四-照片.jpg").exists());
        assert!(root.join("无分隔符").join("无分隔符.jpg").exists());
        assert!(root.join("张三").join("已有.jpg").exists());

        let _ = std::fs::remove_dir_all(&root);
    }
}
