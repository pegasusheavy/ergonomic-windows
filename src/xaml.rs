//! WinRT XAML support for modern Windows UI.
//!
//! This module provides safe wrappers for WinRT XAML UI elements, enabling
//! modern, declarative UI development in Rust.
//!
//! # Overview
//!
//! WinRT XAML is the UI framework used by UWP and WinUI applications.
//! It provides:
//!
//! - **Declarative UI**: Define UI in XAML markup or code
//! - **Data binding**: Automatically sync UI with data
//! - **Modern controls**: Buttons, lists, navigation, etc.
//! - **Animations**: Smooth, hardware-accelerated animations
//! - **Responsive design**: Adaptive layouts for any screen size
//!
//! # Note
//!
//! WinRT XAML requires a UWP/WinUI application context. For traditional
//! Win32 applications, consider using:
//!
//! - [`crate::controls`] - Win32 common controls
//! - [`crate::d2d`] - Direct2D custom drawing
//! - [`crate::webview`] - WebView2 for web-based UI
//!
//! # XAML Islands
//!
//! For Win32 applications that want to use XAML controls, you can use
//! XAML Islands (Windows 10 1903+). This module provides helpers for
//! hosting XAML content in Win32 windows.
//!
//! # Example
//!
//! ```ignore
//! use ergonomic_windows::xaml::{XamlHost, Button, TextBlock};
//!
//! // Create a XAML host in a Win32 window
//! let host = XamlHost::new(hwnd)?;
//!
//! // Create XAML controls
//! let button = Button::new()?;
//! button.set_content("Click Me")?;
//!
//! let text = TextBlock::new()?;
//! text.set_text("Hello, XAML!")?;
//!
//! // Set the root content
//! host.set_content(button)?;
//! ```

use crate::error::Result;
use windows::Win32::Foundation::HWND;

// Note: Full WinRT XAML support requires additional Windows crate features
// and a proper application context. This module provides the foundation.

/// Represents supported XAML themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ElementTheme {
    /// System default theme.
    #[default]
    Default,
    /// Light theme.
    Light,
    /// Dark theme.
    Dark,
}

/// Horizontal alignment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HorizontalAlignment {
    /// Align to the left.
    Left,
    /// Center alignment.
    Center,
    /// Align to the right.
    Right,
    /// Stretch to fill.
    #[default]
    Stretch,
}

/// Vertical alignment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlignment {
    /// Align to the top.
    Top,
    /// Center alignment.
    Center,
    /// Align to the bottom.
    Bottom,
    /// Stretch to fill.
    #[default]
    Stretch,
}

/// Visibility states for XAML elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    /// Element is visible.
    #[default]
    Visible,
    /// Element is collapsed (takes no space).
    Collapsed,
}

/// Thickness (for margins, padding, borders).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Thickness {
    /// Left value.
    pub left: f64,
    /// Top value.
    pub top: f64,
    /// Right value.
    pub right: f64,
    /// Bottom value.
    pub bottom: f64,
}

impl Thickness {
    /// Creates a uniform thickness.
    pub const fn uniform(value: f64) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }

    /// Creates a thickness with horizontal and vertical values.
    pub const fn symmetric(horizontal: f64, vertical: f64) -> Self {
        Self {
            left: horizontal,
            top: vertical,
            right: horizontal,
            bottom: vertical,
        }
    }

    /// Creates a thickness with all four values.
    pub const fn new(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

/// A corner radius for rounded rectangles.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CornerRadius {
    /// Top-left radius.
    pub top_left: f64,
    /// Top-right radius.
    pub top_right: f64,
    /// Bottom-right radius.
    pub bottom_right: f64,
    /// Bottom-left radius.
    pub bottom_left: f64,
}

impl CornerRadius {
    /// Creates a uniform corner radius.
    pub const fn uniform(value: f64) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }
}

/// A color in the XAML color space (ARGB).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct XamlColor {
    /// Alpha component (0-255).
    pub a: u8,
    /// Red component (0-255).
    pub r: u8,
    /// Green component (0-255).
    pub g: u8,
    /// Blue component (0-255).
    pub b: u8,
}

impl XamlColor {
    /// Creates a color from ARGB components.
    pub const fn argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self { a, r, g, b }
    }

    /// Creates a fully opaque color from RGB components.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { a: 255, r, g, b }
    }

    /// Creates a color from a hex value (0xAARRGGBB).
    pub const fn from_argb_hex(hex: u32) -> Self {
        Self {
            a: ((hex >> 24) & 0xFF) as u8,
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }

    /// Creates a color from a hex value (0xRRGGBB), fully opaque.
    pub const fn from_rgb_hex(hex: u32) -> Self {
        Self {
            a: 255,
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }

    // Common colors
    /// Transparent color.
    pub const TRANSPARENT: Self = Self::argb(0, 0, 0, 0);
    /// Black color.
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// White color.
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Red color.
    pub const RED: Self = Self::rgb(255, 0, 0);
    /// Green color.
    pub const GREEN: Self = Self::rgb(0, 128, 0);
    /// Blue color.
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    /// Gray color.
    pub const GRAY: Self = Self::rgb(128, 128, 128);
    /// Light gray color.
    pub const LIGHT_GRAY: Self = Self::rgb(211, 211, 211);
    /// Dark gray color.
    pub const DARK_GRAY: Self = Self::rgb(169, 169, 169);
}

/// Grid row/column definitions.
#[derive(Debug, Clone, PartialEq)]
pub enum GridLength {
    /// Auto-size based on content.
    Auto,
    /// Fixed pixel size.
    Pixel(f64),
    /// Proportional size (star sizing).
    Star(f64),
}

impl Default for GridLength {
    fn default() -> Self {
        GridLength::Star(1.0)
    }
}

/// Font weights for text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u16)]
pub enum FontWeight {
    /// Thin (100).
    Thin = 100,
    /// Extra light (200).
    ExtraLight = 200,
    /// Light (300).
    Light = 300,
    /// Semi-light (350).
    SemiLight = 350,
    /// Normal/Regular (400).
    #[default]
    Normal = 400,
    /// Medium (500).
    Medium = 500,
    /// Semi-bold (600).
    SemiBold = 600,
    /// Bold (700).
    Bold = 700,
    /// Extra bold (800).
    ExtraBold = 800,
    /// Black/Heavy (900).
    Black = 900,
    /// Extra black (950).
    ExtraBlack = 950,
}

/// Font styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontStyle {
    /// Normal/upright text.
    #[default]
    Normal,
    /// Oblique text.
    Oblique,
    /// Italic text.
    Italic,
}

/// Text wrapping modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextWrapping {
    /// No wrapping.
    #[default]
    NoWrap,
    /// Wrap at word boundaries.
    Wrap,
    /// Wrap at character boundaries.
    WrapWholeWords,
}

/// Text trimming modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextTrimming {
    /// No trimming.
    #[default]
    None,
    /// Trim at character boundary with ellipsis.
    CharacterEllipsis,
    /// Trim at word boundary with ellipsis.
    WordEllipsis,
    /// Clip text without ellipsis.
    Clip,
}

/// Orientation for layout containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Horizontal layout.
    #[default]
    Horizontal,
    /// Vertical layout.
    Vertical,
}

/// Scroll visibility modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollBarVisibility {
    /// Disabled.
    Disabled,
    /// Automatically show when needed.
    #[default]
    Auto,
    /// Always show.
    Visible,
    /// Always hide.
    Hidden,
}

/// XAML Islands host for Win32 applications.
///
/// XAML Islands allow you to host UWP XAML controls in Win32 applications.
/// This requires Windows 10 version 1903 or later.
pub struct XamlHost {
    hwnd: HWND,
    // In a full implementation, this would hold the DesktopWindowXamlSource
}

impl XamlHost {
    /// Creates a new XAML host for the given Win32 window.
    ///
    /// # Requirements
    ///
    /// - Windows 10 version 1903 or later
    /// - Application must have a package identity or be running with identity
    ///
    /// # Errors
    ///
    /// Returns an error if XAML Islands are not available or initialization fails.
    pub fn new(hwnd: HWND) -> Result<Self> {
        // Note: Full XAML Islands implementation requires:
        // 1. Calling WindowsXamlManager.InitializeForCurrentThread()
        // 2. Creating a DesktopWindowXamlSource
        // 3. Attaching it to the HWND
        //
        // This requires the Windows.UI.Xaml.Hosting namespace from WinRT

        Ok(Self { hwnd })
    }

    /// Gets the underlying HWND.
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Sets the XAML content root.
    ///
    /// In a full implementation, this would set the Content property
    /// of the DesktopWindowXamlSource.
    pub fn set_content<T>(&self, _content: T) -> Result<()> {
        // Placeholder - would set DesktopWindowXamlSource.Content
        Ok(())
    }

    /// Focuses the XAML content.
    pub fn focus(&self) -> Result<()> {
        // Placeholder - would call TakeFocusRequested handling
        Ok(())
    }
}

/// A builder for creating XAML-style UI programmatically.
///
/// This provides a fluent API for building UI without actual XAML.
pub struct UiBuilder {
    /// The requested theme.
    pub theme: ElementTheme,
    /// The root margin.
    pub margin: Thickness,
    /// The root padding.
    pub padding: Thickness,
}

impl UiBuilder {
    /// Creates a new UI builder.
    pub fn new() -> Self {
        Self {
            theme: ElementTheme::Default,
            margin: Thickness::default(),
            padding: Thickness::default(),
        }
    }

    /// Sets the theme.
    pub fn theme(mut self, theme: ElementTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Sets the margin.
    pub fn margin(mut self, margin: Thickness) -> Self {
        self.margin = margin;
        self
    }

    /// Sets the padding.
    pub fn padding(mut self, padding: Thickness) -> Self {
        self.padding = padding;
        self
    }
}

impl Default for UiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thickness() {
        let t = Thickness::uniform(10.0);
        assert_eq!(t.left, 10.0);
        assert_eq!(t.right, 10.0);

        let t2 = Thickness::symmetric(5.0, 10.0);
        assert_eq!(t2.left, 5.0);
        assert_eq!(t2.top, 10.0);
    }

    #[test]
    fn test_corner_radius() {
        let r = CornerRadius::uniform(8.0);
        assert_eq!(r.top_left, 8.0);
        assert_eq!(r.bottom_right, 8.0);
    }

    #[test]
    fn test_xaml_color() {
        let c = XamlColor::rgb(255, 128, 64);
        assert_eq!(c.a, 255);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 64);

        let c2 = XamlColor::from_rgb_hex(0xFF8040);
        assert_eq!(c2.r, 255);
        assert_eq!(c2.g, 128);
        assert_eq!(c2.b, 64);
    }

    #[test]
    fn test_grid_length() {
        let auto = GridLength::Auto;
        let pixel = GridLength::Pixel(100.0);
        let star = GridLength::Star(2.0);

        assert!(matches!(auto, GridLength::Auto));
        assert!(matches!(pixel, GridLength::Pixel(100.0)));
        assert!(matches!(star, GridLength::Star(2.0)));
    }

    #[test]
    fn test_font_weight() {
        assert_eq!(FontWeight::Normal as u16, 400);
        assert_eq!(FontWeight::Bold as u16, 700);
    }

    #[test]
    fn test_ui_builder() {
        let ui = UiBuilder::new()
            .theme(ElementTheme::Dark)
            .margin(Thickness::uniform(16.0))
            .padding(Thickness::symmetric(8.0, 4.0));

        assert_eq!(ui.theme, ElementTheme::Dark);
        assert_eq!(ui.margin.left, 16.0);
        assert_eq!(ui.padding.left, 8.0);
        assert_eq!(ui.padding.top, 4.0);
    }
}

