//! Win32 Common Controls.
//!
//! Provides safe wrappers for Windows common controls including buttons,
//! edit boxes, list views, progress bars, and more.

#![allow(clippy::new_ret_no_self)] // Controls return Control, not Self - by design
#![allow(clippy::too_many_arguments)] // Control constructors need many parameters

use crate::error::{Error, Result};
use crate::string::WideString;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::InvalidateRect;
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, ICC_STANDARD_CLASSES, ICC_WIN95_CLASSES, INITCOMMONCONTROLSEX,
    PBS_MARQUEE, PBS_SMOOTH, PBM_DELTAPOS, PBM_GETPOS, PBM_SETMARQUEE, PBM_SETPOS, PBM_SETRANGE32,
    PBM_SETSTEP, PBM_STEPIT, PROGRESS_CLASSW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetWindowLongPtrW, SendMessageW, SetWindowLongPtrW,
    SetWindowTextW, ShowWindow, HMENU, SW_HIDE, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_GETTEXT,
    WM_GETTEXTLENGTH, WS_BORDER, WS_CHILD, WS_DISABLED, WS_EX_CLIENTEDGE, WS_TABSTOP, WS_VISIBLE,
};

// Button style constants (these are raw i32 values)
const BS_PUSHBUTTON: i32 = 0x0000;
const BS_DEFPUSHBUTTON: i32 = 0x0001;
const BS_CHECKBOX: i32 = 0x0002;
const BS_AUTOCHECKBOX: i32 = 0x0003;
const BS_RADIOBUTTON: i32 = 0x0004;
const BS_AUTORADIOBUTTON: i32 = 0x0009;
const BS_GROUPBOX: i32 = 0x0007;

// Button check states
const BST_UNCHECKED: usize = 0x0000;
const BST_CHECKED: usize = 0x0001;

// Button messages
const BM_GETCHECK: u32 = 0x00F0;
const BM_SETCHECK: u32 = 0x00F1;

// Edit style constants
const ES_LEFT: i32 = 0x0000;
const ES_CENTER: i32 = 0x0001;
const ES_RIGHT: i32 = 0x0002;
const ES_MULTILINE: i32 = 0x0004;
const ES_PASSWORD: i32 = 0x0020;
const ES_AUTOVSCROLL: i32 = 0x0040;
const ES_AUTOHSCROLL: i32 = 0x0080;
const ES_READONLY: i32 = 0x0800;
const ES_NUMBER: i32 = 0x2000;

// Edit messages
const EM_GETSEL: u32 = 0x00B0;
const EM_SETSEL: u32 = 0x00B1;
const EM_LIMITTEXT: u32 = 0x00C5;
const EM_REPLACESEL: u32 = 0x00C2;
const EM_SETREADONLY: u32 = 0x00CF;

// List box messages
const LB_ADDSTRING: u32 = 0x0180;
const LB_INSERTSTRING: u32 = 0x0181;
const LB_DELETESTRING: u32 = 0x0182;
const LB_RESETCONTENT: u32 = 0x0184;
const LB_GETCOUNT: u32 = 0x018B;
const LB_GETCURSEL: u32 = 0x0188;
const LB_SETCURSEL: u32 = 0x0186;

// Combo box messages
const CB_ADDSTRING: u32 = 0x0143;
const CB_RESETCONTENT: u32 = 0x014B;
const CB_GETCOUNT: u32 = 0x0146;
const CB_GETCURSEL: u32 = 0x0147;
const CB_SETCURSEL: u32 = 0x014E;

/// Initialize common controls. Call this before creating any controls.
///
/// This is automatically called by control constructors, but you can call it
/// explicitly for more control over initialization.
pub fn init_common_controls() -> Result<()> {
    let icc = INITCOMMONCONTROLSEX {
        dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_STANDARD_CLASSES | ICC_WIN95_CLASSES,
    };

    // SAFETY: InitCommonControlsEx is safe with valid parameters
    let result = unsafe { InitCommonControlsEx(&icc) };

    if result.as_bool() {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}

/// A Windows control handle with RAII semantics.
#[derive(Debug)]
pub struct Control {
    hwnd: HWND,
    owned: bool,
}

impl Control {
    /// Creates a control from a raw HWND.
    ///
    /// # Safety
    ///
    /// The HWND must be a valid window handle.
    pub unsafe fn from_raw(hwnd: HWND, owned: bool) -> Self {
        Self { hwnd, owned }
    }

    /// Returns the raw HWND.
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Shows the control.
    pub fn show(&self) {
        // SAFETY: ShowWindow is safe with valid HWND
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOW);
        }
    }

    /// Hides the control.
    pub fn hide(&self) {
        // SAFETY: ShowWindow is safe with valid HWND
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    /// Enables the control.
    pub fn enable(&self) {
        // SAFETY: Window style manipulation is safe
        unsafe {
            let style =
                GetWindowLongPtrW(self.hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_STYLE);
            SetWindowLongPtrW(
                self.hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWL_STYLE,
                style & !(WS_DISABLED.0 as isize),
            );
            let _ = InvalidateRect(self.hwnd, None, true);
        }
    }

    /// Disables the control.
    pub fn disable(&self) {
        // SAFETY: Window style manipulation is safe
        unsafe {
            let style =
                GetWindowLongPtrW(self.hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_STYLE);
            SetWindowLongPtrW(
                self.hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWL_STYLE,
                style | (WS_DISABLED.0 as isize),
            );
            let _ = InvalidateRect(self.hwnd, None, true);
        }
    }

    /// Gets the control's text.
    pub fn text(&self) -> String {
        // SAFETY: WM_GETTEXTLENGTH and WM_GETTEXT are safe
        unsafe {
            let len = SendMessageW(self.hwnd, WM_GETTEXTLENGTH, WPARAM(0), LPARAM(0)).0 as usize;
            if len == 0 {
                return String::new();
            }

            let mut buffer = vec![0u16; len + 1];
            SendMessageW(
                self.hwnd,
                WM_GETTEXT,
                WPARAM(buffer.len()),
                LPARAM(buffer.as_mut_ptr() as isize),
            );

            String::from_utf16_lossy(&buffer[..len])
        }
    }

    /// Sets the control's text.
    pub fn set_text(&self, text: &str) {
        let wide = WideString::new(text);
        // SAFETY: SetWindowTextW is safe with valid parameters
        unsafe {
            let _ = SetWindowTextW(self.hwnd, wide.as_pcwstr());
        }
    }

    /// Sets user data associated with the control.
    pub fn set_user_data(&self, data: isize) {
        // SAFETY: GWLP_USERDATA manipulation is safe
        unsafe {
            SetWindowLongPtrW(
                self.hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                data,
            );
        }
    }

    /// Gets user data associated with the control.
    pub fn user_data(&self) -> isize {
        // SAFETY: GWLP_USERDATA access is safe
        unsafe {
            GetWindowLongPtrW(
                self.hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
            )
        }
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        if self.owned && !self.hwnd.is_invalid() {
            // SAFETY: DestroyWindow is safe for owned windows
            unsafe {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}

/// Button styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    /// Standard push button.
    Push,
    /// Default push button (responds to Enter key).
    Default,
    /// Checkbox.
    Checkbox,
    /// Auto checkbox (toggles automatically).
    AutoCheckbox,
    /// Radio button.
    Radio,
    /// Auto radio button.
    AutoRadio,
    /// Group box (container).
    GroupBox,
}

impl ButtonStyle {
    fn to_style(self) -> u32 {
        match self {
            ButtonStyle::Push => BS_PUSHBUTTON as u32,
            ButtonStyle::Default => BS_DEFPUSHBUTTON as u32,
            ButtonStyle::Checkbox => BS_CHECKBOX as u32,
            ButtonStyle::AutoCheckbox => BS_AUTOCHECKBOX as u32,
            ButtonStyle::Radio => BS_RADIOBUTTON as u32,
            ButtonStyle::AutoRadio => BS_AUTORADIOBUTTON as u32,
            ButtonStyle::GroupBox => BS_GROUPBOX as u32,
        }
    }
}

/// A Windows button control.
pub struct Button;

impl Button {
    /// Creates a new button.
    pub fn new(
        parent: HWND,
        text: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        id: u16,
        style: ButtonStyle,
    ) -> Result<Control> {
        init_common_controls()?;

        let text_wide = WideString::new(text);
        let class_wide = WideString::new("BUTTON");

        let base_style = WS_CHILD | WS_VISIBLE | WS_TABSTOP;
        let button_style = WINDOW_STYLE(style.to_style());

        // SAFETY: CreateWindowExW is safe with valid parameters
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_wide.as_pcwstr(),
                text_wide.as_pcwstr(),
                base_style | button_style,
                x,
                y,
                width,
                height,
                parent,
                HMENU(id as isize as *mut _),
                HINSTANCE::default(),
                None,
            )?
        };

        Ok(unsafe { Control::from_raw(hwnd, true) })
    }

    /// Checks if a checkbox/radio button is checked.
    pub fn is_checked(control: &Control) -> bool {
        // SAFETY: BM_GETCHECK is safe
        let result = unsafe { SendMessageW(control.hwnd(), BM_GETCHECK, WPARAM(0), LPARAM(0)) };
        result.0 == BST_CHECKED as isize
    }

    /// Sets the check state of a checkbox/radio button.
    pub fn set_checked(control: &Control, checked: bool) {
        let state = if checked { BST_CHECKED } else { BST_UNCHECKED };
        // SAFETY: BM_SETCHECK is safe
        unsafe {
            SendMessageW(control.hwnd(), BM_SETCHECK, WPARAM(state), LPARAM(0));
        }
    }
}

/// Edit control styles.
#[derive(Debug, Clone, Copy, Default)]
pub struct EditStyle {
    /// Allow multiple lines.
    pub multiline: bool,
    /// Password mode (shows dots).
    pub password: bool,
    /// Read-only mode.
    pub readonly: bool,
    /// Only allow numbers.
    pub number: bool,
    /// Text alignment.
    pub align: TextAlign,
    /// Auto horizontal scroll.
    pub auto_hscroll: bool,
    /// Auto vertical scroll (multiline only).
    pub auto_vscroll: bool,
}

/// Text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    /// Left-aligned (default).
    #[default]
    Left,
    /// Center-aligned.
    Center,
    /// Right-aligned.
    Right,
}

/// A Windows edit (text box) control.
pub struct Edit;

impl Edit {
    /// Creates a new edit control.
    pub fn new(
        parent: HWND,
        text: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        id: u16,
        style: EditStyle,
    ) -> Result<Control> {
        init_common_controls()?;

        let text_wide = WideString::new(text);
        let class_wide = WideString::new("EDIT");

        let mut win_style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER;

        if style.multiline {
            win_style |= WINDOW_STYLE(ES_MULTILINE as u32);
        }
        if style.password {
            win_style |= WINDOW_STYLE(ES_PASSWORD as u32);
        }
        if style.readonly {
            win_style |= WINDOW_STYLE(ES_READONLY as u32);
        }
        if style.number {
            win_style |= WINDOW_STYLE(ES_NUMBER as u32);
        }
        if style.auto_hscroll {
            win_style |= WINDOW_STYLE(ES_AUTOHSCROLL as u32);
        }
        if style.auto_vscroll {
            win_style |= WINDOW_STYLE(ES_AUTOVSCROLL as u32);
        }

        win_style |= WINDOW_STYLE(match style.align {
            TextAlign::Left => ES_LEFT as u32,
            TextAlign::Center => ES_CENTER as u32,
            TextAlign::Right => ES_RIGHT as u32,
        });

        // SAFETY: CreateWindowExW is safe with valid parameters
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_CLIENTEDGE,
                class_wide.as_pcwstr(),
                text_wide.as_pcwstr(),
                win_style,
                x,
                y,
                width,
                height,
                parent,
                HMENU(id as isize as *mut _),
                HINSTANCE::default(),
                None,
            )?
        };

        Ok(unsafe { Control::from_raw(hwnd, true) })
    }

    /// Sets the maximum text length.
    pub fn set_limit(control: &Control, max_chars: usize) {
        // SAFETY: EM_LIMITTEXT is safe
        unsafe {
            SendMessageW(control.hwnd(), EM_LIMITTEXT, WPARAM(max_chars), LPARAM(0));
        }
    }

    /// Sets the read-only state.
    pub fn set_readonly(control: &Control, readonly: bool) {
        // SAFETY: EM_SETREADONLY is safe
        unsafe {
            SendMessageW(
                control.hwnd(),
                EM_SETREADONLY,
                WPARAM(readonly as usize),
                LPARAM(0),
            );
        }
    }

    /// Selects all text.
    pub fn select_all(control: &Control) {
        // SAFETY: EM_SETSEL is safe
        unsafe {
            SendMessageW(control.hwnd(), EM_SETSEL, WPARAM(0), LPARAM(-1));
        }
    }

    /// Gets the current selection range.
    pub fn selection(control: &Control) -> (u32, u32) {
        let mut start = 0u32;
        let mut end = 0u32;
        // SAFETY: EM_GETSEL is safe
        unsafe {
            SendMessageW(
                control.hwnd(),
                EM_GETSEL,
                WPARAM(&mut start as *mut _ as usize),
                LPARAM(&mut end as *mut _ as isize),
            );
        }
        (start, end)
    }

    /// Replaces the current selection with text.
    pub fn replace_selection(control: &Control, text: &str) {
        let wide = WideString::new(text);
        // SAFETY: EM_REPLACESEL is safe
        unsafe {
            SendMessageW(
                control.hwnd(),
                EM_REPLACESEL,
                WPARAM(1), // Can be undone
                LPARAM(wide.as_ptr() as isize),
            );
        }
    }
}

/// A Windows static label control.
pub struct Label;

impl Label {
    /// Creates a new static label.
    pub fn new(
        parent: HWND,
        text: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        id: u16,
    ) -> Result<Control> {
        init_common_controls()?;

        let text_wide = WideString::new(text);
        let class_wide = WideString::new("STATIC");

        // SAFETY: CreateWindowExW is safe with valid parameters
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_wide.as_pcwstr(),
                text_wide.as_pcwstr(),
                WS_CHILD | WS_VISIBLE,
                x,
                y,
                width,
                height,
                parent,
                HMENU(id as isize as *mut _),
                HINSTANCE::default(),
                None,
            )?
        };

        Ok(unsafe { Control::from_raw(hwnd, true) })
    }
}

/// Progress bar style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressStyle {
    /// Standard segmented progress bar.
    #[default]
    Standard,
    /// Smooth continuous progress bar.
    Smooth,
    /// Marquee (indeterminate) progress bar.
    Marquee,
}

/// A Windows progress bar control.
pub struct ProgressBar;

impl ProgressBar {
    /// Creates a new progress bar.
    pub fn new(
        parent: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        id: u16,
        style: ProgressStyle,
    ) -> Result<Control> {
        init_common_controls()?;

        let mut win_style = WS_CHILD | WS_VISIBLE;
        match style {
            ProgressStyle::Standard => {}
            ProgressStyle::Smooth => win_style |= WINDOW_STYLE(PBS_SMOOTH),
            ProgressStyle::Marquee => win_style |= WINDOW_STYLE(PBS_MARQUEE),
        }

        // SAFETY: CreateWindowExW is safe with valid parameters
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PROGRESS_CLASSW,
                None,
                win_style,
                x,
                y,
                width,
                height,
                parent,
                HMENU(id as isize as *mut _),
                HINSTANCE::default(),
                None,
            )?
        };

        Ok(unsafe { Control::from_raw(hwnd, true) })
    }

    /// Sets the range of the progress bar.
    pub fn set_range(control: &Control, min: i32, max: i32) {
        // SAFETY: PBM_SETRANGE32 is safe
        unsafe {
            SendMessageW(
                control.hwnd(),
                PBM_SETRANGE32,
                WPARAM(min as usize),
                LPARAM(max as isize),
            );
        }
    }

    /// Sets the current position.
    pub fn set_pos(control: &Control, pos: i32) {
        // SAFETY: PBM_SETPOS is safe
        unsafe {
            SendMessageW(control.hwnd(), PBM_SETPOS, WPARAM(pos as usize), LPARAM(0));
        }
    }

    /// Gets the current position.
    pub fn pos(control: &Control) -> i32 {
        // SAFETY: PBM_GETPOS is safe
        unsafe { SendMessageW(control.hwnd(), PBM_GETPOS, WPARAM(0), LPARAM(0)).0 as i32 }
    }

    /// Advances the position by the step amount.
    pub fn step(control: &Control) {
        // SAFETY: PBM_STEPIT is safe
        unsafe {
            SendMessageW(control.hwnd(), PBM_STEPIT, WPARAM(0), LPARAM(0));
        }
    }

    /// Sets the step increment.
    pub fn set_step(control: &Control, step: i32) {
        // SAFETY: PBM_SETSTEP is safe
        unsafe {
            SendMessageW(control.hwnd(), PBM_SETSTEP, WPARAM(step as usize), LPARAM(0));
        }
    }

    /// Advances by a delta amount.
    pub fn advance(control: &Control, delta: i32) {
        // SAFETY: PBM_DELTAPOS is safe
        unsafe {
            SendMessageW(
                control.hwnd(),
                PBM_DELTAPOS,
                WPARAM(delta as usize),
                LPARAM(0),
            );
        }
    }

    /// Starts or stops marquee animation.
    pub fn set_marquee(control: &Control, enable: bool, interval_ms: u32) {
        // SAFETY: PBM_SETMARQUEE is safe
        unsafe {
            SendMessageW(
                control.hwnd(),
                PBM_SETMARQUEE,
                WPARAM(enable as usize),
                LPARAM(interval_ms as isize),
            );
        }
    }
}

/// A Windows list box control.
pub struct ListBox;

impl ListBox {
    /// Creates a new list box.
    pub fn new(
        parent: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        id: u16,
        multi_select: bool,
    ) -> Result<Control> {
        init_common_controls()?;

        let class_wide = WideString::new("LISTBOX");

        let mut win_style = WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP;
        win_style |= WINDOW_STYLE(0x00200000); // LBS_HASSTRINGS
        win_style |= WINDOW_STYLE(0x00000001); // LBS_NOTIFY

        if multi_select {
            win_style |= WINDOW_STYLE(0x00000008); // LBS_MULTIPLESEL
        }

        // SAFETY: CreateWindowExW is safe with valid parameters
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_CLIENTEDGE,
                class_wide.as_pcwstr(),
                None,
                win_style,
                x,
                y,
                width,
                height,
                parent,
                HMENU(id as isize as *mut _),
                HINSTANCE::default(),
                None,
            )?
        };

        Ok(unsafe { Control::from_raw(hwnd, true) })
    }

    /// Adds a string to the list box.
    pub fn add_string(control: &Control, text: &str) -> i32 {
        let wide = WideString::new(text);
        // SAFETY: LB_ADDSTRING is safe
        unsafe {
            SendMessageW(
                control.hwnd(),
                LB_ADDSTRING,
                WPARAM(0),
                LPARAM(wide.as_ptr() as isize),
            )
            .0 as i32
        }
    }

    /// Inserts a string at a specific index.
    pub fn insert_string(control: &Control, index: i32, text: &str) -> i32 {
        let wide = WideString::new(text);
        // SAFETY: LB_INSERTSTRING is safe
        unsafe {
            SendMessageW(
                control.hwnd(),
                LB_INSERTSTRING,
                WPARAM(index as usize),
                LPARAM(wide.as_ptr() as isize),
            )
            .0 as i32
        }
    }

    /// Removes a string at a specific index.
    pub fn delete_string(control: &Control, index: i32) {
        // SAFETY: LB_DELETESTRING is safe
        unsafe {
            SendMessageW(
                control.hwnd(),
                LB_DELETESTRING,
                WPARAM(index as usize),
                LPARAM(0),
            );
        }
    }

    /// Gets the number of items.
    pub fn count(control: &Control) -> i32 {
        // SAFETY: LB_GETCOUNT is safe
        unsafe { SendMessageW(control.hwnd(), LB_GETCOUNT, WPARAM(0), LPARAM(0)).0 as i32 }
    }

    /// Gets the currently selected index (-1 if none).
    pub fn selected_index(control: &Control) -> i32 {
        // SAFETY: LB_GETCURSEL is safe
        unsafe { SendMessageW(control.hwnd(), LB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as i32 }
    }

    /// Sets the selected index.
    pub fn set_selected_index(control: &Control, index: i32) {
        // SAFETY: LB_SETCURSEL is safe
        unsafe {
            SendMessageW(
                control.hwnd(),
                LB_SETCURSEL,
                WPARAM(index as usize),
                LPARAM(0),
            );
        }
    }

    /// Clears all items.
    pub fn clear(control: &Control) {
        // SAFETY: LB_RESETCONTENT is safe
        unsafe {
            SendMessageW(control.hwnd(), LB_RESETCONTENT, WPARAM(0), LPARAM(0));
        }
    }
}

/// A Windows combo box control.
pub struct ComboBox;

impl ComboBox {
    /// Creates a new combo box.
    pub fn new(
        parent: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        id: u16,
        dropdown: bool,
    ) -> Result<Control> {
        init_common_controls()?;

        let class_wide = WideString::new("COMBOBOX");

        let mut win_style = WS_CHILD | WS_VISIBLE | WS_TABSTOP;
        win_style |= WINDOW_STYLE(0x00000200); // CBS_HASSTRINGS

        if dropdown {
            win_style |= WINDOW_STYLE(0x0003); // CBS_DROPDOWNLIST
        } else {
            win_style |= WINDOW_STYLE(0x0002); // CBS_DROPDOWN
        }

        // SAFETY: CreateWindowExW is safe with valid parameters
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_wide.as_pcwstr(),
                None,
                win_style,
                x,
                y,
                width,
                height,
                parent,
                HMENU(id as isize as *mut _),
                HINSTANCE::default(),
                None,
            )?
        };

        Ok(unsafe { Control::from_raw(hwnd, true) })
    }

    /// Adds a string to the combo box.
    pub fn add_string(control: &Control, text: &str) -> i32 {
        let wide = WideString::new(text);
        // SAFETY: CB_ADDSTRING is safe
        unsafe {
            SendMessageW(
                control.hwnd(),
                CB_ADDSTRING,
                WPARAM(0),
                LPARAM(wide.as_ptr() as isize),
            )
            .0 as i32
        }
    }

    /// Gets the number of items.
    pub fn count(control: &Control) -> i32 {
        // SAFETY: CB_GETCOUNT is safe
        unsafe { SendMessageW(control.hwnd(), CB_GETCOUNT, WPARAM(0), LPARAM(0)).0 as i32 }
    }

    /// Gets the currently selected index (-1 if none).
    pub fn selected_index(control: &Control) -> i32 {
        // SAFETY: CB_GETCURSEL is safe
        unsafe { SendMessageW(control.hwnd(), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as i32 }
    }

    /// Sets the selected index.
    pub fn set_selected_index(control: &Control, index: i32) {
        // SAFETY: CB_SETCURSEL is safe
        unsafe {
            SendMessageW(
                control.hwnd(),
                CB_SETCURSEL,
                WPARAM(index as usize),
                LPARAM(0),
            );
        }
    }

    /// Clears all items.
    pub fn clear(control: &Control) {
        // SAFETY: CB_RESETCONTENT is safe
        unsafe {
            SendMessageW(control.hwnd(), CB_RESETCONTENT, WPARAM(0), LPARAM(0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_common_controls() {
        assert!(init_common_controls().is_ok());
    }

    #[test]
    fn test_button_style() {
        assert_eq!(ButtonStyle::Push.to_style(), BS_PUSHBUTTON as u32);
        assert_eq!(ButtonStyle::Checkbox.to_style(), BS_CHECKBOX as u32);
    }

    #[test]
    fn test_edit_style_default() {
        let style = EditStyle::default();
        assert!(!style.multiline);
        assert!(!style.password);
        assert_eq!(style.align, TextAlign::Left);
    }
}
