//! Threading and synchronization primitives.
//!
//! Provides safe wrappers for Windows threading APIs including threads,
//! mutexes, events, semaphores, and critical sections.

use crate::error::{Error, Result};
use crate::handle::OwnedHandle;
use crate::string::WideString;
use std::time::Duration;
use windows::Win32::Foundation::{HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, CreateSemaphoreW, CreateThread, GetCurrentThreadId,
    GetExitCodeThread, GetThreadId, OpenEventW, OpenMutexW, OpenSemaphoreW, ReleaseMutex,
    ReleaseSemaphore, ResetEvent, ResumeThread, SetEvent, SuspendThread, TerminateThread,
    WaitForSingleObject, EVENT_ALL_ACCESS, EVENT_MODIFY_STATE, INFINITE, MUTEX_ALL_ACCESS,
    SEMAPHORE_ALL_ACCESS, THREAD_CREATION_FLAGS,
};

/// Result of waiting on a synchronization object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitResult {
    /// The object was signaled.
    Signaled,
    /// The wait timed out.
    Timeout,
    /// The mutex was abandoned (owner thread terminated).
    Abandoned,
}

/// A Windows thread handle with RAII cleanup.
pub struct Thread {
    handle: OwnedHandle,
}

impl Thread {
    /// Spawns a new thread that executes the given closure.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ergonomic_windows::thread::Thread;
    ///
    /// let thread = Thread::spawn(|| {
    ///     println!("Hello from thread!");
    ///     42u32
    /// })?;
    ///
    /// let exit_code = thread.join()?;
    /// assert_eq!(exit_code, 42);
    /// # Ok::<(), ergonomic_windows::error::Error>(())
    /// ```
    pub fn spawn<F>(f: F) -> Result<Self>
    where
        F: FnOnce() -> u32 + Send + 'static,
    {
        // Box the closure and leak it - the thread proc will reclaim it
        let boxed: Box<dyn FnOnce() -> u32 + Send> = Box::new(f);
        let raw = Box::into_raw(Box::new(boxed));

        // SAFETY: CreateThread is safe to call with valid parameters.
        // The thread procedure will reclaim the boxed closure.
        let handle = unsafe {
            CreateThread(
                None,
                0,
                Some(thread_proc),
                Some(raw as *const _),
                THREAD_CREATION_FLAGS(0),
                None,
            )?
        };

        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Returns the thread ID.
    pub fn id(&self) -> u32 {
        // SAFETY: handle is valid and we own it
        unsafe { GetThreadId(self.handle.as_raw()) }
    }

    /// Waits for the thread to finish and returns its exit code.
    pub fn join(self) -> Result<u32> {
        self.wait(None)?;
        self.exit_code()
    }

    /// Waits for the thread to finish with an optional timeout.
    pub fn wait(&self, timeout: Option<Duration>) -> Result<WaitResult> {
        let timeout_ms = timeout
            .map(|d| d.as_millis() as u32)
            .unwrap_or(INFINITE);

        // SAFETY: handle is valid
        let result = unsafe { WaitForSingleObject(self.handle.as_raw(), timeout_ms) };

        match result {
            WAIT_OBJECT_0 => Ok(WaitResult::Signaled),
            WAIT_TIMEOUT => Ok(WaitResult::Timeout),
            WAIT_ABANDONED => Ok(WaitResult::Abandoned),
            _ => Err(crate::error::last_error()),
        }
    }

    /// Gets the exit code of the thread.
    ///
    /// Returns `None` if the thread is still running.
    pub fn exit_code(&self) -> Result<u32> {
        let mut exit_code = 0u32;
        // SAFETY: handle is valid, exit_code is a valid output parameter
        unsafe {
            GetExitCodeThread(self.handle.as_raw(), &mut exit_code)?;
        }
        Ok(exit_code)
    }

    /// Suspends the thread.
    ///
    /// Returns the previous suspend count.
    pub fn suspend(&self) -> Result<u32> {
        // SAFETY: handle is valid
        let count = unsafe { SuspendThread(self.handle.as_raw()) };
        if count == u32::MAX {
            Err(crate::error::last_error())
        } else {
            Ok(count)
        }
    }

    /// Resumes a suspended thread.
    ///
    /// Returns the previous suspend count.
    pub fn resume(&self) -> Result<u32> {
        // SAFETY: handle is valid
        let count = unsafe { ResumeThread(self.handle.as_raw()) };
        if count == u32::MAX {
            Err(crate::error::last_error())
        } else {
            Ok(count)
        }
    }

    /// Terminates the thread with the given exit code.
    ///
    /// # Safety
    ///
    /// This is inherently unsafe as it doesn't allow the thread to clean up.
    /// Use only as a last resort.
    pub unsafe fn terminate(&self, exit_code: u32) -> Result<()> {
        TerminateThread(self.handle.as_raw(), exit_code)?;
        Ok(())
    }

    /// Returns the raw handle.
    pub fn as_raw(&self) -> HANDLE {
        self.handle.as_raw()
    }
}

/// Thread procedure that executes the boxed closure.
unsafe extern "system" fn thread_proc(param: *mut std::ffi::c_void) -> u32 {
    // Reclaim the boxed closure
    let boxed: Box<Box<dyn FnOnce() -> u32 + Send>> = Box::from_raw(param as *mut _);
    boxed()
}

/// Gets the current thread ID.
#[inline]
pub fn current_thread_id() -> u32 {
    // SAFETY: GetCurrentThreadId has no preconditions and always succeeds
    unsafe { GetCurrentThreadId() }
}

/// A Windows mutex (mutual exclusion) object.
pub struct Mutex {
    handle: OwnedHandle,
}

impl Mutex {
    /// Creates a new mutex.
    ///
    /// If `initial_owner` is true, the calling thread takes ownership.
    pub fn new(initial_owner: bool) -> Result<Self> {
        // SAFETY: CreateMutexW is safe with these parameters
        let handle = unsafe { CreateMutexW(None, initial_owner, None)? };
        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Creates a named mutex.
    ///
    /// Named mutexes can be shared across processes.
    pub fn new_named(name: &str, initial_owner: bool) -> Result<Self> {
        let name_wide = WideString::new(name);
        // SAFETY: CreateMutexW is safe with valid string
        let handle = unsafe { CreateMutexW(None, initial_owner, name_wide.as_pcwstr())? };
        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Opens an existing named mutex.
    pub fn open(name: &str) -> Result<Self> {
        let name_wide = WideString::new(name);
        // SAFETY: OpenMutexW is safe with valid string
        let handle = unsafe { OpenMutexW(MUTEX_ALL_ACCESS, false, name_wide.as_pcwstr())? };
        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Acquires the mutex, blocking until available.
    pub fn lock(&self) -> Result<MutexGuard<'_>> {
        self.lock_timeout(None)
    }

    /// Tries to acquire the mutex with a timeout.
    pub fn lock_timeout(&self, timeout: Option<Duration>) -> Result<MutexGuard<'_>> {
        let timeout_ms = timeout
            .map(|d| d.as_millis() as u32)
            .unwrap_or(INFINITE);

        // SAFETY: handle is valid
        let result = unsafe { WaitForSingleObject(self.handle.as_raw(), timeout_ms) };

        match result {
            WAIT_OBJECT_0 | WAIT_ABANDONED => Ok(MutexGuard { mutex: self }),
            WAIT_TIMEOUT => Err(Error::custom("Mutex lock timed out")),
            _ => Err(crate::error::last_error()),
        }
    }

    /// Tries to acquire the mutex without blocking.
    pub fn try_lock(&self) -> Result<Option<MutexGuard<'_>>> {
        // SAFETY: handle is valid
        let result = unsafe { WaitForSingleObject(self.handle.as_raw(), 0) };

        match result {
            WAIT_OBJECT_0 | WAIT_ABANDONED => Ok(Some(MutexGuard { mutex: self })),
            WAIT_TIMEOUT => Ok(None),
            _ => Err(crate::error::last_error()),
        }
    }
}

/// RAII guard for a locked mutex.
pub struct MutexGuard<'a> {
    mutex: &'a Mutex,
}

impl Drop for MutexGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: We hold the mutex, so releasing is safe
        unsafe {
            let _ = ReleaseMutex(self.mutex.handle.as_raw());
        }
    }
}

/// A Windows event object for thread signaling.
pub struct Event {
    handle: OwnedHandle,
}

impl Event {
    /// Creates a new manual-reset event.
    ///
    /// A manual-reset event stays signaled until explicitly reset.
    pub fn new_manual(initial_state: bool) -> Result<Self> {
        // SAFETY: CreateEventW is safe with these parameters
        let handle = unsafe { CreateEventW(None, true, initial_state, None)? };
        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Creates a new auto-reset event.
    ///
    /// An auto-reset event automatically resets after releasing a single waiting thread.
    pub fn new_auto(initial_state: bool) -> Result<Self> {
        // SAFETY: CreateEventW is safe with these parameters
        let handle = unsafe { CreateEventW(None, false, initial_state, None)? };
        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Creates a named manual-reset event.
    pub fn new_manual_named(name: &str, initial_state: bool) -> Result<Self> {
        let name_wide = WideString::new(name);
        // SAFETY: CreateEventW is safe with valid string
        let handle = unsafe { CreateEventW(None, true, initial_state, name_wide.as_pcwstr())? };
        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Creates a named auto-reset event.
    pub fn new_auto_named(name: &str, initial_state: bool) -> Result<Self> {
        let name_wide = WideString::new(name);
        // SAFETY: CreateEventW is safe with valid string
        let handle = unsafe { CreateEventW(None, false, initial_state, name_wide.as_pcwstr())? };
        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Opens an existing named event.
    pub fn open(name: &str) -> Result<Self> {
        let name_wide = WideString::new(name);
        // SAFETY: OpenEventW is safe with valid string
        let handle =
            unsafe { OpenEventW(EVENT_ALL_ACCESS | EVENT_MODIFY_STATE, false, name_wide.as_pcwstr())? };
        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Sets the event to signaled state.
    pub fn set(&self) -> Result<()> {
        // SAFETY: handle is valid
        unsafe {
            SetEvent(self.handle.as_raw())?;
        }
        Ok(())
    }

    /// Resets the event to non-signaled state.
    pub fn reset(&self) -> Result<()> {
        // SAFETY: handle is valid
        unsafe {
            ResetEvent(self.handle.as_raw())?;
        }
        Ok(())
    }

    /// Waits for the event to be signaled.
    pub fn wait(&self) -> Result<()> {
        self.wait_timeout(None).map(|_| ())
    }

    /// Waits for the event with a timeout.
    pub fn wait_timeout(&self, timeout: Option<Duration>) -> Result<WaitResult> {
        let timeout_ms = timeout
            .map(|d| d.as_millis() as u32)
            .unwrap_or(INFINITE);

        // SAFETY: handle is valid
        let result = unsafe { WaitForSingleObject(self.handle.as_raw(), timeout_ms) };

        match result {
            WAIT_OBJECT_0 => Ok(WaitResult::Signaled),
            WAIT_TIMEOUT => Ok(WaitResult::Timeout),
            _ => Err(crate::error::last_error()),
        }
    }
}

/// A Windows semaphore object.
pub struct Semaphore {
    handle: OwnedHandle,
}

impl Semaphore {
    /// Creates a new semaphore.
    ///
    /// `initial_count` is the initial count, `max_count` is the maximum.
    pub fn new(initial_count: i32, max_count: i32) -> Result<Self> {
        // SAFETY: CreateSemaphoreW is safe with these parameters
        let handle = unsafe { CreateSemaphoreW(None, initial_count, max_count, None)? };
        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Creates a named semaphore.
    pub fn new_named(name: &str, initial_count: i32, max_count: i32) -> Result<Self> {
        let name_wide = WideString::new(name);
        // SAFETY: CreateSemaphoreW is safe with valid string
        let handle =
            unsafe { CreateSemaphoreW(None, initial_count, max_count, name_wide.as_pcwstr())? };
        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Opens an existing named semaphore.
    pub fn open(name: &str) -> Result<Self> {
        let name_wide = WideString::new(name);
        // SAFETY: OpenSemaphoreW is safe with valid string
        let handle = unsafe { OpenSemaphoreW(SEMAPHORE_ALL_ACCESS, false, name_wide.as_pcwstr())? };
        Ok(Self {
            handle: OwnedHandle::new(handle)?,
        })
    }

    /// Acquires the semaphore, blocking until available.
    pub fn acquire(&self) -> Result<()> {
        self.acquire_timeout(None).map(|_| ())
    }

    /// Tries to acquire the semaphore with a timeout.
    pub fn acquire_timeout(&self, timeout: Option<Duration>) -> Result<WaitResult> {
        let timeout_ms = timeout
            .map(|d| d.as_millis() as u32)
            .unwrap_or(INFINITE);

        // SAFETY: handle is valid
        let result = unsafe { WaitForSingleObject(self.handle.as_raw(), timeout_ms) };

        match result {
            WAIT_OBJECT_0 => Ok(WaitResult::Signaled),
            WAIT_TIMEOUT => Ok(WaitResult::Timeout),
            _ => Err(crate::error::last_error()),
        }
    }

    /// Releases the semaphore, incrementing its count.
    ///
    /// Returns the previous count.
    pub fn release(&self) -> Result<i32> {
        self.release_count(1)
    }

    /// Releases the semaphore, incrementing its count by the specified amount.
    ///
    /// Returns the previous count.
    pub fn release_count(&self, count: i32) -> Result<i32> {
        let mut previous = 0i32;
        // SAFETY: handle is valid, previous is a valid output parameter
        unsafe {
            ReleaseSemaphore(self.handle.as_raw(), count, Some(&mut previous))?;
        }
        Ok(previous)
    }
}

/// Sleeps the current thread for the specified duration.
pub fn sleep(duration: Duration) {
    use windows::Win32::System::Threading::Sleep;
    let ms = duration.as_millis() as u32;
    // SAFETY: Sleep has no preconditions
    unsafe {
        Sleep(ms);
    }
}

/// Yields the current thread's time slice.
pub fn yield_now() {
    use windows::Win32::System::Threading::SwitchToThread;
    // SAFETY: SwitchToThread has no preconditions
    unsafe {
        let _ = SwitchToThread();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_thread_id() {
        let id = current_thread_id();
        assert!(id > 0);
    }

    #[test]
    fn test_thread_spawn_join() {
        let thread = Thread::spawn(|| 42).unwrap();
        let exit_code = thread.join().unwrap();
        assert_eq!(exit_code, 42);
    }

    #[test]
    fn test_mutex_basic() {
        let mutex = Mutex::new(false).unwrap();
        {
            let _guard = mutex.lock().unwrap();
            // Mutex is locked
        }
        // Mutex is released
        let _guard2 = mutex.lock().unwrap();
    }

    #[test]
    fn test_mutex_try_lock() {
        let mutex = Mutex::new(false).unwrap();
        let guard = mutex.try_lock().unwrap();
        assert!(guard.is_some());
        drop(guard);
    }

    #[test]
    fn test_event_manual() {
        let event = Event::new_manual(false).unwrap();

        // Should timeout since event is not signaled
        let result = event.wait_timeout(Some(Duration::from_millis(10))).unwrap();
        assert_eq!(result, WaitResult::Timeout);

        // Signal the event
        event.set().unwrap();

        // Should succeed immediately
        let result = event.wait_timeout(Some(Duration::from_millis(10))).unwrap();
        assert_eq!(result, WaitResult::Signaled);

        // Manual reset - should still be signaled
        let result = event.wait_timeout(Some(Duration::from_millis(10))).unwrap();
        assert_eq!(result, WaitResult::Signaled);

        // Reset and check
        event.reset().unwrap();
        let result = event.wait_timeout(Some(Duration::from_millis(10))).unwrap();
        assert_eq!(result, WaitResult::Timeout);
    }

    #[test]
    fn test_event_auto() {
        let event = Event::new_auto(true).unwrap();

        // Should succeed and auto-reset
        let result = event.wait_timeout(Some(Duration::from_millis(10))).unwrap();
        assert_eq!(result, WaitResult::Signaled);

        // Should timeout since event auto-reset
        let result = event.wait_timeout(Some(Duration::from_millis(10))).unwrap();
        assert_eq!(result, WaitResult::Timeout);
    }

    #[test]
    fn test_semaphore() {
        let sem = Semaphore::new(2, 2).unwrap();

        // Acquire twice (initial count is 2)
        sem.acquire().unwrap();
        sem.acquire().unwrap();

        // Third acquire should timeout
        let result = sem.acquire_timeout(Some(Duration::from_millis(10))).unwrap();
        assert_eq!(result, WaitResult::Timeout);

        // Release one
        let prev = sem.release().unwrap();
        assert_eq!(prev, 0);

        // Now we can acquire again
        sem.acquire().unwrap();
    }

    #[test]
    fn test_sleep() {
        let start = std::time::Instant::now();
        sleep(Duration::from_millis(50));
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(40)); // Allow some tolerance
    }
}

