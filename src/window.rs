//! Window creation and message handling utilities.
//!
//! Provides ergonomic wrappers for creating windows and handling Windows messages.

use crate::error::Result;
use crate::string::WideString;
use std::cell::RefCell;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, HBRUSH, WHITE_BRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, LoadCursorW, PostQuitMessage, RegisterClassExW, SetWindowLongPtrW,
    ShowWindow, TranslateMessage, UnregisterClassW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
    GWLP_USERDATA, IDC_ARROW, MSG, SW_HIDE, SW_SHOW, SW_SHOWDEFAULT, WM_CLOSE, WM_CREATE,
    WM_DESTROY, WM_NCCREATE, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSEXW, WS_CAPTION,
    WS_OVERLAPPEDWINDOW, WS_SYSMENU, WS_VISIBLE,
};

/// Window styles for creating windows.
#[derive(Clone, Copy, Debug)]
pub struct Style(pub WINDOW_STYLE);

impl Style {
    /// A standard overlapped window with title bar, border, and system menu.
    pub const OVERLAPPED: Self = Self(WS_OVERLAPPEDWINDOW);

    /// A window with a caption.
    pub const CAPTION: Self = Self(WS_CAPTION);

    /// A window with a system menu.
    pub const SYSMENU: Self = Self(WS_SYSMENU);

    /// A visible window.
    pub const VISIBLE: Self = Self(WS_VISIBLE);

    /// Combines two styles.
    pub fn with(self, other: Self) -> Self {
        Self(WINDOW_STYLE(self.0 .0 | other.0 .0))
    }
}

/// Extended window styles.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExStyle(pub WINDOW_EX_STYLE);

impl ExStyle {
    /// No extended styles.
    pub const NONE: Self = Self(WINDOW_EX_STYLE(0));

    /// Combines two extended styles.
    pub fn with(self, other: Self) -> Self {
        Self(WINDOW_EX_STYLE(self.0 .0 | other.0 .0))
    }
}

/// Show window commands.
#[derive(Clone, Copy, Debug)]
pub struct ShowCommand(pub windows::Win32::UI::WindowsAndMessaging::SHOW_WINDOW_CMD);

impl ShowCommand {
    /// Shows the window in its default state.
    pub const DEFAULT: Self = Self(SW_SHOWDEFAULT);

    /// Shows the window normally.
    pub const SHOW: Self = Self(SW_SHOW);

    /// Hides the window.
    pub const HIDE: Self = Self(SW_HIDE);
}

/// A Windows message.
#[derive(Clone, Copy, Debug)]
pub struct Message {
    /// The window handle.
    pub hwnd: HWND,
    /// The message identifier.
    pub msg: u32,
    /// Additional message information.
    pub wparam: WPARAM,
    /// Additional message information.
    pub lparam: LPARAM,
}

impl Message {
    /// WM_CREATE message.
    pub const CREATE: u32 = WM_CREATE;
    /// WM_DESTROY message.
    pub const DESTROY: u32 = WM_DESTROY;
    /// WM_CLOSE message.
    pub const CLOSE: u32 = WM_CLOSE;
}

/// Trait for handling window messages.
pub trait MessageHandler {
    /// Handles a window message.
    ///
    /// Return `Some(result)` to indicate the message was handled, or `None` to
    /// let the default window procedure handle it.
    fn handle_message(&mut self, msg: Message) -> Option<LRESULT>;

    /// Called when the window is created.
    fn on_create(&mut self, _hwnd: HWND) -> bool {
        true
    }

    /// Called when the window is about to be destroyed.
    fn on_destroy(&mut self) {}

    /// Called when the window receives a close request.
    fn on_close(&mut self, hwnd: HWND) -> bool {
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
        true
    }
}

/// A default message handler that does nothing.
pub struct DefaultHandler;

impl MessageHandler for DefaultHandler {
    fn handle_message(&mut self, _msg: Message) -> Option<LRESULT> {
        None
    }
}

/// Builder for creating windows.
pub struct WindowBuilder {
    class_name: String,
    title: String,
    style: Style,
    ex_style: ExStyle,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Default for WindowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowBuilder {
    /// Creates a new window builder with default settings.
    pub fn new() -> Self {
        Self {
            class_name: String::new(),
            title: String::from("Window"),
            style: Style::OVERLAPPED,
            ex_style: ExStyle::NONE,
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
            width: CW_USEDEFAULT,
            height: CW_USEDEFAULT,
        }
    }

    /// Sets the window class name.
    pub fn class_name(mut self, name: impl Into<String>) -> Self {
        self.class_name = name.into();
        self
    }

    /// Sets the window title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the window style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the extended window style.
    pub fn ex_style(mut self, ex_style: ExStyle) -> Self {
        self.ex_style = ex_style;
        self
    }

    /// Sets the window position.
    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// Sets the window size.
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Creates the window.
    ///
    /// # Errors
    ///
    /// Returns an error if window class registration or window creation fails.
    pub fn build<H: MessageHandler + 'static>(self, handler: H) -> Result<Window<H>> {
        let class_name = if self.class_name.is_empty() {
            format!("ErgonomicWindow_{}", std::process::id())
        } else {
            self.class_name
        };

        // Register the window class
        let class_name_wide = WideString::new(&class_name);
        // SAFETY: GetModuleHandleW(None) returns the handle of the current executable.
        // It always succeeds and doesn't need to be freed.
        let hinstance = unsafe { GetModuleHandleW(None)? };

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc::<H>),
            hInstance: hinstance.into(),
            // SAFETY: LoadCursorW with None and IDC_ARROW loads a system cursor.
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
            // SAFETY: GetStockObject(WHITE_BRUSH) returns a system brush handle.
            hbrBackground: unsafe { HBRUSH(GetStockObject(WHITE_BRUSH).0) },
            lpszClassName: class_name_wide.as_pcwstr(),
            ..Default::default()
        };

        // SAFETY: wc is a properly initialized WNDCLASSEXW struct.
        let atom = unsafe { RegisterClassExW(&wc) };
        if atom == 0 {
            return Err(crate::error::last_error());
        }

        // Box the handler and convert to raw pointer.
        // We'll reconstruct and drop this in Window::drop.
        let handler = Box::new(RefCell::new(handler));
        let handler_ptr = Box::into_raw(handler);

        // Create the window
        let title_wide = WideString::new(&self.title);
        // SAFETY: All string parameters are valid null-terminated wide strings.
        // handler_ptr is passed via lpParam and will be stored in GWLP_USERDATA during WM_NCCREATE.
        let hwnd = unsafe {
            CreateWindowExW(
                self.ex_style.0,
                class_name_wide.as_pcwstr(),
                title_wide.as_pcwstr(),
                self.style.0,
                self.x,
                self.y,
                self.width,
                self.height,
                None,
                None,
                hinstance,
                Some(handler_ptr as *const _),
            )?
        };

        Ok(Window {
            hwnd,
            class_name: class_name_wide,
            handler: handler_ptr,
            hinstance,
        })
    }
}

/// A Windows window.
pub struct Window<H: MessageHandler> {
    hwnd: HWND,
    class_name: WideString,
    handler: *mut RefCell<H>,
    hinstance: windows::Win32::Foundation::HMODULE,
}

impl<H: MessageHandler> Window<H> {
    /// Returns the window handle.
    #[inline]
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Shows the window.
    pub fn show(&self, cmd: ShowCommand) {
        // SAFETY: self.hwnd is a valid window handle created by CreateWindowExW.
        // ShowWindow is safe to call on any valid window handle.
        unsafe {
            let _ = ShowWindow(self.hwnd, cmd.0);
        }
    }

    /// Gets a mutable reference to the message handler.
    ///
    /// # Panics
    ///
    /// Panics if the handler is already borrowed (e.g., during message handling).
    pub fn handler_mut(&self) -> std::cell::RefMut<'_, H> {
        // SAFETY: self.handler is a valid pointer to a Box<RefCell<H>> that we created
        // in WindowBuilder::build. The pointer remains valid until Window is dropped.
        // RefCell provides runtime borrow checking.
        unsafe { (*self.handler).borrow_mut() }
    }

    /// Gets a reference to the message handler.
    ///
    /// # Panics
    ///
    /// Panics if the handler is already mutably borrowed (e.g., during message handling).
    pub fn handler(&self) -> std::cell::Ref<'_, H> {
        // SAFETY: self.handler is a valid pointer to a Box<RefCell<H>> that we created
        // in WindowBuilder::build. The pointer remains valid until Window is dropped.
        // RefCell provides runtime borrow checking.
        unsafe { (*self.handler).borrow() }
    }

    /// Destroys the window.
    ///
    /// This is equivalent to dropping the window.
    pub fn destroy(self) {
        // Drop will handle cleanup
    }
}

impl<H: MessageHandler> Drop for Window<H> {
    fn drop(&mut self) {
        // SAFETY: We're being dropped, so we have exclusive ownership.
        // - self.hwnd is a valid window handle we created
        // - self.class_name contains the class we registered
        // - self.handler is a valid Box pointer we created via Box::into_raw
        unsafe {
            let _ = DestroyWindow(self.hwnd);
            let _ = UnregisterClassW(self.class_name.as_pcwstr(), self.hinstance);
            // Reconstruct the Box and drop it to free the memory
            drop(Box::from_raw(self.handler));
        }
    }
}

/// The window procedure that forwards messages to the handler.
///
/// # Safety
///
/// This function is called by Windows as a callback. It must be marked `unsafe extern "system"`
/// to match the Windows calling convention. The safety of this function relies on:
/// - Windows calling it with valid parameters for the registered window class
/// - The handler pointer stored in GWLP_USERDATA being valid (set in WM_NCCREATE)
/// - The handler not being dropped while messages are being processed
unsafe extern "system" fn window_proc<H: MessageHandler>(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Get the handler from the window's user data
    let handler_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut RefCell<H>;

    // Handle WM_NCCREATE to set up the handler pointer.
    // This is the first message sent to a window, before WM_CREATE.
    if msg == WM_NCCREATE {
        // SAFETY: During WM_NCCREATE, lparam points to a CREATESTRUCTW.
        // lpCreateParams contains the pointer we passed to CreateWindowExW.
        let create_struct =
            &*(lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW);
        let handler_ptr = create_struct.lpCreateParams as *mut RefCell<H>;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, handler_ptr as isize);
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    // If we don't have a handler yet (shouldn't happen after WM_NCCREATE), use default handling.
    if handler_ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    // SAFETY: handler_ptr was set in WM_NCCREATE from a valid Box<RefCell<H>>.
    // The Window struct ensures the handler outlives the window.
    let handler = &*handler_ptr;
    let message = Message {
        hwnd,
        msg,
        wparam,
        lparam,
    };

    // Handle special messages
    match msg {
        WM_CREATE => {
            let mut handler = handler.borrow_mut();
            if handler.on_create(hwnd) {
                LRESULT(0)
            } else {
                LRESULT(-1)
            }
        }
        WM_DESTROY => {
            handler.borrow_mut().on_destroy();
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_CLOSE => {
            let mut handler = handler.borrow_mut();
            let _ = handler.on_close(hwnd);
            LRESULT(0)
        }
        _ => {
            let mut handler = handler.borrow_mut();
            if let Some(result) = handler.handle_message(message) {
                result
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
    }
}

/// Runs the message loop until WM_QUIT is received.
///
/// This function blocks until the application receives a WM_QUIT message,
/// typically sent by calling `PostQuitMessage`.
///
/// # Returns
///
/// The exit code passed to `PostQuitMessage`.
pub fn run_message_loop() -> i32 {
    let mut msg = MSG::default();

    // SAFETY: GetMessageW, TranslateMessage, and DispatchMessageW are safe to call.
    // - msg is a valid stack-allocated MSG struct
    // - None for hwnd means we get messages for all windows on this thread
    // - 0, 0 for filter range means we get all message types
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    msg.wParam.0 as i32
}

/// Processes pending messages without blocking.
///
/// Call this in a loop if you need to do other work while processing messages.
///
/// # Returns
///
/// Returns `true` if a WM_QUIT message was received, indicating the application should exit.
pub fn process_messages() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{PeekMessageW, PM_REMOVE, WM_QUIT};

    let mut msg = MSG::default();

    // SAFETY: PeekMessageW, TranslateMessage, and DispatchMessageW are safe to call.
    // - msg is a valid stack-allocated MSG struct
    // - PM_REMOVE removes messages from the queue after reading
    unsafe {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_QUIT {
                return true;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    false
}
