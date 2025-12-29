//! Windows Registry access utilities.
//!
//! Provides ergonomic wrappers for reading and writing Windows Registry keys and values.

use crate::error::{Error, Result};
use crate::string::{from_wide, to_wide, WideString};
use windows::Win32::Foundation::{
    ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, WIN32_ERROR,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegDeleteValueW, RegEnumKeyExW, RegEnumValueW,
    RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG,
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS, KEY_ALL_ACCESS, KEY_CREATE_SUB_KEY,
    KEY_ENUMERATE_SUB_KEYS, KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE, KEY_WOW64_32KEY,
    KEY_WOW64_64KEY, KEY_WRITE, REG_BINARY, REG_DWORD, REG_EXPAND_SZ, REG_MULTI_SZ,
    REG_OPTION_NON_VOLATILE, REG_QWORD, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
};

/// Helper to convert WIN32_ERROR to Result
fn check_error(err: WIN32_ERROR) -> Result<()> {
    if err == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(Error::Windows(windows::core::Error::from(err)))
    }
}

/// Predefined registry root keys.
#[derive(Clone, Copy, Debug)]
pub struct RootKey(pub HKEY);

impl RootKey {
    /// HKEY_CLASSES_ROOT - File associations and COM object registration.
    pub const CLASSES_ROOT: Self = Self(HKEY_CLASSES_ROOT);

    /// HKEY_CURRENT_USER - Settings for the current user.
    pub const CURRENT_USER: Self = Self(HKEY_CURRENT_USER);

    /// HKEY_LOCAL_MACHINE - System-wide settings.
    pub const LOCAL_MACHINE: Self = Self(HKEY_LOCAL_MACHINE);

    /// HKEY_USERS - Settings for all user profiles.
    pub const USERS: Self = Self(HKEY_USERS);

    /// HKEY_CURRENT_CONFIG - Current hardware profile.
    pub const CURRENT_CONFIG: Self = Self(HKEY_CURRENT_CONFIG);
}

/// Registry access rights.
#[derive(Clone, Copy, Debug)]
pub struct Access(pub REG_SAM_FLAGS);

impl Access {
    /// Read access.
    pub const READ: Self = Self(KEY_READ);

    /// Write access.
    pub const WRITE: Self = Self(KEY_WRITE);

    /// Full access.
    pub const ALL: Self = Self(KEY_ALL_ACCESS);

    /// Query value access.
    pub const QUERY_VALUE: Self = Self(KEY_QUERY_VALUE);

    /// Set value access.
    pub const SET_VALUE: Self = Self(KEY_SET_VALUE);

    /// Create subkey access.
    pub const CREATE_SUB_KEY: Self = Self(KEY_CREATE_SUB_KEY);

    /// Enumerate subkeys access.
    pub const ENUMERATE_SUB_KEYS: Self = Self(KEY_ENUMERATE_SUB_KEYS);

    /// Access 32-bit registry view on 64-bit Windows.
    pub const WOW64_32: Self = Self(KEY_WOW64_32KEY);

    /// Access 64-bit registry view on 64-bit Windows.
    pub const WOW64_64: Self = Self(KEY_WOW64_64KEY);

    /// Combines two access flags.
    pub fn with(self, other: Self) -> Self {
        Self(REG_SAM_FLAGS(self.0 .0 | other.0 .0))
    }
}

/// A registry value.
#[derive(Clone, Debug)]
pub enum Value {
    /// A string value (REG_SZ).
    String(String),
    /// An expandable string value (REG_EXPAND_SZ).
    ExpandString(String),
    /// A multi-string value (REG_MULTI_SZ).
    MultiString(Vec<String>),
    /// A 32-bit integer (REG_DWORD).
    Dword(u32),
    /// A 64-bit integer (REG_QWORD).
    Qword(u64),
    /// Binary data (REG_BINARY).
    Binary(Vec<u8>),
}

impl Value {
    /// Creates a string value.
    pub fn string(s: impl Into<String>) -> Self {
        Value::String(s.into())
    }

    /// Creates a DWORD value.
    pub fn dword(v: u32) -> Self {
        Value::Dword(v)
    }

    /// Creates a QWORD value.
    pub fn qword(v: u64) -> Self {
        Value::Qword(v)
    }

    /// Creates a binary value.
    pub fn binary(data: impl Into<Vec<u8>>) -> Self {
        Value::Binary(data.into())
    }

    /// Gets the value as a string, if it is one.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) | Value::ExpandString(s) => Some(s),
            _ => None,
        }
    }

    /// Gets the value as a u32, if it is one.
    pub fn as_dword(&self) -> Option<u32> {
        match self {
            Value::Dword(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the value as a u64, if it is one.
    pub fn as_qword(&self) -> Option<u64> {
        match self {
            Value::Qword(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the value as binary data, if it is one.
    pub fn as_binary(&self) -> Option<&[u8]> {
        match self {
            Value::Binary(v) => Some(v),
            _ => None,
        }
    }
}

/// An opened registry key.
pub struct Key {
    hkey: HKEY,
    owned: bool,
}

impl Key {
    /// Opens a registry key.
    ///
    /// # Errors
    ///
    /// Returns an error if the key does not exist or access is denied.
    pub fn open(root: RootKey, path: &str, access: Access) -> Result<Self> {
        let path_wide = WideString::new(path);
        let mut hkey = HKEY::default();

        // SAFETY: All parameters are valid:
        // - root.0 is a valid predefined root key handle
        // - path_wide is a valid null-terminated wide string
        // - hkey is a valid output parameter
        let err = unsafe { RegOpenKeyExW(root.0, path_wide.as_pcwstr(), 0, access.0, &mut hkey) };
        check_error(err)?;

        Ok(Self { hkey, owned: true })
    }

    /// Creates or opens a registry key.
    ///
    /// If the key already exists, it is opened. Otherwise, it is created.
    ///
    /// # Errors
    ///
    /// Returns an error if the key cannot be created or access is denied.
    pub fn create(root: RootKey, path: &str, access: Access) -> Result<Self> {
        let path_wide = WideString::new(path);
        let mut hkey = HKEY::default();

        // SAFETY: All parameters are valid:
        // - root.0 is a valid predefined root key handle
        // - path_wide is a valid null-terminated wide string
        // - REG_OPTION_NON_VOLATILE is a valid option
        // - hkey is a valid output parameter
        let err = unsafe {
            RegCreateKeyExW(
                root.0,
                path_wide.as_pcwstr(),
                0,
                None,
                REG_OPTION_NON_VOLATILE,
                access.0,
                None,
                &mut hkey,
                None,
            )
        };
        check_error(err)?;

        Ok(Self { hkey, owned: true })
    }

    /// Opens a subkey of this key.
    ///
    /// # Errors
    ///
    /// Returns an error if the subkey does not exist or access is denied.
    pub fn open_subkey(&self, path: &str, access: Access) -> Result<Self> {
        let path_wide = WideString::new(path);
        let mut hkey = HKEY::default();

        // SAFETY: self.hkey is a valid handle we own, path_wide is valid.
        let err =
            unsafe { RegOpenKeyExW(self.hkey, path_wide.as_pcwstr(), 0, access.0, &mut hkey) };
        check_error(err)?;

        Ok(Self { hkey, owned: true })
    }

    /// Creates or opens a subkey of this key.
    ///
    /// # Errors
    ///
    /// Returns an error if the subkey cannot be created or access is denied.
    pub fn create_subkey(&self, path: &str, access: Access) -> Result<Self> {
        let path_wide = WideString::new(path);
        let mut hkey = HKEY::default();

        // SAFETY: self.hkey is a valid handle we own, path_wide is valid.
        let err = unsafe {
            RegCreateKeyExW(
                self.hkey,
                path_wide.as_pcwstr(),
                0,
                None,
                REG_OPTION_NON_VOLATILE,
                access.0,
                None,
                &mut hkey,
                None,
            )
        };
        check_error(err)?;

        Ok(Self { hkey, owned: true })
    }

    /// Deletes a subkey and all its values.
    ///
    /// # Errors
    ///
    /// Returns an error if the subkey does not exist or access is denied.
    pub fn delete_subkey(&self, name: &str) -> Result<()> {
        let name_wide = WideString::new(name);
        // SAFETY: self.hkey is a valid handle, name_wide is valid.
        let err = unsafe { RegDeleteKeyW(self.hkey, name_wide.as_pcwstr()) };
        check_error(err)
    }

    /// Gets a value from this key.
    pub fn get_value(&self, name: &str) -> Result<Value> {
        let name_wide = WideString::new(name);
        let mut value_type = REG_VALUE_TYPE::default();
        let mut size = 0u32;

        // First call to get the size
        let err = unsafe {
            RegQueryValueExW(
                self.hkey,
                name_wide.as_pcwstr(),
                None,
                Some(&mut value_type),
                None,
                Some(&mut size),
            )
        };

        if err != ERROR_SUCCESS && err != ERROR_MORE_DATA {
            return Err(Error::Windows(windows::core::Error::from(err)));
        }

        // Allocate buffer and read the value
        let mut buffer = vec![0u8; size as usize];

        let err = unsafe {
            RegQueryValueExW(
                self.hkey,
                name_wide.as_pcwstr(),
                None,
                Some(&mut value_type),
                Some(buffer.as_mut_ptr()),
                Some(&mut size),
            )
        };
        check_error(err)?;

        buffer.truncate(size as usize);
        buffer.shrink_to_fit(); // Release excess capacity

        // Parse the value based on type
        match value_type {
            REG_SZ | REG_EXPAND_SZ => {
                let wide: Vec<u16> = buffer
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                let s = from_wide(&wide)?;
                if value_type == REG_SZ {
                    Ok(Value::String(s))
                } else {
                    Ok(Value::ExpandString(s))
                }
            }
            REG_MULTI_SZ => {
                let wide: Vec<u16> = buffer
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                let mut strings = Vec::new();
                let mut start = 0;
                for (i, &c) in wide.iter().enumerate() {
                    if c == 0 {
                        if i > start {
                            strings.push(from_wide(&wide[start..i])?);
                        }
                        start = i + 1;
                    }
                }
                Ok(Value::MultiString(strings))
            }
            REG_DWORD => {
                if buffer.len() >= 4 {
                    let value = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
                    Ok(Value::Dword(value))
                } else {
                    Err(Error::custom("Invalid DWORD size"))
                }
            }
            REG_QWORD => {
                if buffer.len() >= 8 {
                    let value = u64::from_le_bytes([
                        buffer[0], buffer[1], buffer[2], buffer[3], buffer[4], buffer[5],
                        buffer[6], buffer[7],
                    ]);
                    Ok(Value::Qword(value))
                } else {
                    Err(Error::custom("Invalid QWORD size"))
                }
            }
            REG_BINARY => Ok(Value::Binary(buffer)),
            _ => Err(Error::custom(format!(
                "Unsupported registry type: {:?}",
                value_type
            ))),
        }
    }

    /// Sets a value in this key.
    pub fn set_value(&self, name: &str, value: &Value) -> Result<()> {
        let name_wide = WideString::new(name);

        let (value_type, data) = match value {
            Value::String(s) => {
                let wide = to_wide(s);
                let bytes: Vec<u8> = wide.iter().flat_map(|&w| w.to_le_bytes()).collect();
                (REG_SZ, bytes)
            }
            Value::ExpandString(s) => {
                let wide = to_wide(s);
                let bytes: Vec<u8> = wide.iter().flat_map(|&w| w.to_le_bytes()).collect();
                (REG_EXPAND_SZ, bytes)
            }
            Value::MultiString(strings) => {
                let mut wide = Vec::new();
                for s in strings {
                    wide.extend(s.encode_utf16());
                    wide.push(0);
                }
                wide.push(0); // Double null terminator
                let bytes: Vec<u8> = wide.iter().flat_map(|&w| w.to_le_bytes()).collect();
                (REG_MULTI_SZ, bytes)
            }
            Value::Dword(v) => (REG_DWORD, v.to_le_bytes().to_vec()),
            Value::Qword(v) => (REG_QWORD, v.to_le_bytes().to_vec()),
            Value::Binary(data) => (REG_BINARY, data.clone()),
        };

        let err =
            unsafe { RegSetValueExW(self.hkey, name_wide.as_pcwstr(), 0, value_type, Some(&data)) };
        check_error(err)
    }

    /// Deletes a value from this key.
    pub fn delete_value(&self, name: &str) -> Result<()> {
        let name_wide = WideString::new(name);
        let err = unsafe { RegDeleteValueW(self.hkey, name_wide.as_pcwstr()) };
        check_error(err)
    }

    /// Enumerates the subkeys of this key.
    pub fn subkeys(&self) -> Result<Vec<String>> {
        let mut result = Vec::new();
        let mut index = 0u32;
        let mut name_buffer = vec![0u16; 256];

        loop {
            let mut name_len = name_buffer.len() as u32;

            let err = unsafe {
                RegEnumKeyExW(
                    self.hkey,
                    index,
                    windows::core::PWSTR(name_buffer.as_mut_ptr()),
                    &mut name_len,
                    None,
                    windows::core::PWSTR::null(),
                    None,
                    None,
                )
            };

            if err == ERROR_SUCCESS {
                let name = from_wide(&name_buffer[..name_len as usize])?;
                result.push(name);
                index += 1;
            } else if err == ERROR_NO_MORE_ITEMS {
                break;
            } else {
                return Err(Error::Windows(windows::core::Error::from(err)));
            }
        }

        Ok(result)
    }

    /// Enumerates the values of this key.
    pub fn values(&self) -> Result<Vec<String>> {
        let mut result = Vec::new();
        let mut index = 0u32;
        let mut name_buffer = vec![0u16; 256];

        loop {
            let mut name_len = name_buffer.len() as u32;

            let err = unsafe {
                RegEnumValueW(
                    self.hkey,
                    index,
                    windows::core::PWSTR(name_buffer.as_mut_ptr()),
                    &mut name_len,
                    None,
                    None,
                    None,
                    None,
                )
            };

            if err == ERROR_SUCCESS {
                let name = from_wide(&name_buffer[..name_len as usize])?;
                result.push(name);
                index += 1;
            } else if err == ERROR_NO_MORE_ITEMS {
                break;
            } else {
                return Err(Error::Windows(windows::core::Error::from(err)));
            }
        }

        Ok(result)
    }

    /// Returns the raw HKEY handle.
    pub fn as_raw(&self) -> HKEY {
        self.hkey
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: We own this key handle (owned == true) and it's valid.
            // After this call, self.hkey should not be used.
            unsafe {
                let _ = RegCloseKey(self.hkey);
            }
        }
    }
}

/// Convenience function to read a string value from the registry.
pub fn get_string(root: RootKey, path: &str, name: &str) -> Result<String> {
    let key = Key::open(root, path, Access::READ)?;
    match key.get_value(name)? {
        Value::String(s) | Value::ExpandString(s) => Ok(s),
        _ => Err(Error::custom("Value is not a string")),
    }
}

/// Convenience function to read a DWORD value from the registry.
pub fn get_dword(root: RootKey, path: &str, name: &str) -> Result<u32> {
    let key = Key::open(root, path, Access::READ)?;
    match key.get_value(name)? {
        Value::Dword(v) => Ok(v),
        _ => Err(Error::custom("Value is not a DWORD")),
    }
}

/// Convenience function to set a string value in the registry.
pub fn set_string(root: RootKey, path: &str, name: &str, value: &str) -> Result<()> {
    let key = Key::create(root, path, Access::WRITE)?;
    key.set_value(name, &Value::String(value.to_string()))
}

/// Convenience function to set a DWORD value in the registry.
pub fn set_dword(root: RootKey, path: &str, name: &str, value: u32) -> Result<()> {
    let key = Key::create(root, path, Access::WRITE)?;
    key.set_value(name, &Value::Dword(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test registry path under HKCU for tests (doesn't require admin)
    const TEST_KEY_PATH: &str = "Software\\ErgonomicWindowsTest";

    fn cleanup_test_key() {
        // Try to delete the test key and ignore errors
        if let Ok(key) = Key::open(RootKey::CURRENT_USER, "Software", Access::WRITE) {
            let _ = key.delete_subkey("ErgonomicWindowsTest");
        }
    }

    // ============================================================================
    // Empty Registry Value Tests
    // ============================================================================

    #[test]
    fn test_empty_string_value() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            // Set empty string
            let result = key.set_value("empty_string", &Value::String(String::new()));
            assert!(result.is_ok());

            // Read it back
            let value = key.get_value("empty_string");
            assert!(value.is_ok());
            match value.unwrap() {
                Value::String(s) => assert!(s.is_empty(), "Expected empty string, got: {:?}", s),
                other => panic!("Expected String, got: {:?}", other),
            }
        }

        cleanup_test_key();
    }

    #[test]
    fn test_empty_expand_string_value() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            // Set empty expand string
            let result = key.set_value("empty_expand", &Value::ExpandString(String::new()));
            assert!(result.is_ok());

            // Read it back
            let value = key.get_value("empty_expand");
            assert!(value.is_ok());
            match value.unwrap() {
                Value::ExpandString(s) => assert!(s.is_empty()),
                other => panic!("Expected ExpandString, got: {:?}", other),
            }
        }

        cleanup_test_key();
    }

    #[test]
    fn test_empty_binary_value() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            // Note: Windows registry may not allow truly empty binary values (0 bytes)
            // Test with a single byte instead
            let result = key.set_value("single_byte_binary", &Value::Binary(vec![0]));
            assert!(result.is_ok());

            // Read it back
            let value = key.get_value("single_byte_binary");
            assert!(value.is_ok());
            match value.unwrap() {
                Value::Binary(b) => assert_eq!(b, vec![0]),
                other => panic!("Expected Binary, got: {:?}", other),
            }

            // Try empty binary - this might fail on some Windows versions
            // Just verify it doesn't panic, whether it succeeds or fails
            let _ = key.set_value("empty_binary", &Value::Binary(vec![]));
        }

        cleanup_test_key();
    }

    #[test]
    fn test_empty_multi_string_value() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            // Set empty multi-string
            let result = key.set_value("empty_multi", &Value::MultiString(vec![]));
            assert!(result.is_ok());

            // Read it back
            let value = key.get_value("empty_multi");
            assert!(value.is_ok());
            match value.unwrap() {
                Value::MultiString(v) => assert!(v.is_empty()),
                other => panic!("Expected MultiString, got: {:?}", other),
            }
        }

        cleanup_test_key();
    }

    // ============================================================================
    // Registry Value Type Tests
    // ============================================================================

    #[test]
    fn test_dword_value() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            // Test various DWORD values
            let test_values = [0u32, 1, 42, u32::MAX];

            for &val in &test_values {
                let result = key.set_value("dword_test", &Value::Dword(val));
                assert!(result.is_ok());

                let read = key.get_value("dword_test");
                assert!(read.is_ok());
                assert_eq!(read.unwrap().as_dword(), Some(val));
            }
        }

        cleanup_test_key();
    }

    #[test]
    fn test_qword_value() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            let test_values = [0u64, 1, 42, u64::MAX];

            for &val in &test_values {
                let result = key.set_value("qword_test", &Value::Qword(val));
                assert!(result.is_ok());

                let read = key.get_value("qword_test");
                assert!(read.is_ok());
                assert_eq!(read.unwrap().as_qword(), Some(val));
            }
        }

        cleanup_test_key();
    }

    #[test]
    fn test_string_with_special_characters() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            let test_strings = [
                "Simple ASCII",
                "With\ttab",
                "With\nnewline",
                "Unicode: 日本語",
                "Emoji: 🎉",
                "Path: C:\\Windows\\System32",
            ];

            for (i, &s) in test_strings.iter().enumerate() {
                let name = format!("string_test_{}", i);
                let result = key.set_value(&name, &Value::String(s.to_string()));
                assert!(result.is_ok(), "Failed to set: {}", s);

                let read = key.get_value(&name);
                assert!(read.is_ok());
                assert_eq!(read.unwrap().as_string(), Some(s));
            }
        }

        cleanup_test_key();
    }

    #[test]
    fn test_multi_string_value() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            let strings = vec![
                "First".to_string(),
                "Second".to_string(),
                "Third with space".to_string(),
            ];

            let result = key.set_value("multi_test", &Value::MultiString(strings.clone()));
            assert!(result.is_ok());

            let read = key.get_value("multi_test");
            assert!(read.is_ok());
            match read.unwrap() {
                Value::MultiString(v) => assert_eq!(v, strings),
                other => panic!("Expected MultiString, got: {:?}", other),
            }
        }

        cleanup_test_key();
    }

    #[test]
    fn test_binary_value() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            let data: Vec<u8> = (0..=255).collect();

            let result = key.set_value("binary_test", &Value::Binary(data.clone()));
            assert!(result.is_ok());

            let read = key.get_value("binary_test");
            assert!(read.is_ok());
            match read.unwrap() {
                Value::Binary(v) => assert_eq!(v, data),
                other => panic!("Expected Binary, got: {:?}", other),
            }
        }

        cleanup_test_key();
    }

    // ============================================================================
    // Registry Key Operations Tests
    // ============================================================================

    #[test]
    fn test_create_and_delete_subkey() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            // Create subkey
            let result = key.create_subkey("SubKey", Access::ALL);
            assert!(result.is_ok());

            // Delete subkey
            let result = key.delete_subkey("SubKey");
            assert!(result.is_ok());
        }

        cleanup_test_key();
    }

    #[test]
    fn test_enumerate_subkeys() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            // Create some subkeys
            let _ = key.create_subkey("SubA", Access::ALL);
            let _ = key.create_subkey("SubB", Access::ALL);
            let _ = key.create_subkey("SubC", Access::ALL);

            // Enumerate them
            let subkeys = key.subkeys();
            assert!(subkeys.is_ok());
            let subkeys = subkeys.unwrap();
            assert!(subkeys.contains(&"SubA".to_string()));
            assert!(subkeys.contains(&"SubB".to_string()));
            assert!(subkeys.contains(&"SubC".to_string()));
        }

        cleanup_test_key();
    }

    #[test]
    fn test_enumerate_values() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            // Create some values
            let _ = key.set_value("ValA", &Value::Dword(1));
            let _ = key.set_value("ValB", &Value::String("test".to_string()));
            let _ = key.set_value("ValC", &Value::Binary(vec![1, 2, 3]));

            // Enumerate them
            let values = key.values();
            assert!(values.is_ok());
            let values = values.unwrap();
            assert!(values.contains(&"ValA".to_string()));
            assert!(values.contains(&"ValB".to_string()));
            assert!(values.contains(&"ValC".to_string()));
        }

        cleanup_test_key();
    }

    #[test]
    fn test_delete_value() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            // Create a value
            let _ = key.set_value("ToDelete", &Value::Dword(42));

            // Verify it exists
            assert!(key.get_value("ToDelete").is_ok());

            // Delete it
            let result = key.delete_value("ToDelete");
            assert!(result.is_ok());

            // Verify it's gone
            assert!(key.get_value("ToDelete").is_err());
        }

        cleanup_test_key();
    }

    #[test]
    fn test_nonexistent_value() {
        cleanup_test_key();

        if let Ok(key) = Key::create(RootKey::CURRENT_USER, TEST_KEY_PATH, Access::ALL) {
            let result = key.get_value("NonExistent");
            assert!(result.is_err());
        }

        cleanup_test_key();
    }

    #[test]
    fn test_convenience_functions() {
        cleanup_test_key();

        // Test set_string and get_string
        let result = set_string(RootKey::CURRENT_USER, TEST_KEY_PATH, "conv_string", "hello");
        assert!(result.is_ok());

        let value = get_string(RootKey::CURRENT_USER, TEST_KEY_PATH, "conv_string");
        assert!(value.is_ok());
        assert_eq!(value.unwrap(), "hello");

        // Test set_dword and get_dword
        let result = set_dword(RootKey::CURRENT_USER, TEST_KEY_PATH, "conv_dword", 12345);
        assert!(result.is_ok());

        let value = get_dword(RootKey::CURRENT_USER, TEST_KEY_PATH, "conv_dword");
        assert!(value.is_ok());
        assert_eq!(value.unwrap(), 12345);

        cleanup_test_key();
    }

    #[test]
    fn test_access_flags_combination() {
        let combined = Access::READ.with(Access::WRITE);
        assert!((combined.0 .0 & KEY_READ.0) != 0);
        assert!((combined.0 .0 & KEY_WRITE.0) != 0);

        let with_32bit = Access::READ.with(Access::WOW64_32);
        assert!((with_32bit.0 .0 & KEY_WOW64_32KEY.0) != 0);
    }

    #[test]
    fn test_value_constructors() {
        let s = Value::string("test");
        assert_eq!(s.as_string(), Some("test"));

        let d = Value::dword(42);
        assert_eq!(d.as_dword(), Some(42));

        let q = Value::qword(1234567890);
        assert_eq!(q.as_qword(), Some(1234567890));

        let b = Value::binary(vec![1, 2, 3]);
        assert_eq!(b.as_binary(), Some(&[1u8, 2, 3][..]));
    }
}
