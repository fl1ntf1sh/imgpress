use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "imgpress",
    version,
    about = "Compress images to a target file size"
)]
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

    #[arg(long, default_value_t = true)]
    pub preserve_structure: bool,

    #[arg(long = "no-preserve-structure")]
    pub no_preserve_structure: bool,

    #[arg(long, default_value_t = true)]
    pub skip_if_smaller: bool,

    #[arg(long = "no-skip-if-smaller")]
    pub no_skip_if_smaller: bool,

    #[arg(long)]
    pub no_recursive: bool,

    #[arg(long)]
    pub delete_source: bool,

    #[arg(long, requires = "delete_source")]
    pub yes: bool,

    #[arg(long)]
    pub log_file: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_defaults_preserve_structure_and_skip_small_files() {
        let cli = Cli::parse_from(["imgpress", "cli", "-i", "in", "-o", "out"]);
        let Command::Cli(args) = cli.command.unwrap();

        assert!(args.preserve_structure);
        assert!(args.skip_if_smaller);
        assert!(!args.no_preserve_structure);
        assert!(!args.no_skip_if_smaller);
    }

    #[test]
    fn cli_can_disable_preserve_structure_and_skip_small_files() {
        let cli = Cli::parse_from([
            "imgpress",
            "cli",
            "-i",
            "in",
            "-o",
            "out",
            "--no-preserve-structure",
            "--no-skip-if-smaller",
        ]);
        let Command::Cli(args) = cli.command.unwrap();

        assert!(args.no_preserve_structure);
        assert!(args.no_skip_if_smaller);
    }

    #[test]
    fn cli_keeps_legacy_positive_flags() {
        let cli = Cli::parse_from([
            "imgpress",
            "cli",
            "-i",
            "in",
            "-o",
            "out",
            "--preserve-structure",
            "--skip-if-smaller",
        ]);
        let Command::Cli(args) = cli.command.unwrap();

        assert!(args.preserve_structure);
        assert!(args.skip_if_smaller);
    }
}
