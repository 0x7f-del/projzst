//! String utilities for handling optional string conversions.
//!
//! This module provides a flexible trait [`IntoOpStr`] for converting various string-like types
//! into an `Option<String>`. It is particularly useful when dealing with APIs that accept
//! optional string parameters from multiple input types.
//!
//! # Overview
//!
//! The [`IntoOpStr`] trait is implemented for:
//! - `&str` - Converts to `Some(String)`
//! - `String` - Converts to `Some(self)` (consumes the String)
//! - `Option<String>` - Passes through as-is
//!
//! # Examples
//!
//! ```
//! use string_utils::IntoOpStr;
//!
//! // From &str
//! let s1 = "hello".into_op_str();
//! assert_eq!(s1, Some("hello".to_string()));
//!
//! // From String
//! let s2 = String::from("world").into_op_str();
//! assert_eq!(s2, Some("world".to_string()));
//!
//! // From Option<String>
//! let s3: Option<String> = Some("foo".to_string());
//! assert_eq!(s3.into_op_str(), Some("foo".to_string()));
//!
//! let s4: Option<String> = None;
//! assert_eq!(s4.into_op_str(), None);
//! ```
//!
//! # Helper Function
//!
//! The module also provides a convenience function [`_convert`] that wraps the trait method.

/// A trait for converting a value into an optional string (`Option<String>`).
///
/// This trait allows multiple string-like types to be uniformly converted into
/// `Option<String>`, simplifying APIs that work with optional string parameters.
///
/// # Implementations
///
/// - For `&str`: Always returns `Some(self.to_string())`
/// - For `String`: Always returns `Some(self)` (consumes the String)
/// - For `Option<String>`: Returns the option unchanged
///
/// # Examples
///
/// ```
/// use string_utils::IntoOpStr;
///
/// fn process_string<T: IntoOpStr>(input: T) -> Option<String> {
///     input.into_op_str()
/// }
///
/// assert_eq!(process_string("test"), Some("test".to_string()));
/// assert_eq!(process_string(String::from("test")), Some("test".to_string()));
/// assert_eq!(process_string(Some("test".to_string())), Some("test".to_string()));
/// assert_eq!(process_string(None::<String>), None);
/// ```
pub trait IntoOpStr {
    /// Converts the value into an `Option<String>`.
    ///
    /// # Returns
    ///
    /// * `Some(String)` - For non-optional inputs (`&str`, `String`)
    /// * The original option - For `Option<String>` inputs
    fn into_op_str(self) -> Option<String>;
}

impl IntoOpStr for &str {
    fn into_op_str(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl IntoOpStr for String {
    fn into_op_str(self) -> Option<String> {
        Some(self)
    }
}

impl IntoOpStr for Option<String> {
    fn into_op_str(self) -> Option<String> {
        self
    }
}

/// Convenience function to convert any [`IntoOpStr`] implementor into an `Option<String>`.
///
/// This is a thin wrapper around [`IntoOpStr::into_op_str`] that can be useful in
/// situations where you need to explicitly invoke the conversion.
///
/// # Examples
///
/// ```
/// use string_utils::_convert;
///
/// let result = _convert("example");
/// assert_eq!(result, Some("example".to_string()));
/// ```
pub fn _convert<T: IntoOpStr>(value: T) -> Option<String> {
    value.into_op_str()
}
