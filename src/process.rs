//! Process management utilities.
//!
//! Provides ergonomic wrappers for creating, managing, and querying Windows processes.

use crate::error::{Error, Result};
use crate::handle::OwnedHandle;
use crate::string::{to_wide, WideString};
use std::borrow::Cow;
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::System::Threading::{
    CreateProcessW, GetExitCodeProcess, OpenProcess, TerminateProcess, WaitForSingleObject,
    CREATE_NEW_CONSOLE, CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, PROCESS_CREATION_FLAGS,
    PROCESS_INFORMATION, PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE, STARTUPINFOW,
};

/// Represents a running or completed process.
pub struct Process {
    handle: OwnedHandle,
    pid: u32,
}

impl Process {
    /// Opens an existing process by its process ID.
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID to open.
    /// * `access` - The desired access rights. Use `ProcessAccess` for common combinations.
    ///
    /// # Errors
    ///
    /// Returns an error if the process doesn't exist or access is denied.
    pub fn open(pid: u32, access: ProcessAccess) -> Result<Self> {
        // SAFETY: OpenProcess is safe to call with any pid and access rights.
        // It will return an error if the process doesn't exist or access is denied.
        // The returned handle is valid and we take ownership of it.
        let handle = unsafe { OpenProcess(access.0, false, pid)? };
        Ok(Self {
            handle: OwnedHandle::new(handle)?,
            pid,
        })
    }

    /// Returns the process ID.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Returns the raw process handle.
    pub fn handle(&self) -> HANDLE {
        self.handle.as_raw()
    }

    /// Waits for the process to exit.
    ///
    /// Returns `Ok(exit_code)` when the process exits, or an error if waiting fails.
    pub fn wait(&self) -> Result<u32> {
        self.wait_timeout(None)
    }

    /// Waits for the process to exit with a timeout.
    ///
    /// Returns `Ok(exit_code)` if the process exits within the timeout,
    /// or an error if the timeout expires or waiting fails.
    pub fn wait_timeout(&self, timeout: Option<Duration>) -> Result<u32> {
        let timeout_ms = timeout
            .map(|d| d.as_millis() as u32)
            .unwrap_or(windows::Win32::System::Threading::INFINITE);

        // SAFETY: self.handle is a valid process handle that we own.
        // WaitForSingleObject is safe to call on any valid handle.
        let result = unsafe { WaitForSingleObject(self.handle.as_raw(), timeout_ms) };

        match result {
            WAIT_OBJECT_0 => self.exit_code(),
            WAIT_TIMEOUT => Err(Error::custom("Wait timed out")),
            _ => Err(Error::custom("Wait failed")),
        }
    }

    /// Checks if the process has exited without blocking.
    ///
    /// Returns `Ok(Some(exit_code))` if exited, `Ok(None)` if still running.
    pub fn try_wait(&self) -> Result<Option<u32>> {
        // SAFETY: self.handle is a valid process handle that we own.
        // A timeout of 0 makes this a non-blocking check.
        let result = unsafe { WaitForSingleObject(self.handle.as_raw(), 0) };

        match result {
            WAIT_OBJECT_0 => Ok(Some(self.exit_code()?)),
            WAIT_TIMEOUT => Ok(None),
            _ => Err(Error::custom("Wait failed")),
        }
    }

    /// Gets the exit code of the process.
    ///
    /// If the process is still running, this returns `STILL_ACTIVE` (259).
    pub fn exit_code(&self) -> Result<u32> {
        let mut exit_code = 0u32;
        // SAFETY: self.handle is a valid process handle with PROCESS_QUERY_INFORMATION access.
        // exit_code is a valid output parameter.
        unsafe {
            GetExitCodeProcess(self.handle.as_raw(), &mut exit_code)?;
        }
        Ok(exit_code)
    }

    /// Terminates the process.
    ///
    /// # Arguments
    ///
    /// * `exit_code` - The exit code to use for the terminated process.
    ///
    /// # Warning
    ///
    /// This function immediately terminates the process without allowing it to clean up.
    /// Use with caution.
    pub fn terminate(&self, exit_code: u32) -> Result<()> {
        // SAFETY: self.handle is a valid process handle with PROCESS_TERMINATE access.
        // The process will be terminated immediately.
        unsafe {
            TerminateProcess(self.handle.as_raw(), exit_code)?;
        }
        Ok(())
    }

    /// Checks if the process is still running.
    pub fn is_running(&self) -> Result<bool> {
        Ok(self.try_wait()?.is_none())
    }
}

/// Process access rights for opening existing processes.
#[derive(Clone, Copy, Debug)]
pub struct ProcessAccess(pub windows::Win32::System::Threading::PROCESS_ACCESS_RIGHTS);

impl ProcessAccess {
    /// Full access to the process.
    pub const ALL: Self = Self(windows::Win32::System::Threading::PROCESS_ALL_ACCESS);

    /// Access to query process information.
    pub const QUERY: Self = Self(PROCESS_QUERY_INFORMATION);

    /// Access to terminate the process.
    pub const TERMINATE: Self = Self(PROCESS_TERMINATE);

    /// Access to query information and terminate.
    pub const QUERY_AND_TERMINATE: Self =
        Self(windows::Win32::System::Threading::PROCESS_ACCESS_RIGHTS(
            PROCESS_QUERY_INFORMATION.0 | PROCESS_TERMINATE.0,
        ));
}

/// Builder for creating new processes.
pub struct Command {
    program: String,
    args: Vec<String>,
    current_dir: Option<String>,
    creation_flags: PROCESS_CREATION_FLAGS,
    env: Option<Vec<(String, String)>>,
}

impl Command {
    /// Creates a new command for the specified program.
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
            creation_flags: PROCESS_CREATION_FLAGS(0),
            env: None,
        }
    }

    /// Adds an argument to the command.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Adds multiple arguments to the command.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Sets the working directory for the process.
    pub fn current_dir(mut self, dir: impl Into<String>) -> Self {
        self.current_dir = Some(dir.into());
        self
    }

    /// Creates the process in a new console window.
    pub fn new_console(mut self) -> Self {
        self.creation_flags.0 |= CREATE_NEW_CONSOLE.0;
        self
    }

    /// Creates the process without a window.
    pub fn no_window(mut self) -> Self {
        self.creation_flags.0 |= CREATE_NO_WINDOW.0;
        self
    }

    /// Sets an environment variable for the process.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env
            .get_or_insert_with(Vec::new)
            .push((key.into(), value.into()));
        self
    }

    /// Spawns the process.
    ///
    /// # Errors
    ///
    /// Returns an error if the process cannot be created (e.g., program not found).
    pub fn spawn(self) -> Result<Process> {
        let command_line = self.build_command_line();
        let mut command_line_wide = to_wide(&command_line);

        let current_dir_wide = self.current_dir.as_ref().map(|d| WideString::new(d));

        let env_block = self.build_env_block();

        let startup_info = STARTUPINFOW {
            cb: std::mem::size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };

        let mut process_info = PROCESS_INFORMATION::default();

        let creation_flags = if env_block.is_some() {
            PROCESS_CREATION_FLAGS(self.creation_flags.0 | CREATE_UNICODE_ENVIRONMENT.0)
        } else {
            self.creation_flags
        };

        // SAFETY: All pointers passed to CreateProcessW are valid:
        // - command_line_wide is a valid mutable buffer (CreateProcessW may modify it)
        // - env_block is either None or points to a valid double-null-terminated block
        // - current_dir_wide is either None or a valid null-terminated string
        // - startup_info and process_info are valid stack-allocated structs
        unsafe {
            match &current_dir_wide {
                Some(dir) => CreateProcessW(
                    None,
                    windows::core::PWSTR(command_line_wide.as_mut_ptr()),
                    None,
                    None,
                    false,
                    creation_flags,
                    env_block.as_ref().map(|e| e.as_ptr() as *const _),
                    dir.as_pcwstr(),
                    &startup_info,
                    &mut process_info,
                )?,
                None => CreateProcessW(
                    None,
                    windows::core::PWSTR(command_line_wide.as_mut_ptr()),
                    None,
                    None,
                    false,
                    creation_flags,
                    env_block.as_ref().map(|e| e.as_ptr() as *const _),
                    None,
                    &startup_info,
                    &mut process_info,
                )?,
            };
        }

        // Close the thread handle immediately - we don't need it.
        // SAFETY: process_info.hThread is a valid handle returned by CreateProcessW.
        // We're responsible for closing it, and we don't need to keep it.
        if !process_info.hThread.is_invalid() {
            unsafe {
                let _ = CloseHandle(process_info.hThread);
            }
        }

        Ok(Process {
            handle: OwnedHandle::new(process_info.hProcess)?,
            pid: process_info.dwProcessId,
        })
    }

    /// Spawns the process and waits for it to complete.
    pub fn run(self) -> Result<u32> {
        let process = self.spawn()?;
        process.wait()
    }

    fn build_command_line(&self) -> String {
        // Pre-calculate total length to minimize allocations.
        // Each arg needs at most: original length + 2 (quotes) + 1 (space separator)
        // Plus extra for potential backslash escaping (worst case: double the length)
        let total_len =
            self.program.len() + 3 + self.args.iter().map(|a| a.len() * 2 + 3).sum::<usize>();

        let mut cmd = String::with_capacity(total_len);
        cmd.push_str(&quote_arg(&self.program));
        for arg in &self.args {
            cmd.push(' ');
            cmd.push_str(&quote_arg(arg));
        }
        cmd
    }

    fn build_env_block(&self) -> Option<Vec<u16>> {
        let env = self.env.as_ref()?;
        let mut block = Vec::new();

        for (key, value) in env {
            let entry = format!("{}={}", key, value);
            block.extend(entry.encode_utf16());
            block.push(0);
        }
        block.push(0); // Double null terminator

        Some(block)
    }
}

/// Quotes a command-line argument if necessary.
///
/// Returns `Cow::Borrowed` when no quoting is needed to avoid allocation.
#[inline]
fn quote_arg(arg: &str) -> Cow<'_, str> {
    // Check if quoting is needed
    let needs_quoting = arg.is_empty() || arg.bytes().any(|b| b == b' ' || b == b'\t' || b == b'"');

    if needs_quoting {
        let mut quoted = String::with_capacity(arg.len() + 2);
        quoted.push('"');

        let mut chars = arg.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                let mut backslash_count = 1;
                while chars.peek() == Some(&'\\') {
                    chars.next();
                    backslash_count += 1;
                }

                if chars.peek() == Some(&'"') || chars.peek().is_none() {
                    // Escape all backslashes
                    for _ in 0..backslash_count * 2 {
                        quoted.push('\\');
                    }
                } else {
                    for _ in 0..backslash_count {
                        quoted.push('\\');
                    }
                }
            } else if c == '"' {
                quoted.push('\\');
                quoted.push('"');
            } else {
                quoted.push(c);
            }
        }

        quoted.push('"');
        Cow::Owned(quoted)
    } else {
        Cow::Borrowed(arg)
    }
}

/// Gets the current process ID.
///
/// This function always succeeds and is completely safe.
/// It simply returns the process ID of the calling process.
#[inline]
pub fn current_pid() -> u32 {
    // SAFETY: GetCurrentProcessId is a pure function with no preconditions.
    // It has no side effects, takes no parameters, and always succeeds.
    // The Windows documentation guarantees this function cannot fail.
    // This unsafe block is only required because it's FFI, not because
    // the operation itself is unsafe in any way.
    unsafe { windows::Win32::System::Threading::GetCurrentProcessId() }
}

/// Gets a pseudo-handle to the current process.
///
/// This function always succeeds and returns a special pseudo-handle
/// that represents the current process. This pseudo-handle:
/// - Does NOT need to be closed (and must not be passed to CloseHandle)
/// - Is only valid within the current process
/// - Can be used anywhere a process handle is expected for the current process
///
/// # Safety Note
///
/// While this function is marked as requiring `unsafe` due to FFI,
/// the operation itself is completely safe - it cannot fail and has
/// no preconditions. The returned handle is a constant value (-1)
/// that Windows interprets as "the current process".
#[inline]
pub fn current_process() -> HANDLE {
    // SAFETY: GetCurrentProcess is a pure function with no preconditions.
    // It returns a pseudo-handle (constant -1) that represents the current process.
    // This function cannot fail and has no side effects.
    // The unsafe block is only required because it's FFI.
    unsafe { windows::Win32::System::Threading::GetCurrentProcess() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_arg() {
        assert_eq!(quote_arg("simple"), "simple");
        assert_eq!(quote_arg("with space"), "\"with space\"");
        assert_eq!(quote_arg(""), "\"\"");
    }

    #[test]
    fn test_current_pid() {
        let pid = current_pid();
        assert!(pid > 0);
    }

    // ============================================================================
    // Process Spawning Integration Tests
    // ============================================================================

    #[test]
    fn test_spawn_cmd_echo() {
        // Spawn cmd.exe with /c to run a simple command
        let process = Command::new("cmd.exe")
            .arg("/c")
            .arg("echo hello")
            .no_window()
            .spawn();

        assert!(process.is_ok(), "Failed to spawn cmd.exe");
        let process = process.unwrap();
        assert!(process.pid() > 0);

        let exit_code = process.wait();
        assert!(exit_code.is_ok());
        assert_eq!(exit_code.unwrap(), 0);
    }

    #[test]
    fn test_spawn_cmd_exit_code() {
        // Test that we can get non-zero exit codes
        let exit_code = Command::new("cmd.exe")
            .arg("/c")
            .arg("exit 42")
            .no_window()
            .run();

        assert!(exit_code.is_ok());
        assert_eq!(exit_code.unwrap(), 42);
    }

    #[test]
    fn test_spawn_nonexistent_program() {
        // Spawning a nonexistent program should fail
        let result = Command::new("this_program_does_not_exist_12345.exe").spawn();
        assert!(result.is_err());
    }

    #[test]
    fn test_spawn_with_args() {
        // Test passing arguments with spaces and special chars
        let exit_code = Command::new("cmd.exe")
            .arg("/c")
            .arg("echo")
            .arg("hello world")
            .no_window()
            .run();

        assert!(exit_code.is_ok());
        assert_eq!(exit_code.unwrap(), 0);
    }

    #[test]
    fn test_spawn_with_working_directory() {
        // Test setting working directory
        let temp_dir = std::env::temp_dir();
        let temp_str = temp_dir.to_string_lossy().into_owned();

        let exit_code = Command::new("cmd.exe")
            .arg("/c")
            .arg("cd")
            .current_dir(temp_str)
            .no_window()
            .run();

        assert!(exit_code.is_ok());
        assert_eq!(exit_code.unwrap(), 0);
    }

    #[test]
    fn test_spawn_with_env() {
        // Test setting environment variables
        let exit_code = Command::new("cmd.exe")
            .arg("/c")
            .arg("echo %TEST_VAR%")
            .env("TEST_VAR", "hello_test")
            .no_window()
            .run();

        assert!(exit_code.is_ok());
        assert_eq!(exit_code.unwrap(), 0);
    }

    #[test]
    fn test_try_wait_running_process() {
        // Spawn a process that sleeps briefly
        let process = Command::new("cmd.exe")
            .arg("/c")
            .arg("timeout /t 1 /nobreak > nul")
            .no_window()
            .spawn();

        assert!(process.is_ok());
        let process = process.unwrap();

        // Immediately try_wait - process should still be running
        let result = process.try_wait();
        assert!(result.is_ok());
        // May or may not have finished yet, depending on timing
        // Just verify the call doesn't fail
    }

    #[test]
    fn test_wait_timeout() {
        // Spawn a process that takes a while
        let process = Command::new("cmd.exe")
            .arg("/c")
            .arg("timeout /t 10 /nobreak > nul")
            .no_window()
            .spawn();

        assert!(process.is_ok());
        let process = process.unwrap();

        // Wait with a short timeout - should timeout
        let result = process.wait_timeout(Some(Duration::from_millis(100)));
        assert!(result.is_err()); // Should timeout

        // Terminate the process so we don't leave it running
        let _ = process.terminate(1);
    }

    #[test]
    fn test_is_running() {
        let process = Command::new("cmd.exe")
            .arg("/c")
            .arg("exit 0")
            .no_window()
            .spawn();

        assert!(process.is_ok());
        let process = process.unwrap();

        // Wait for it to finish
        let _ = process.wait();

        // Should no longer be running
        let is_running = process.is_running();
        assert!(is_running.is_ok());
        assert!(!is_running.unwrap());
    }

    #[test]
    fn test_terminate_process() {
        // Spawn a long-running process
        let process = Command::new("cmd.exe")
            .arg("/c")
            .arg("timeout /t 60 /nobreak > nul")
            .no_window()
            .spawn();

        assert!(process.is_ok());
        let process = process.unwrap();

        // Terminate it
        let result = process.terminate(99);
        assert!(result.is_ok());

        // Wait for it to actually terminate
        let exit_code = process.wait();
        assert!(exit_code.is_ok());
        // Exit code should be 99 or 1 depending on timing
    }

    #[test]
    fn test_open_existing_process() {
        // Open the current process
        let pid = current_pid();
        let result = Process::open(pid, ProcessAccess::QUERY);
        assert!(result.is_ok());

        let process = result.unwrap();
        assert_eq!(process.pid(), pid);
    }

    #[test]
    fn test_open_nonexistent_process() {
        // Try to open a process with an invalid PID
        // PID 4 is usually System, but PID 99999999 shouldn't exist
        let result = Process::open(99999999, ProcessAccess::QUERY);
        assert!(result.is_err());
    }

    #[test]
    fn test_quote_arg_edge_cases() {
        // Empty string
        assert_eq!(quote_arg(""), "\"\"");

        // With tab
        assert_eq!(quote_arg("a\tb"), "\"a\tb\"");

        // With quote
        assert_eq!(quote_arg("a\"b"), "\"a\\\"b\"");

        // Backslash before quote
        assert_eq!(quote_arg("a\\\"b"), "\"a\\\\\\\"b\"");

        // Trailing backslash (no spaces, so no quoting needed)
        assert_eq!(quote_arg("path\\"), "path\\");

        // Trailing backslash with space (needs quoting)
        assert_eq!(quote_arg("path with space\\"), "\"path with space\\\\\"");

        // No quoting needed
        assert_eq!(quote_arg("simple-path.txt"), "simple-path.txt");
    }

    #[test]
    fn test_command_line_building() {
        let cmd = Command::new("program.exe")
            .arg("arg1")
            .arg("arg with space")
            .arg("arg\"quote");

        let cmd_line = cmd.build_command_line();
        assert!(cmd_line.contains("program.exe"));
        assert!(cmd_line.contains("arg1"));
        assert!(cmd_line.contains("\"arg with space\""));
        assert!(cmd_line.contains("\\\""));
    }

    #[test]
    fn test_spawn_unicode_args() {
        // Test with Unicode arguments
        let exit_code = Command::new("cmd.exe")
            .arg("/c")
            .arg("echo")
            .arg("日本語")
            .no_window()
            .run();

        assert!(exit_code.is_ok());
        assert_eq!(exit_code.unwrap(), 0);
    }
}
