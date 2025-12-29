//! RAII wrappers for Windows handles.
//!
//! Provides safe, ergonomic wrappers around raw Windows handles that automatically
//! close the handle when dropped.

use crate::error::{Error, Result};
use windows::Win32::Foundation::{CloseHandle, DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE, INVALID_HANDLE_VALUE};

/// A safe wrapper around a Windows `HANDLE` that automatically closes when dropped.
///
/// This type ensures that Windows handles are properly closed, preventing resource leaks.
///
/// # Example
///
/// ```ignore
/// use ergonomic_windows::handle::OwnedHandle;
/// use windows::Win32::Foundation::HANDLE;
///
/// // Handle will be automatically closed when `handle` goes out of scope
/// let handle = OwnedHandle::new(some_raw_handle)?;
/// ```
#[derive(Debug)]
pub struct OwnedHandle {
    handle: HANDLE,
}

impl OwnedHandle {
    /// Creates a new `OwnedHandle` from a raw `HANDLE`.
    ///
    /// Returns an error if the handle is null or invalid.
    #[inline]
    pub fn new(handle: HANDLE) -> Result<Self> {
        if handle.is_invalid() || handle.0.is_null() {
            return Err(Error::invalid_handle("Cannot create OwnedHandle from invalid handle"));
        }
        Ok(Self { handle })
    }

    /// Creates a new `OwnedHandle` from a raw `HANDLE`, allowing null handles.
    ///
    /// This is useful for handles that may legitimately be null.
    pub fn new_allow_null(handle: HANDLE) -> Result<Self> {
        if handle == INVALID_HANDLE_VALUE {
            return Err(Error::invalid_handle("Cannot create OwnedHandle from INVALID_HANDLE_VALUE"));
        }
        Ok(Self { handle })
    }

    /// Creates an `OwnedHandle` without checking validity.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `handle` is a valid Windows handle obtained from a Windows API call
    /// - The handle has not already been closed
    /// - Ownership of the handle is being transferred to this `OwnedHandle`
    /// - No other code will close this handle (it will be closed when this `OwnedHandle` is dropped)
    /// - The handle is not `INVALID_HANDLE_VALUE` or null (unless specifically intended)
    #[inline]
    pub unsafe fn new_unchecked(handle: HANDLE) -> Self {
        Self { handle }
    }

    /// Returns the raw `HANDLE`.
    #[inline]
    pub fn as_raw(&self) -> HANDLE {
        self.handle
    }

    /// Consumes the `OwnedHandle` and returns the raw `HANDLE` without closing it.
    ///
    /// The caller is now responsible for closing the handle.
    #[inline]
    pub fn into_raw(self) -> HANDLE {
        let handle = self.handle;
        std::mem::forget(self);
        handle
    }

    /// Duplicates this handle, creating a new independently-owned handle.
    ///
    /// The new handle has the same access rights as the original.
    ///
    /// # Errors
    ///
    /// Returns an error if the handle cannot be duplicated (e.g., insufficient access rights).
    pub fn try_clone(&self) -> Result<Self> {
        use windows::Win32::System::Threading::GetCurrentProcess;

        let mut new_handle = HANDLE::default();
        // SAFETY: GetCurrentProcess returns a pseudo-handle that doesn't need to be closed.
        // It is always valid for the lifetime of the current process.
        let current_process = unsafe { GetCurrentProcess() };

        // SAFETY: All parameters are valid:
        // - current_process is valid (pseudo-handle to current process)
        // - self.handle is valid (we own it)
        // - new_handle is a valid output parameter
        // - DUPLICATE_SAME_ACCESS requests the same access rights
        unsafe {
            DuplicateHandle(
                current_process,
                self.handle,
                current_process,
                &mut new_handle,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )?;
        }

        Self::new(new_handle)
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.handle.is_invalid() && !self.handle.0.is_null() {
            // SAFETY: We own this handle exclusively (enforced by the type system),
            // and we've verified it's not invalid or null. After this call,
            // self.handle should not be used (but won't be, as we're being dropped).
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

impl AsRef<HANDLE> for OwnedHandle {
    fn as_ref(&self) -> &HANDLE {
        &self.handle
    }
}

/// A borrowed reference to a Windows handle.
///
/// This type does not close the handle when dropped.
#[derive(Clone, Copy, Debug)]
pub struct BorrowedHandle<'a> {
    handle: HANDLE,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> BorrowedHandle<'a> {
    /// Creates a new `BorrowedHandle` from a raw `HANDLE`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `handle` is a valid Windows handle
    /// - The handle will remain valid and open for the entire lifetime `'a`
    /// - The handle will not be closed by other code during lifetime `'a`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ergonomic_windows::handle::BorrowedHandle;
    /// use windows::Win32::Foundation::HANDLE;
    ///
    /// fn use_handle(handle: HANDLE) {
    ///     // SAFETY: We know handle is valid for the duration of this function
    ///     let borrowed = unsafe { BorrowedHandle::new(handle) };
    ///     // Use borrowed...
    /// }
    /// ```
    #[inline]
    pub unsafe fn new(handle: HANDLE) -> Self {
        Self {
            handle,
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates a borrowed handle from an owned handle.
    #[inline]
    pub fn from_owned(owned: &'a OwnedHandle) -> Self {
        Self {
            handle: owned.handle,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the raw `HANDLE`.
    #[inline]
    pub fn as_raw(&self) -> HANDLE {
        self.handle
    }
}

impl<'a> From<&'a OwnedHandle> for BorrowedHandle<'a> {
    fn from(owned: &'a OwnedHandle) -> Self {
        BorrowedHandle::from_owned(owned)
    }
}

/// Extension trait for working with Windows handles.
pub trait HandleExt {
    /// Returns true if this handle is valid (not null and not INVALID_HANDLE_VALUE).
    fn is_valid(&self) -> bool;
}

impl HandleExt for HANDLE {
    #[inline]
    fn is_valid(&self) -> bool {
        !self.is_invalid() && !self.0.is_null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_handle_rejected() {
        let result = OwnedHandle::new(INVALID_HANDLE_VALUE);
        assert!(result.is_err());
    }

    #[test]
    fn test_null_handle_rejected() {
        let result = OwnedHandle::new(HANDLE::default());
        assert!(result.is_err());
    }

    // ============================================================================
    // Handle Management Stress Tests
    // ============================================================================

    #[test]
    fn test_handle_extension_trait() {
        assert!(!INVALID_HANDLE_VALUE.is_valid());
        assert!(!HANDLE::default().is_valid());
    }

    #[test]
    fn test_new_allow_null_rejects_invalid() {
        let result = OwnedHandle::new_allow_null(INVALID_HANDLE_VALUE);
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_into_raw_prevents_double_close() {
        // Create a real handle via file creation
        use crate::fs::OpenOptions;
        use std::env;

        let temp_path = env::temp_dir().join("handle_test_into_raw.tmp");

        // Create and open the file
        if let Ok(handle) = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&temp_path)
        {
            // Get the raw handle
            let raw = handle.into_raw();

            // Handle should be valid
            assert!(raw.is_valid());

            // Manually close it
            unsafe {
                let _ = CloseHandle(raw);
            }

            // Clean up
            let _ = std::fs::remove_file(&temp_path);
        }
    }

    #[test]
    fn test_handle_try_clone() {
        use crate::fs::OpenOptions;
        use std::env;

        let temp_path = env::temp_dir().join("handle_test_clone.tmp");

        if let Ok(handle) = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&temp_path)
        {
            // Clone the handle
            let cloned = handle.try_clone();
            assert!(cloned.is_ok());

            let cloned = cloned.unwrap();

            // Both handles should be valid and different
            assert!(handle.as_raw().is_valid());
            assert!(cloned.as_raw().is_valid());
            assert_ne!(handle.as_raw().0, cloned.as_raw().0);

            // Clean up (handles will auto-close on drop)
            drop(handle);
            drop(cloned);
            let _ = std::fs::remove_file(&temp_path);
        }
    }

    #[test]
    fn test_borrowed_handle_from_owned() {
        use crate::fs::OpenOptions;
        use std::env;

        let temp_path = env::temp_dir().join("handle_test_borrow.tmp");

        if let Ok(handle) = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&temp_path)
        {
            // Create borrowed handle
            let borrowed = BorrowedHandle::from_owned(&handle);
            assert_eq!(borrowed.as_raw().0, handle.as_raw().0);

            // Also test From trait
            let borrowed2: BorrowedHandle = (&handle).into();
            assert_eq!(borrowed2.as_raw().0, handle.as_raw().0);

            drop(handle);
            let _ = std::fs::remove_file(&temp_path);
        }
    }

    #[test]
    fn test_stress_handle_creation_and_cleanup() {
        use crate::fs::OpenOptions;
        use std::env;

        // Create and close many handles rapidly
        for i in 0..100 {
            let temp_path = env::temp_dir().join(format!("handle_stress_{}.tmp", i));

            if let Ok(_handle) = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&temp_path)
            {
                // Handle auto-closes on drop
            }

            // Clean up file
            let _ = std::fs::remove_file(&temp_path);
        }
    }

    #[test]
    fn test_stress_handle_cloning() {
        use crate::fs::OpenOptions;
        use std::env;

        let temp_path = env::temp_dir().join("handle_stress_clone.tmp");

        if let Ok(handle) = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&temp_path)
        {
            // Clone the handle many times
            let mut clones = Vec::new();
            for _ in 0..50 {
                if let Ok(cloned) = handle.try_clone() {
                    clones.push(cloned);
                }
            }

            // Verify all clones are valid
            for clone in &clones {
                assert!(clone.as_raw().is_valid());
            }

            // Drop all clones
            drop(clones);
            drop(handle);

            let _ = std::fs::remove_file(&temp_path);
        }
    }

    #[test]
    fn test_multiple_handles_same_file() {
        use crate::fs::OpenOptions;
        use std::env;

        let temp_path = env::temp_dir().join("handle_multiple.tmp");

        // Open the same file multiple times with sharing enabled
        let h1 = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .share_read(true)
            .share_write(true)
            .open(&temp_path);

        let h2 = OpenOptions::new()
            .read(true)
            .share_read(true)
            .share_write(true)
            .open(&temp_path);

        let h3 = OpenOptions::new()
            .read(true)
            .share_read(true)
            .share_write(true)
            .open(&temp_path);

        // All handles should be valid
        if let (Ok(h1), Ok(h2), Ok(h3)) = (h1, h2, h3) {
            assert!(h1.as_raw().is_valid());
            assert!(h2.as_raw().is_valid());
            assert!(h3.as_raw().is_valid());

            // All handles should be different
            assert_ne!(h1.as_raw().0, h2.as_raw().0);
            assert_ne!(h2.as_raw().0, h3.as_raw().0);
        }

        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_handle_drop_order() {
        use crate::fs::OpenOptions;
        use std::env;

        // Test that dropping handles in different orders doesn't cause issues
        let temp_path1 = env::temp_dir().join("handle_order_1.tmp");
        let temp_path2 = env::temp_dir().join("handle_order_2.tmp");
        let temp_path3 = env::temp_dir().join("handle_order_3.tmp");

        let h1 = OpenOptions::new().read(true).write(true).create(true).open(&temp_path1).ok();
        let h2 = OpenOptions::new().read(true).write(true).create(true).open(&temp_path2).ok();
        let h3 = OpenOptions::new().read(true).write(true).create(true).open(&temp_path3).ok();

        // Drop in reverse order
        drop(h3);
        drop(h1);
        drop(h2);

        // Clean up
        let _ = std::fs::remove_file(&temp_path1);
        let _ = std::fs::remove_file(&temp_path2);
        let _ = std::fs::remove_file(&temp_path3);
    }
}
