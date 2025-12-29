//! File system utilities for Windows.
//!
//! Provides ergonomic wrappers for Windows-specific file system operations.

use crate::error::Result;
use crate::handle::OwnedHandle;
use crate::string::{from_wide, WideString};
use std::path::{Path, PathBuf};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, DeleteFileW, GetFileAttributesW, MoveFileExW, SetFileAttributesW, CREATE_ALWAYS,
    CREATE_NEW, FILE_ACCESS_RIGHTS, FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_SYSTEM,
    FILE_ATTRIBUTE_TEMPORARY, FILE_CREATION_DISPOSITION, FILE_FLAGS_AND_ATTRIBUTES,
    FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    INVALID_FILE_ATTRIBUTES, MOVEFILE_COPY_ALLOWED, MOVEFILE_REPLACE_EXISTING,
    MOVEFILE_WRITE_THROUGH, MOVE_FILE_FLAGS, OPEN_ALWAYS, OPEN_EXISTING,
};

/// File attributes for Windows files.
#[derive(Clone, Copy, Debug, Default)]
pub struct FileAttributes(pub FILE_FLAGS_AND_ATTRIBUTES);

impl FileAttributes {
    /// Normal file with no special attributes.
    pub const NORMAL: Self = Self(FILE_ATTRIBUTE_NORMAL);

    /// The file is read-only.
    pub const READONLY: Self = Self(FILE_ATTRIBUTE_READONLY);

    /// The file is hidden.
    pub const HIDDEN: Self = Self(FILE_ATTRIBUTE_HIDDEN);

    /// The file is a system file.
    pub const SYSTEM: Self = Self(FILE_ATTRIBUTE_SYSTEM);

    /// The file is a directory.
    pub const DIRECTORY: Self = Self(FILE_ATTRIBUTE_DIRECTORY);

    /// The file is marked for archiving.
    pub const ARCHIVE: Self = Self(FILE_ATTRIBUTE_ARCHIVE);

    /// The file is temporary.
    pub const TEMPORARY: Self = Self(FILE_ATTRIBUTE_TEMPORARY);

    /// Checks if this represents a directory.
    pub fn is_directory(&self) -> bool {
        (self.0 .0 & FILE_ATTRIBUTE_DIRECTORY.0) != 0
    }

    /// Checks if this file is read-only.
    pub fn is_readonly(&self) -> bool {
        (self.0 .0 & FILE_ATTRIBUTE_READONLY.0) != 0
    }

    /// Checks if this file is hidden.
    pub fn is_hidden(&self) -> bool {
        (self.0 .0 & FILE_ATTRIBUTE_HIDDEN.0) != 0
    }

    /// Checks if this file is a system file.
    pub fn is_system(&self) -> bool {
        (self.0 .0 & FILE_ATTRIBUTE_SYSTEM.0) != 0
    }

    /// Combines two sets of attributes.
    pub fn with(self, other: Self) -> Self {
        Self(FILE_FLAGS_AND_ATTRIBUTES(self.0 .0 | other.0 .0))
    }
}

/// Gets the attributes of a file or directory.
///
/// # Errors
///
/// Returns an error if the path does not exist or is not accessible.
pub fn get_attributes(path: impl AsRef<Path>) -> Result<FileAttributes> {
    let wide = WideString::from_path(path.as_ref());
    // SAFETY: wide.as_pcwstr() returns a valid null-terminated wide string.
    // GetFileAttributesW is safe to call with any valid string.
    let attrs = unsafe { GetFileAttributesW(wide.as_pcwstr()) };

    if attrs == INVALID_FILE_ATTRIBUTES {
        return Err(crate::error::last_error());
    }

    Ok(FileAttributes(FILE_FLAGS_AND_ATTRIBUTES(attrs)))
}

/// Sets the attributes of a file or directory.
///
/// # Errors
///
/// Returns an error if the path does not exist or access is denied.
pub fn set_attributes(path: impl AsRef<Path>, attributes: FileAttributes) -> Result<()> {
    let wide = WideString::from_path(path.as_ref());
    // SAFETY: wide.as_pcwstr() returns a valid null-terminated wide string.
    unsafe {
        SetFileAttributesW(wide.as_pcwstr(), attributes.0)?;
    }
    Ok(())
}

/// Checks if a path exists.
pub fn exists(path: impl AsRef<Path>) -> bool {
    get_attributes(path).is_ok()
}

/// Checks if a path is a directory.
pub fn is_dir(path: impl AsRef<Path>) -> bool {
    get_attributes(path)
        .map(|a| a.is_directory())
        .unwrap_or(false)
}

/// Checks if a path is a file (not a directory).
pub fn is_file(path: impl AsRef<Path>) -> bool {
    get_attributes(path)
        .map(|a| !a.is_directory())
        .unwrap_or(false)
}

/// Deletes a file.
///
/// # Errors
///
/// Returns an error if the file does not exist or access is denied.
pub fn delete_file(path: impl AsRef<Path>) -> Result<()> {
    let wide = WideString::from_path(path.as_ref());
    // SAFETY: wide.as_pcwstr() returns a valid null-terminated wide string.
    unsafe {
        DeleteFileW(wide.as_pcwstr())?;
    }
    Ok(())
}

/// Options for moving files.
#[derive(Clone, Copy, Debug, Default)]
pub struct MoveOptions {
    /// Replace the destination if it exists.
    pub replace_existing: bool,
    /// Copy the file if it cannot be moved (e.g., across volumes).
    pub copy_allowed: bool,
    /// Don't return until the file is flushed to disk.
    pub write_through: bool,
}

impl MoveOptions {
    /// Creates new move options with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allows replacing an existing file at the destination.
    pub fn replace(mut self) -> Self {
        self.replace_existing = true;
        self
    }

    /// Allows copying if the file cannot be moved.
    pub fn allow_copy(mut self) -> Self {
        self.copy_allowed = true;
        self
    }

    /// Waits for the operation to be flushed to disk.
    pub fn write_through(mut self) -> Self {
        self.write_through = true;
        self
    }

    fn to_flags(self) -> MOVE_FILE_FLAGS {
        let mut flags = MOVE_FILE_FLAGS(0);
        if self.replace_existing {
            flags.0 |= MOVEFILE_REPLACE_EXISTING.0;
        }
        if self.copy_allowed {
            flags.0 |= MOVEFILE_COPY_ALLOWED.0;
        }
        if self.write_through {
            flags.0 |= MOVEFILE_WRITE_THROUGH.0;
        }
        flags
    }
}

/// Moves or renames a file.
pub fn move_file(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    move_file_with_options(from, to, MoveOptions::default())
}

/// Moves or renames a file with the specified options.
///
/// # Errors
///
/// Returns an error if the source doesn't exist, destination is not writable, etc.
pub fn move_file_with_options(
    from: impl AsRef<Path>,
    to: impl AsRef<Path>,
    options: MoveOptions,
) -> Result<()> {
    let from_wide = WideString::from_path(from.as_ref());
    let to_wide = WideString::from_path(to.as_ref());

    // SAFETY: Both paths are valid null-terminated wide strings.
    unsafe {
        MoveFileExW(
            from_wide.as_pcwstr(),
            to_wide.as_pcwstr(),
            options.to_flags(),
        )?;
    }
    Ok(())
}

/// Options for opening files.
pub struct OpenOptions {
    read: bool,
    write: bool,
    create: bool,
    create_new: bool,
    truncate: bool,
    share_read: bool,
    share_write: bool,
    attributes: FileAttributes,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenOptions {
    /// Creates a new set of options with default settings.
    pub fn new() -> Self {
        Self {
            read: false,
            write: false,
            create: false,
            create_new: false,
            truncate: false,
            share_read: true,
            share_write: false,
            attributes: FileAttributes::NORMAL,
        }
    }

    /// Opens the file for reading.
    pub fn read(mut self, read: bool) -> Self {
        self.read = read;
        self
    }

    /// Opens the file for writing.
    pub fn write(mut self, write: bool) -> Self {
        self.write = write;
        self
    }

    /// Creates the file if it doesn't exist.
    pub fn create(mut self, create: bool) -> Self {
        self.create = create;
        self
    }

    /// Creates a new file, failing if it already exists.
    pub fn create_new(mut self, create_new: bool) -> Self {
        self.create_new = create_new;
        self
    }

    /// Truncates the file to zero length.
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    /// Allows other processes to read the file.
    pub fn share_read(mut self, share: bool) -> Self {
        self.share_read = share;
        self
    }

    /// Allows other processes to write to the file.
    pub fn share_write(mut self, share: bool) -> Self {
        self.share_write = share;
        self
    }

    /// Sets the file attributes.
    pub fn attributes(mut self, attrs: FileAttributes) -> Self {
        self.attributes = attrs;
        self
    }

    /// Opens the file with these options.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened with the requested options.
    pub fn open(self, path: impl AsRef<Path>) -> Result<OwnedHandle> {
        let wide = WideString::from_path(path.as_ref());

        let access = self.get_access();
        let share_mode = self.get_share_mode();
        let creation = self.get_creation_disposition();

        // SAFETY: All parameters are valid:
        // - wide.as_pcwstr() is a valid null-terminated wide string
        // - access, share_mode, creation, attributes are all valid flag values
        // - None for security attributes and template file is valid
        let handle = unsafe {
            CreateFileW(
                wide.as_pcwstr(),
                access.0,
                share_mode,
                None,
                creation,
                self.attributes.0,
                None,
            )?
        };

        OwnedHandle::new(handle)
    }

    fn get_access(&self) -> FILE_ACCESS_RIGHTS {
        let mut access = FILE_ACCESS_RIGHTS(0);
        if self.read {
            access.0 |= FILE_GENERIC_READ.0;
        }
        if self.write {
            access.0 |= FILE_GENERIC_WRITE.0;
        }
        access
    }

    fn get_share_mode(&self) -> FILE_SHARE_MODE {
        let mut mode = FILE_SHARE_MODE(0);
        if self.share_read {
            mode.0 |= FILE_SHARE_READ.0;
        }
        if self.share_write {
            mode.0 |= FILE_SHARE_WRITE.0;
        }
        mode
    }

    fn get_creation_disposition(&self) -> FILE_CREATION_DISPOSITION {
        if self.create_new {
            CREATE_NEW
        } else if self.truncate && self.create {
            CREATE_ALWAYS
        } else if self.create {
            OPEN_ALWAYS
        } else {
            OPEN_EXISTING
        }
    }
}

/// Gets the Windows system directory path (e.g., `C:\Windows\System32`).
pub fn get_system_directory() -> Result<PathBuf> {
    use windows::Win32::System::SystemInformation::GetSystemDirectoryW;

    let mut buffer = vec![0u16; 260]; // MAX_PATH
                                      // SAFETY: buffer is a valid mutable slice with sufficient capacity.
                                      // GetSystemDirectoryW writes at most buffer.len() characters.
    let len = unsafe { GetSystemDirectoryW(Some(&mut buffer)) } as usize;

    if len == 0 {
        return Err(crate::error::last_error());
    }

    buffer.truncate(len);
    let path_str = from_wide(&buffer)?;
    Ok(PathBuf::from(path_str))
}

/// Gets the Windows directory path (e.g., `C:\Windows`).
pub fn get_windows_directory() -> Result<PathBuf> {
    use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;

    let mut buffer = vec![0u16; 260]; // MAX_PATH
                                      // SAFETY: buffer is a valid mutable slice with sufficient capacity.
    let len = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) } as usize;

    if len == 0 {
        return Err(crate::error::last_error());
    }

    buffer.truncate(len);
    let path_str = from_wide(&buffer)?;
    Ok(PathBuf::from(path_str))
}

/// Gets the temporary directory path.
pub fn get_temp_directory() -> Result<PathBuf> {
    use windows::Win32::Storage::FileSystem::GetTempPathW;

    let mut buffer = vec![0u16; 260]; // MAX_PATH
                                      // SAFETY: buffer is a valid mutable slice with sufficient capacity.
    let len = unsafe { GetTempPathW(Some(&mut buffer)) } as usize;

    if len == 0 {
        return Err(crate::error::last_error());
    }

    buffer.truncate(len);
    let path_str = from_wide(&buffer)?;
    Ok(PathBuf::from(path_str))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_file_attributes() {
        let attrs = FileAttributes::READONLY.with(FileAttributes::HIDDEN);
        assert!(attrs.is_readonly());
        assert!(attrs.is_hidden());
        assert!(!attrs.is_directory());
    }

    // ============================================================================
    // MAX_PATH Length Tests
    // ============================================================================

    /// MAX_PATH on Windows is 260 characters including null terminator
    const MAX_PATH_LEN: usize = 260;

    #[test]
    fn test_path_near_max_path_limit() {
        // Create a path that's just under MAX_PATH (259 chars + null = 260)
        let temp = env::temp_dir();
        let temp_str = temp.to_string_lossy();

        // Calculate how many characters we can use for the filename
        // Path format: C:\temp\<filename>
        let remaining = MAX_PATH_LEN - 1 - temp_str.len() - 1; // -1 for null, -1 for separator

        if remaining > 10 {
            let filename = "a".repeat(remaining.min(200));
            let path = temp.join(&filename);

            // Test that we can create a WideString for paths near MAX_PATH
            let wide = WideString::from_path(&path);
            assert!(!wide.is_empty());
        }
    }

    #[test]
    fn test_path_at_max_path_boundary() {
        // Test path exactly at 259 characters (MAX_PATH - 1 for null)
        let base = "C:\\";
        let remaining = 259 - base.len();
        let long_component = "a".repeat(remaining);
        let path_str = format!("{}{}", base, long_component);
        let path = Path::new(&path_str);

        let wide = WideString::from_path(path);
        assert_eq!(wide.len(), 259);
    }

    #[test]
    fn test_path_over_max_path() {
        // Test path that exceeds MAX_PATH
        // Windows can handle long paths with \\?\ prefix
        let base = "C:\\";
        let long_component = "a".repeat(300);
        let path_str = format!("{}{}", base, long_component);
        let path = Path::new(&path_str);

        // Should not panic - WideString should handle it
        let wide = WideString::from_path(path);
        assert!(wide.len() > MAX_PATH_LEN);
    }

    #[test]
    fn test_extended_length_path_prefix() {
        // Extended-length paths use \\?\ prefix and can exceed MAX_PATH
        let long_name = "a".repeat(300);
        let path_str = format!("\\\\?\\C:\\{}", long_name);
        let path = Path::new(&path_str);

        let wide = WideString::from_path(path);
        // The prefix "\\?\" is 4 chars, "C:\" is 3 chars = 7 chars + 300 = 307
        assert!(wide.len() > 300);
    }

    #[test]
    fn test_unc_path() {
        // UNC paths: \\server\share\path
        let path = Path::new("\\\\server\\share\\folder\\file.txt");
        let wide = WideString::from_path(path);
        let back = wide.to_string_lossy();
        assert!(back.starts_with("\\\\server\\share"));
    }

    #[test]
    fn test_deep_nested_path() {
        // Create a deeply nested path
        let mut path_buf = PathBuf::from("C:\\");
        for _ in 0..50 {
            path_buf.push("subdir");
        }
        path_buf.push("file.txt");

        let wide = WideString::from_path(&path_buf);
        assert!(!wide.is_empty());

        let back = wide.to_string_lossy();
        assert!(back.contains("subdir"));
    }

    #[test]
    fn test_path_with_special_windows_names() {
        // Windows has reserved names like CON, PRN, AUX, NUL, etc.
        // These paths are technically valid to encode, even if they behave specially
        let reserved_names = ["CON", "PRN", "AUX", "NUL", "COM1", "LPT1"];

        for name in reserved_names {
            let path = Path::new(name);
            let wide = WideString::from_path(path);
            let back = wide.to_string_lossy();
            assert_eq!(back, name, "Failed for reserved name: {}", name);
        }
    }

    #[test]
    fn test_path_with_unicode_characters() {
        // Paths with various Unicode characters
        let paths = [
            "C:\\日本語\\ファイル.txt",
            "C:\\Données\\fichier.txt",
            "C:\\Документы\\файл.txt",
            "C:\\数据\\文件.txt",
        ];

        for path_str in paths {
            let path = Path::new(path_str);
            let wide = WideString::from_path(path);
            let back = wide.to_string_lossy();
            assert_eq!(back, path_str, "Failed for path: {}", path_str);
        }
    }

    #[test]
    fn test_system_directory_path_length() {
        // Verify system directories return valid paths
        if let Ok(sys_dir) = get_system_directory() {
            assert!(sys_dir.to_string_lossy().len() < MAX_PATH_LEN);
        }
    }

    #[test]
    fn test_temp_directory_path_length() {
        if let Ok(temp_dir) = get_temp_directory() {
            assert!(temp_dir.to_string_lossy().len() < MAX_PATH_LEN);
        }
    }

    #[test]
    fn test_windows_directory_path_length() {
        if let Ok(win_dir) = get_windows_directory() {
            assert!(win_dir.to_string_lossy().len() < MAX_PATH_LEN);
        }
    }
}
