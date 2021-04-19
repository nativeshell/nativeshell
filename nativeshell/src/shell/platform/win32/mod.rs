pub mod binary_messenger;
pub mod display;
pub mod dpi;
pub mod drag_com;
pub mod drag_context;
pub mod drag_data;
pub mod drag_util;
pub mod dxgi_hook;
pub mod engine;
pub mod error;
pub mod flutter_api;
pub mod init;
pub mod key_event;
pub mod menu;
pub mod run_loop;
pub mod util;
pub mod window;
pub mod window_adapter;
pub mod window_base;
pub mod window_menu;

#[allow(dead_code)]
mod bindings {
    ::windows::include_bindings!();
}

// This bit of a lie, it doesn't have dxgi
mod all_bindings {
    pub use super::bindings::{
        Windows::Win32::Com::*, Windows::Win32::Controls::*, Windows::Win32::DataExchange::*,
        Windows::Win32::Debug::*, Windows::Win32::DisplayDevices::*, Windows::Win32::Dwm::*,
        Windows::Win32::Gdi::*, Windows::Win32::KeyboardAndMouseInput::*,
        Windows::Win32::MenusAndResources::*, Windows::Win32::Shell::*,
        Windows::Win32::StructuredStorage::*, Windows::Win32::SystemServices::*,
        Windows::Win32::WindowsAndMessaging::*,
    };
    pub use windows::*;
}
