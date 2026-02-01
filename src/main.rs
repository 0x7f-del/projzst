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
        #[arg(short, long)]
        input: PathBuf,

        /// Package name
        #[arg(short, long)]
        name: String,

        /// Author name
        #[arg(short, long)]
        auth: Option<String>,

        /// Package format identifier
        #[arg(short, long)]
        fmt: Option<String>,

        /// Format edition
        #[arg(short, long)]
        ed: Option<String>,

        /// Project version
        #[arg(short, long)]
        ver: Option<String>,

        /// Package description
        #[arg(short, long)]
        desc: Option<String>,

        /// Path to extra metadata JSON file
        #[arg(short = 'x', long)]
        extra: Option<PathBuf>,

        /// Zstd compression level (1-22)
        #[arg(short, long, default_value_t = DEFAULT_ZSTD_LEVEL)]
        level: i32,

        /// Output .pjz file path
        #[arg(short, long)]
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
            input,
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
            pack(&input, &output, metadata, extra.as_ref(), level)?;
            println!("Successfully packed: {}", output.display());
        }

        Commands::Unpack { input, output } => {
            let metadata = unpack(&input, &output)?;
            println!("Successfully unpacked: {}", output.display());
            println!("Package: {} v{}", metadata.name, metadata.ver.unwrap_or_default());
        }

        Commands::Info { input, output } => {
            let metadata = info(&input, &output)?;
            println!("Metadata saved to: {}", output.display());
            println!("---");
            println!("Name: {}", metadata.name);
            if let Some(author) = metadata.auth {
                println!("Author: {}", author);
            }
            if let Some(version) = metadata.ver {
                println!("Version: {}", version);
            }
            if let Some(format) = metadata.fmt {
                match metadata.ed {
                    Some(edition) => println!("Format: {} ({})", format, edition),
                    None => println!("Format: {}", format),
                }
            }
            if let Some(description) = metadata.desc {
                println!("Description: {}", description);
            }
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