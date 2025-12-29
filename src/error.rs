//! Error handling utilities for Windows API calls.
//!
//! Provides ergonomic error types that wrap Windows error codes and convert them
//! into idiomatic Rust `Result` types.

use thiserror::Error;
use windows::core::Error as WinError;

/// The main error type for this crate.
#[derive(Error, Debug)]
pub enum Error {
    /// A Windows API error with its error code.
    #[error("Windows API error: {0}")]
    Windows(#[from] WinError),

    /// A null pointer was encountered where a valid pointer was expected.
    #[error("Null pointer error: {context}")]
    NullPointer {
        /// Description of where the null pointer was encountered.
        context: &'static str,
    },

    /// An invalid handle was provided or returned.
    #[error("Invalid handle: {context}")]
    InvalidHandle {
        /// Description of the invalid handle context.
        context: &'static str,
    },

    /// A string conversion error occurred.
    #[error("String conversion error: {0}")]
    StringConversion(String),

    /// A buffer was too small for the requested operation.
    #[error("Buffer too small: needed {needed}, got {actual}")]
    BufferTooSmall {
        /// The required buffer size.
        needed: usize,
        /// The actual buffer size provided.
        actual: usize,
    },

    /// The requested resource was not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Access was denied to the requested resource.
    #[error("Access denied: {0}")]
    AccessDenied(String),

    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A custom error with a message.
    #[error("{0}")]
    Custom(String),
}

/// A specialized `Result` type for Windows API operations.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Creates a new null pointer error with the given context.
    pub fn null_pointer(context: &'static str) -> Self {
        Error::NullPointer { context }
    }

    /// Creates a new invalid handle error with the given context.
    pub fn invalid_handle(context: &'static str) -> Self {
        Error::InvalidHandle { context }
    }

    /// Creates a new string conversion error.
    pub fn string_conversion(msg: impl Into<String>) -> Self {
        Error::StringConversion(msg.into())
    }

    /// Creates a new buffer too small error.
    pub fn buffer_too_small(needed: usize, actual: usize) -> Self {
        Error::BufferTooSmall { needed, actual }
    }

    /// Creates a new not found error.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Error::NotFound(msg.into())
    }

    /// Creates a new access denied error.
    pub fn access_denied(msg: impl Into<String>) -> Self {
        Error::AccessDenied(msg.into())
    }

    /// Creates a custom error with the given message.
    pub fn custom(msg: impl Into<String>) -> Self {
        Error::Custom(msg.into())
    }

    /// Returns the Windows error code if this is a Windows error.
    pub fn win32_error_code(&self) -> Option<u32> {
        match self {
            Error::Windows(e) => Some(e.code().0 as u32),
            _ => None,
        }
    }
}

/// Extension trait for converting Windows `Result` types.
pub trait ResultExt<T> {
    /// Converts a Windows result to our Result type.
    fn to_result(self) -> Result<T>;
}

impl<T> ResultExt<T> for windows::core::Result<T> {
    fn to_result(self) -> Result<T> {
        self.map_err(Error::from)
    }
}

/// Gets the last Windows error as our Error type.
pub fn last_error() -> Error {
    Error::Windows(WinError::from_win32())
}

/// Checks if the last error indicates success and returns Ok(()), otherwise returns the error.
pub fn check_last_error() -> Result<()> {
    let err = WinError::from_win32();
    if err.code().is_ok() {
        Ok(())
    } else {
        Err(Error::Windows(err))
    }
}
