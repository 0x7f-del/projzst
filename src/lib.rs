//! Core library for projzst - pack and unpack .pjz files
//!
//! File format specification (new):
//! [Skippable Frame (metadata)] + [tar.zst data]
//! Skippable Frame: [4-byte magic (0x184D2A50..0x184D2A5F)] + [4-byte little-endian size] + [MessagePack metadata]
//! The metadata is stored in one or more ZStd skippable frames at the beginning of the file,
//! followed by a standard ZStd compressed frame containing the tar archive.

use serde::{Deserialize, Serialize};
use serde_ignored;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;
use thiserror::Error;

mod string_utils;
pub use crate::string_utils::IntoOpStr;

/// Default zstd compression level for pack operation
pub const DEFAULT_ZSTD_LEVEL: i32 = 6;

/// Maximum allowed metadata size (10 MB) to prevent malicious files
const MAX_METADATA_SIZE: usize = 10 * 1024 * 1024;

/// Minimum value of ZStd skippable frame magic number (inclusive)
const SKIPPABLE_FRAME_MAGIC_MIN: u32 = 0x184D2A50;
/// Maximum value of ZStd skippable frame magic number (inclusive)
const SKIPPABLE_FRAME_MAGIC_MAX: u32 = 0x184D2A5F;
/// Fixed magic number used for metadata frames (any value in the range works)
const METADATA_FRAME_MAGIC: u32 = 0x184D2A50;

/// Custom error types for projzst operations
#[derive(Error, Debug)]
pub enum ProjzstError {
    /// IO operation failed (file read/write, directory creation, etc.)
    #[error("IO operation failed: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization failed
    #[error("JSON parsing failed: {0}")]
    Json(#[from] serde_json::Error),

    /// MessagePack encoding failed during metadata serialization
    #[error("MessagePack encoding failed: {0}")]
    MsgPackEncode(#[from] rmp_serde::encode::Error),

    /// MessagePack decoding failed during metadata deserialization
    #[error("MessagePack decoding failed: {0}")]
    MsgPackDecode(#[from] rmp_serde::decode::Error),

    /// Metadata size is invalid (zero or exceeds MAX_METADATA_SIZE)
    #[error("Invalid metadata length: got {0} bytes")]
    InvalidMetadataLength(usize),

    /// Extra metadata file specified but not found
    #[error("Extra metadata file not found: {0}")]
    ExtraFileNotFound(String),

    /// Source directory to pack does not exist
    #[error("Source directory does not exist: {0}")]
    SourceNotFound(String),

    /// File header is invalid (missing magic numbers, corrupt format, etc.)
    #[error("Failed to read file header or invalid file format")]
    InvalidFileHeader,

    /// Unknown fields detected in metadata when ignore_unknown is false
    #[error("Unknown fields detected in metadata: {0}")]
    UnknownFields(String),

    /// Invalid ignore_unknown parameter value
    #[error("Invalid ignore_unknown parameter: must be 'on', 'off', or 'export'")]
    InvalidIgnoreUnknownParam,
}

/// Result type alias for projzst operations
pub type Result<T> = std::result::Result<T, ProjzstError>;

/// Ignore unknown fields behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreUnknown {
    /// Silently ignore unknown fields (default)
    On,
    /// Error on unknown fields
    Off,
    /// Collect unknown fields and export them to extra.ignored
    Export,
}

impl IgnoreUnknown {
    /// Create from string parameter
    pub fn from_str<I: IntoOpStr>(s: I) -> Result<Self> {
        let a = s.into_op_str().unwrap_or_default();
        let s :&str = a.as_ref();
        match s.to_lowercase().as_str() {
            "on" | "true" | "yes" | "1" => Ok(IgnoreUnknown::On),
            "off" | "false" | "no" | "0" => Ok(IgnoreUnknown::Off),
            "export" | "extra" => Ok(IgnoreUnknown::Export),
            _ => Err(ProjzstError::InvalidIgnoreUnknownParam),
        }
    }
}

impl Default for IgnoreUnknown {
    fn default() -> Self {
        IgnoreUnknown::On
    }
}

/// Metadata structure stored in .pjz file header
/// All fields are optional except extra which defaults to empty object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    /// Package name
    #[serde(default)]
    pub name: Option<String>,

    /// Author name
    #[serde(default)]
    pub auth: Option<String>,

    /// Package format identifier
    #[serde(default)]
    pub fmt: Option<String>,

    /// Format edition
    #[serde(default)]
    pub ed: Option<String>,

    /// Project version
    #[serde(default)]
    pub ver: Option<String>,

    /// Package description
    #[serde(default)]
    pub desc: Option<String>,

    /// Extra metadata (arbitrary JSON structure)
    /// When ignore_unknown = Export, unknown fields are stored in extra.ignored
    #[serde(default)]
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
    /// All parameters accept types that can be converted to Option<String>
    pub fn new<I1, I2, I3, I4, I5, I6>(
        name: I1,
        auth: I2,
        fmt: I3,
        ed: I4,
        ver: I5,
        desc: I6,
    ) -> Self 
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
    /// Consumes self and returns updated Metadata
    pub fn with_extra(mut self, extra: serde_json::Value) -> Self {
        self.extra = extra;
        self
    }

    /// Merge unknown fields into extra.ignored
    /// This is used when ignore_unknown = Export
    pub fn merge_unknown_fields(&mut self, unknown: serde_json::Value) {
        if let serde_json::Value::Object(unknown_map) = unknown {
            // Ensure extra is an object
            if !self.extra.is_object() {
                self.extra = serde_json::Value::Object(serde_json::Map::new());
            }
            
            if let serde_json::Value::Object(extra_map) = &mut self.extra {
                // Create or get the "ignored" field
                let ignored = extra_map
                    .entry("ignored".to_string())
                    .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                
                // Ensure ignored is an object
                if !ignored.is_object() {
                    *ignored = serde_json::Value::Object(serde_json::Map::new());
                }
                
                // Merge unknown fields into ignored
                if let serde_json::Value::Object(ignored_map) = ignored {
                    for (key, value) in unknown_map {
                        ignored_map.insert(key, value);
                    }
                }
            }
        }
    }
}

/// Pack a directory into a .pjz file
/// Creates archive with MessagePack metadata stored in ZStd skippable frames,
/// followed by tar.zst compressed content
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
    let metadata_len = metadata_bytes.len();

    // Validate metadata size
    if metadata_len > MAX_METADATA_SIZE {
        return Err(ProjzstError::InvalidMetadataLength(metadata_len));
    }

    // Create parent directories if needed
    if let Some(parent) = output_file.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    // Write final .pjz file: [skippable frame][tar.zst data]
    let mut output = File::create(output_file)?;

    // Write skippable frame header (magic + size)
    output.write_all(&METADATA_FRAME_MAGIC.to_le_bytes())?;
    output.write_all(&(metadata_len as u32).to_le_bytes())?;
    // Write metadata bytes as frame data
    output.write_all(&metadata_bytes)?;

    // Append tar.zst compressed data as a standard ZStd frame
    let mut zst_encoder = zstd::stream::Encoder::new(&mut output, compression_level)?;
    {
        let mut tar_builder = tar::Builder::new(&mut zst_encoder);
        // Add all files from source directory
        tar_builder.append_dir_all(".", source_dir)?;
    }
    // Finalize zstd stream
    zst_encoder.finish()?;

    Ok(())
}

/// Internal helper: read metadata from a file with ignore_unknown parameter
/// Returns metadata and leaves file cursor at the start of the first ZStd frame
fn read_metadata_from_file(file: &mut File, ignore_unknown: IgnoreUnknown) -> Result<Metadata> {
    let mut metadata_bytes = Vec::new();

    loop {
        let mut magic_buf = [0u8; 4];
        match file.read_exact(&mut magic_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // EOF while reading magic: if we already have metadata, accept it;
                // otherwise the file is completely invalid
                if metadata_bytes.is_empty() {
                    return Err(ProjzstError::InvalidFileHeader);
                } else {
                    break; // metadata only, no ZStd frame
                }
            }
            Err(e) => return Err(e.into()),
        }

        let magic = u32::from_le_bytes(magic_buf);

        // Check if this is a skippable frame
        if (SKIPPABLE_FRAME_MAGIC_MIN..=SKIPPABLE_FRAME_MAGIC_MAX).contains(&magic) {
            // Read frame size (little-endian)
            let mut size_buf = [0u8; 4];
            file.read_exact(&mut size_buf)?;
            let frame_size = u32::from_le_bytes(size_buf) as usize;

            // Validate total metadata size
            if metadata_bytes.len() + frame_size > MAX_METADATA_SIZE {
                return Err(ProjzstError::InvalidMetadataLength(frame_size));
            }

            // Read frame data
            let mut frame_data = vec![0u8; frame_size];
            file.read_exact(&mut frame_data)?;
            metadata_bytes.extend_from_slice(&frame_data);
        } else {
            // Not a skippable frame - assume it's the start of ZStd compressed data
            // Rewind so the ZStd decoder can read the magic again
            file.seek(SeekFrom::Current(-4))?;
            break;
        }
    }

    // Ensure we actually read some metadata
    if metadata_bytes.is_empty() {
        return Err(ProjzstError::InvalidFileHeader);
    }

    // Deserialize MessagePack to Metadata struct with ignore_unknown handling
    match ignore_unknown {
        IgnoreUnknown::On => {
            // Silently ignore unknown fields
            let metadata: Metadata = rmp_serde::from_slice(&metadata_bytes)?;
            Ok(metadata)
        }
        IgnoreUnknown::Off => {
            // Check for unknown fields using serde_ignored
            let mut deserializer = rmp_serde::Deserializer::new(&metadata_bytes[..]);
            let mut unknown_fields = Vec::new();
            
            let metadata: Metadata = serde_ignored::deserialize(&mut deserializer, |path| {
                unknown_fields.push(path.to_string());
            })?;
            
            if !unknown_fields.is_empty() {
                return Err(ProjzstError::UnknownFields(unknown_fields.join(", ")));
            }
            
            Ok(metadata)
        }
        IgnoreUnknown::Export => {
            // Deserialize into a generic Value first
            let full_value: serde_json::Value = rmp_serde::from_slice(&metadata_bytes)?;
            
            if let serde_json::Value::Object(map) = full_value {
                // Known fields we want to extract
                let known_fields = [
                    "name", "auth", "fmt", "ed", "ver", "desc", "extra"
                ];
                
                // Build a map of known fields
                let mut known_map = serde_json::Map::new();
                let mut unknown_map = serde_json::Map::new();
                
                for (key, value) in map {
                    if known_fields.contains(&key.as_str()) {
                        known_map.insert(key, value);
                    } else {
                        unknown_map.insert(key, value);
                    }
                }
                
                // Deserialize known fields into Metadata
                let known_value = serde_json::Value::Object(known_map);
                let mut metadata: Metadata = serde_json::from_value(known_value)?;
                
                // Merge unknown fields into extra.ignored
                if !unknown_map.is_empty() {
                    metadata.merge_unknown_fields(serde_json::Value::Object(unknown_map));
                }
                
                Ok(metadata)
            } else {
                // Not an object - just try normal deserialization
                Ok(rmp_serde::from_slice(&metadata_bytes)?)
            }
        }
    }
}

/// Read only metadata from a .pjz file without extracting content
/// Returns the metadata found in the skippable frames
/// 
/// # Arguments
/// * `input_file` - Path to the .pjz file
/// * `ignore_unknown` - How to handle unknown fields in metadata
pub fn read_metadata<P: AsRef<Path>>(
    input_file: P, 
    ignore_unknown: IgnoreUnknown
) -> Result<Metadata> {
    let mut file = File::open(input_file.as_ref())?;
    read_metadata_from_file(&mut file, ignore_unknown)
}

/// Unpack a .pjz file to target directory
/// Extracts content, writes metadata.json to parent directory of output,
/// and returns the metadata
/// 
/// # Arguments
/// * `input_file` - Path to the .pjz file
/// * `output_dir` - Directory to extract contents to
/// * `ignore_unknown` - How to handle unknown fields in metadata
pub fn unpack<P1, P2>(
    input_file: P1, 
    output_dir: P2,
    ignore_unknown: IgnoreUnknown,
) -> Result<Metadata>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let input_file = input_file.as_ref();
    let output_dir = output_dir.as_ref();

    let mut file = File::open(input_file)?;
    // Read metadata and position cursor at start of ZStd frame
    let metadata = read_metadata_from_file(&mut file, ignore_unknown)?;

    // Decompress zstd and extract tar archive
    // File cursor is now at the start of the ZStd compressed data
    let zst_decoder = zstd::stream::Decoder::new(&mut file)?;
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
/// Returns the metadata and writes it to the specified JSON file
/// 
/// # Arguments
/// * `input_file` - Path to the .pjz file
/// * `output_json` - Path where to save the JSON file
/// * `ignore_unknown` - How to handle unknown fields in metadata
pub fn info<P1, P2>(
    input_file: P1, 
    output_json: P2,
    ignore_unknown: IgnoreUnknown,
) -> Result<Metadata>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let metadata = read_metadata(input_file, ignore_unknown)?;

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
