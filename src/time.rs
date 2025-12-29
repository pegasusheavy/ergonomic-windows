//! Time and timer utilities.
//!
//! Provides safe wrappers for Windows high-resolution timers,
//! system time, and time zone information.

use crate::error::Result;
use std::time::Duration;
use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::System::SystemInformation::{GetLocalTime, GetSystemTime, GetTickCount64};
use windows::Win32::System::Time::{
    FileTimeToSystemTime, GetTimeZoneInformation, SystemTimeToFileTime, TIME_ZONE_INFORMATION,
};

/// A high-resolution performance counter.
pub struct PerformanceCounter {
    frequency: i64,
}

impl PerformanceCounter {
    /// Creates a new performance counter.
    pub fn new() -> Result<Self> {
        let mut frequency = 0i64;
        // SAFETY: QueryPerformanceFrequency is safe with valid output parameter
        unsafe {
            QueryPerformanceFrequency(&mut frequency)?;
        }
        Ok(Self { frequency })
    }

    /// Gets the current counter value.
    pub fn now(&self) -> Result<i64> {
        let mut count = 0i64;
        // SAFETY: QueryPerformanceCounter is safe with valid output parameter
        unsafe {
            QueryPerformanceCounter(&mut count)?;
        }
        Ok(count)
    }

    /// Calculates the elapsed time between two counter values.
    pub fn elapsed(&self, start: i64, end: i64) -> Duration {
        let delta = end - start;
        let secs = delta / self.frequency;
        let nanos = ((delta % self.frequency) * 1_000_000_000) / self.frequency;
        Duration::new(secs as u64, nanos as u32)
    }

    /// Gets the frequency of the counter (counts per second).
    pub fn frequency(&self) -> i64 {
        self.frequency
    }

    /// Measures the duration of a closure.
    pub fn measure<F, R>(&self, f: F) -> Result<(R, Duration)>
    where
        F: FnOnce() -> R,
    {
        let start = self.now()?;
        let result = f();
        let end = self.now()?;
        Ok((result, self.elapsed(start, end)))
    }
}

impl Default for PerformanceCounter {
    fn default() -> Self {
        Self::new().expect("Failed to create performance counter")
    }
}

/// Gets the number of milliseconds since the system started.
pub fn tick_count() -> u64 {
    // SAFETY: GetTickCount64 has no preconditions
    unsafe { GetTickCount64() }
}

/// System time with date and time components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemTime {
    pub year: u16,
    pub month: u16,
    pub day_of_week: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
    pub milliseconds: u16,
}

impl SystemTime {
    /// Gets the current system time (UTC).
    pub fn now_utc() -> Self {
        // SAFETY: GetSystemTime is safe and always succeeds
        let st = unsafe { GetSystemTime() };
        Self::from_windows(st)
    }

    /// Gets the current local time.
    pub fn now_local() -> Self {
        // SAFETY: GetLocalTime is safe and always succeeds
        let st = unsafe { GetLocalTime() };
        Self::from_windows(st)
    }

    fn from_windows(st: SYSTEMTIME) -> Self {
        Self {
            year: st.wYear,
            month: st.wMonth,
            day_of_week: st.wDayOfWeek,
            day: st.wDay,
            hour: st.wHour,
            minute: st.wMinute,
            second: st.wSecond,
            milliseconds: st.wMilliseconds,
        }
    }

    fn to_windows(&self) -> SYSTEMTIME {
        SYSTEMTIME {
            wYear: self.year,
            wMonth: self.month,
            wDayOfWeek: self.day_of_week,
            wDay: self.day,
            wHour: self.hour,
            wMinute: self.minute,
            wSecond: self.second,
            wMilliseconds: self.milliseconds,
        }
    }

    /// Converts to a file time (100-nanosecond intervals since Jan 1, 1601).
    pub fn to_file_time(&self) -> Result<u64> {
        let st = self.to_windows();
        let mut ft = FILETIME::default();
        // SAFETY: SystemTimeToFileTime is safe with valid parameters
        unsafe {
            SystemTimeToFileTime(&st, &mut ft)?;
        }
        Ok(((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64))
    }

    /// Creates from a file time.
    pub fn from_file_time(file_time: u64) -> Result<Self> {
        let ft = FILETIME {
            dwLowDateTime: file_time as u32,
            dwHighDateTime: (file_time >> 32) as u32,
        };
        let mut st = SYSTEMTIME::default();
        // SAFETY: FileTimeToSystemTime is safe with valid parameters
        unsafe {
            FileTimeToSystemTime(&ft, &mut st)?;
        }
        Ok(Self::from_windows(st))
    }

    /// Returns the day name.
    pub fn day_name(&self) -> &'static str {
        match self.day_of_week {
            0 => "Sunday",
            1 => "Monday",
            2 => "Tuesday",
            3 => "Wednesday",
            4 => "Thursday",
            5 => "Friday",
            6 => "Saturday",
            _ => "Unknown",
        }
    }

    /// Returns the month name.
    pub fn month_name(&self) -> &'static str {
        match self.month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        }
    }
}

impl std::fmt::Display for SystemTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.milliseconds
        )
    }
}

/// Time zone status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeZoneStatus {
    /// Time zone uses standard time.
    Standard,
    /// Time zone uses daylight saving time.
    Daylight,
    /// Time zone status is unknown.
    Unknown,
}

/// Time zone information.
#[derive(Debug)]
pub struct TimeZone {
    /// Bias in minutes from UTC.
    pub bias: i32,
    /// Standard time name.
    pub standard_name: String,
    /// Standard time bias in minutes.
    pub standard_bias: i32,
    /// Daylight time name.
    pub daylight_name: String,
    /// Daylight time bias in minutes.
    pub daylight_bias: i32,
    /// Current status.
    pub status: TimeZoneStatus,
}

impl TimeZone {
    /// Gets the current time zone information.
    pub fn current() -> Result<Self> {
        let mut tzi = TIME_ZONE_INFORMATION::default();
        // SAFETY: GetTimeZoneInformation is safe
        let result = unsafe { GetTimeZoneInformation(&mut tzi) };

        // TIME_ZONE_ID_UNKNOWN = 0, TIME_ZONE_ID_STANDARD = 1, TIME_ZONE_ID_DAYLIGHT = 2
        let status = match result {
            1 => TimeZoneStatus::Standard,
            2 => TimeZoneStatus::Daylight,
            _ => TimeZoneStatus::Unknown,
        };

        let standard_name = String::from_utf16_lossy(
            &tzi.StandardName[..tzi.StandardName.iter().position(|&c| c == 0).unwrap_or(32)]
        );
        let daylight_name = String::from_utf16_lossy(
            &tzi.DaylightName[..tzi.DaylightName.iter().position(|&c| c == 0).unwrap_or(32)]
        );

        Ok(Self {
            bias: tzi.Bias,
            standard_name,
            standard_bias: tzi.StandardBias,
            daylight_name,
            daylight_bias: tzi.DaylightBias,
            status,
        })
    }

    /// Gets the total bias (including daylight bias if applicable) in minutes.
    pub fn total_bias(&self) -> i32 {
        match self.status {
            TimeZoneStatus::Daylight => self.bias + self.daylight_bias,
            _ => self.bias + self.standard_bias,
        }
    }

    /// Gets the UTC offset as a duration.
    pub fn utc_offset(&self) -> Duration {
        Duration::from_secs((self.total_bias().abs() * 60) as u64)
    }

    /// Returns true if currently in daylight saving time.
    pub fn is_daylight_saving(&self) -> bool {
        self.status == TimeZoneStatus::Daylight
    }
}

/// A simple stopwatch for measuring elapsed time.
pub struct Stopwatch {
    counter: PerformanceCounter,
    start: i64,
    elapsed: Duration,
    running: bool,
}

impl Stopwatch {
    /// Creates and starts a new stopwatch.
    pub fn start_new() -> Result<Self> {
        let counter = PerformanceCounter::new()?;
        let start = counter.now()?;
        Ok(Self {
            counter,
            start,
            elapsed: Duration::ZERO,
            running: true,
        })
    }

    /// Creates a new stopped stopwatch.
    pub fn new() -> Result<Self> {
        let counter = PerformanceCounter::new()?;
        Ok(Self {
            counter,
            start: 0,
            elapsed: Duration::ZERO,
            running: false,
        })
    }

    /// Starts or resumes the stopwatch.
    pub fn start(&mut self) -> Result<()> {
        if !self.running {
            self.start = self.counter.now()?;
            self.running = true;
        }
        Ok(())
    }

    /// Stops the stopwatch.
    pub fn stop(&mut self) -> Result<()> {
        if self.running {
            let end = self.counter.now()?;
            self.elapsed += self.counter.elapsed(self.start, end);
            self.running = false;
        }
        Ok(())
    }

    /// Resets the stopwatch.
    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
        self.running = false;
    }

    /// Restarts the stopwatch (reset + start).
    pub fn restart(&mut self) -> Result<()> {
        self.reset();
        self.start()
    }

    /// Gets the elapsed time.
    pub fn elapsed(&self) -> Result<Duration> {
        if self.running {
            let end = self.counter.now()?;
            Ok(self.elapsed + self.counter.elapsed(self.start, end))
        } else {
            Ok(self.elapsed)
        }
    }

    /// Returns true if the stopwatch is running.
    pub fn is_running(&self) -> bool {
        self.running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_counter() {
        let counter = PerformanceCounter::new().unwrap();
        assert!(counter.frequency() > 0);

        let start = counter.now().unwrap();
        std::thread::sleep(Duration::from_millis(10));
        let end = counter.now().unwrap();

        let elapsed = counter.elapsed(start, end);
        assert!(elapsed >= Duration::from_millis(5));
    }

    #[test]
    fn test_tick_count() {
        let t1 = tick_count();
        std::thread::sleep(Duration::from_millis(50));
        let t2 = tick_count();
        // Allow for some tolerance - t2 should be >= t1
        assert!(t2 >= t1, "tick_count should be monotonically increasing");
    }

    #[test]
    fn test_system_time() {
        let utc = SystemTime::now_utc();
        let local = SystemTime::now_local();

        assert!(utc.year >= 2024);
        assert!(local.year >= 2024);
        assert!(utc.month >= 1 && utc.month <= 12);
    }

    #[test]
    fn test_time_zone() {
        let tz = TimeZone::current().unwrap();
        // Bias should be reasonable (within +/- 14 hours)
        assert!(tz.bias.abs() <= 14 * 60);
    }

    #[test]
    fn test_stopwatch() {
        let mut sw = Stopwatch::start_new().unwrap();
        std::thread::sleep(Duration::from_millis(10));
        sw.stop().unwrap();

        let elapsed = sw.elapsed().unwrap();
        assert!(elapsed >= Duration::from_millis(5));

        // After stop, elapsed should not change
        std::thread::sleep(Duration::from_millis(10));
        let elapsed2 = sw.elapsed().unwrap();
        assert_eq!(elapsed, elapsed2);
    }
}

