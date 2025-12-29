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

/// Maximum inline capacity for small string optimization.
/// Strings up to this length (in UTF-16 code units, including null terminator)
/// are stored inline without heap allocation.
///
/// This value is chosen so that the inline buffer + length fits in 48 bytes,
/// which is the same size as `Vec<u16>` on 64-bit platforms.
const INLINE_CAP: usize = 23;

/// PCWSTR helper - a wrapper for passing wide strings to Windows APIs.
///
/// This type holds ownership of the string buffer and provides a pointer
/// that can be passed to Windows APIs expecting `PCWSTR`.
///
/// # Small String Optimization
///
/// Strings with 22 or fewer UTF-16 code units (plus null terminator) are stored
/// inline without heap allocation. This eliminates allocation overhead for the
/// most common case of short strings like filenames and registry keys.
pub struct WideString {
    repr: WideStringRepr,
}

/// Internal representation for WideString with small string optimization.
enum WideStringRepr {
    /// Inline storage for small strings (up to INLINE_CAP - 1 chars + null).
    Inline {
        buf: [u16; INLINE_CAP],
        len: u8, // Length including null terminator
    },
    /// Heap storage for larger strings.
    Heap(Vec<u16>),
}

impl Clone for WideString {
    fn clone(&self) -> Self {
        match &self.repr {
            WideStringRepr::Inline { buf, len } => Self {
                repr: WideStringRepr::Inline {
                    buf: *buf,
                    len: *len,
                },
            },
            WideStringRepr::Heap(vec) => Self {
                repr: WideStringRepr::Heap(vec.clone()),
            },
        }
    }
}

impl WideString {
    /// Creates a new `WideString` from a Rust string.
    ///
    /// If the string is short enough, it will be stored inline without allocation.
    #[inline]
    pub fn new(s: &str) -> Self {
        // Calculate UTF-16 length
        let utf16_len: usize = s.chars().map(|c| c.len_utf16()).sum();
        let total_len = utf16_len + 1; // +1 for null terminator

        if total_len <= INLINE_CAP {
            // Use inline storage
            let mut buf = [0u16; INLINE_CAP];
            let mut idx = 0;
            for unit in s.encode_utf16() {
                buf[idx] = unit;
                idx += 1;
            }
            buf[idx] = 0; // Null terminator
            Self {
                repr: WideStringRepr::Inline {
                    buf,
                    len: total_len as u8,
                },
            }
        } else {
            // Use heap storage
            Self {
                repr: WideStringRepr::Heap(to_wide(s)),
            }
        }
    }

    /// Creates a new `WideString` with the specified capacity.
    ///
    /// The capacity should include space for the null terminator.
    /// If capacity <= INLINE_CAP, no heap allocation occurs.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity <= INLINE_CAP {
            Self {
                repr: WideStringRepr::Inline {
                    buf: [0u16; INLINE_CAP],
                    len: 1, // Just the null terminator
                },
            }
        } else {
            Self {
                repr: WideStringRepr::Heap(Vec::with_capacity(capacity)),
            }
        }
    }

    /// Creates a new `WideString` from a path.
    #[inline]
    pub fn from_path(path: &Path) -> Self {
        let wide = path_to_wide(path);
        if wide.len() <= INLINE_CAP {
            let mut buf = [0u16; INLINE_CAP];
            buf[..wide.len()].copy_from_slice(&wide);
            Self {
                repr: WideStringRepr::Inline {
                    buf,
                    len: wide.len() as u8,
                },
            }
        } else {
            Self {
                repr: WideStringRepr::Heap(wide),
            }
        }
    }

    /// Creates a `WideString` from a pre-existing Vec<u16>.
    ///
    /// The Vec should be null-terminated.
    #[inline]
    pub fn from_vec(vec: Vec<u16>) -> Self {
        if vec.len() <= INLINE_CAP {
            let mut buf = [0u16; INLINE_CAP];
            buf[..vec.len()].copy_from_slice(&vec);
            Self {
                repr: WideStringRepr::Inline {
                    buf,
                    len: vec.len() as u8,
                },
            }
        } else {
            Self {
                repr: WideStringRepr::Heap(vec),
            }
        }
    }

    /// Returns a pointer to the null-terminated wide string.
    #[inline]
    pub fn as_ptr(&self) -> *const u16 {
        match &self.repr {
            WideStringRepr::Inline { buf, .. } => buf.as_ptr(),
            WideStringRepr::Heap(vec) => vec.as_ptr(),
        }
    }

    /// Returns the string as a PCWSTR for use with Windows APIs.
    #[inline]
    pub fn as_pcwstr(&self) -> windows::core::PCWSTR {
        windows::core::PCWSTR::from_raw(self.as_ptr())
    }

    /// Returns the length in UTF-16 code units, not including the null terminator.
    #[inline]
    pub fn len(&self) -> usize {
        match &self.repr {
            WideStringRepr::Inline { len, .. } => (*len as usize).saturating_sub(1),
            WideStringRepr::Heap(vec) => vec.len().saturating_sub(1),
        }
    }

    /// Returns true if the string is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if the string is stored inline (no heap allocation).
    #[inline]
    pub fn is_inline(&self) -> bool {
        matches!(self.repr, WideStringRepr::Inline { .. })
    }

    /// Converts back to a Rust String.
    #[inline]
    pub fn to_string_lossy(&self) -> String {
        from_wide(self.as_slice()).unwrap_or_else(|_| String::from("�"))
    }

    /// Returns the underlying buffer as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[u16] {
        match &self.repr {
            WideStringRepr::Inline { buf, len } => &buf[..*len as usize],
            WideStringRepr::Heap(vec) => vec,
        }
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

impl From<Vec<u16>> for WideString {
    fn from(vec: Vec<u16>) -> Self {
        Self::from_vec(vec)
    }
}

// ============================================================================
// Object Pool for High-Throughput Scenarios
// ============================================================================

/// A pool of reusable `Vec<u16>` buffers for high-throughput string conversion.
///
/// This pool reduces allocation overhead when converting many strings by reusing
/// previously allocated buffers.
///
/// # Example
///
/// ```
/// use ergonomic_windows::string::WideStringPool;
///
/// let mut pool = WideStringPool::new();
///
/// // Get a pooled string (may reuse an existing buffer)
/// let wide1 = pool.get("Hello, World!");
///
/// // Use it with Windows APIs
/// // let ptr = wide1.as_pcwstr();
///
/// // Return it to the pool when done
/// pool.put(wide1);
///
/// // The next get() may reuse the returned buffer
/// let wide2 = pool.get("Another string");
/// pool.put(wide2);
/// ```
pub struct WideStringPool {
    /// Pool of reusable buffers, sorted by capacity (smallest first).
    pool: Vec<Vec<u16>>,
    /// Maximum number of buffers to keep in the pool.
    max_size: usize,
    /// Maximum buffer capacity to keep (larger buffers are dropped).
    max_capacity: usize,
}

impl WideStringPool {
    /// Creates a new empty pool with default settings.
    ///
    /// Default: max 16 buffers, max 4KB capacity per buffer.
    #[inline]
    pub fn new() -> Self {
        Self {
            pool: Vec::new(),
            max_size: 16,
            max_capacity: 4096,
        }
    }

    /// Creates a pool with custom limits.
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum number of buffers to keep in the pool.
    /// * `max_capacity` - Maximum capacity (in u16 units) of buffers to keep.
    #[inline]
    pub fn with_limits(max_size: usize, max_capacity: usize) -> Self {
        Self {
            pool: Vec::with_capacity(max_size),
            max_size,
            max_capacity,
        }
    }

    /// Creates a pool pre-populated with buffers of the given capacity.
    ///
    /// Useful when you know the typical string size upfront.
    pub fn with_preallocated(count: usize, capacity: usize) -> Self {
        let mut pool = Self::with_limits(count, capacity.max(4096));
        for _ in 0..count {
            pool.pool.push(Vec::with_capacity(capacity));
        }
        pool
    }

    /// Gets a wide string from the pool, converting the given string.
    ///
    /// If the pool has a buffer of sufficient capacity, it will be reused.
    /// Otherwise, a new buffer is allocated.
    #[inline]
    pub fn get(&mut self, s: &str) -> PooledWideString {
        let utf16_len: usize = s.chars().map(|c| c.len_utf16()).sum();
        let required = utf16_len + 1;

        // Find a buffer with sufficient capacity
        let buffer = if let Some(idx) = self.pool.iter().position(|b| b.capacity() >= required) {
            self.pool.swap_remove(idx)
        } else {
            Vec::with_capacity(required)
        };

        let mut pooled = PooledWideString { buffer };
        pooled.buffer.clear();
        pooled.buffer.extend(s.encode_utf16());
        pooled.buffer.push(0);
        pooled
    }

    /// Gets a wide string for a path from the pool.
    #[inline]
    pub fn get_path(&mut self, path: &Path) -> PooledWideString {
        let os_str = path.as_os_str();
        let required = os_str.len() + 1;

        let buffer = if let Some(idx) = self.pool.iter().position(|b| b.capacity() >= required) {
            self.pool.swap_remove(idx)
        } else {
            Vec::with_capacity(required)
        };

        let mut pooled = PooledWideString { buffer };
        pooled.buffer.clear();
        pooled.buffer.extend(os_str.encode_wide());
        pooled.buffer.push(0);
        pooled
    }

    /// Returns a buffer to the pool for reuse.
    ///
    /// If the pool is full or the buffer is too large, it will be dropped.
    #[inline]
    pub fn put(&mut self, mut pooled: PooledWideString) {
        if self.pool.len() < self.max_size && pooled.buffer.capacity() <= self.max_capacity {
            pooled.buffer.clear();
            self.pool.push(pooled.buffer);
        }
        // Otherwise, let the buffer drop
    }

    /// Returns the number of buffers currently in the pool.
    #[inline]
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    /// Returns true if the pool is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }

    /// Clears all buffers from the pool.
    #[inline]
    pub fn clear(&mut self) {
        self.pool.clear();
    }

    /// Shrinks the pool to the given size, dropping excess buffers.
    pub fn shrink_to(&mut self, size: usize) {
        self.pool.truncate(size);
    }
}

impl Default for WideStringPool {
    fn default() -> Self {
        Self::new()
    }
}

/// A wide string backed by a pooled buffer.
///
/// This type should be returned to a `WideStringPool` via `pool.put()` when
/// no longer needed, to enable buffer reuse.
pub struct PooledWideString {
    buffer: Vec<u16>,
}

impl PooledWideString {
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

    /// Returns the underlying buffer as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[u16] {
        &self.buffer
    }

    /// Converts to a Rust String.
    #[inline]
    pub fn to_string_lossy(&self) -> String {
        from_wide(&self.buffer).unwrap_or_else(|_| String::from("�"))
    }

    /// Consumes this pooled string and returns the underlying buffer.
    ///
    /// Use this if you need to keep the buffer without returning it to the pool.
    #[inline]
    pub fn into_vec(self) -> Vec<u16> {
        self.buffer
    }

    /// Converts to a WideString, consuming this pooled string.
    #[inline]
    pub fn into_wide_string(self) -> WideString {
        WideString::from_vec(self.buffer)
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

    #[test]
    fn test_wide_string_sso_short() {
        // Short string should be inline
        let ws = WideString::new("Hello");
        assert!(ws.is_inline());
        assert_eq!(ws.len(), 5);
        assert_eq!(ws.to_string_lossy(), "Hello");
    }

    #[test]
    fn test_wide_string_sso_exact_boundary() {
        // String at exactly INLINE_CAP - 1 characters should be inline
        let s = "a".repeat(INLINE_CAP - 1);
        let ws = WideString::new(&s);
        assert!(ws.is_inline());
        assert_eq!(ws.len(), INLINE_CAP - 1);
    }

    #[test]
    fn test_wide_string_sso_over_boundary() {
        // String over INLINE_CAP - 1 characters should be on heap
        let s = "a".repeat(INLINE_CAP);
        let ws = WideString::new(&s);
        assert!(!ws.is_inline());
        assert_eq!(ws.len(), INLINE_CAP);
    }

    #[test]
    fn test_wide_string_sso_empty() {
        let ws = WideString::new("");
        assert!(ws.is_inline());
        assert_eq!(ws.len(), 0);
        assert!(ws.is_empty());
    }

    #[test]
    fn test_wide_string_sso_unicode() {
        // Unicode characters may take 2 UTF-16 code units
        let ws = WideString::new("Hello 🌍"); // 🌍 is 2 UTF-16 units
        assert!(ws.is_inline()); // Still fits in inline
        assert_eq!(ws.to_string_lossy(), "Hello 🌍");
    }

    #[test]
    fn test_wide_string_clone() {
        let ws1 = WideString::new("Hello");
        let ws2 = ws1.clone();
        assert_eq!(ws1.to_string_lossy(), ws2.to_string_lossy());
        assert!(ws1.is_inline());
        assert!(ws2.is_inline());

        let ws3 = WideString::new(&"a".repeat(100));
        let ws4 = ws3.clone();
        assert_eq!(ws3.to_string_lossy(), ws4.to_string_lossy());
        assert!(!ws3.is_inline());
        assert!(!ws4.is_inline());
    }

    #[test]
    fn test_wide_string_pool_basic() {
        let mut pool = WideStringPool::new();
        assert!(pool.is_empty());

        let s1 = pool.get("Hello");
        assert_eq!(s1.len(), 5);
        assert_eq!(s1.to_string_lossy(), "Hello");

        pool.put(s1);
        assert_eq!(pool.len(), 1);

        // Get another string - should reuse the buffer
        let s2 = pool.get("Hi");
        assert_eq!(s2.len(), 2);
        assert_eq!(pool.len(), 0); // Buffer taken from pool

        pool.put(s2);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn test_wide_string_pool_preallocated() {
        let mut pool = WideStringPool::with_preallocated(4, 256);
        assert_eq!(pool.len(), 4);

        let s1 = pool.get("Test");
        assert_eq!(pool.len(), 3);

        pool.put(s1);
        assert_eq!(pool.len(), 4);
    }

    #[test]
    fn test_wide_string_pool_max_size() {
        let mut pool = WideStringPool::with_limits(2, 1024);

        let s1 = pool.get("A");
        let s2 = pool.get("B");
        let s3 = pool.get("C");

        pool.put(s1);
        pool.put(s2);
        pool.put(s3); // Should be dropped, pool is full

        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn test_wide_string_pool_convert_to_wide_string() {
        let mut pool = WideStringPool::new();
        let pooled = pool.get("Hello");

        // Convert to WideString
        let ws = pooled.into_wide_string();
        assert_eq!(ws.to_string_lossy(), "Hello");
        // Pooled buffer is now owned by WideString, can't return to pool
    }

    // ============================================================================
    // Unicode Edge Cases Tests
    // ============================================================================

    #[test]
    fn test_unicode_surrogate_pairs() {
        // Characters outside the BMP (Basic Multilingual Plane) require surrogate pairs
        // 🎉 U+1F389 = surrogate pair D83C DF89
        let emoji = "🎉";
        let wide = to_wide(emoji);
        assert_eq!(wide.len(), 3); // 2 UTF-16 units + null terminator
        assert_eq!(wide[0], 0xD83C); // High surrogate
        assert_eq!(wide[1], 0xDF89); // Low surrogate
        assert_eq!(wide[2], 0); // Null terminator

        let back = from_wide(&wide).unwrap();
        assert_eq!(back, emoji);
    }

    #[test]
    fn test_unicode_multiple_surrogate_pairs() {
        // Multiple emoji that require surrogate pairs
        let text = "Hello 🌍🌎🌏!";
        let wide = to_wide(text);
        let back = from_wide(&wide).unwrap();
        assert_eq!(back, text);
    }

    #[test]
    fn test_unicode_bom() {
        // Byte Order Mark U+FEFF
        let with_bom = "\u{FEFF}Hello";
        let wide = to_wide(with_bom);
        assert_eq!(wide[0], 0xFEFF); // BOM
        let back = from_wide(&wide).unwrap();
        assert_eq!(back, with_bom);
    }

    #[test]
    fn test_unicode_various_scripts() {
        // Test various Unicode scripts
        let texts = [
            "ASCII only",
            "日本語テスト",          // Japanese
            "한국어 테스트",         // Korean
            "中文测试",              // Chinese
            "Тест на русском",       // Russian Cyrillic
            "Ελληνικά",              // Greek
            "עברית",                 // Hebrew
            "العربية",               // Arabic
            "हिन्दी",                // Hindi
            "ไทย",                   // Thai
        ];

        for text in texts {
            let wide = to_wide(text);
            let back = from_wide(&wide).unwrap();
            assert_eq!(back, text, "Failed roundtrip for: {}", text);
        }
    }

    #[test]
    fn test_unicode_zero_width_chars() {
        // Zero-width joiner U+200D, zero-width non-joiner U+200C
        let text = "a\u{200D}b\u{200C}c";
        let wide = to_wide(text);
        let back = from_wide(&wide).unwrap();
        assert_eq!(back, text);
    }

    #[test]
    fn test_unicode_combining_characters() {
        // e followed by combining acute accent = é
        let text = "e\u{0301}";
        let wide = to_wide(text);
        let back = from_wide(&wide).unwrap();
        assert_eq!(back, text);
    }

    #[test]
    fn test_unicode_emoji_sequences() {
        // Family emoji with ZWJ sequence
        let text = "👨\u{200D}👩\u{200D}👧";
        let wide = to_wide(text);
        let back = from_wide(&wide).unwrap();
        assert_eq!(back, text);
    }

    #[test]
    fn test_invalid_utf16_lone_high_surrogate() {
        // A lone high surrogate (0xD800-0xDBFF) without following low surrogate
        let invalid: Vec<u16> = vec![0xD800, 0];
        let result = from_wide(&invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_utf16_lone_low_surrogate() {
        // A lone low surrogate (0xDC00-0xDFFF) without preceding high surrogate
        let invalid: Vec<u16> = vec![0xDC00, 0];
        let result = from_wide(&invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_utf16_reversed_surrogates() {
        // Low surrogate followed by high surrogate (reversed)
        let invalid: Vec<u16> = vec![0xDC00, 0xD800, 0];
        let result = from_wide(&invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_wide_string_sso_with_surrogate_pairs() {
        // Emoji that requires surrogate pair should work with SSO
        let ws = WideString::new("🎉");
        assert!(ws.is_inline());
        assert_eq!(ws.len(), 2); // 2 UTF-16 units
        assert_eq!(ws.to_string_lossy(), "🎉");
    }
}
