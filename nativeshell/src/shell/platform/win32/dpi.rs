use std::sync::Once;

use windows::Win32::{
    Foundation::BOOL,
    System::LibraryLoader::{FreeLibrary, GetProcAddress, LoadLibraryW},
    UI::WindowsAndMessaging::SetProcessDPIAware,
};

fn set_per_monitor_aware() -> bool {
    const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
    const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE: isize = -3;

    let mut res = false;
    unsafe {
        let module = LoadLibraryW("User32.dll");
        if module.0 != 0 {
            let function = GetProcAddress(module, "SetProcessDpiAwarenessContext");
            if let Some(set_awareness_context) = function {
                let function: extern "system" fn(isize) -> BOOL =
                    std::mem::transmute(set_awareness_context);
                //  Windows 10 Anniversary Update (1607) or later
                if !function(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).as_bool() {
                    function(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE);
                }
                res = true;
            }
            FreeLibrary(module);
        }
    }
    res
}

fn set_per_monitor_dpi_aware_fallback() -> bool {
    const PROCESS_PER_MONITOR_DPI_AWARE: i32 = 2;

    let mut res = false;
    unsafe {
        let module = LoadLibraryW("Shcore.dll");
        if module.0 != 0 {
            let function = GetProcAddress(module, "SetProcessDpiAwareness");
            if let Some(set_awareness) = function {
                let function: extern "system" fn(i32) -> BOOL = std::mem::transmute(set_awareness);
                function(PROCESS_PER_MONITOR_DPI_AWARE);
                res = true;
            }
            FreeLibrary(module);
        }
    }
    res
}

pub fn become_dpi_aware() {
    static BECOME_AWARE: Once = Once::new();
    BECOME_AWARE.call_once(|| {
        if !set_per_monitor_aware() && !set_per_monitor_dpi_aware_fallback() {
            unsafe { SetProcessDPIAware() };
        }
    });
}
