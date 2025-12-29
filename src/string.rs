//! String conversion utilities for Windows APIs.
//!
//! Windows APIs typically use UTF-16 encoded strings (wide strings), while Rust uses UTF-8.
//! This module provides ergonomic conversions between these formats.

use crate::error::{Error, Result};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

/// Converts a Rust string to a null-terminated UTF-16 vector.
///
/// # Example
///
/// ```
/// use ergonomic_windows::string::to_wide;
///
/// let wide = to_wide("Hello");
/// assert_eq!(wide, vec![72, 101, 108, 108, 111, 0]);
/// ```
#[inline]
pub fn to_wide(s: &str) -> Vec<u16> {
    // Pre-allocate exact capacity to avoid reallocations.
    // UTF-16 length is at most equal to UTF-8 length (each UTF-8 char encodes to 1-2 UTF-16 units).
    // +1 for null terminator.
    let mut result = Vec::with_capacity(s.len() + 1);
    result.extend(s.encode_utf16());
    result.push(0);
    result
}

/// Converts a path to a null-terminated UTF-16 vector.
#[inline]
pub fn path_to_wide(path: &Path) -> Vec<u16> {
    // Pre-allocate capacity based on path length estimate.
    let os_str = path.as_os_str();
    let mut result = Vec::with_capacity(os_str.len() + 1);
    result.extend(os_str.encode_wide());
    result.push(0);
    result
}

/// Converts a null-terminated UTF-16 slice to a Rust `String`.
///
/// The slice may or may not include the null terminator.
///
/// # Example
///
/// ```
/// use ergonomic_windows::string::{to_wide, from_wide};
///
/// let wide = to_wide("Hello");
/// let s = from_wide(&wide).unwrap();
/// assert_eq!(s, "Hello");
/// ```
#[inline]
pub fn from_wide(wide: &[u16]) -> Result<String> {
    // Find null terminator if present
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    // Use String::from_utf16 directly instead of going through OsString
    String::from_utf16(&wide[..len])
        .map_err(|_| Error::string_conversion("Invalid UTF-16 sequence"))
}

/// Converts a null-terminated UTF-16 pointer to a Rust `String`.
///
/// # Safety
///
/// The caller must ensure that:
/// - `ptr` is non-null (checked at runtime, returns error if null)
/// - `ptr` points to valid memory that is properly aligned for `u16`
/// - `ptr` points to a null-terminated UTF-16 string
/// - The memory region from `ptr` to the null terminator is valid for reads
/// - The memory pointed to by `ptr` is not mutated during this call
/// - The memory remains valid for the entire duration of this function call
///
/// # Errors
///
/// Returns an error if:
/// - `ptr` is null
/// - The UTF-16 data contains invalid sequences
///
/// # Example
///
/// ```ignore
/// use ergonomic_windows::string::from_wide_ptr;
///
/// // From a Windows API that returns PCWSTR
/// let ptr: *const u16 = some_windows_api();
///
/// // SAFETY: The Windows API guarantees the pointer is valid and null-terminated
/// let s = unsafe { from_wide_ptr(ptr) }?;
/// # Ok::<(), ergonomic_windows::error::Error>(())
/// ```
pub unsafe fn from_wide_ptr(ptr: *const u16) -> Result<String> {
    if ptr.is_null() {
        return Err(Error::null_pointer("from_wide_ptr received null pointer"));
    }

    // SAFETY: Caller guarantees ptr is valid and null-terminated.
    // We iterate until we find the null terminator.
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }

    // SAFETY: We've verified the string is null-terminated at position `len`,
    // so reading `len` u16 values is safe.
    let slice = std::slice::from_raw_parts(ptr, len);
    from_wide(slice)
}

/// Converts a UTF-16 slice with a known length to a Rust `String`.
///
/// Unlike `from_wide`, this does not look for a null terminator.
#[inline]
pub fn from_wide_with_len(wide: &[u16], len: usize) -> Result<String> {
    let actual_len = len.min(wide.len());
    // Use String::from_utf16 directly instead of going through OsString
    String::from_utf16(&wide[..actual_len])
        .map_err(|_| Error::string_conversion("Invalid UTF-16 sequence"))
}

/// A builder for creating wide strings with proper null termination.
#[derive(Default)]
pub struct WideStringBuilder {
    buffer: Vec<u16>,
}

impl WideStringBuilder {
    /// Creates a new empty builder.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new builder with the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Appends a string to the builder.
    #[inline]
    pub fn push(&mut self, s: &str) -> &mut Self {
        self.buffer.extend(s.encode_utf16());
        self
    }

    /// Appends a single UTF-16 code unit.
    #[inline]
    pub fn push_char(&mut self, c: u16) -> &mut Self {
        self.buffer.push(c);
        self
    }

    /// Appends a null terminator and returns the completed vector.
    #[inline]
    pub fn build(mut self) -> Vec<u16> {
        self.buffer.push(0);
        self.buffer
    }

    /// Clears the builder for reuse without deallocating.
    #[inline]
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Appends a null terminator, returns the completed vector, and clears for reuse.
    ///
    /// This is useful for building multiple wide strings while reusing the same buffer.
    #[inline]
    pub fn build_and_clear(&mut self) -> Vec<u16> {
        self.buffer.push(0);
        std::mem::take(&mut self.buffer)
    }

    /// Returns the current length without the null terminator.
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns true if the builder is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Returns the current capacity.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }
}

/// PCWSTR helper - a wrapper for passing wide strings to Windows APIs.
///
/// This type holds ownership of the string buffer and provides a pointer
/// that can be passed to Windows APIs expecting `PCWSTR`.
#[derive(Clone)]
pub struct WideString {
    buffer: Vec<u16>,
}

impl WideString {
    /// Creates a new `WideString` from a Rust string.
    #[inline]
    pub fn new(s: &str) -> Self {
        Self {
            buffer: to_wide(s),
        }
    }

    /// Creates a new `WideString` with the specified capacity.
    ///
    /// The capacity should include space for the null terminator.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Creates a new `WideString` from a path.
    #[inline]
    pub fn from_path(path: &Path) -> Self {
        Self {
            buffer: path_to_wide(path),
        }
    }

    /// Returns a pointer to the null-terminated wide string.
    #[inline]
    pub fn as_ptr(&self) -> *const u16 {
        self.buffer.as_ptr()
    }

    /// Returns the string as a PCWSTR for use with Windows APIs.
    #[inline]
    pub fn as_pcwstr(&self) -> windows::core::PCWSTR {
        windows::core::PCWSTR::from_raw(self.buffer.as_ptr())
    }

    /// Returns the length in UTF-16 code units, not including the null terminator.
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len().saturating_sub(1)
    }

    /// Returns true if the string is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Converts back to a Rust String.
    #[inline]
    pub fn to_string_lossy(&self) -> String {
        from_wide(&self.buffer).unwrap_or_else(|_| String::from("�"))
    }

    /// Returns the underlying buffer as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[u16] {
        &self.buffer
    }
}

impl From<&str> for WideString {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<&Path> for WideString {
    fn from(path: &Path) -> Self {
        Self::from_path(path)
    }
}

impl From<String> for WideString {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let original = "Hello, World! 🌍";
        let wide = to_wide(original);
        let back = from_wide(&wide).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_empty_string() {
        let wide = to_wide("");
        assert_eq!(wide, vec![0]);
        let back = from_wide(&wide).unwrap();
        assert_eq!(back, "");
    }

    #[test]
    fn test_wide_string_builder() {
        let mut builder = WideStringBuilder::new();
        builder.push("Hello").push(", ").push("World!");
        let wide = builder.build();
        let s = from_wide(&wide).unwrap();
        assert_eq!(s, "Hello, World!");
    }
}
