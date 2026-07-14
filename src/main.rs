#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use imgpress::cli::{Cli, Command};
use imgpress::config::{CompressOptions, Format, SizeLimit};
use imgpress::pipeline::{compress_directory, CompressReport};
use imgpress::progress::CliProgress;
use std::time::Instant;

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
            eprintln!("config error: {}", e);
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
            eprintln!("fatal: {}", e);
            std::process::exit(1);
        }
    };

    let elapsed = start.elapsed();
    print_report(&report, elapsed);
}

fn build_options(args: imgpress::cli::CliArgs) -> anyhow::Result<CompressOptions> {
    Ok(CompressOptions {
        input: args.input,
        output: args.output,
        max_size: parse_size(&args.max_size)?,
        format: match args.format.to_lowercase().as_str() {
            "jpeg" | "jpg" => Format::Jpeg,
            "webp" => Format::WebP,
            other => anyhow::bail!("unsupported format: {}", other),
        },
        min_quality: args.min_quality.min(100),
        max_quality: args.max_quality.min(100),
        scale_step: args.scale_step.clamp(0.1, 0.99),
        max_scales: args.max_scales,
        preserve_structure: args.preserve_structure,
        skip_if_smaller: args.skip_if_smaller,
        recursive: !args.no_recursive,
    })
}

fn parse_size(s: &str) -> anyhow::Result<SizeLimit> {
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
    let v: f64 = num.parse()?;
    Ok(SizeLimit::from_bytes((v * mult as f64) as u64))
}

fn print_report(report: &CompressReport, elapsed: std::time::Duration) {
    println!();
    println!("========================================");
    println!("Done in {:.2?}", elapsed);
    println!("  Total:    {}", report.total);
    println!("  Success:  {}", report.success);
    println!("  Failed:   {}", report.failed.len());
    println!(
        "  Bytes:    {:.2} MB -> {:.2} MB ({:.1}%)",
        report.bytes_in as f64 / 1_048_576.0,
        report.bytes_out as f64 / 1_048_576.0,
        if report.bytes_in > 0 {
            (report.bytes_out as f64 / report.bytes_in as f64) * 100.0
        } else {
            0.0
        }
    );
    if !report.failed.is_empty() {
        println!("\nFailed files:");
        for (p, msg) in &report.failed {
            println!("  {} - {}", p.display(), msg);
        }
    }
}

fn run_gui() {
    if let Err(e) = imgpress::gui::run() {
        log::error!("GUI failed: {}", e);
        std::process::exit(1);
    }
}