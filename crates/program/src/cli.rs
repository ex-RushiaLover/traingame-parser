use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "tg-parser",
    version = "1.0",
    about = "Tool for parsing certain anime game resources."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Process textmap only
    Textmap {
        /// Persistent path or design data URL
        input_url: String,
        /// Output directory
        output_dir: PathBuf,

        /// Parse full textmap structure as array, rather than just key-value pair
        #[arg(long)]
        full_textmap: bool,

        /// Save .bytes file after downloading the files
        #[arg(long, name = "save-bytes-file")]
        save_bytes_file: bool,
    },

    /// Process excel only
    Excels(ExcelArgs),

    /// Process excel, config, textmap parse
    All(ExcelArgs),
}

#[derive(Args)]
pub struct ExcelArgs {
    /// data.json schema file path
    pub data_json: String,
    /// excel_paths.json file path
    pub excel_path_json: String,
    /// Persistent path or design data URL
    pub input_url: String,
    /// Output directory
    pub output_dir: PathBuf,

    /// Parse full textmap structure as array, rather than just key-value pair
    #[arg(long)]
    pub full_textmap: bool,

    /// Save .bytes file after downloading the files
    #[arg(long, name = "save-bytes-file")]
    pub save_bytes_file: bool,

    /// Log all error into console
    #[arg(long, name = "log-error")]
    pub log_error: bool,

    /// Additional configs path to parse, with type as key, and array of paths as values
    #[arg(long, name = "config-paths")]
    pub config_paths: Option<PathBuf>,
}
