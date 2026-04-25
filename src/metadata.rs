use crate::errors::ProjzstError;
use crate::errors::Result;
use crate::string_utils::IntoOpStr;
use serde::{Deserialize, Serialize};

/// Ignore unknown fields behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IgnoreUnknown {
    /// Silently ignore unknown fields (default)
    #[default]
    On,
    /// Error on unknown fields
    Off,
    /// Collect unknown fields and export them to extra.ignored
    Export,
}

impl IgnoreUnknown {
    /// Create from string parameter
    pub fn from_str_tmp<I: IntoOpStr>(s: I) -> Result<Self> {
        let a = s.into_op_str().unwrap_or_default();
        let s: &str = a.as_ref();
        match s.to_lowercase().as_str() {
            "on" | "true" | "yes" | "1" => Ok(IgnoreUnknown::On),
            "off" | "false" | "no" | "0" => Ok(IgnoreUnknown::Off),
            "export" | "extra" => Ok(IgnoreUnknown::Export),
            _ => Err(ProjzstError::InvalidIgnoreUnknownParam),
        }
    }
}

pub trait Metadata {
    fn name(self, name: Option<String>) -> Self;
    fn auth(self, auth: Option<String>) -> Self;
    fn fmt(self, fmt: Option<String>) -> Self;
    fn ed(self, ed: Option<String>) -> Self;
    fn ver(self, ver: Option<String>) -> Self;
    fn desc(self, desc: Option<String>) -> Self;
}

/// New Structure about basic metadata structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BasicMetadata {
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
}

impl Metadata for BasicMetadata {
    fn auth(mut self, auth: Option<String>) -> Self {
        self.auth = auth;
        self
    }
    fn desc(mut self, desc: Option<String>) -> Self {
        self.desc = desc;
        self
    }
    fn ed(mut self, ed: Option<String>) -> Self {
        self.ed = ed;
        self
    }
    fn fmt(mut self, fmt: Option<String>) -> Self {
        self.fmt = fmt;
        self
    }
    fn name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }
    fn ver(mut self, ver: Option<String>) -> Self {
        self.ver = ver;
        self
    }
}

impl BasicMetadata {
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
        }
    }
}

/// Metadata structure stored in .pjz file header
/// All fields are optional except extra which defaults to empty object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FullMetadata {
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

impl Default for FullMetadata {
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

impl Metadata for FullMetadata {
    fn auth(mut self, auth: Option<String>) -> Self {
        self.auth = auth;
        self
    }
    fn desc(mut self, desc: Option<String>) -> Self {
        self.desc = desc;
        self
    }
    fn ed(mut self, ed: Option<String>) -> Self {
        self.ed = ed;
        self
    }
    fn fmt(mut self, fmt: Option<String>) -> Self {
        self.fmt = fmt;
        self
    }
    fn name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }
    fn ver(mut self, ver: Option<String>) -> Self {
        self.ver = ver;
        self
    }
}
impl FullMetadata {
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
