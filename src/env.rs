//! Environment variable utilities.
//!
//! Provides safe wrappers for Windows environment variable operations.

use crate::error::{Error, Result};
use crate::string::{from_wide, to_wide, WideString};
use std::collections::HashMap;
use std::path::PathBuf;
use windows::Win32::System::Environment::{
    ExpandEnvironmentStringsW, GetEnvironmentVariableW, SetEnvironmentVariableW,
};

/// Gets an environment variable.
///
/// Returns `None` if the variable doesn't exist.
pub fn get(name: &str) -> Option<String> {
    let name_wide = WideString::new(name);
    
    // First call to get the required size
    // SAFETY: GetEnvironmentVariableW is safe with valid parameters
    let size = unsafe { GetEnvironmentVariableW(name_wide.as_pcwstr(), None) };
    
    if size == 0 {
        return None;
    }

    let mut buffer = vec![0u16; size as usize];
    
    // SAFETY: GetEnvironmentVariableW is safe with valid buffer
    let len = unsafe { GetEnvironmentVariableW(name_wide.as_pcwstr(), Some(&mut buffer)) } as usize;
    
    if len == 0 {
        return None;
    }

    from_wide(&buffer[..len]).ok()
}

/// Sets an environment variable.
pub fn set(name: &str, value: &str) -> Result<()> {
    let name_wide = WideString::new(name);
    let value_wide = WideString::new(value);
    
    // SAFETY: SetEnvironmentVariableW is safe with valid strings
    unsafe {
        SetEnvironmentVariableW(name_wide.as_pcwstr(), value_wide.as_pcwstr())?;
    }
    
    Ok(())
}

/// Removes an environment variable.
pub fn remove(name: &str) -> Result<()> {
    let name_wide = WideString::new(name);
    
    // SAFETY: SetEnvironmentVariableW with NULL value removes the variable
    unsafe {
        SetEnvironmentVariableW(name_wide.as_pcwstr(), None)?;
    }
    
    Ok(())
}

/// Expands environment variable references in a string.
///
/// Replaces `%VARNAME%` with the value of the environment variable.
pub fn expand(s: &str) -> Result<String> {
    let wide = to_wide(s);
    
    // First call to get the required size
    // SAFETY: ExpandEnvironmentStringsW is safe with valid parameters
    let size = unsafe { ExpandEnvironmentStringsW(windows::core::PCWSTR(wide.as_ptr()), None) };
    
    if size == 0 {
        return Err(crate::error::last_error());
    }

    let mut buffer = vec![0u16; size as usize];
    
    // SAFETY: ExpandEnvironmentStringsW is safe with valid buffer
    let len = unsafe { 
        ExpandEnvironmentStringsW(
            windows::core::PCWSTR(wide.as_ptr()), 
            Some(&mut buffer)
        ) 
    } as usize;
    
    if len == 0 {
        return Err(crate::error::last_error());
    }

    // len includes null terminator
    from_wide(&buffer[..len.saturating_sub(1)])
}

/// Gets all environment variables as a HashMap.
pub fn vars() -> HashMap<String, String> {
    std::env::vars().collect()
}

/// Gets the PATH environment variable as a list of paths.
pub fn path() -> Vec<PathBuf> {
    get("PATH")
        .unwrap_or_default()
        .split(';')
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// Gets the TEMP directory path.
pub fn temp_dir() -> Option<PathBuf> {
    get("TEMP").or_else(|| get("TMP")).map(PathBuf::from)
}

/// Gets the user's home directory (USERPROFILE).
pub fn home_dir() -> Option<PathBuf> {
    get("USERPROFILE").map(PathBuf::from)
}

/// Gets the current user's username.
pub fn username() -> Option<String> {
    get("USERNAME")
}

/// Gets the computer name.
pub fn computer_name() -> Option<String> {
    get("COMPUTERNAME")
}

/// Gets the system root directory (typically C:\Windows).
pub fn system_root() -> Option<PathBuf> {
    get("SystemRoot").map(PathBuf::from)
}

/// Gets the Windows directory.
pub fn windows_dir() -> Option<PathBuf> {
    get("windir").map(PathBuf::from)
}

/// Gets the program data directory.
pub fn program_data() -> Option<PathBuf> {
    get("ProgramData").map(PathBuf::from)
}

/// Gets the Program Files directory.
pub fn program_files() -> Option<PathBuf> {
    get("ProgramFiles").map(PathBuf::from)
}

/// Gets the Program Files (x86) directory.
pub fn program_files_x86() -> Option<PathBuf> {
    get("ProgramFiles(x86)").map(PathBuf::from)
}

/// Gets the user's application data directory.
pub fn app_data() -> Option<PathBuf> {
    get("APPDATA").map(PathBuf::from)
}

/// Gets the user's local application data directory.
pub fn local_app_data() -> Option<PathBuf> {
    get("LOCALAPPDATA").map(PathBuf::from)
}

/// Gets the number of processors.
pub fn processor_count() -> Option<u32> {
    get("NUMBER_OF_PROCESSORS").and_then(|s| s.parse().ok())
}

/// Gets the processor architecture.
pub fn processor_architecture() -> Option<String> {
    get("PROCESSOR_ARCHITECTURE")
}

/// Checks if an environment variable exists.
pub fn exists(name: &str) -> bool {
    get(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_set_remove() {
        let var_name = "ERGONOMIC_WINDOWS_TEST_VAR";
        
        // Initially should not exist
        assert!(get(var_name).is_none());
        
        // Set it
        set(var_name, "test_value").unwrap();
        assert_eq!(get(var_name), Some("test_value".to_string()));
        
        // Remove it
        remove(var_name).unwrap();
        assert!(get(var_name).is_none());
    }

    #[test]
    fn test_expand() {
        let expanded = expand("%SystemRoot%\\System32").unwrap();
        assert!(expanded.contains("System32"));
        assert!(!expanded.contains("%"));
    }

    #[test]
    fn test_path() {
        let paths = path();
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_standard_vars() {
        assert!(home_dir().is_some());
        assert!(username().is_some());
        assert!(computer_name().is_some());
        assert!(system_root().is_some());
        assert!(temp_dir().is_some());
    }

    #[test]
    fn test_exists() {
        assert!(exists("PATH"));
        assert!(!exists("NONEXISTENT_VAR_12345"));
    }
}

