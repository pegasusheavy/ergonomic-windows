//! Pipe utilities for inter-process communication.
//!
//! Provides safe wrappers for Windows anonymous and named pipes.

use crate::error::Result;
use crate::handle::OwnedHandle;
use crate::string::WideString;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ,
    FILE_GENERIC_WRITE, FILE_SHARE_NONE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX, PIPE_ACCESS_INBOUND,
    PIPE_ACCESS_OUTBOUND,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, CreatePipe, DisconnectNamedPipe, PeekNamedPipe,
    SetNamedPipeHandleState, WaitNamedPipeW, NAMED_PIPE_MODE, PIPE_READMODE_BYTE,
    PIPE_READMODE_MESSAGE, PIPE_TYPE_BYTE, PIPE_TYPE_MESSAGE, PIPE_WAIT,
};

/// An anonymous pipe pair for parent-child process communication.
pub struct AnonymousPipe {
    /// The read end of the pipe.
    pub read: OwnedHandle,
    /// The write end of the pipe.
    pub write: OwnedHandle,
}

impl AnonymousPipe {
    /// Creates a new anonymous pipe.
    ///
    /// Returns a pair of handles: (read_handle, write_handle).
    pub fn new() -> Result<Self> {
        Self::with_size(0)
    }

    /// Creates a new anonymous pipe with a specific buffer size.
    pub fn with_size(size: u32) -> Result<Self> {
        let mut read_handle = HANDLE::default();
        let mut write_handle = HANDLE::default();

        // SAFETY: CreatePipe is safe with valid output parameters
        unsafe {
            CreatePipe(&mut read_handle, &mut write_handle, None, size)?;
        }

        Ok(Self {
            read: OwnedHandle::new(read_handle)?,
            write: OwnedHandle::new(write_handle)?,
        })
    }
}

/// Pipe access mode for named pipes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeAccess {
    /// Read-only access.
    Inbound,
    /// Write-only access.
    Outbound,
    /// Read-write access.
    Duplex,
}

impl PipeAccess {
    fn to_flags(self) -> FILE_FLAGS_AND_ATTRIBUTES {
        match self {
            PipeAccess::Inbound => FILE_FLAGS_AND_ATTRIBUTES(PIPE_ACCESS_INBOUND.0),
            PipeAccess::Outbound => FILE_FLAGS_AND_ATTRIBUTES(PIPE_ACCESS_OUTBOUND.0),
            PipeAccess::Duplex => FILE_FLAGS_AND_ATTRIBUTES(PIPE_ACCESS_DUPLEX.0),
        }
    }
}

/// Pipe mode for named pipes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeMode {
    /// Byte mode - data is read/written as a stream of bytes.
    Byte,
    /// Message mode - data is read/written as discrete messages.
    Message,
}

impl PipeMode {
    fn to_type_flags(self) -> u32 {
        match self {
            PipeMode::Byte => PIPE_TYPE_BYTE.0,
            PipeMode::Message => PIPE_TYPE_MESSAGE.0,
        }
    }

    fn to_read_flags(self) -> u32 {
        match self {
            PipeMode::Byte => PIPE_READMODE_BYTE.0,
            PipeMode::Message => PIPE_READMODE_MESSAGE.0,
        }
    }
}

/// A named pipe server.
pub struct NamedPipeServer {
    handle: OwnedHandle,
    name: String,
}

impl NamedPipeServer {
    /// Creates a new named pipe server.
    ///
    /// The name should be in the format `\\.\pipe\pipename`.
    pub fn new(name: &str, access: PipeAccess, mode: PipeMode) -> Result<Self> {
        Self::with_options(name, access, mode, 1, 4096, 4096, 0)
    }

    /// Creates a named pipe server with full options.
    pub fn with_options(
        name: &str,
        access: PipeAccess,
        mode: PipeMode,
        max_instances: u32,
        out_buffer_size: u32,
        in_buffer_size: u32,
        default_timeout: u32,
    ) -> Result<Self> {
        let name_wide = WideString::new(name);

        let pipe_mode = NAMED_PIPE_MODE(mode.to_type_flags() | mode.to_read_flags() | PIPE_WAIT.0);

        // SAFETY: CreateNamedPipeW is safe with valid parameters
        let handle = unsafe {
            CreateNamedPipeW(
                name_wide.as_pcwstr(),
                access.to_flags(),
                pipe_mode,
                max_instances,
                out_buffer_size,
                in_buffer_size,
                default_timeout,
                None,
            )
        };

        if handle.is_invalid() {
            return Err(crate::error::last_error());
        }

        Ok(Self {
            handle: OwnedHandle::new(handle)?,
            name: name.to_string(),
        })
    }

    /// Waits for a client to connect.
    pub fn accept(&self) -> Result<()> {
        // SAFETY: ConnectNamedPipe is safe with valid handle
        unsafe {
            ConnectNamedPipe(self.handle.as_raw(), None)?;
        }
        Ok(())
    }

    /// Disconnects from the current client.
    pub fn disconnect(&self) -> Result<()> {
        // SAFETY: DisconnectNamedPipe is safe with valid handle
        unsafe {
            DisconnectNamedPipe(self.handle.as_raw())?;
        }
        Ok(())
    }

    /// Reads data from the pipe.
    pub fn read(&self, buffer: &mut [u8]) -> Result<usize> {
        let mut bytes_read = 0u32;
        // SAFETY: ReadFile is safe with valid parameters
        unsafe {
            ReadFile(
                self.handle.as_raw(),
                Some(buffer),
                Some(&mut bytes_read),
                None,
            )?;
        }
        Ok(bytes_read as usize)
    }

    /// Writes data to the pipe.
    pub fn write(&self, data: &[u8]) -> Result<usize> {
        let mut bytes_written = 0u32;
        // SAFETY: WriteFile is safe with valid parameters
        unsafe {
            WriteFile(
                self.handle.as_raw(),
                Some(data),
                Some(&mut bytes_written),
                None,
            )?;
        }
        Ok(bytes_written as usize)
    }

    /// Peeks at data in the pipe without removing it.
    pub fn peek(&self, buffer: &mut [u8]) -> Result<(usize, usize)> {
        let mut bytes_read = 0u32;
        let mut total_bytes_avail = 0u32;

        // SAFETY: PeekNamedPipe is safe with valid parameters
        unsafe {
            PeekNamedPipe(
                self.handle.as_raw(),
                Some(buffer.as_mut_ptr() as *mut _),
                buffer.len() as u32,
                Some(&mut bytes_read),
                Some(&mut total_bytes_avail),
                None,
            )?;
        }

        Ok((bytes_read as usize, total_bytes_avail as usize))
    }

    /// Returns the pipe name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A named pipe client.
pub struct NamedPipeClient {
    handle: OwnedHandle,
}

impl NamedPipeClient {
    /// Connects to a named pipe server.
    pub fn connect(name: &str) -> Result<Self> {
        Self::connect_timeout(name, None)
    }

    /// Connects to a named pipe server with a timeout.
    pub fn connect_timeout(name: &str, timeout_ms: Option<u32>) -> Result<Self> {
        let name_wide = WideString::new(name);

        // Wait for pipe to be available
        if let Some(timeout) = timeout_ms {
            // SAFETY: WaitNamedPipeW is safe with valid parameters
            let result = unsafe { WaitNamedPipeW(name_wide.as_pcwstr(), timeout) };
            if !result.as_bool() {
                return Err(crate::error::last_error());
            }
        }

        // Connect to the pipe
        // SAFETY: CreateFileW is safe with valid parameters
        let handle = unsafe {
            CreateFileW(
                name_wide.as_pcwstr(),
                (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )?
        };

        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Sets the pipe to message mode.
    pub fn set_message_mode(&self) -> Result<()> {
        let mode = PIPE_READMODE_MESSAGE;
        // SAFETY: SetNamedPipeHandleState is safe with valid parameters
        unsafe {
            SetNamedPipeHandleState(self.handle.as_raw(), Some(&mode), None, None)?;
        }
        Ok(())
    }

    /// Reads data from the pipe.
    pub fn read(&self, buffer: &mut [u8]) -> Result<usize> {
        let mut bytes_read = 0u32;
        // SAFETY: ReadFile is safe with valid parameters
        unsafe {
            ReadFile(
                self.handle.as_raw(),
                Some(buffer),
                Some(&mut bytes_read),
                None,
            )?;
        }
        Ok(bytes_read as usize)
    }

    /// Writes data to the pipe.
    pub fn write(&self, data: &[u8]) -> Result<usize> {
        let mut bytes_written = 0u32;
        // SAFETY: WriteFile is safe with valid parameters
        unsafe {
            WriteFile(
                self.handle.as_raw(),
                Some(data),
                Some(&mut bytes_written),
                None,
            )?;
        }
        Ok(bytes_written as usize)
    }
}

/// Helper to generate a unique pipe name.
pub fn unique_pipe_name(prefix: &str) -> String {
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    let pid = process::id();
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    format!(r"\\.\pipe\{}_{}_{}", prefix, pid, time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anonymous_pipe() {
        let pipe = AnonymousPipe::new().unwrap();

        // Write from write end
        let data = b"Hello, pipe!";
        let mut written = 0u32;
        unsafe {
            WriteFile(pipe.write.as_raw(), Some(data), Some(&mut written), None).unwrap();
        }
        assert_eq!(written as usize, data.len());

        // Read from read end
        let mut buffer = [0u8; 32];
        let mut read = 0u32;
        unsafe {
            ReadFile(pipe.read.as_raw(), Some(&mut buffer), Some(&mut read), None).unwrap();
        }
        assert_eq!(&buffer[..read as usize], data);
    }

    #[test]
    fn test_unique_pipe_name() {
        let name1 = unique_pipe_name("test");
        let _name2 = unique_pipe_name("test");

        assert!(name1.starts_with(r"\\.\pipe\test_"));
        // Names should be different (different timestamps)
        // But they might be the same if called too fast, so we just check format
    }
}
