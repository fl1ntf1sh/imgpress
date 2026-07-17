use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "imgpress", version, about = "Compress images to a target file size")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Cli(CliArgs),
}

#[derive(Parser, Debug)]
pub struct CliArgs {
    #[arg(short, long)]
    pub input: std::path::PathBuf,

    #[arg(short, long)]
    pub output: std::path::PathBuf,

    #[arg(long, default_value = "500kb")]
    pub max_size: String,

    #[arg(long, default_value = "jpeg")]
    pub format: String,

    #[arg(long, default_value_t = 20)]
    pub min_quality: u8,

    #[arg(long, default_value_t = 95)]
    pub max_quality: u8,

    #[arg(long, default_value_t = 0.85)]
    pub scale_step: f32,

    #[arg(long, default_value_t = 8)]
    pub max_scales: u32,

    #[arg(long)]
    pub preserve_structure: bool,

    #[arg(long)]
    pub skip_if_smaller: bool,

    #[arg(long)]
    pub no_recursive: bool,

    #[arg(long)]
    pub delete_source: bool,

    #[arg(long)]
    pub log_file: bool,
}