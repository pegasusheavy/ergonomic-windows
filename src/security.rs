//! Security utilities.
//!
//! Provides safe wrappers for Windows security operations
//! including tokens, privileges, and access control.

use crate::error::Result;
use crate::handle::OwnedHandle;
use crate::string::WideString;
use windows::Win32::Foundation::{HANDLE, LUID};
use windows::Win32::Security::{
    AdjustTokenPrivileges, GetTokenInformation, LookupPrivilegeNameW, LookupPrivilegeValueW,
    TokenElevation, TokenPrivileges, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_ACCESS_MASK, TOKEN_ADJUST_PRIVILEGES, TOKEN_ELEVATION, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// Well-known privilege names.
pub mod privileges {
    /// Required to debug processes.
    pub const SE_DEBUG_NAME: &str = "SeDebugPrivilege";
    /// Required to shut down the system.
    pub const SE_SHUTDOWN_NAME: &str = "SeShutdownPrivilege";
    /// Required to back up files.
    pub const SE_BACKUP_NAME: &str = "SeBackupPrivilege";
    /// Required to restore files.
    pub const SE_RESTORE_NAME: &str = "SeRestorePrivilege";
    /// Required to change system time.
    pub const SE_SYSTEMTIME_NAME: &str = "SeSystemtimePrivilege";
    /// Required to take ownership of objects.
    pub const SE_TAKE_OWNERSHIP_NAME: &str = "SeTakeOwnershipPrivilege";
    /// Required to load drivers.
    pub const SE_LOAD_DRIVER_NAME: &str = "SeLoadDriverPrivilege";
    /// Required to manage auditing and security log.
    pub const SE_SECURITY_NAME: &str = "SeSecurityPrivilege";
    /// Required to increase process priority.
    pub const SE_INC_BASE_PRIORITY_NAME: &str = "SeIncreaseBasePriorityPrivilege";
    /// Required to create symbolic links.
    pub const SE_CREATE_SYMBOLIC_LINK_NAME: &str = "SeCreateSymbolicLinkPrivilege";
}

/// A Windows access token.
pub struct Token {
    handle: OwnedHandle,
}

impl Token {
    /// Opens the token for the current process.
    pub fn current_process() -> Result<Self> {
        Self::current_process_with_access(TOKEN_QUERY | TOKEN_ADJUST_PRIVILEGES)
    }

    /// Opens the token for the current process with specific access.
    pub fn current_process_with_access(access: TOKEN_ACCESS_MASK) -> Result<Self> {
        let mut handle = HANDLE::default();

        // SAFETY: GetCurrentProcess always returns a valid pseudo-handle,
        // and OpenProcessToken is safe with valid parameters
        unsafe {
            let process = GetCurrentProcess();
            OpenProcessToken(process, access, &mut handle)?;
        }

        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Opens the token for a specific process.
    pub fn for_process(process: HANDLE) -> Result<Self> {
        Self::for_process_with_access(process, TOKEN_QUERY | TOKEN_ADJUST_PRIVILEGES)
    }

    /// Opens the token for a specific process with specific access.
    pub fn for_process_with_access(process: HANDLE, access: TOKEN_ACCESS_MASK) -> Result<Self> {
        let mut handle = HANDLE::default();

        // SAFETY: OpenProcessToken is safe with valid parameters
        unsafe {
            OpenProcessToken(process, access, &mut handle)?;
        }

        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Checks if the token is elevated (running as administrator).
    pub fn is_elevated(&self) -> Result<bool> {
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0u32;

        // SAFETY: GetTokenInformation is safe with valid parameters
        unsafe {
            GetTokenInformation(
                self.handle.as_raw(),
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            )?;
        }

        Ok(elevation.TokenIsElevated != 0)
    }

    /// Enables a privilege in the token.
    pub fn enable_privilege(&self, privilege_name: &str) -> Result<bool> {
        self.adjust_privilege(privilege_name, true)
    }

    /// Disables a privilege in the token.
    pub fn disable_privilege(&self, privilege_name: &str) -> Result<bool> {
        self.adjust_privilege(privilege_name, false)
    }

    /// Adjusts a privilege (enable or disable).
    ///
    /// Returns true if the privilege was previously enabled.
    fn adjust_privilege(&self, privilege_name: &str, enable: bool) -> Result<bool> {
        let name_wide = WideString::new(privilege_name);
        let mut luid = LUID::default();

        // SAFETY: LookupPrivilegeValueW is safe with valid parameters
        unsafe {
            LookupPrivilegeValueW(None, name_wide.as_pcwstr(), &mut luid)?;
        }

        let mut tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: if enable { SE_PRIVILEGE_ENABLED } else { Default::default() },
            }],
        };

        let mut previous_state = TOKEN_PRIVILEGES::default();
        let mut return_length = 0u32;

        // SAFETY: AdjustTokenPrivileges is safe with valid parameters
        unsafe {
            AdjustTokenPrivileges(
                self.handle.as_raw(),
                false,
                Some(&tp),
                std::mem::size_of::<TOKEN_PRIVILEGES>() as u32,
                Some(&mut previous_state),
                Some(&mut return_length),
            )?;
        }

        // Return whether it was previously enabled
        if previous_state.PrivilegeCount > 0 {
            Ok(previous_state.Privileges[0].Attributes.0 & SE_PRIVILEGE_ENABLED.0 != 0)
        } else {
            Ok(false)
        }
    }

    /// Checks if a privilege is enabled.
    pub fn has_privilege(&self, privilege_name: &str) -> Result<bool> {
        let name_wide = WideString::new(privilege_name);
        let mut luid = LUID::default();

        // SAFETY: LookupPrivilegeValueW is safe with valid parameters
        unsafe {
            LookupPrivilegeValueW(None, name_wide.as_pcwstr(), &mut luid)?;
        }

        // Get token privileges
        let mut size = 0u32;
        let _ = unsafe {
            GetTokenInformation(
                self.handle.as_raw(),
                TokenPrivileges,
                None,
                0,
                &mut size,
            )
        };

        if size == 0 {
            return Ok(false);
        }

        let mut buffer = vec![0u8; size as usize];

        // SAFETY: GetTokenInformation is safe with valid buffer
        unsafe {
            GetTokenInformation(
                self.handle.as_raw(),
                TokenPrivileges,
                Some(buffer.as_mut_ptr() as *mut _),
                size,
                &mut size,
            )?;
        }

        // Parse the token privileges
        let privs = buffer.as_ptr() as *const TOKEN_PRIVILEGES;
        let count = unsafe { (*privs).PrivilegeCount } as usize;

        for i in 0..count {
            let priv_ptr = unsafe { (*privs).Privileges.as_ptr().add(i) };
            let priv_luid = unsafe { (*priv_ptr).Luid };
            let priv_attrs = unsafe { (*priv_ptr).Attributes };

            if priv_luid.LowPart == luid.LowPart && priv_luid.HighPart == luid.HighPart {
                return Ok(priv_attrs.0 & SE_PRIVILEGE_ENABLED.0 != 0);
            }
        }

        Ok(false)
    }

    /// Returns the raw token handle.
    pub fn as_raw(&self) -> HANDLE {
        self.handle.as_raw()
    }
}

/// Checks if the current process is running as administrator.
pub fn is_elevated() -> Result<bool> {
    Token::current_process()?.is_elevated()
}

/// Gets the name of a privilege from its LUID.
pub fn privilege_name(luid: LUID) -> Result<String> {
    let mut size = 0u32;

    // Get required size
    let _ = unsafe {
        LookupPrivilegeNameW(None, &luid, windows::core::PWSTR::null(), &mut size)
    };

    if size == 0 {
        return Err(crate::error::last_error());
    }

    let mut buffer = vec![0u16; size as usize];

    // SAFETY: LookupPrivilegeNameW is safe with valid buffer
    unsafe {
        LookupPrivilegeNameW(
            None,
            &luid,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )?;
    }

    crate::string::from_wide(&buffer[..size as usize])
}

/// RAII guard that restores a privilege to its original state when dropped.
pub struct PrivilegeGuard<'a> {
    token: &'a Token,
    privilege_name: String,
    was_enabled: bool,
}

impl<'a> PrivilegeGuard<'a> {
    /// Enables a privilege and returns a guard that will restore it.
    pub fn enable(token: &'a Token, privilege_name: &str) -> Result<Self> {
        let was_enabled = token.enable_privilege(privilege_name)?;
        Ok(Self {
            token,
            privilege_name: privilege_name.to_string(),
            was_enabled,
        })
    }
}

impl Drop for PrivilegeGuard<'_> {
    fn drop(&mut self) {
        if !self.was_enabled {
            let _ = self.token.disable_privilege(&self.privilege_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_process_token() {
        let token = Token::current_process().unwrap();
        // Just verify we can open the token
        let _ = token.as_raw();
    }

    #[test]
    fn test_is_elevated() {
        let elevated = is_elevated().unwrap();
        // Just verify we can check - result depends on how test is run
        println!("Running elevated: {}", elevated);
    }

    #[test]
    fn test_privilege_check() {
        let token = Token::current_process().unwrap();
        // SeChangeNotifyPrivilege is typically enabled for all users
        let has_change_notify = token.has_privilege("SeChangeNotifyPrivilege");
        // Just verify we can check
        println!("Has SeChangeNotifyPrivilege: {:?}", has_change_notify);
    }
}

