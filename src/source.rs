use std::path::Path;

#[derive(Debug, Clone, Default)]
pub enum SourceAction {
    #[default]
    NotRequested,
    Deleted,
    Skipped {
        reason: String,
    },
    Errored {
        error: String,
    },
}

pub fn delete_source(input: &Path, output: &Path) -> std::io::Result<()> {
    if input.is_file() {
        return std::fs::remove_file(input);
    }
    if !input.is_dir() {
        return Ok(());
    }

    let skip_path = std::fs::canonicalize(output).ok().filter(|out| {
        std::fs::canonicalize(input)
            .map(|inp| out.starts_with(&inp))
            .unwrap_or(false)
    });

    for entry in std::fs::read_dir(input)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ref skip) = skip_path {
            if std::fs::canonicalize(&path)
                .map(|p| p.starts_with(skip))
                .unwrap_or(false)
            {
                log::info!("跳过输出目录: {}", path.display());
                continue;
            }
        }
        let result = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if let Err(e) = result {
            log::warn!("删除 {} 失败: {}", path.display(), e);
        }
    }
    Ok(())
}
