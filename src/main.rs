//! Command-line interface for projzst tool

use clap::{Parser, Subcommand};
use projzst::{info, pack, unpack, Metadata, ProjzstError, DEFAULT_ZSTD_LEVEL};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "projzst")]
#[command(version, about = "Pack and unpack .pjz files with metadata")]
#[command(long_about = "A tool for creating and extracting .pjz archives \
    with MessagePack metadata and zstd compression")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Pack a directory into a .pjz file with metadata
    Pack {
        /// Source directory to pack
        source: PathBuf,

        /// Package name
        #[arg(long)]
        name: String,

        /// Author name
        #[arg(long)]
        auth: String,

        /// Package format identifier
        #[arg(long)]
        fmt: String,

        /// Format edition
        #[arg(long)]
        ed: String,

        /// Project version
        #[arg(long)]
        ver: String,

        /// Package description
        #[arg(long)]
        desc: String,

        /// Path to extra metadata JSON file
        #[arg(long)]
        extra: Option<PathBuf>,

        /// Zstd compression level (1-22)
        #[arg(long, default_value_t = DEFAULT_ZSTD_LEVEL)]
        level: i32,

        /// Output .pjz file path
        output: PathBuf,
    },

    /// Unpack a .pjz file to a directory
    Unpack {
        /// Input .pjz file path
        input: PathBuf,

        /// Output directory path
        output: PathBuf,
    },

    /// Extract metadata info from a .pjz file to JSON
    Info {
        /// Input .pjz file path
        input: PathBuf,

        /// Output JSON file path
        output: PathBuf,
    },
}

fn run() -> Result<(), ProjzstError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack {
            source,
            name,
            auth,
            fmt,
            ed,
            ver,
            desc,
            extra,
            level,
            output,
        } => {
            let metadata = Metadata::new(name, auth, fmt, ed, ver, desc);
            pack(&source, &output, metadata, extra.as_ref(), level)?;
            println!("Successfully packed: {}", output.display());
        }

        Commands::Unpack { input, output } => {
            let metadata = unpack(&input, &output)?;
            println!("Successfully unpacked: {}", output.display());
            println!("Package: {} v{}", metadata.name, metadata.ver);
        }

        Commands::Info { input, output } => {
            let metadata = info(&input, &output)?;
            println!("Metadata saved to: {}", output.display());
            println!("---");
            println!("Name: {}", metadata.name);
            println!("Author: {}", metadata.auth);
            println!("Version: {}", metadata.ver);
            println!("Format: {} ({})", metadata.fmt, metadata.ed);
            println!("Description: {}", metadata.desc);
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}