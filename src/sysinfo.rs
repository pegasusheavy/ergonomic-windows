//! System information utilities.
//!
//! Provides safe wrappers for querying Windows system information
//! including OS version, hardware, and computer details.

use crate::error::Result;
use crate::string::from_wide;
use std::path::PathBuf;
use windows::Win32::System::SystemInformation::{
    GetComputerNameExW, GetNativeSystemInfo, GetVersionExW, ComputerNameDnsDomain,
    ComputerNameDnsFullyQualified, ComputerNameDnsHostname, ComputerNameNetBIOS,
    ComputerNamePhysicalDnsDomain, ComputerNamePhysicalDnsFullyQualified,
    ComputerNamePhysicalDnsHostname, ComputerNamePhysicalNetBIOS, OSVERSIONINFOEXW, SYSTEM_INFO,
};

/// Processor architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessorArchitecture {
    X86,
    X64,
    Arm,
    Arm64,
    Unknown(u16),
}

impl ProcessorArchitecture {
    fn from_id(id: u16) -> Self {
        match id {
            0 => ProcessorArchitecture::X86,
            9 => ProcessorArchitecture::X64,
            5 => ProcessorArchitecture::Arm,
            12 => ProcessorArchitecture::Arm64,
            other => ProcessorArchitecture::Unknown(other),
        }
    }
}

/// System processor information.
#[derive(Debug, Clone)]
pub struct ProcessorInfo {
    /// Processor architecture.
    pub architecture: ProcessorArchitecture,
    /// Number of logical processors.
    pub processor_count: u32,
    /// Processor type.
    pub processor_type: u32,
    /// Processor level.
    pub processor_level: u16,
    /// Processor revision.
    pub processor_revision: u16,
    /// Page size in bytes.
    pub page_size: u32,
    /// Minimum application address.
    pub min_address: usize,
    /// Maximum application address.
    pub max_address: usize,
    /// Active processor mask.
    pub active_processor_mask: usize,
    /// Allocation granularity.
    pub allocation_granularity: u32,
}

/// Gets processor information.
pub fn processor_info() -> ProcessorInfo {
    let mut info = SYSTEM_INFO::default();
    // SAFETY: GetNativeSystemInfo is safe
    unsafe {
        GetNativeSystemInfo(&mut info);
    }

    let arch = unsafe { info.Anonymous.Anonymous.wProcessorArchitecture };

    ProcessorInfo {
        architecture: ProcessorArchitecture::from_id(arch.0),
        processor_count: info.dwNumberOfProcessors,
        processor_type: info.dwProcessorType,
        processor_level: 0, // Not directly available in the union
        processor_revision: 0, // Not directly available in the union
        page_size: info.dwPageSize,
        min_address: info.lpMinimumApplicationAddress as usize,
        max_address: info.lpMaximumApplicationAddress as usize,
        active_processor_mask: info.dwActiveProcessorMask,
        allocation_granularity: info.dwAllocationGranularity,
    }
}

/// Operating system version information.
#[derive(Debug, Clone)]
pub struct OsVersion {
    /// Major version number.
    pub major: u32,
    /// Minor version number.
    pub minor: u32,
    /// Build number.
    pub build: u32,
    /// Service pack major version.
    pub service_pack_major: u16,
    /// Service pack minor version.
    pub service_pack_minor: u16,
    /// Product type (workstation, server, etc.).
    pub product_type: u8,
}

impl OsVersion {
    /// Gets the OS version.
    ///
    /// Note: Starting with Windows 8.1, this function is deprecated
    /// and may return Windows 8 version unless the app is manifested.
    pub fn get() -> Result<Self> {
        let mut info = OSVERSIONINFOEXW {
            dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOEXW>() as u32,
            ..Default::default()
        };

        // SAFETY: GetVersionExW is safe with properly initialized struct
        unsafe {
            GetVersionExW(&mut info as *mut _ as *mut _)?;
        }

        Ok(Self {
            major: info.dwMajorVersion,
            minor: info.dwMinorVersion,
            build: info.dwBuildNumber,
            service_pack_major: info.wServicePackMajor,
            service_pack_minor: info.wServicePackMinor,
            product_type: info.wProductType,
        })
    }

    /// Returns true if running on Windows 10 or later.
    pub fn is_windows_10_or_later(&self) -> bool {
        self.major >= 10
    }

    /// Returns true if running on Windows 11 or later.
    pub fn is_windows_11_or_later(&self) -> bool {
        self.major >= 10 && self.build >= 22000
    }

    /// Returns a display string for the version.
    pub fn display_string(&self) -> String {
        if self.major >= 10 {
            if self.build >= 22000 {
                format!("Windows 11 (Build {})", self.build)
            } else {
                format!("Windows 10 (Build {})", self.build)
            }
        } else if self.major == 6 {
            match self.minor {
                3 => format!("Windows 8.1 (Build {})", self.build),
                2 => format!("Windows 8 (Build {})", self.build),
                1 => format!("Windows 7 (Build {})", self.build),
                0 => format!("Windows Vista (Build {})", self.build),
                _ => format!("Windows {}.{} (Build {})", self.major, self.minor, self.build),
            }
        } else {
            format!("Windows {}.{} (Build {})", self.major, self.minor, self.build)
        }
    }
}

impl std::fmt::Display for OsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_string())
    }
}

/// Computer name types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputerNameType {
    /// NetBIOS name.
    NetBios,
    /// DNS hostname.
    DnsHostname,
    /// DNS domain.
    DnsDomain,
    /// Fully qualified DNS name.
    DnsFullyQualified,
    /// Physical NetBIOS name.
    PhysicalNetBios,
    /// Physical DNS hostname.
    PhysicalDnsHostname,
    /// Physical DNS domain.
    PhysicalDnsDomain,
    /// Physical fully qualified DNS name.
    PhysicalDnsFullyQualified,
}

impl ComputerNameType {
    fn to_native(self) -> windows::Win32::System::SystemInformation::COMPUTER_NAME_FORMAT {
        match self {
            ComputerNameType::NetBios => ComputerNameNetBIOS,
            ComputerNameType::DnsHostname => ComputerNameDnsHostname,
            ComputerNameType::DnsDomain => ComputerNameDnsDomain,
            ComputerNameType::DnsFullyQualified => ComputerNameDnsFullyQualified,
            ComputerNameType::PhysicalNetBios => ComputerNamePhysicalNetBIOS,
            ComputerNameType::PhysicalDnsHostname => ComputerNamePhysicalDnsHostname,
            ComputerNameType::PhysicalDnsDomain => ComputerNamePhysicalDnsDomain,
            ComputerNameType::PhysicalDnsFullyQualified => ComputerNamePhysicalDnsFullyQualified,
        }
    }
}

/// Gets a computer name.
pub fn computer_name(name_type: ComputerNameType) -> Result<String> {
    let mut size = 0u32;

    // First call to get size
    // SAFETY: GetComputerNameExW is safe
    let _ = unsafe { GetComputerNameExW(name_type.to_native(), windows::core::PWSTR::null(), &mut size) };

    if size == 0 {
        return Ok(String::new());
    }

    let mut buffer = vec![0u16; size as usize];

    // SAFETY: GetComputerNameExW is safe with valid buffer
    unsafe {
        GetComputerNameExW(
            name_type.to_native(),
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )?;
    }

    from_wide(&buffer[..size as usize])
}

/// Gets the NetBIOS computer name.
pub fn hostname() -> Result<String> {
    computer_name(ComputerNameType::NetBios)
}

/// Gets the DNS hostname.
pub fn dns_hostname() -> Result<String> {
    computer_name(ComputerNameType::DnsHostname)
}

/// Gets the DNS domain name.
pub fn dns_domain() -> Result<String> {
    computer_name(ComputerNameType::DnsDomain)
}

/// Gets the fully qualified domain name.
pub fn fqdn() -> Result<String> {
    computer_name(ComputerNameType::DnsFullyQualified)
}

/// Windows directories.
#[derive(Debug, Clone)]
pub struct WindowsDirectories {
    /// Windows directory (e.g., C:\Windows).
    pub windows: PathBuf,
    /// System directory (e.g., C:\Windows\System32).
    pub system: PathBuf,
    /// Temp directory.
    pub temp: PathBuf,
}

/// Gets standard Windows directories.
pub fn windows_directories() -> Result<WindowsDirectories> {
    Ok(WindowsDirectories {
        windows: crate::fs::get_windows_directory()?,
        system: crate::fs::get_system_directory()?,
        temp: crate::fs::get_temp_directory()?,
    })
}

/// Summary of system information.
#[derive(Debug)]
pub struct SystemSummary {
    /// Operating system version.
    pub os_version: OsVersion,
    /// Processor information.
    pub processor: ProcessorInfo,
    /// Computer hostname.
    pub hostname: String,
    /// Memory status.
    pub memory: crate::mem::MemoryStatus,
}

/// Gets a summary of system information.
pub fn system_summary() -> Result<SystemSummary> {
    Ok(SystemSummary {
        os_version: OsVersion::get()?,
        processor: processor_info(),
        hostname: hostname()?,
        memory: crate::mem::memory_status()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_info() {
        let info = processor_info();
        assert!(info.processor_count > 0);
        assert!(info.page_size > 0);
    }

    #[test]
    fn test_os_version() {
        let version = OsVersion::get().unwrap();
        assert!(version.major >= 6); // At least Vista
        println!("OS: {}", version);
    }

    #[test]
    fn test_hostname() {
        let name = hostname().unwrap();
        assert!(!name.is_empty());
    }

    #[test]
    fn test_system_summary() {
        let summary = system_summary().unwrap();
        println!("OS: {}", summary.os_version);
        println!("Hostname: {}", summary.hostname);
        println!("Processors: {}", summary.processor.processor_count);
        println!("Memory: {} MB total", summary.memory.total_physical / 1024 / 1024);
    }
}

