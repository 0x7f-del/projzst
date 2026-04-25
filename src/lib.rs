//! Core library for projzst - pack and unpack .pjz files
//!
//! File format specification (new):
//! [Skippable Frame (metadata)] + [tar.zst data]
//! Skippable Frame: [4-byte magic (0x184D2A50..0x184D2A5F)] + [4-byte little-endian size] + [MessagePack metadata]
//! The metadata is stored in one or more ZStd skippable frames at the beginning of the file,
//! followed by a standard ZStd compressed frame containing the tar archive.

mod string_utils;
pub use crate::string_utils::convert;
pub use crate::string_utils::IntoOpStr;

mod builder;
pub use crate::builder::DEFAULT_ZSTD_LEVEL;
pub use crate::builder::{info, pack, read_metadata, unpack};
pub use crate::builder::{Packer /*Unpacker*/};

mod errors;
pub use crate::errors::ProjzstError;
pub use crate::errors::Result;

mod metadata;
pub use crate::metadata::BasicMetadata;
pub use crate::metadata::FullMetadata;
pub use crate::metadata::IgnoreUnknown;
pub use crate::metadata::Metadata;
