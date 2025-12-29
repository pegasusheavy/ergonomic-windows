//! # Ergonomic Windows
//!
//! Ergonomic wrappers around Windows APIs for Rust.
//!
//! This crate provides safe, idiomatic Rust interfaces to common Windows functionality:
//!
//! - **Error Handling**: Rich error types with Windows error code support
//! - **Handle Management**: RAII wrappers for Windows handles
//! - **String Utilities**: Easy conversion between Rust and Windows strings
//! - **Process Management**: Create, manage, and query Windows processes
//! - **File System**: Windows-specific file operations
//! - **Registry**: Read and write Windows Registry keys
//! - **Windows**: Create windows and handle messages
//!
//! ## Quick Start
//!
//! ```no_run
//! use ergonomic_windows::process::Command;
//! use ergonomic_windows::registry::{Key, RootKey, Access};
//!
//! // Spawn a process
//! let process = Command::new("notepad.exe")
//!     .arg("file.txt")
//!     .spawn()?;
//!
//! // Read from the registry
//! let key = Key::open(
//!     RootKey::CURRENT_USER,
//!     r"Software\Microsoft\Windows\CurrentVersion",
//!     Access::READ,
//! )?;
//! let value = key.get_value("ProgramFilesDir")?;
//!
//! # Ok::<(), ergonomic_windows::error::Error>(())
//! ```
//!
//! ## Feature Highlights
//!
//! ### RAII Handle Management
//!
//! Windows handles are automatically closed when dropped:
//!
//! ```ignore
//! use ergonomic_windows::handle::OwnedHandle;
//!
//! {
//!     let handle = OwnedHandle::new(some_raw_handle)?;
//!     // Use the handle...
//! } // Handle is automatically closed here
//! # Ok::<(), ergonomic_windows::error::Error>(())
//! ```
//!
//! ### Easy String Conversion
//!
//! Convert between Rust and Windows strings effortlessly:
//!
//! ```
//! use ergonomic_windows::string::{to_wide, from_wide, WideString};
//!
//! // To wide string
//! let wide = to_wide("Hello, Windows!");
//!
//! // From wide string
//! let back = from_wide(&wide).unwrap();
//!
//! // For Windows APIs
//! let ws = WideString::new("Hello");
//! // use ws.as_pcwstr() with Windows APIs
//! ```
//!
//! ### Process Management
//!
//! Create and manage processes with a fluent API:
//!
//! ```no_run
//! use ergonomic_windows::process::Command;
//!
//! // Run a command and wait for completion
//! let exit_code = Command::new("cmd.exe")
//!     .args(["/c", "echo", "Hello"])
//!     .no_window()
//!     .run()?;
//!
//! # Ok::<(), ergonomic_windows::error::Error>(())
//! ```
//!
//! ### Registry Access
//!
//! Read and write registry values with type safety:
//!
//! ```no_run
//! use ergonomic_windows::registry::{Key, RootKey, Access, Value};
//!
//! // Write a value
//! let key = Key::create(RootKey::CURRENT_USER, r"Software\MyApp", Access::ALL)?;
//! key.set_value("Setting", &Value::dword(42))?;
//!
//! // Read it back
//! let value = key.get_value("Setting")?;
//! assert_eq!(value.as_dword(), Some(42));
//!
//! # Ok::<(), ergonomic_windows::error::Error>(())
//! ```

#![cfg(windows)]
#![warn(missing_docs)]

pub mod error;
pub mod fs;
pub mod handle;
pub mod process;
pub mod registry;
pub mod string;
pub mod window;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::error::{Error, Result, ResultExt};
    pub use crate::fs::{exists, is_dir, is_file, FileAttributes, OpenOptions};
    pub use crate::handle::{BorrowedHandle, HandleExt, OwnedHandle};
    pub use crate::process::{Command, Process, ProcessAccess};
    pub use crate::registry::{Access, Key, RootKey, Value};
    pub use crate::string::{from_wide, from_wide_buffer, to_wide, WideString};
    pub use crate::window::{
        ExStyle, Message, MessageHandler, ShowCommand, Style, Window, WindowBuilder,
    };
}
