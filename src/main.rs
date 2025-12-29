//! Example usage of the ergonomic-windows library.

use ergonomic_windows::prelude::*;

fn main() -> Result<()> {
    println!("Ergonomic Windows API Wrapper Demo\n");

    // Demonstrate string conversion
    println!("=== String Conversion ===");
    let text = "Hello, Windows! 🪟";
    let wide = to_wide(text);
    let back = from_wide(&wide)?;
    println!("Original: {}", text);
    println!("Roundtrip: {}", back);
    println!();

    // Demonstrate process utilities
    println!("=== Process Info ===");
    let pid = ergonomic_windows::process::current_pid();
    println!("Current process ID: {}", pid);
    println!();

    // Demonstrate filesystem utilities
    println!("=== File System ===");
    let temp_dir = ergonomic_windows::fs::get_temp_directory()?;
    println!("Temp directory: {}", temp_dir.display());

    let system_dir = ergonomic_windows::fs::get_system_directory()?;
    println!("System directory: {}", system_dir.display());

    let windows_dir = ergonomic_windows::fs::get_windows_directory()?;
    println!("Windows directory: {}", windows_dir.display());
    println!();

    // Demonstrate file existence check
    if exists(&system_dir) {
        println!("System directory exists (as expected)");
    }

    // Demonstrate registry (read-only, safe operations)
    println!("=== Registry ===");
    match Key::open(
        RootKey::LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        Access::READ,
    ) {
        Ok(key) => {
            if let Ok(value) = key.get_value("ProductName") {
                if let Some(name) = value.as_string() {
                    println!("Windows Product: {}", name);
                }
            }
            if let Ok(value) = key.get_value("CurrentBuild") {
                if let Some(build) = value.as_string() {
                    println!("Windows Build: {}", build);
                }
            }
        }
        Err(e) => {
            println!("Could not read registry: {}", e);
        }
    }

    println!("\nDemo complete!");
    Ok(())
}
