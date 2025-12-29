//! Memory management utilities.
//!
//! Provides safe wrappers for Windows virtual memory operations,
//! heap management, and memory information queries.

use crate::error::{Error, Result};
use std::ptr::NonNull;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Memory::{
    GetProcessHeap, HeapAlloc, HeapCreate, HeapDestroy, HeapFree, HeapReAlloc, HeapSize,
    VirtualAlloc, VirtualFree, VirtualLock, VirtualProtect, VirtualQuery, VirtualUnlock,
    HEAP_FLAGS, HEAP_NONE, MEMORY_BASIC_INFORMATION, MEM_COMMIT, MEM_DECOMMIT, MEM_RELEASE,
    MEM_RESERVE, PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_NOACCESS,
    PAGE_PROTECTION_FLAGS, PAGE_READONLY, PAGE_READWRITE,
};
use windows::Win32::System::SystemInformation::{
    GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO,
};

/// Memory protection flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protection {
    /// No access allowed.
    NoAccess,
    /// Read-only access.
    ReadOnly,
    /// Read-write access.
    ReadWrite,
    /// Execute only.
    Execute,
    /// Execute and read.
    ExecuteRead,
    /// Execute, read, and write.
    ExecuteReadWrite,
}

impl Protection {
    fn to_flags(self) -> PAGE_PROTECTION_FLAGS {
        match self {
            Protection::NoAccess => PAGE_NOACCESS,
            Protection::ReadOnly => PAGE_READONLY,
            Protection::ReadWrite => PAGE_READWRITE,
            Protection::Execute => PAGE_EXECUTE,
            Protection::ExecuteRead => PAGE_EXECUTE_READ,
            Protection::ExecuteReadWrite => PAGE_EXECUTE_READWRITE,
        }
    }
}

/// A region of virtual memory.
pub struct VirtualMemory {
    ptr: NonNull<u8>,
    size: usize,
}

impl VirtualMemory {
    /// Allocates a region of virtual memory.
    ///
    /// The memory is committed (backed by physical storage) and initialized to zero.
    pub fn alloc(size: usize, protection: Protection) -> Result<Self> {
        Self::alloc_at(None, size, protection)
    }

    /// Allocates a region of virtual memory at a preferred address.
    ///
    /// The system may choose a different address if the requested one is unavailable.
    pub fn alloc_at(address: Option<*mut u8>, size: usize, protection: Protection) -> Result<Self> {
        // SAFETY: VirtualAlloc is safe to call with valid parameters.
        // NULL address means the system chooses the location.
        let ptr = unsafe {
            VirtualAlloc(
                address.map(|p| p as *const _),
                size,
                MEM_COMMIT | MEM_RESERVE,
                protection.to_flags(),
            )
        };

        if ptr.is_null() {
            return Err(crate::error::last_error());
        }

        Ok(Self {
            ptr: NonNull::new(ptr as *mut u8).unwrap(),
            size,
        })
    }

    /// Reserves a region of virtual memory without committing it.
    ///
    /// Reserved memory must be committed before it can be accessed.
    pub fn reserve(size: usize) -> Result<Self> {
        // SAFETY: VirtualAlloc is safe with MEM_RESERVE
        let ptr = unsafe {
            VirtualAlloc(
                None,
                size,
                MEM_RESERVE,
                PAGE_NOACCESS,
            )
        };

        if ptr.is_null() {
            return Err(crate::error::last_error());
        }

        Ok(Self {
            ptr: NonNull::new(ptr as *mut u8).unwrap(),
            size,
        })
    }

    /// Commits a portion of reserved memory.
    pub fn commit(&self, offset: usize, size: usize, protection: Protection) -> Result<()> {
        if offset + size > self.size {
            return Err(Error::custom("Commit region exceeds allocation size"));
        }

        // SAFETY: We own this memory and the parameters are valid
        let ptr = unsafe {
            VirtualAlloc(
                Some(self.ptr.as_ptr().add(offset) as *const _),
                size,
                MEM_COMMIT,
                protection.to_flags(),
            )
        };

        if ptr.is_null() {
            return Err(crate::error::last_error());
        }

        Ok(())
    }

    /// Decommits a portion of committed memory.
    pub fn decommit(&self, offset: usize, size: usize) -> Result<()> {
        if offset + size > self.size {
            return Err(Error::custom("Decommit region exceeds allocation size"));
        }

        // SAFETY: We own this memory and the parameters are valid
        unsafe {
            VirtualFree(
                self.ptr.as_ptr().add(offset) as *mut _,
                size,
                MEM_DECOMMIT,
            )?;
        }

        Ok(())
    }

    /// Changes the protection of a region.
    pub fn protect(&self, offset: usize, size: usize, protection: Protection) -> Result<Protection> {
        if offset + size > self.size {
            return Err(Error::custom("Protect region exceeds allocation size"));
        }

        let mut old_protect = PAGE_PROTECTION_FLAGS(0);

        // SAFETY: We own this memory and the parameters are valid
        unsafe {
            VirtualProtect(
                self.ptr.as_ptr().add(offset) as *const _,
                size,
                protection.to_flags(),
                &mut old_protect,
            )?;
        }

        // Map back to Protection enum
        let old = match old_protect {
            PAGE_NOACCESS => Protection::NoAccess,
            PAGE_READONLY => Protection::ReadOnly,
            PAGE_READWRITE => Protection::ReadWrite,
            PAGE_EXECUTE => Protection::Execute,
            PAGE_EXECUTE_READ => Protection::ExecuteRead,
            PAGE_EXECUTE_READWRITE => Protection::ExecuteReadWrite,
            _ => Protection::NoAccess,
        };

        Ok(old)
    }

    /// Locks a region into physical memory (prevents paging).
    pub fn lock(&self, offset: usize, size: usize) -> Result<()> {
        if offset + size > self.size {
            return Err(Error::custom("Lock region exceeds allocation size"));
        }

        // SAFETY: We own this memory
        unsafe {
            VirtualLock(self.ptr.as_ptr().add(offset) as *const _, size)?;
        }

        Ok(())
    }

    /// Unlocks a previously locked region.
    pub fn unlock(&self, offset: usize, size: usize) -> Result<()> {
        if offset + size > self.size {
            return Err(Error::custom("Unlock region exceeds allocation size"));
        }

        // SAFETY: We own this memory
        unsafe {
            VirtualUnlock(self.ptr.as_ptr().add(offset) as *const _, size)?;
        }

        Ok(())
    }

    /// Returns a pointer to the allocated memory.
    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Returns the size of the allocation.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns a slice of the memory.
    ///
    /// # Safety
    ///
    /// The caller must ensure the memory is committed and has appropriate protection.
    pub unsafe fn as_slice(&self) -> &[u8] {
        std::slice::from_raw_parts(self.ptr.as_ptr(), self.size)
    }

    /// Returns a mutable slice of the memory.
    ///
    /// # Safety
    ///
    /// The caller must ensure the memory is committed and has read-write protection.
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size)
    }
}

impl Drop for VirtualMemory {
    fn drop(&mut self) {
        // SAFETY: We own this memory
        unsafe {
            let _ = VirtualFree(self.ptr.as_ptr() as *mut _, 0, MEM_RELEASE);
        }
    }
}

/// Information about a memory region.
#[derive(Debug)]
pub struct MemoryInfo {
    /// Base address of the region.
    pub base_address: *mut u8,
    /// Size of the region in bytes.
    pub region_size: usize,
    /// Current protection flags.
    pub protection: Protection,
    /// Whether the memory is committed.
    pub is_committed: bool,
    /// Whether the memory is reserved.
    pub is_reserved: bool,
    /// Whether the memory is free.
    pub is_free: bool,
}

/// Queries information about a memory region.
pub fn query_memory(address: *const u8) -> Result<MemoryInfo> {
    let mut info = MEMORY_BASIC_INFORMATION::default();

    // SAFETY: VirtualQuery is safe with valid parameters
    let result = unsafe {
        VirtualQuery(
            Some(address as *const _),
            &mut info,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };

    if result == 0 {
        return Err(crate::error::last_error());
    }

    let protection = match info.Protect {
        PAGE_NOACCESS => Protection::NoAccess,
        PAGE_READONLY => Protection::ReadOnly,
        PAGE_READWRITE => Protection::ReadWrite,
        PAGE_EXECUTE => Protection::Execute,
        PAGE_EXECUTE_READ => Protection::ExecuteRead,
        PAGE_EXECUTE_READWRITE => Protection::ExecuteReadWrite,
        _ => Protection::NoAccess,
    };

    Ok(MemoryInfo {
        base_address: info.BaseAddress as *mut u8,
        region_size: info.RegionSize,
        protection,
        is_committed: info.State.0 & MEM_COMMIT.0 != 0,
        is_reserved: info.State.0 & MEM_RESERVE.0 != 0,
        is_free: info.State.0 == 0x10000, // MEM_FREE
    })
}

/// A Windows heap.
pub struct Heap {
    handle: HANDLE,
    owned: bool,
}

impl Heap {
    /// Creates a new private heap.
    pub fn new() -> Result<Self> {
        // SAFETY: HeapCreate is safe with these parameters
        let handle = unsafe { HeapCreate(HEAP_NONE, 0, 0)? };
        Ok(Self {
            handle,
            owned: true,
        })
    }

    /// Creates a new heap with initial and maximum sizes.
    ///
    /// If `max_size` is 0, the heap can grow as needed.
    pub fn with_size(initial_size: usize, max_size: usize) -> Result<Self> {
        // SAFETY: HeapCreate is safe with valid size parameters
        let handle = unsafe { HeapCreate(HEAP_NONE, initial_size, max_size)? };
        Ok(Self {
            handle,
            owned: true,
        })
    }

    /// Gets the process's default heap.
    pub fn process_heap() -> Result<Self> {
        // SAFETY: GetProcessHeap always succeeds
        let handle = unsafe { GetProcessHeap()? };
        Ok(Self {
            handle,
            owned: false, // Don't destroy the process heap
        })
    }

    /// Allocates memory from this heap.
    pub fn alloc(&self, size: usize) -> Result<NonNull<u8>> {
        // SAFETY: handle is valid
        let ptr = unsafe { HeapAlloc(self.handle, HEAP_NONE, size) };

        if ptr.is_null() {
            return Err(crate::error::last_error());
        }

        Ok(NonNull::new(ptr as *mut u8).unwrap())
    }

    /// Allocates zero-initialized memory from this heap.
    pub fn alloc_zeroed(&self, size: usize) -> Result<NonNull<u8>> {
        use windows::Win32::System::Memory::HEAP_ZERO_MEMORY;

        // SAFETY: handle is valid
        let ptr = unsafe { HeapAlloc(self.handle, HEAP_ZERO_MEMORY, size) };

        if ptr.is_null() {
            return Err(crate::error::last_error());
        }

        Ok(NonNull::new(ptr as *mut u8).unwrap())
    }

    /// Reallocates memory from this heap.
    ///
    /// # Safety
    ///
    /// `ptr` must have been allocated from this heap.
    pub unsafe fn realloc(&self, ptr: NonNull<u8>, new_size: usize) -> Result<NonNull<u8>> {
        let new_ptr = HeapReAlloc(self.handle, HEAP_NONE, Some(ptr.as_ptr() as *const _), new_size);

        if new_ptr.is_null() {
            return Err(crate::error::last_error());
        }

        Ok(NonNull::new(new_ptr as *mut u8).unwrap())
    }

    /// Frees memory allocated from this heap.
    ///
    /// # Safety
    ///
    /// `ptr` must have been allocated from this heap.
    pub unsafe fn free(&self, ptr: NonNull<u8>) -> Result<()> {
        HeapFree(self.handle, HEAP_NONE, Some(ptr.as_ptr() as *const _))?;
        Ok(())
    }

    /// Gets the size of an allocated block.
    ///
    /// # Safety
    ///
    /// `ptr` must have been allocated from this heap.
    pub unsafe fn size(&self, ptr: NonNull<u8>) -> Result<usize> {
        let size = HeapSize(self.handle, HEAP_NONE, ptr.as_ptr() as *const _);
        if size == usize::MAX {
            return Err(crate::error::last_error());
        }
        Ok(size)
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: We own this heap
            unsafe {
                let _ = HeapDestroy(self.handle);
            }
        }
    }
}

/// Global memory status information.
#[derive(Debug, Clone)]
pub struct MemoryStatus {
    /// Percentage of physical memory in use.
    pub memory_load: u32,
    /// Total physical memory in bytes.
    pub total_physical: u64,
    /// Available physical memory in bytes.
    pub available_physical: u64,
    /// Total page file size in bytes.
    pub total_page_file: u64,
    /// Available page file size in bytes.
    pub available_page_file: u64,
    /// Total virtual memory in bytes.
    pub total_virtual: u64,
    /// Available virtual memory in bytes.
    pub available_virtual: u64,
}

/// Gets the current memory status.
pub fn memory_status() -> Result<MemoryStatus> {
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };

    // SAFETY: status is properly initialized
    unsafe {
        GlobalMemoryStatusEx(&mut status)?;
    }

    Ok(MemoryStatus {
        memory_load: status.dwMemoryLoad,
        total_physical: status.ullTotalPhys,
        available_physical: status.ullAvailPhys,
        total_page_file: status.ullTotalPageFile,
        available_page_file: status.ullAvailPageFile,
        total_virtual: status.ullTotalVirtual,
        available_virtual: status.ullAvailVirtual,
    })
}

/// System information.
#[derive(Debug, Clone)]
pub struct SystemMemoryInfo {
    /// Page size in bytes.
    pub page_size: u32,
    /// Allocation granularity in bytes.
    pub allocation_granularity: u32,
    /// Minimum application address.
    pub minimum_address: *const u8,
    /// Maximum application address.
    pub maximum_address: *const u8,
    /// Number of processors.
    pub processor_count: u32,
}

/// Gets system memory information.
pub fn system_info() -> SystemMemoryInfo {
    let mut info = SYSTEM_INFO::default();
    // SAFETY: GetSystemInfo always succeeds
    unsafe {
        GetSystemInfo(&mut info);
    }

    SystemMemoryInfo {
        page_size: info.dwPageSize,
        allocation_granularity: info.dwAllocationGranularity,
        minimum_address: info.lpMinimumApplicationAddress as *const u8,
        maximum_address: info.lpMaximumApplicationAddress as *const u8,
        processor_count: info.dwNumberOfProcessors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_memory_alloc() {
        let mut mem = VirtualMemory::alloc(4096, Protection::ReadWrite).unwrap();
        assert!(!mem.as_ptr().is_null());
        assert_eq!(mem.size(), 4096);

        // Write and read back
        unsafe {
            let slice = mem.as_mut_slice();
            slice[0] = 42;
            slice[1] = 43;
            assert_eq!(slice[0], 42);
            assert_eq!(slice[1], 43);
        }
    }

    #[test]
    fn test_virtual_memory_reserve_commit() {
        let mem = VirtualMemory::reserve(65536).unwrap();

        // Commit the first page
        mem.commit(0, 4096, Protection::ReadWrite).unwrap();

        // Now we can use it
        unsafe {
            let ptr = mem.as_ptr();
            *ptr = 42;
            assert_eq!(*ptr, 42);
        }
    }

    #[test]
    fn test_heap() {
        let heap = Heap::new().unwrap();

        // Allocate some memory
        let ptr = heap.alloc(1024).unwrap();

        // Write to it
        unsafe {
            *ptr.as_ptr() = 42;
            assert_eq!(*ptr.as_ptr(), 42);
        }

        // Get size
        let size = unsafe { heap.size(ptr).unwrap() };
        assert!(size >= 1024);

        // Free it
        unsafe {
            heap.free(ptr).unwrap();
        }
    }

    #[test]
    fn test_memory_status() {
        let status = memory_status().unwrap();
        assert!(status.total_physical > 0);
        assert!(status.available_physical > 0);
        assert!(status.memory_load <= 100);
    }

    #[test]
    fn test_system_info() {
        let info = system_info();
        assert!(info.page_size > 0);
        assert!(info.processor_count > 0);
    }

    #[test]
    fn test_query_memory() {
        let mem = VirtualMemory::alloc(4096, Protection::ReadWrite).unwrap();
        let info = query_memory(mem.as_ptr()).unwrap();

        assert!(info.is_committed);
        assert!(!info.is_free);
        assert!(info.region_size >= 4096);
    }
}

