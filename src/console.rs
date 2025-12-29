//! Console I/O utilities.
//!
//! Provides safe wrappers for Windows console operations including
//! reading, writing, colors, cursor positioning, and screen buffers.

use crate::error::Result;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Console::{
    AllocConsole, FillConsoleOutputAttribute, FillConsoleOutputCharacterW, FreeConsole,
    GetConsoleCursorInfo, GetConsoleMode, GetConsoleScreenBufferInfo, GetConsoleTitleW,
    GetStdHandle, ReadConsoleW, SetConsoleCursorInfo, SetConsoleCursorPosition, SetConsoleMode,
    SetConsoleTextAttribute, SetConsoleTitleW, WriteConsoleW, CONSOLE_CHARACTER_ATTRIBUTES,
    CONSOLE_CURSOR_INFO, CONSOLE_MODE, CONSOLE_SCREEN_BUFFER_INFO, COORD, ENABLE_ECHO_INPUT,
    ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, ENABLE_PROCESSED_OUTPUT,
    ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};

/// Standard console handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdHandle {
    /// Standard input (stdin).
    Input,
    /// Standard output (stdout).
    Output,
    /// Standard error (stderr).
    Error,
}

impl StdHandle {
    fn to_id(self) -> windows::Win32::System::Console::STD_HANDLE {
        match self {
            StdHandle::Input => STD_INPUT_HANDLE,
            StdHandle::Output => STD_OUTPUT_HANDLE,
            StdHandle::Error => STD_ERROR_HANDLE,
        }
    }
}

/// Console text colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Color {
    Black = 0,
    DarkBlue = 1,
    DarkGreen = 2,
    DarkCyan = 3,
    DarkRed = 4,
    DarkMagenta = 5,
    DarkYellow = 6,
    Gray = 7,
    DarkGray = 8,
    Blue = 9,
    Green = 10,
    Cyan = 11,
    Red = 12,
    Magenta = 13,
    Yellow = 14,
    White = 15,
}

impl Color {
    fn from_u16(value: u16) -> Self {
        match value & 0x0F {
            0 => Color::Black,
            1 => Color::DarkBlue,
            2 => Color::DarkGreen,
            3 => Color::DarkCyan,
            4 => Color::DarkRed,
            5 => Color::DarkMagenta,
            6 => Color::DarkYellow,
            7 => Color::Gray,
            8 => Color::DarkGray,
            9 => Color::Blue,
            10 => Color::Green,
            11 => Color::Cyan,
            12 => Color::Red,
            13 => Color::Magenta,
            14 => Color::Yellow,
            _ => Color::White,
        }
    }
}

/// Console text attributes.
#[derive(Debug, Clone, Copy)]
pub struct TextAttribute {
    foreground: Color,
    background: Color,
}

impl TextAttribute {
    /// Creates a new text attribute with the given colors.
    pub fn new(foreground: Color, background: Color) -> Self {
        Self {
            foreground,
            background,
        }
    }

    /// Creates a text attribute with default colors (gray on black).
    pub fn default_colors() -> Self {
        Self::new(Color::Gray, Color::Black)
    }

    fn to_u16(self) -> u16 {
        (self.foreground as u16) | ((self.background as u16) << 4)
    }
}

impl Default for TextAttribute {
    fn default() -> Self {
        Self::default_colors()
    }
}

/// A Windows console.
pub struct Console {
    input: HANDLE,
    output: HANDLE,
}

impl Console {
    /// Gets the console for the current process.
    pub fn current() -> Result<Self> {
        let input = get_std_handle(StdHandle::Input)?;
        let output = get_std_handle(StdHandle::Output)?;

        Ok(Self { input, output })
    }

    /// Allocates a new console for the current process.
    ///
    /// A process can only have one console, so this will fail if one already exists.
    pub fn alloc() -> Result<Self> {
        // SAFETY: AllocConsole is safe to call
        unsafe {
            AllocConsole()?;
        }
        Self::current()
    }

    /// Frees the console.
    pub fn free() -> Result<()> {
        // SAFETY: FreeConsole is safe to call
        unsafe {
            FreeConsole()?;
        }
        Ok(())
    }

    /// Gets the console title.
    pub fn title() -> Result<String> {
        let mut buffer = vec![0u16; 1024];
        // SAFETY: GetConsoleTitle is safe with a valid buffer
        let len = unsafe { GetConsoleTitleW(&mut buffer) } as usize;

        if len == 0 {
            return Ok(String::new());
        }

        crate::string::from_wide(&buffer[..len])
    }

    /// Sets the console title.
    pub fn set_title(title: &str) -> Result<()> {
        let title_wide = crate::string::WideString::new(title);
        // SAFETY: SetConsoleTitleW is safe with a valid string
        unsafe {
            SetConsoleTitleW(title_wide.as_pcwstr())?;
        }
        Ok(())
    }

    /// Writes a string to the console.
    pub fn write(&self, text: &str) -> Result<usize> {
        let wide: Vec<u16> = text.encode_utf16().collect();
        let mut written = 0u32;

        // SAFETY: WriteConsoleW is safe with valid parameters
        unsafe {
            WriteConsoleW(self.output, &wide, Some(&mut written), None)?;
        }

        Ok(written as usize)
    }

    /// Writes a line to the console (adds newline).
    pub fn write_line(&self, text: &str) -> Result<usize> {
        let mut total = self.write(text)?;
        total += self.write("\r\n")?;
        Ok(total)
    }

    /// Reads a line from the console.
    pub fn read_line(&self) -> Result<String> {
        let mut buffer = vec![0u16; 4096];
        let mut read = 0u32;

        // SAFETY: ReadConsoleW is safe with valid parameters
        unsafe {
            ReadConsoleW(
                self.input,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                &mut read,
                None
            )?;
        }

        // Trim trailing \r\n
        let len = read as usize;
        let end = if len >= 2 && buffer[len - 2] == 0x0D && buffer[len - 1] == 0x0A {
            len - 2
        } else {
            len
        };

        crate::string::from_wide(&buffer[..end])
    }

    /// Sets the text color.
    pub fn set_text_attribute(&self, attr: TextAttribute) -> Result<()> {
        // SAFETY: SetConsoleTextAttribute is safe with valid handle
        unsafe {
            SetConsoleTextAttribute(self.output, CONSOLE_CHARACTER_ATTRIBUTES(attr.to_u16()))?;
        }
        Ok(())
    }

    /// Sets the foreground color.
    pub fn set_foreground(&self, color: Color) -> Result<()> {
        let info = self.screen_buffer_info()?;
        let current_bg = (info.attributes >> 4) & 0x0F;
        let attr = TextAttribute::new(color, Color::from_u16(current_bg));
        self.set_text_attribute(attr)
    }

    /// Sets the background color.
    pub fn set_background(&self, color: Color) -> Result<()> {
        let info = self.screen_buffer_info()?;
        let current_fg = info.attributes & 0x0F;
        let attr = TextAttribute::new(Color::from_u16(current_fg), color);
        self.set_text_attribute(attr)
    }

    /// Gets the cursor position.
    pub fn cursor_position(&self) -> Result<(i16, i16)> {
        let info = self.screen_buffer_info()?;
        Ok((info.cursor_x, info.cursor_y))
    }

    /// Sets the cursor position.
    pub fn set_cursor_position(&self, x: i16, y: i16) -> Result<()> {
        let coord = COORD { X: x, Y: y };
        // SAFETY: SetConsoleCursorPosition is safe with valid parameters
        unsafe {
            SetConsoleCursorPosition(self.output, coord)?;
        }
        Ok(())
    }

    /// Gets cursor visibility and size.
    pub fn cursor_info(&self) -> Result<(bool, u32)> {
        let mut info = CONSOLE_CURSOR_INFO::default();
        // SAFETY: GetConsoleCursorInfo is safe with valid parameters
        unsafe {
            GetConsoleCursorInfo(self.output, &mut info)?;
        }
        Ok((info.bVisible.as_bool(), info.dwSize))
    }

    /// Sets cursor visibility.
    pub fn set_cursor_visible(&self, visible: bool) -> Result<()> {
        let (_, size) = self.cursor_info()?;
        let info = CONSOLE_CURSOR_INFO {
            dwSize: size,
            bVisible: visible.into(),
        };
        // SAFETY: SetConsoleCursorInfo is safe with valid parameters
        unsafe {
            SetConsoleCursorInfo(self.output, &info)?;
        }
        Ok(())
    }

    /// Gets the screen buffer info.
    pub fn screen_buffer_info(&self) -> Result<ScreenBufferInfo> {
        let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
        // SAFETY: GetConsoleScreenBufferInfo is safe with valid parameters
        unsafe {
            GetConsoleScreenBufferInfo(self.output, &mut info)?;
        }

        Ok(ScreenBufferInfo {
            size_x: info.dwSize.X,
            size_y: info.dwSize.Y,
            cursor_x: info.dwCursorPosition.X,
            cursor_y: info.dwCursorPosition.Y,
            attributes: info.wAttributes.0,
            window_left: info.srWindow.Left,
            window_top: info.srWindow.Top,
            window_right: info.srWindow.Right,
            window_bottom: info.srWindow.Bottom,
            max_window_x: info.dwMaximumWindowSize.X,
            max_window_y: info.dwMaximumWindowSize.Y,
        })
    }

    /// Clears the screen.
    pub fn clear(&self) -> Result<()> {
        let info = self.screen_buffer_info()?;
        let size = (info.size_x as u32) * (info.size_y as u32);
        let coord = COORD { X: 0, Y: 0 };
        let mut written = 0u32;

        // Fill with spaces
        // SAFETY: FillConsoleOutputCharacterW is safe with valid parameters
        unsafe {
            FillConsoleOutputCharacterW(self.output, ' ' as u16, size, coord, &mut written)?;
        }

        // Reset attributes
        // SAFETY: FillConsoleOutputAttribute is safe with valid parameters
        unsafe {
            FillConsoleOutputAttribute(self.output, info.attributes, size, coord, &mut written)?;
        }

        // Move cursor to top-left
        self.set_cursor_position(0, 0)?;

        Ok(())
    }

    /// Enables virtual terminal processing (ANSI escape codes).
    pub fn enable_virtual_terminal(&self) -> Result<()> {
        let mut mode = CONSOLE_MODE(0);
        // SAFETY: GetConsoleMode is safe with valid handle
        unsafe {
            GetConsoleMode(self.output, &mut mode)?;
        }

        let new_mode = CONSOLE_MODE(mode.0 | ENABLE_VIRTUAL_TERMINAL_PROCESSING.0 | ENABLE_PROCESSED_OUTPUT.0);
        // SAFETY: SetConsoleMode is safe with valid handle
        unsafe {
            SetConsoleMode(self.output, new_mode)?;
        }

        Ok(())
    }

    /// Enables raw input mode (no line buffering or echo).
    pub fn enable_raw_input(&self) -> Result<()> {
        let mut mode = CONSOLE_MODE(0);
        // SAFETY: GetConsoleMode is safe with valid handle
        unsafe {
            GetConsoleMode(self.input, &mut mode)?;
        }

        // Disable line input and echo
        let new_mode = CONSOLE_MODE(
            mode.0 & !(ENABLE_LINE_INPUT.0 | ENABLE_ECHO_INPUT.0 | ENABLE_PROCESSED_INPUT.0)
        );
        // SAFETY: SetConsoleMode is safe with valid handle
        unsafe {
            SetConsoleMode(self.input, new_mode)?;
        }

        Ok(())
    }

    /// Restores normal input mode.
    pub fn restore_input_mode(&self) -> Result<()> {
        let mode = CONSOLE_MODE(
            ENABLE_LINE_INPUT.0 | ENABLE_ECHO_INPUT.0 | ENABLE_PROCESSED_INPUT.0
        );
        // SAFETY: SetConsoleMode is safe with valid handle
        unsafe {
            SetConsoleMode(self.input, mode)?;
        }
        Ok(())
    }
}

/// Information about the screen buffer.
#[derive(Debug, Clone)]
pub struct ScreenBufferInfo {
    /// Buffer width in characters.
    pub size_x: i16,
    /// Buffer height in characters.
    pub size_y: i16,
    /// Cursor X position.
    pub cursor_x: i16,
    /// Cursor Y position.
    pub cursor_y: i16,
    /// Current text attributes.
    pub attributes: u16,
    /// Window left edge.
    pub window_left: i16,
    /// Window top edge.
    pub window_top: i16,
    /// Window right edge.
    pub window_right: i16,
    /// Window bottom edge.
    pub window_bottom: i16,
    /// Maximum window width.
    pub max_window_x: i16,
    /// Maximum window height.
    pub max_window_y: i16,
}

impl ScreenBufferInfo {
    /// Gets the visible window width.
    pub fn window_width(&self) -> i16 {
        self.window_right - self.window_left + 1
    }

    /// Gets the visible window height.
    pub fn window_height(&self) -> i16 {
        self.window_bottom - self.window_top + 1
    }
}

/// Gets a standard handle.
pub fn get_std_handle(handle: StdHandle) -> Result<HANDLE> {
    // SAFETY: GetStdHandle is safe to call
    let h = unsafe { GetStdHandle(handle.to_id())? };
    Ok(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_attribute() {
        let attr = TextAttribute::new(Color::White, Color::DarkBlue);
        assert_eq!(attr.to_u16(), 0x1F);
    }

    #[test]
    fn test_screen_buffer_info() {
        // This test only works if we have a console with valid handles
        if let Ok(console) = Console::current() {
            // Screen buffer info may fail in CI/non-console environments
            if let Ok(info) = console.screen_buffer_info() {
                assert!(info.size_x > 0);
                assert!(info.size_y > 0);
            }
        }
    }

    #[test]
    fn test_console_title() {
        // This test only works if we have a console
        if Console::current().is_ok() {
            let original = Console::title().unwrap_or_default();
            let _ = Console::set_title("Test Title");
            let _ = Console::set_title(&original);
        }
    }
}

