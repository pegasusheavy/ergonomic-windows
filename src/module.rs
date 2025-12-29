//! Dynamic library (DLL) loading utilities.
//!
//! Provides safe wrappers for loading and using Windows DLLs.

use crate::error::{Error, Result};
use crate::string::WideString;
use std::path::Path;
use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleW, GetProcAddress, LoadLibraryExW, LoadLibraryW,
    LOAD_LIBRARY_AS_DATAFILE, LOAD_LIBRARY_AS_IMAGE_RESOURCE, LOAD_LIBRARY_FLAGS,
    LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32,
};

/// A loaded dynamic library (DLL).
pub struct Library {
    handle: HMODULE,
    owned: bool,
}

impl Library {
    /// Loads a library from the specified path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path_wide = WideString::from_path(path.as_ref());

        // SAFETY: LoadLibraryW is safe with a valid path
        let handle = unsafe { LoadLibraryW(path_wide.as_pcwstr())? };

        Ok(Self {
            handle,
            owned: true,
        })
    }

    /// Loads a library with specific flags.
    pub fn load_with_flags(path: impl AsRef<Path>, flags: LoadFlags) -> Result<Self> {
        let path_wide = WideString::from_path(path.as_ref());

        // SAFETY: LoadLibraryExW is safe with valid parameters
        let handle = unsafe { LoadLibraryExW(path_wide.as_pcwstr(), None, flags.to_native())? };

        Ok(Self {
            handle,
            owned: true,
        })
    }

    /// Gets a handle to an already-loaded library.
    pub fn get(name: &str) -> Result<Self> {
        let name_wide = WideString::new(name);

        // SAFETY: GetModuleHandleW is safe with a valid name
        let handle = unsafe { GetModuleHandleW(name_wide.as_pcwstr())? };

        Ok(Self {
            handle,
            owned: false, // Don't free - we didn't load it
        })
    }

    /// Gets a handle to the current executable.
    pub fn current() -> Result<Self> {
        // SAFETY: GetModuleHandleW with NULL returns the current module
        let handle = unsafe { GetModuleHandleW(None)? };

        Ok(Self {
            handle,
            owned: false,
        })
    }

    /// Gets a function pointer from the library.
    ///
    /// # Safety
    ///
    /// The caller must ensure the function signature matches the actual function.
    pub unsafe fn get_proc<F>(&self, name: &str) -> Result<F>
    where
        F: Copy,
    {
        let name_cstr =
            std::ffi::CString::new(name).map_err(|_| Error::custom("Invalid function name"))?;

        let proc = GetProcAddress(
            self.handle,
            windows::core::PCSTR(name_cstr.as_ptr() as *const u8),
        );

        match proc {
            Some(p) => Ok(std::mem::transmute_copy(&p)),
            None => Err(Error::custom(format!("Function '{}' not found", name))),
        }
    }

    /// Gets the path to the loaded library.
    pub fn path(&self) -> Result<std::path::PathBuf> {
        let mut buffer = vec![0u16; 32768]; // MAX_PATH is not enough for extended paths

        // SAFETY: GetModuleFileNameW is safe with valid parameters
        let len = unsafe { GetModuleFileNameW(self.handle, &mut buffer) } as usize;

        if len == 0 {
            return Err(crate::error::last_error());
        }

        let path_str = crate::string::from_wide(&buffer[..len])?;
        Ok(std::path::PathBuf::from(path_str))
    }

    /// Returns the raw module handle.
    pub fn as_raw(&self) -> HMODULE {
        self.handle
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: We own this library handle
            unsafe {
                let _ = FreeLibrary(self.handle);
            }
        }
    }
}

/// Flags for loading libraries.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoadFlags(u32);

impl LoadFlags {
    /// No special flags.
    pub const NONE: Self = Self(0);

    /// Load as a data file (no execution).
    pub const AS_DATAFILE: Self = Self(LOAD_LIBRARY_AS_DATAFILE.0);

    /// Load as an image resource.
    pub const AS_IMAGE_RESOURCE: Self = Self(LOAD_LIBRARY_AS_IMAGE_RESOURCE.0);

    /// Search the DLL's directory for dependencies.
    pub const SEARCH_DLL_LOAD_DIR: Self = Self(LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR.0);

    /// Search only System32 for dependencies.
    pub const SEARCH_SYSTEM32: Self = Self(LOAD_LIBRARY_SEARCH_SYSTEM32.0);

    /// Creates new flags.
    pub fn new() -> Self {
        Self(0)
    }

    /// Adds a flag.
    pub fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn to_native(self) -> LOAD_LIBRARY_FLAGS {
        LOAD_LIBRARY_FLAGS(self.0)
    }
}

/// Gets the path to the current executable.
pub fn current_exe() -> Result<std::path::PathBuf> {
    Library::current()?.path()
}

/// Gets the directory containing the current executable.
pub fn current_exe_dir() -> Result<std::path::PathBuf> {
    let exe = current_exe()?;
    exe.parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| Error::custom("Cannot determine executable directory"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_module() {
        let module = Library::current().unwrap();
        let path = module.path().unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_load_system_dll() {
        // kernel32.dll is always loaded
        let kernel32 = Library::get("kernel32.dll").unwrap();
        let path = kernel32.path().unwrap();
        assert!(path.to_string_lossy().to_lowercase().contains("kernel32"));
    }

    #[test]
    fn test_get_proc() {
        let kernel32 = Library::get("kernel32.dll").unwrap();

        // GetCurrentProcessId is always available
        type GetCurrentProcessIdFn = unsafe extern "system" fn() -> u32;
        let get_pid: GetCurrentProcessIdFn =
            unsafe { kernel32.get_proc("GetCurrentProcessId").unwrap() };

        let pid = unsafe { get_pid() };
        assert!(pid > 0);
        assert_eq!(pid, std::process::id());
    }

    #[test]
    fn test_current_exe() {
        let exe = current_exe().unwrap();
        assert!(exe.exists());
    }

    #[test]
    fn test_current_exe_dir() {
        let dir = current_exe_dir().unwrap();
        assert!(dir.is_dir());
    }
}
