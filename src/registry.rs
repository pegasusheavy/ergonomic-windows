//! Windows Registry access utilities.
//!
//! Provides ergonomic wrappers for reading and writing Windows Registry keys and values.

use crate::error::{Error, Result};
use crate::string::{from_wide, to_wide, WideString};
use windows::Win32::Foundation::{ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegDeleteValueW, RegEnumKeyExW, RegEnumValueW,
    RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG,
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS, KEY_ALL_ACCESS, KEY_CREATE_SUB_KEY,
    KEY_ENUMERATE_SUB_KEYS, KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE, KEY_WRITE, KEY_WOW64_32KEY,
    KEY_WOW64_64KEY, REG_BINARY, REG_DWORD, REG_EXPAND_SZ, REG_MULTI_SZ, REG_OPTION_NON_VOLATILE,
    REG_QWORD, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
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

        let err = unsafe {
            RegSetValueExW(
                self.hkey,
                name_wide.as_pcwstr(),
                0,
                value_type,
                Some(&data),
            )
        };
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
