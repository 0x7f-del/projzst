//! Core library for projzst - pack and unpack .pjz files
//!
//! File format specification:
//! [4-byte metadata length (big-endian)] + [MessagePack metadata] + [tar.zst data]

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use std::path::Path;
use thiserror::Error;

mod string_utils;
use crate::string_utils::IntoOpStr;

/// Default zstd compression level for pack operation
pub const DEFAULT_ZSTD_LEVEL: i32 = 6;

/// Maximum allowed metadata size (10 MB) to prevent malicious files
const MAX_METADATA_SIZE: usize = 10 * 1024 * 1024;

/// Custom error types for projzst operations
#[derive(Error, Debug)]
pub enum ProjzstError {
    #[error("IO operation failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("MessagePack encoding failed: {0}")]
    MsgPackEncode(#[from] rmp_serde::encode::Error),

    #[error("MessagePack decoding failed: {0}")]
    MsgPackDecode(#[from] rmp_serde::decode::Error),

    #[error("Invalid metadata length: got {0} bytes")]
    InvalidMetadataLength(usize),

    #[error("Extra metadata file not found: {0}")]
    ExtraFileNotFound(String),

    #[error("Source directory does not exist: {0}")]
    SourceNotFound(String),

    #[error("Failed to read file header")]
    InvalidFileHeader,
}

/// Result type alias for projzst operations
pub type Result<T> = std::result::Result<T, ProjzstError>;

/// Metadata structure stored in .pjz file header
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    /// Package name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Author name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,

    /// Package format identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fmt: Option<String>,

    /// Format edition
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ed: Option<String>,

    /// Project version
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ver: Option<String>,

    /// Package description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,

    /// Extra metadata (arbitrary JSON structure)
    pub extra: serde_json::Value,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            name: None,
            auth: None,
            fmt: None,
            ed: None,
            ver: None,
            desc: None,
            extra: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

impl Metadata {
    /// Create new Metadata with specified fields
    pub fn new<I1, I2, I3, I4, I5, I6>(
        name: I1,
        auth: I2,
        fmt: I3,
        ed: I4,
        ver: I5,
        desc: I6,
    )-> Self 
    where
        I1: IntoOpStr,
        I2: IntoOpStr,
        I3: IntoOpStr,
        I4: IntoOpStr,
        I5: IntoOpStr,
        I6: IntoOpStr,
    {
        Self {
            name: name.into_op_str(),
            auth: auth.into_op_str(),
            fmt: fmt.into_op_str(),
            ed: ed.into_op_str(),
            ver: ver.into_op_str(),
            desc: desc.into_op_str(),
            extra: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Set extra metadata from JSON value
    pub fn with_extra(mut self, extra: serde_json::Value) -> Self {
        self.extra = extra;
        self
    }
}

/// Pack a directory into a .pjz file
///
/// Creates archive with MessagePack metadata header followed by tar.zst compressed content
pub fn pack<P1, P2, P3>(
    source_dir: P1,
    output_file: P2,
    mut metadata: Metadata,
    extra_file: Option<P3>,
    compression_level: i32,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    let source_dir = source_dir.as_ref();
    let output_file = output_file.as_ref();

    // Validate source directory exists
    if !source_dir.exists() {
        return Err(ProjzstError::SourceNotFound(
            source_dir.display().to_string(),
        ));
    }

    // Load extra metadata from JSON file if provided
    if let Some(extra_path) = extra_file {
        let extra_path = extra_path.as_ref();
        let extra_content = fs::read_to_string(extra_path).map_err(|_| {
            ProjzstError::ExtraFileNotFound(extra_path.display().to_string())
        })?;
        metadata.extra = serde_json::from_str(&extra_content)?;
    }

    // Serialize metadata to MessagePack bytes
    let metadata_bytes = rmp_serde::to_vec(&metadata)?;
    let metadata_len = metadata_bytes.len() as u32;

    // Create tar.zst archive in memory buffer
    let mut tar_zst_buffer = Vec::new();
    {
        let zst_encoder =
            zstd::stream::Encoder::new(&mut tar_zst_buffer, compression_level)?;
        let mut tar_builder = tar::Builder::new(zst_encoder);

        // Add all files from source directory
        tar_builder.append_dir_all(".", source_dir)?;

        // Finalize tar and zstd streams
        let zst_encoder = tar_builder.into_inner()?;
        zst_encoder.finish()?;
    }

    // Create parent directories if needed
    if let Some(parent) = output_file.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    // Write final .pjz file: [length][metadata][tar.zst]
    let mut output = BufWriter::new(File::create(output_file)?);
    output.write_all(&metadata_len.to_be_bytes())?;
    output.write_all(&metadata_bytes)?;
    output.write_all(&tar_zst_buffer)?;
    output.flush()?;

    Ok(())
}

/// Read only metadata from a .pjz file without extracting content
pub fn read_metadata<P: AsRef<Path>>(input_file: P) -> Result<Metadata> {
    let mut file = BufReader::new(File::open(input_file.as_ref())?);

    // Read 4-byte metadata length (big-endian)
    let mut len_bytes = [0u8; 4];
    file.read_exact(&mut len_bytes)
        .map_err(|_| ProjzstError::InvalidFileHeader)?;
    let metadata_len = u32::from_be_bytes(len_bytes) as usize;

    // Validate metadata length
    if metadata_len == 0 || metadata_len > MAX_METADATA_SIZE {
        return Err(ProjzstError::InvalidMetadataLength(metadata_len));
    }

    // Read metadata bytes
    let mut metadata_bytes = vec![0u8; metadata_len];
    file.read_exact(&mut metadata_bytes)?;

    // Deserialize MessagePack to Metadata struct
    let metadata: Metadata = rmp_serde::from_slice(&metadata_bytes)?;

    Ok(metadata)
}

/// Unpack a .pjz file to target directory
///
/// Extracts content and writes metadata.json to parent directory of output
pub fn unpack<P1, P2>(input_file: P1, output_dir: P2) -> Result<Metadata>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let input_file = input_file.as_ref();
    let output_dir = output_dir.as_ref();

    let mut file = BufReader::new(File::open(input_file)?);

    // Read metadata length header
    let mut len_bytes = [0u8; 4];
    file.read_exact(&mut len_bytes)
        .map_err(|_| ProjzstError::InvalidFileHeader)?;
    let metadata_len = u32::from_be_bytes(len_bytes) as usize;

    if metadata_len == 0 || metadata_len > MAX_METADATA_SIZE {
        return Err(ProjzstError::InvalidMetadataLength(metadata_len));
    }

    // Read and deserialize metadata
    let mut metadata_bytes = vec![0u8; metadata_len];
    file.read_exact(&mut metadata_bytes)?;
    let metadata: Metadata = rmp_serde::from_slice(&metadata_bytes)?;

    // Read remaining tar.zst data
    let mut zst_data = Vec::new();
    file.read_to_end(&mut zst_data)?;

    // Decompress zstd and extract tar archive
    let zst_decoder = zstd::stream::Decoder::new(Cursor::new(zst_data))?;
    let mut tar_archive = tar::Archive::new(zst_decoder);

    // Create output directory and extract files
    fs::create_dir_all(output_dir)?;
    tar_archive.unpack(output_dir)?;

    // Write metadata.json to parent directory of output_dir
    let metadata_json_path = output_dir
        .parent()
        .unwrap_or(Path::new("."))
        .join("metadata.json");
    let json_content = serde_json::to_string_pretty(&metadata)?;
    fs::write(metadata_json_path, json_content)?;

    Ok(metadata)
}

/// Extract metadata from .pjz file and save as JSON
pub fn info<P1, P2>(input_file: P1, output_json: P2) -> Result<Metadata>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let metadata = read_metadata(input_file)?;

    // Create parent directory if needed
    let output_json = output_json.as_ref();
    if let Some(parent) = output_json.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    // Write pretty-printed JSON
    let json_content = serde_json::to_string_pretty(&metadata)?;
    fs::write(output_json, json_content)?;

    Ok(metadata)
}
