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
}
