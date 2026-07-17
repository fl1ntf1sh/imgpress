use crate::config::CompressOptions;
use crate::pipeline::CompressReport;
use crate::source::SourceAction;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn log_file_path() -> Option<PathBuf> {
    Some(crate::app_data_dir()?.join("log.txt"))
}

fn format_utc_now() -> String {
    let d = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let secs_of_day = d % 86400;
    let days = d / 86400;
    let h = secs_of_day / 3600;
    let m = (secs_of_day % 3600) / 60;
    let s = secs_of_day % 60;
    let mut y = 1970i64;
    let mut rem = days as i64;
    loop {
        let diy = if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) { 366 } else { 365 };
        if rem < diy { break; }
        rem -= diy;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
    let dim: &[i64] = if leap { &[31,29,31,30,31,30,31,31,30,31,30,31] } else { &[31,28,31,30,31,30,31,31,30,31,30,31] };
    let mut mo = 1u64;
    for &d in dim {
        if rem < d { break; }
        rem -= d;
        mo += 1;
    }
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", y, mo, rem + 1, h, m, s)
}

pub fn write_log_file(
    report: &CompressReport,
    opts: &CompressOptions,
    path: &Path,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f)?;
    writeln!(f, "=== Run {} ===", format_utc_now())?;
    writeln!(f, "Input:       {}", opts.input.display())?;
    writeln!(f, "Output:      {}", opts.output.display())?;
    writeln!(f, "Target:      {} KB", opts.max_size.bytes / 1024)?;
    writeln!(f, "Format:      {:?}", opts.format)?;
    writeln!(f, "Quality:     {}-{}", opts.min_quality, opts.max_quality)?;
    writeln!(f, "Scale step:  {}", opts.scale_step)?;
    writeln!(f, "Structure:   {}", opts.preserve_structure)?;
    writeln!(f, "Skip small:  {}", opts.skip_if_smaller)?;
    writeln!(f, "Recursive:   {}", opts.recursive)?;
    writeln!(f, "Delete src:  {}", opts.delete_source)?;
    writeln!(f)?;
    writeln!(f, "Total:       {}", report.total)?;
    writeln!(f, "Success:     {}", report.success)?;
    writeln!(f, "Failed:      {}", report.failed.len())?;
    writeln!(f, "Bytes in:    {}", report.bytes_in)?;
    writeln!(f, "Bytes out:   {}", report.bytes_out)?;
    writeln!(f, "Source:")?;
    match &report.source_action {
        SourceAction::NotRequested => {
            writeln!(f, "  not requested")?;
        }
        SourceAction::Deleted => {
            writeln!(f, "  deleted")?;
        }
        SourceAction::Skipped { reason } => {
            writeln!(f, "  skipped: {}", reason)?;
        }
        SourceAction::Errored { error } => {
            writeln!(f, "  error: {}", error)?;
        }
    }
    if !report.failed.is_empty() {
        writeln!(f)?;
        writeln!(f, "--- Failed files ---")?;
        for (path, msg) in &report.failed {
            writeln!(f, "{} - {}", path.display(), msg)?;
        }
    }
    f.flush()
}
