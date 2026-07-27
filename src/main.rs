#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use imgpress::cli::{Cli, Command};
use imgpress::config::{validate_options, CompressOptions, Format, SizeLimit};
use imgpress::log::write_log_file;
use imgpress::pipeline::{compress_directory, CompressReport};
use imgpress::progress::CliProgress;
use std::time::Instant;

type CliResult<T> = std::result::Result<T, String>;

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();
    match cli.command {
        None => run_gui(),
        Some(Command::Cli(args)) => run_cli(args),
    }
}

fn run_cli(args: imgpress::cli::CliArgs) {
    let opts = match build_options(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("配置错误: {}", e);
            std::process::exit(2);
        }
    };

    log::info!(
        "start: input={} output={} target={}KB",
        opts.input.display(),
        opts.output.display(),
        opts.max_size.bytes / 1024
    );

    let start = Instant::now();
    let report = match compress_directory(&opts.input, &opts.output, &opts, &CliProgress) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("致命错误: {}", e);
            std::process::exit(1);
        }
    };

    let elapsed = start.elapsed();
    print_report(&report, elapsed);

    match imgpress::log::log_file_path() {
        Some(path) => {
            if let Err(e) = write_log_file(&report, &opts, &path) {
                eprintln!("写入日志失败: {}", e);
            } else {
                eprintln!("日志已写入: {}", path.display());
            }
        }
        None => eprintln!("无法解析配置目录路径，跳过日志写入"),
    }
}

fn build_options(args: imgpress::cli::CliArgs) -> CliResult<CompressOptions> {
    if args.delete_source && !args.yes {
        return Err("删除源文件需要同时传入 --yes 确认".into());
    }
    let opts = CompressOptions {
        input: args.input,
        output: args.output,
        max_size: parse_size(&args.max_size)?,
        format: match args.format.to_lowercase().as_str() {
            "jpeg" | "jpg" => Format::Jpeg,
            "webp" => Format::WebP,
            other => return Err(format!("unsupported format: {}", other)),
        },
        min_quality: args.min_quality,
        max_quality: args.max_quality,
        scale_step: args.scale_step,
        max_scales: args.max_scales,
        skip_if_smaller: args.skip_if_smaller && !args.no_skip_if_smaller,
        recursive: !args.no_recursive,
        organize_after_success: args.organize_after_success,
        delete_source: args.delete_source,
    };
    validate_options(&opts)?;
    Ok(opts)
}

fn parse_size(s: &str) -> CliResult<SizeLimit> {
    let s = s.trim().to_lowercase();
    let (num, mult) = if let Some(n) = s.strip_suffix("kb") {
        (n.trim(), 1024u64)
    } else if let Some(n) = s.strip_suffix("mb") {
        (n.trim(), 1024u64 * 1024)
    } else if let Some(n) = s.strip_suffix("b") {
        (n.trim(), 1u64)
    } else {
        (s.as_str(), 1024u64)
    };
    let v: f64 = num
        .parse()
        .map_err(|e| format!("invalid size '{}': {}", s, e))?;
    Ok(SizeLimit::from_bytes((v * mult as f64) as u64))
}

fn print_report(report: &CompressReport, elapsed: std::time::Duration) {
    println!();
    println!("========================================");
    println!("完成，用时 {:.2?}", elapsed);
    println!("  总数:   {}", report.total);
    println!("  成功:   {}", report.success);
    println!("  失败:   {}", report.failed.len());
    println!(
        "  体积:   {:.2} MB -> {:.2} MB ({:.1}%)",
        report.bytes_in as f64 / 1_048_576.0,
        report.bytes_out as f64 / 1_048_576.0,
        if report.bytes_in > 0 {
            (report.bytes_out as f64 / report.bytes_in as f64) * 100.0
        } else {
            0.0
        }
    );
    if !report.failed.is_empty() {
        println!("\n失败文件:");
        for (p, msg) in &report.failed {
            println!("  {} - {}", p.display(), msg);
        }
    }
    use imgpress::source::SourceAction;
    match &report.source_action {
        SourceAction::NotRequested => {}
        SourceAction::Deleted => println!("\n源文件: 已删除"),
        SourceAction::Skipped { reason } => {
            println!("\n源文件: 未删除 ({})", reason);
        }
        SourceAction::Errored { error } => {
            println!("\n源文件: 删除失败 ({})", error);
        }
    }
}

fn run_gui() {
    if let Err(e) = imgpress::gui::run() {
        log::error!("GUI failed: {}", e);
        std::process::exit(1);
    }
}
