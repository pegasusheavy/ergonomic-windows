//! WebView2 - Chromium-based web view.
//!
//! Provides safe wrappers for embedding a Chromium-based web browser
//! in Windows applications using Microsoft Edge WebView2.
//!
//! This module requires the `webview2` feature to be enabled.
//!
//! # Prerequisites
//!
//! WebView2 requires the Microsoft Edge WebView2 Runtime to be installed.
//! It's pre-installed on Windows 10 (version 1803+) and Windows 11.
//!
//! # Example
//!
//! ```ignore
//! use ergonomic_windows::webview::{WebView, WebViewBuilder};
//!
//! let webview = WebViewBuilder::new()
//!     .with_url("https://www.rust-lang.org")
//!     .build(parent_hwnd)?;
//!
//! // Navigate to a URL
//! webview.navigate("https://docs.rs")?;
//!
//! // Execute JavaScript
//! webview.execute_script("document.body.style.background = 'red';")?;
//! ```

#[cfg(feature = "webview2")]
mod inner {
    use crate::error::{Error, Result};
    use crate::string::WideString;
    use std::cell::RefCell;
    use std::path::Path;
    use std::rc::Rc;
    use std::sync::mpsc;
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    /// A builder for creating WebView2 instances.
    pub struct WebViewBuilder {
        url: Option<String>,
        user_data_folder: Option<String>,
        enable_dev_tools: bool,
        enable_context_menu: bool,
        enable_zoom: bool,
    }

    impl WebViewBuilder {
        /// Creates a new WebView builder.
        pub fn new() -> Self {
            Self {
                url: None,
                user_data_folder: None,
                enable_dev_tools: false,
                enable_context_menu: true,
                enable_zoom: true,
            }
        }

        /// Sets the initial URL to navigate to.
        pub fn with_url(mut self, url: &str) -> Self {
            self.url = Some(url.to_string());
            self
        }

        /// Sets the user data folder for the browser profile.
        pub fn with_user_data_folder(mut self, path: &str) -> Self {
            self.user_data_folder = Some(path.to_string());
            self
        }

        /// Enables developer tools (F12).
        pub fn with_dev_tools(mut self, enable: bool) -> Self {
            self.enable_dev_tools = enable;
            self
        }

        /// Enables the context menu (right-click).
        pub fn with_context_menu(mut self, enable: bool) -> Self {
            self.enable_context_menu = enable;
            self
        }

        /// Enables zoom controls.
        pub fn with_zoom(mut self, enable: bool) -> Self {
            self.enable_zoom = enable;
            self
        }

        /// Builds the WebView2 instance.
        ///
        /// This is an asynchronous operation. The WebView will be created
        /// and attached to the parent window.
        pub fn build(self, parent: HWND) -> Result<WebView> {
            use webview2_com::Microsoft::Web::WebView2::Win32::*;
            use webview2_com::*;
            use windows::core::Interface;

            // Get parent window bounds
            let mut rect = RECT::default();
            unsafe {
                GetClientRect(parent, &mut rect).map_err(|e| Error::from_win32(e.into()))?;
            }

            // Create environment and controller synchronously using a channel
            let (tx, rx) = mpsc::channel();

            let user_data = self.user_data_folder.clone();
            let url = self.url.clone();
            let enable_dev_tools = self.enable_dev_tools;
            let enable_context_menu = self.enable_context_menu;
            let enable_zoom = self.enable_zoom;

            // Create the environment
            let create_result = unsafe {
                CreateCoreWebView2EnvironmentWithOptions(
                    None,
                    user_data.as_ref().map(|s| {
                        let wide = WideString::new(s);
                        wide.as_pcwstr()
                    }),
                    None,
                    &CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
                        move |_err, env| {
                            if let Some(env) = env {
                                // Create controller
                                let _ = env.CreateCoreWebView2Controller(
                                    parent,
                                    &CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
                                        move |_err, controller| {
                                            let _ = tx.send(controller);
                                            Ok(())
                                        },
                                    )),
                                );
                            } else {
                                let _ = tx.send(None);
                            }
                            Ok(())
                        },
                    )),
                )
            };

            if create_result.is_err() {
                return Err(Error::custom(
                    "Failed to create WebView2 environment. Is WebView2 Runtime installed?",
                ));
            }

            // Wait for the controller to be created
            // Note: In a real application, you'd integrate this with your message loop
            let controller = rx
                .recv()
                .map_err(|_| Error::custom("WebView2 controller creation failed"))?
                .ok_or_else(|| Error::custom("WebView2 controller was not created"))?;

            // Get the webview
            let webview = unsafe { controller.CoreWebView2() }
                .map_err(|_| Error::custom("Failed to get CoreWebView2"))?;

            // Configure settings
            if let Ok(settings) = unsafe { webview.Settings() } {
                unsafe {
                    let _ = settings.SetAreDevToolsEnabled(enable_dev_tools);
                    let _ = settings.SetAreDefaultContextMenusEnabled(enable_context_menu);
                    let _ = settings.SetIsZoomControlEnabled(enable_zoom);
                }
            }

            // Set bounds
            unsafe {
                let _ = controller.SetBounds(rect);
            }

            // Navigate to initial URL
            if let Some(url) = url {
                let url_wide = WideString::new(&url);
                unsafe {
                    let _ = webview.Navigate(url_wide.as_pcwstr());
                }
            }

            Ok(WebView {
                controller,
                webview,
                parent,
            })
        }
    }

    impl Default for WebViewBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    /// A WebView2 browser control.
    pub struct WebView {
        controller: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller,
        webview: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2,
        parent: HWND,
    }

    impl WebView {
        /// Navigates to a URL.
        pub fn navigate(&self, url: &str) -> Result<()> {
            let url_wide = WideString::new(url);
            unsafe {
                self.webview
                    .Navigate(url_wide.as_pcwstr())
                    .map_err(|_| Error::custom("Navigation failed"))?;
            }
            Ok(())
        }

        /// Navigates to a string of HTML content.
        pub fn navigate_to_string(&self, html: &str) -> Result<()> {
            let html_wide = WideString::new(html);
            unsafe {
                self.webview
                    .NavigateToString(html_wide.as_pcwstr())
                    .map_err(|_| Error::custom("NavigateToString failed"))?;
            }
            Ok(())
        }

        /// Executes JavaScript in the context of the current page.
        pub fn execute_script(&self, script: &str) -> Result<()> {
            use webview2_com::Microsoft::Web::WebView2::Win32::*;

            let script_wide = WideString::new(script);
            unsafe {
                self.webview
                    .ExecuteScript(
                        script_wide.as_pcwstr(),
                        &ExecuteScriptCompletedHandler::create(Box::new(|_err, _result| Ok(()))),
                    )
                    .map_err(|_| Error::custom("ExecuteScript failed"))?;
            }
            Ok(())
        }

        /// Refreshes the current page.
        pub fn reload(&self) -> Result<()> {
            unsafe {
                self.webview
                    .Reload()
                    .map_err(|_| Error::custom("Reload failed"))?;
            }
            Ok(())
        }

        /// Goes back in history.
        pub fn go_back(&self) -> Result<()> {
            unsafe {
                self.webview
                    .GoBack()
                    .map_err(|_| Error::custom("GoBack failed"))?;
            }
            Ok(())
        }

        /// Goes forward in history.
        pub fn go_forward(&self) -> Result<()> {
            unsafe {
                self.webview
                    .GoForward()
                    .map_err(|_| Error::custom("GoForward failed"))?;
            }
            Ok(())
        }

        /// Stops loading the current page.
        pub fn stop(&self) -> Result<()> {
            unsafe {
                self.webview
                    .Stop()
                    .map_err(|_| Error::custom("Stop failed"))?;
            }
            Ok(())
        }

        /// Resizes the WebView to match the parent window.
        pub fn resize_to_parent(&self) -> Result<()> {
            let mut rect = RECT::default();
            unsafe {
                GetClientRect(self.parent, &mut rect).map_err(|e| Error::from_win32(e.into()))?;
                self.controller
                    .SetBounds(rect)
                    .map_err(|_| Error::custom("SetBounds failed"))?;
            }
            Ok(())
        }

        /// Sets the bounds of the WebView.
        pub fn set_bounds(&self, left: i32, top: i32, right: i32, bottom: i32) -> Result<()> {
            let rect = RECT {
                left,
                top,
                right,
                bottom,
            };
            unsafe {
                self.controller
                    .SetBounds(rect)
                    .map_err(|_| Error::custom("SetBounds failed"))?;
            }
            Ok(())
        }

        /// Shows the WebView.
        pub fn show(&self) {
            unsafe {
                let _ = self.controller.SetIsVisible(true);
            }
        }

        /// Hides the WebView.
        pub fn hide(&self) {
            unsafe {
                let _ = self.controller.SetIsVisible(false);
            }
        }

        /// Gets the current URL.
        pub fn url(&self) -> Result<String> {
            unsafe {
                let source = self
                    .webview
                    .Source()
                    .map_err(|_| Error::custom("Failed to get Source"))?;
                Ok(source.to_string())
            }
        }

        /// Posts a web message (JSON) to the page.
        pub fn post_web_message_as_json(&self, json: &str) -> Result<()> {
            let json_wide = WideString::new(json);
            unsafe {
                self.webview
                    .PostWebMessageAsJson(json_wide.as_pcwstr())
                    .map_err(|_| Error::custom("PostWebMessageAsJson failed"))?;
            }
            Ok(())
        }

        /// Posts a web message (string) to the page.
        pub fn post_web_message_as_string(&self, message: &str) -> Result<()> {
            let message_wide = WideString::new(message);
            unsafe {
                self.webview
                    .PostWebMessageAsString(message_wide.as_pcwstr())
                    .map_err(|_| Error::custom("PostWebMessageAsString failed"))?;
            }
            Ok(())
        }
    }
}

#[cfg(feature = "webview2")]
pub use inner::*;

/// Placeholder types when webview2 feature is not enabled.
#[cfg(not(feature = "webview2"))]
mod placeholder {
    use crate::error::{Error, Result};
    use windows::Win32::Foundation::HWND;

/// WebView2 is not available without the `webview2` feature.
#[derive(Default)]
pub struct WebViewBuilder;

impl WebViewBuilder {
    /// Creates a new WebView builder.
    ///
    /// **Note**: Enable the `webview2` feature to use WebView2.
    pub fn new() -> Self {
        Self
    }

        /// Builds the WebView2 instance.
        ///
        /// Always returns an error because the `webview2` feature is not enabled.
        pub fn build(self, _parent: HWND) -> Result<WebView> {
            Err(Error::custom(
                "WebView2 support requires the 'webview2' feature to be enabled",
            ))
        }
    }

    /// WebView2 placeholder when feature is disabled.
    pub struct WebView;
}

#[cfg(not(feature = "webview2"))]
pub use placeholder::*;

