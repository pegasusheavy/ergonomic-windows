//! Direct2D and DirectWrite graphics.
//!
//! Provides safe wrappers for hardware-accelerated 2D graphics rendering
//! and high-quality text rendering using Direct2D and DirectWrite.

#![allow(clippy::too_many_arguments)] // Drawing functions need many coordinate parameters

use crate::error::Result;
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_POINT_2F, D2D_RECT_F,
    D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
    D2D1_BRUSH_PROPERTIES, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_ELLIPSE, D2D1_FACTORY_OPTIONS,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_HWND_RENDER_TARGET_PROPERTIES,
    D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT,
    D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    DWRITE_PARAGRAPH_ALIGNMENT_FAR, DWRITE_PARAGRAPH_ALIGNMENT_NEAR, DWRITE_TEXT_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_JUSTIFIED, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

/// A color with red, green, blue, and alpha components (0.0 - 1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red component (0.0 - 1.0).
    pub r: f32,
    /// Green component (0.0 - 1.0).
    pub g: f32,
    /// Blue component (0.0 - 1.0).
    pub b: f32,
    /// Alpha component (0.0 - 1.0).
    pub a: f32,
}

impl Color {
    /// Creates a new color with the given RGB values and full opacity.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Creates a new color with the given RGBA values.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Creates a color from 8-bit RGB values (0-255).
    pub fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// Creates a color from a hex value (0xRRGGBB).
    pub fn from_hex(hex: u32) -> Self {
        Self::from_rgb8(
            ((hex >> 16) & 0xFF) as u8,
            ((hex >> 8) & 0xFF) as u8,
            (hex & 0xFF) as u8,
        )
    }

    // Common colors
    /// Black color.
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    /// White color.
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    /// Red color.
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    /// Green color.
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    /// Blue color.
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    /// Yellow color.
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
    /// Cyan color.
    pub const CYAN: Self = Self::rgb(0.0, 1.0, 1.0);
    /// Magenta color.
    pub const MAGENTA: Self = Self::rgb(1.0, 0.0, 1.0);
    /// Gray color.
    pub const GRAY: Self = Self::rgb(0.5, 0.5, 0.5);
    /// Transparent color.
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);

    fn as_d2d1(&self) -> D2D1_COLOR_F {
        D2D1_COLOR_F {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

/// Text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlignment {
    /// Left-aligned (leading edge).
    #[default]
    Left,
    /// Right-aligned (trailing edge).
    Right,
    /// Center-aligned.
    Center,
    /// Justified.
    Justified,
}

/// Paragraph alignment (vertical).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParagraphAlignment {
    /// Top-aligned.
    #[default]
    Top,
    /// Bottom-aligned.
    Bottom,
    /// Center-aligned.
    Center,
}

/// The Direct2D factory - entry point for creating D2D resources.
pub struct D2DFactory {
    factory: ID2D1Factory,
}

impl D2DFactory {
    /// Creates a new Direct2D factory.
    pub fn new() -> Result<Self> {
        let options = D2D1_FACTORY_OPTIONS::default();

        // SAFETY: D2D1CreateFactory is safe with valid parameters
        let factory: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, Some(&options))? };

        Ok(Self { factory })
    }

    /// Creates a render target for a window.
    pub fn create_hwnd_render_target(&self, hwnd: HWND) -> Result<RenderTarget> {
        // Get window size
        let mut rect = windows::Win32::Foundation::RECT::default();
        // SAFETY: GetClientRect is safe
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect)?;
        }

        let size = D2D_SIZE_U {
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
        };

        let render_target_properties = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 0.0,
            dpiY: 0.0,
            ..Default::default()
        };

        let hwnd_render_target_properties = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd,
            pixelSize: size,
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };

        // SAFETY: CreateHwndRenderTarget is safe with valid parameters
        let render_target = unsafe {
            self.factory
                .CreateHwndRenderTarget(&render_target_properties, &hwnd_render_target_properties)?
        };

        Ok(RenderTarget {
            target: render_target,
        })
    }
}

/// A Direct2D render target for drawing.
pub struct RenderTarget {
    target: ID2D1HwndRenderTarget,
}

impl RenderTarget {
    /// Resizes the render target to match the window size.
    pub fn resize(&self, width: u32, height: u32) -> Result<()> {
        let size = D2D_SIZE_U { width, height };
        // SAFETY: Resize is safe
        unsafe {
            self.target.Resize(&size)?;
        }
        Ok(())
    }

    /// Begins drawing operations.
    pub fn begin_draw(&self) {
        // SAFETY: BeginDraw is safe
        unsafe {
            self.target.BeginDraw();
        }
    }

    /// Ends drawing operations.
    pub fn end_draw(&self) -> Result<()> {
        // SAFETY: EndDraw is safe
        unsafe {
            self.target.EndDraw(None, None)?;
        }
        Ok(())
    }

    /// Clears the render target with a color.
    pub fn clear(&self, color: Color) {
        // SAFETY: Clear is safe
        unsafe {
            self.target.Clear(Some(&color.as_d2d1()));
        }
    }

    /// Creates a solid color brush.
    pub fn create_solid_brush(&self, color: Color) -> Result<SolidBrush> {
        let props = D2D1_BRUSH_PROPERTIES {
            opacity: 1.0,
            transform: Matrix3x2::identity(),
        };

        // SAFETY: CreateSolidColorBrush is safe
        let brush = unsafe {
            self.target
                .CreateSolidColorBrush(&color.as_d2d1(), Some(&props))?
        };

        Ok(SolidBrush { brush })
    }

    /// Draws a line.
    pub fn draw_line(
        &self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        brush: &SolidBrush,
        stroke_width: f32,
    ) {
        let p1 = D2D_POINT_2F { x: x1, y: y1 };
        let p2 = D2D_POINT_2F { x: x2, y: y2 };

        // SAFETY: DrawLine is safe
        unsafe {
            self.target
                .DrawLine(p1, p2, &brush.brush, stroke_width, None);
        }
    }

    /// Draws a rectangle outline.
    pub fn draw_rect(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        brush: &SolidBrush,
        stroke_width: f32,
    ) {
        let rect = D2D_RECT_F {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        };

        // SAFETY: DrawRectangle is safe
        unsafe {
            self.target
                .DrawRectangle(&rect, &brush.brush, stroke_width, None);
        }
    }

    /// Fills a rectangle.
    pub fn fill_rect(&self, x: f32, y: f32, width: f32, height: f32, brush: &SolidBrush) {
        let rect = D2D_RECT_F {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        };

        // SAFETY: FillRectangle is safe
        unsafe {
            self.target.FillRectangle(&rect, &brush.brush);
        }
    }

    /// Draws a rounded rectangle outline.
    pub fn draw_rounded_rect(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius_x: f32,
        radius_y: f32,
        brush: &SolidBrush,
        stroke_width: f32,
    ) {
        let rect = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: x,
                top: y,
                right: x + width,
                bottom: y + height,
            },
            radiusX: radius_x,
            radiusY: radius_y,
        };

        // SAFETY: DrawRoundedRectangle is safe
        unsafe {
            self.target
                .DrawRoundedRectangle(&rect, &brush.brush, stroke_width, None);
        }
    }

    /// Fills a rounded rectangle.
    pub fn fill_rounded_rect(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius_x: f32,
        radius_y: f32,
        brush: &SolidBrush,
    ) {
        let rect = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: x,
                top: y,
                right: x + width,
                bottom: y + height,
            },
            radiusX: radius_x,
            radiusY: radius_y,
        };

        // SAFETY: FillRoundedRectangle is safe
        unsafe {
            self.target.FillRoundedRectangle(&rect, &brush.brush);
        }
    }

    /// Draws an ellipse outline.
    pub fn draw_ellipse(
        &self,
        center_x: f32,
        center_y: f32,
        radius_x: f32,
        radius_y: f32,
        brush: &SolidBrush,
        stroke_width: f32,
    ) {
        let ellipse = D2D1_ELLIPSE {
            point: D2D_POINT_2F {
                x: center_x,
                y: center_y,
            },
            radiusX: radius_x,
            radiusY: radius_y,
        };

        // SAFETY: DrawEllipse is safe
        unsafe {
            self.target
                .DrawEllipse(&ellipse, &brush.brush, stroke_width, None);
        }
    }

    /// Fills an ellipse.
    pub fn fill_ellipse(
        &self,
        center_x: f32,
        center_y: f32,
        radius_x: f32,
        radius_y: f32,
        brush: &SolidBrush,
    ) {
        let ellipse = D2D1_ELLIPSE {
            point: D2D_POINT_2F {
                x: center_x,
                y: center_y,
            },
            radiusX: radius_x,
            radiusY: radius_y,
        };

        // SAFETY: FillEllipse is safe
        unsafe {
            self.target.FillEllipse(&ellipse, &brush.brush);
        }
    }

    /// Draws text using a text format.
    pub fn draw_text(
        &self,
        text: &str,
        format: &TextFormat,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        brush: &SolidBrush,
    ) {
        let wide: Vec<u16> = text.encode_utf16().collect();
        let rect = D2D_RECT_F {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        };

        // SAFETY: DrawText is safe
        unsafe {
            self.target.DrawText(
                &wide,
                &format.format,
                &rect,
                &brush.brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
    }

    /// Gets the size of the render target.
    pub fn size(&self) -> (f32, f32) {
        // SAFETY: GetSize is safe
        let size = unsafe { self.target.GetSize() };
        (size.width, size.height)
    }
}

/// A solid color brush for painting.
pub struct SolidBrush {
    brush: ID2D1SolidColorBrush,
}

impl SolidBrush {
    /// Sets the brush color.
    pub fn set_color(&self, color: Color) {
        // SAFETY: SetColor is safe
        unsafe {
            self.brush.SetColor(&color.as_d2d1());
        }
    }

    /// Gets the current brush color.
    pub fn color(&self) -> Color {
        // SAFETY: GetColor is safe
        let c = unsafe { self.brush.GetColor() };
        Color {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        }
    }

    /// Sets the brush opacity.
    pub fn set_opacity(&self, opacity: f32) {
        // SAFETY: SetOpacity is safe
        unsafe {
            self.brush.SetOpacity(opacity);
        }
    }
}

/// The DirectWrite factory - entry point for creating text resources.
pub struct DWriteFactory {
    factory: IDWriteFactory,
}

impl DWriteFactory {
    /// Creates a new DirectWrite factory.
    pub fn new() -> Result<Self> {
        // SAFETY: DWriteCreateFactory is safe
        let factory: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };

        Ok(Self { factory })
    }

    /// Creates a text format for rendering text.
    pub fn create_text_format(&self, font_family: &str, font_size: f32) -> Result<TextFormat> {
        let font_family_wide: Vec<u16> = font_family
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let locale_wide: Vec<u16> = "en-US".encode_utf16().chain(std::iter::once(0)).collect();

        // SAFETY: CreateTextFormat is safe
        let format = unsafe {
            self.factory.CreateTextFormat(
                windows::core::PCWSTR(font_family_wide.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font_size,
                windows::core::PCWSTR(locale_wide.as_ptr()),
            )?
        };

        Ok(TextFormat { format })
    }
}

/// A text format for controlling text appearance.
pub struct TextFormat {
    format: IDWriteTextFormat,
}

impl TextFormat {
    /// Sets the text alignment.
    pub fn set_text_alignment(&self, alignment: TextAlignment) -> Result<()> {
        let align = match alignment {
            TextAlignment::Left => DWRITE_TEXT_ALIGNMENT_LEADING,
            TextAlignment::Right => DWRITE_TEXT_ALIGNMENT_TRAILING,
            TextAlignment::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
            TextAlignment::Justified => DWRITE_TEXT_ALIGNMENT_JUSTIFIED,
        };

        // SAFETY: SetTextAlignment is safe
        unsafe {
            self.format.SetTextAlignment(align)?;
        }
        Ok(())
    }

    /// Sets the paragraph alignment (vertical).
    pub fn set_paragraph_alignment(&self, alignment: ParagraphAlignment) -> Result<()> {
        let align = match alignment {
            ParagraphAlignment::Top => DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
            ParagraphAlignment::Bottom => DWRITE_PARAGRAPH_ALIGNMENT_FAR,
            ParagraphAlignment::Center => DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
        };

        // SAFETY: SetParagraphAlignment is safe
        unsafe {
            self.format.SetParagraphAlignment(align)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_rgb() {
        let c = Color::rgb(0.5, 0.25, 0.75);
        assert_eq!(c.r, 0.5);
        assert_eq!(c.g, 0.25);
        assert_eq!(c.b, 0.75);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn test_color_from_hex() {
        let c = Color::from_hex(0xFF8040);
        assert!((c.r - 1.0).abs() < 0.01);
        assert!((c.g - 0.5).abs() < 0.01);
        assert!((c.b - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_color_constants() {
        assert_eq!(Color::BLACK.r, 0.0);
        assert_eq!(Color::WHITE.r, 1.0);
        assert_eq!(Color::RED.r, 1.0);
        assert_eq!(Color::RED.g, 0.0);
    }

    #[test]
    fn test_d2d_factory_creation() {
        // This may fail if D2D is not available, which is okay for testing
        let _ = D2DFactory::new();
    }

    #[test]
    fn test_dwrite_factory_creation() {
        // This may fail if DWrite is not available
        let _ = DWriteFactory::new();
    }
}
