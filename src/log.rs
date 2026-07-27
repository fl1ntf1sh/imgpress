use crate::config::CompressOptions;
use crate::pipeline::{CompressReport, OrganizeAction};
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
        let diy = if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
            366
        } else {
            365
        };
        if rem < diy {
            break;
        }
        rem -= diy;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
    let dim: &[i64] = if leap {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut mo = 1u64;
    for &d in dim {
        if rem < d {
            break;
        }
        rem -= d;
        mo += 1;
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        y,
        mo,
        rem + 1,
        h,
        m,
        s
    )
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

    let in_mb = report.bytes_in as f64 / 1_048_576.0;
    let out_mb = report.bytes_out as f64 / 1_048_576.0;
    let ratio = if report.bytes_in > 0 {
        report.bytes_out as f64 / report.bytes_in as f64 * 100.0
    } else {
        0.0
    };

    writeln!(f)?;
    writeln!(
        f,
        "============================================================"
    )?;
    writeln!(f, "运行时间: {}", format_utc_now())?;
    writeln!(
        f,
        "------------------------------------------------------------"
    )?;
    writeln!(f, "[路径]")?;
    writeln!(f, "源路径:   {}", opts.input.display())?;
    writeln!(f, "输出路径: {}", opts.output.display())?;
    writeln!(f)?;
    writeln!(f, "[参数]")?;
    writeln!(f, "目标大小:       {} KB", opts.max_size.bytes / 1024)?;
    writeln!(f, "输出格式:       {:?}", opts.format)?;
    writeln!(
        f,
        "质量范围:       {}-{}",
        opts.min_quality, opts.max_quality
    )?;
    writeln!(f, "缩放步长:       {:.0}%", opts.scale_step * 100.0)?;
    writeln!(f, "最大缩放轮数:   {}", opts.max_scales)?;
    writeln!(f, "跳过小文件:     {}", yes_no(opts.skip_if_smaller))?;
    writeln!(f, "递归扫描:       {}", yes_no(opts.recursive))?;
    writeln!(f, "成功后整理文件: {}", yes_no(opts.organize_after_success))?;
    writeln!(f, "成功后删除源:   {}", yes_no(opts.delete_source))?;
    writeln!(f)?;
    writeln!(f, "[结果]")?;
    writeln!(f, "总任务数:       {}", report.total)?;
    writeln!(f, "成功数:         {}", report.success)?;
    writeln!(f, "失败数:         {}", report.failed.len())?;
    writeln!(
        f,
        "输入体积:       {} bytes ({:.2} MB)",
        report.bytes_in, in_mb
    )?;
    writeln!(
        f,
        "输出体积:       {} bytes ({:.2} MB)",
        report.bytes_out, out_mb
    )?;
    writeln!(f, "输出/输入比例:  {:.1}%", ratio)?;
    writeln!(f)?;
    writeln!(f, "[源文件处理]")?;
    match &report.source_action {
        SourceAction::NotRequested => {
            writeln!(f, "状态: 未请求删除")?;
        }
        SourceAction::Deleted => {
            writeln!(f, "状态: 已删除源文件")?;
        }
        SourceAction::Skipped { reason } => {
            writeln!(f, "状态: 已跳过")?;
            writeln!(f, "原因: {}", reason)?;
        }
        SourceAction::Errored { error } => {
            writeln!(f, "状态: 删除失败")?;
            writeln!(f, "错误: {}", error)?;
        }
    }
    writeln!(f)?;
    writeln!(f, "[输出整理]")?;
    match &report.organize_action {
        OrganizeAction::NotRequested => {
            writeln!(f, "状态: 未请求整理")?;
        }
        OrganizeAction::Organized { moved, skipped } => {
            writeln!(f, "状态: 已整理")?;
            writeln!(f, "移动文件数: {}", moved)?;
            writeln!(f, "跳过文件数: {}", skipped)?;
        }
        OrganizeAction::Skipped { reason } => {
            writeln!(f, "状态: 已跳过")?;
            writeln!(f, "原因: {}", reason)?;
        }
        OrganizeAction::Errored { error } => {
            writeln!(f, "状态: 整理失败")?;
            writeln!(f, "错误: {}", error)?;
        }
    }
    if !report.run_log.is_empty() {
        writeln!(f)?;
        writeln!(f, "[运行日志]")?;
        for line in &report.run_log {
            writeln!(f, "{}", line)?;
        }
    }
    if !report.failed.is_empty() {
        writeln!(f)?;
        writeln!(f, "[失败文件]")?;
        for (index, (path, msg)) in report.failed.iter().enumerate() {
            writeln!(f, "{}. {}", index + 1, path.display())?;
            writeln!(f, "   原因: {}", msg)?;
        }
    }
    f.flush()
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "是"
    } else {
        "否"
    }
}
