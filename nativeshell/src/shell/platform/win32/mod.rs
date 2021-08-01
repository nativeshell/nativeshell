pub mod app_delegate;
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
pub mod flutter_sys;
pub mod hot_key;
pub mod init;
pub mod keyboard_map;
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
    pub use super::bindings::Windows::Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
        Storage::StructuredStorage::*,
        System::{
            Com::*, DataExchange::*, Diagnostics::Debug::*, LibraryLoader::*, Memory::*,
            SystemServices::*, Threading::*,
        },
        UI::{
            Controls::*, KeyboardAndMouseInput::*, Shell::*, TextServices::*,
            WindowsAndMessaging::*,
        },
    };
    pub use windows::*;
}
