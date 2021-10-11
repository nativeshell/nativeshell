use nativeshell_build::Flutter;

#[path = "keyboard_map/gen_keyboard_map.rs"]
mod gen_keyboard_map;

fn main() {
    #[cfg(target_os = "windows")]
    {
        windows::build!(
            Windows::Win32::Foundation::{
                CloseHandle,
                // Structs
                POINTL,
                // Constants
                S_OK, S_FALSE, E_NOINTERFACE, E_NOTIMPL,
                BOOL, DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS,
            },
            Windows::Win32::Graphics::Dwm:: {
                DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DwmFlush,
                DWMWINDOWATTRIBUTE, DWMNCRENDERINGPOLICY,
            },
            Windows::Win32::Graphics::Dxgi::{
                IDXGIDevice, IDXGIFactory, IDXGIFactory2, IDXGISwapChain1, IDXGIAdapter,
                DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FULLSCREEN_DESC, DXGI_PRESENT_PARAMETERS
            },
            Windows::Win32::Graphics::Gdi::{
                EnumDisplayMonitors, ClientToScreen, ScreenToClient, CreateSolidBrush, GetDC, ReleaseDC,
                CreateDIBSection, DeleteObject, RedrawWindow, GetDCEx, ExcludeClipRect,
                FillRect, PAINTSTRUCT, BeginPaint, EndPaint, BI_RGB, DIB_USAGE,
            },
            Windows::Win32::Storage::StructuredStorage::{
                IStream, STREAM_SEEK,
            },
            Windows::Win32::System::Com::{
                CoInitializeEx, CoInitializeSecurity, CoUninitialize, CoCreateInstance, COINIT,
                IDataObject, IDropSource, IDropTarget, RevokeDragDrop, OleInitialize, DVASPECT, TYMED,
                ReleaseStgMedium, DATADIR, EOLE_AUTHENTICATION_CAPABILITIES, FORMATETC, IEnumFORMATETC, IEnumSTATDATA,
                IAdviseSink, RegisterDragDrop, DoDragDrop,
                // constants
                DROPEFFECT_COPY, DROPEFFECT_MOVE, DROPEFFECT_LINK, DROPEFFECT_NONE,
            },
            Windows::Win32::System::DataExchange::{
                RegisterClipboardFormatW, GetClipboardFormatNameW
            },
            Windows::Win32::System::Diagnostics::Debug::{
                IsDebuggerPresent, FlashWindowEx, GetLastError, FormatMessageW, FACILITY_CODE,
            },
            Windows::Win32::System::LibraryLoader::{
                LoadLibraryW, GetProcAddress, GetModuleHandleW, FreeLibrary,
            },
            Windows::Win32::System::Memory::{
                GlobalSize, GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock, LocalFree,
            },
            Windows::Win32::System::SystemServices::{
                CLIPBOARD_FORMATS,
            },
            Windows::Win32::System::Threading::{
                CreateEventW, SetEvent, WaitForSingleObject,
                GetCurrentThreadId, MsgWaitForMultipleObjects
            },
            Windows::Win32::System::WindowsProgramming::{
                FORMAT_MESSAGE_MAX_WIDTH_MASK
            },
            Windows::Win32::UI::Controls:: {
                WM_MOUSELEAVE,
            },
            Windows::Win32::UI::HiDpi::EnableNonClientDpiScaling,
            Windows::Win32::UI::KeyboardAndMouseInput::{
                SetFocus, EnableWindow, IsWindowEnabled, SetActiveWindow, ReleaseCapture, SetCapture,
                GetCapture, GetAsyncKeyState, GetKeyboardState, GetKeyState, TrackMouseEvent, ToUnicode,
                ToUnicodeEx, MapVirtualKeyW, ToAscii, GetKeyboardLayout, GetKeyboardLayoutList, RegisterHotKey,
                UnregisterHotKey, TRACKMOUSEEVENT, VIRTUAL_KEY,
            },
            Windows::Win32::UI::Shell::{
                SetWindowSubclass, RemoveWindowSubclass, DefSubclassProc, IDropTargetHelper, IDragSourceHelper,
                DragQueryFileW, DROPFILES, SHCreateMemStream, SHDRAGIMAGE,
            },
            Windows::Win32::UI::TextServices::{
                ITfLanguageProfileNotifySink, ITfInputProcessorProfiles, ITfSource, CLSID_TF_InputProcessorProfiles, TF_INVALID_COOKIE,
            },
            Windows::Win32::UI::WindowsAndMessaging::{
                // Messages
                WM_DPICHANGED, WM_DESTROY, WM_SIZE, WM_ACTIVATE, WM_NCCREATE, WM_NCDESTROY, WM_ENTERMENULOOP,
                WM_QUIT, WM_DISPLAYCHANGE, WM_SHOWWINDOW, WM_CLOSE, WM_PAINT, WM_GETMINMAXINFO,
                WM_WINDOWPOSCHANGING, WM_NCCALCSIZE, WM_MOUSEMOVE, WM_NCMOUSEMOVE, WM_NCHITTEST, WM_NCMOUSEHOVER, WM_NCPAINT,
                WM_MOUSEFIRST, WM_MOUSELAST, WM_LBUTTONDOWN, WM_RBUTTONDOWN, WM_MBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONUP,
                WM_MBUTTONUP, WM_XBUTTONUP,
                WM_TIMER, WM_MENUCOMMAND, WM_COMMAND, WM_USER, WM_CANCELMODE, WM_MENUSELECT,
                WM_CHANGEUISTATE, WM_UPDATEUISTATE, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYUP, WM_SETFOCUS, WM_DWMCOMPOSITIONCHANGED,
                WM_NCLBUTTONDOWN, WM_ERASEBKGND, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE,
                WM_QUERYUISTATE, WM_SYSCOMMAND, WM_INITMENUPOPUP, WM_INPUTLANGCHANGE,
                MK_LBUTTON, WM_HOTKEY,
                // Methods
                GetSystemMenu, EnableMenuItem, CreatePopupMenu, DestroyMenu, AppendMenuW,
                TrackPopupMenuEx, InsertMenuItemW, RemoveMenu, SetMenuItemInfoW, SetMenuInfo, GetMenuInfo,
                GetMenuItemInfoW, GetCursorPos, EndMenu, GetSubMenu, GetMenuItemCount, HiliteMenuItem,
                RegisterClassW, UnregisterClassW, PostMessageW, SendMessageW,
                GetMessageW, PeekMessageW, TranslateMessage, DispatchMessageW, DestroyWindow, CreateWindowExW,
                DefWindowProcW, SetWindowLongW, GetWindowLongW, ShowWindow, SetProcessDPIAware, IsWindowVisible,
                SetWindowPos, GetWindowRect, GetClientRect, SetParent, GetParent, MoveWindow, SetForegroundWindow,
                SetTimer, SetWindowsHookExW, UnhookWindowsHookEx, CallNextHookEx, FindWindowW, SetWindowTextW,
                GetGUIThreadInfo, WindowFromPoint, LoadCursorW, GetWindowPlacement, SetWindowPlacement,
                // Structures
                CREATESTRUCTW, MSG, WINDOWPOS, NCCALCSIZE_PARAMS,
                // Constants
                TRACK_POPUP_MENU_FLAGS, WINDOW_LONG_PTR_INDEX,
                WNDCLASS_STYLES, IDC_ARROW, SC_CLOSE, HTCAPTION, HTTOPLEFT,
                HTTOPRIGHT, HTTOP, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTBOTTOM, HTLEFT, HTRIGHT, HTCLIENT, HTTRANSPARENT,
                MSGF_MENU, MAPVK_VK_TO_VSC, MAPVK_VSC_TO_VK, MAPVK_VK_TO_CHAR,
            },
        );
    }

    #[cfg(target_os = "macos")]
    {
        let files = ["src/shell/platform/macos/window_buttons.m"];
        let mut build = cc::Build::new();
        for file in files.iter() {
            build.file(file);
            cargo_emit::rerun_if_changed!(file);
        }
        build.flag("-fobjc-arc");
        build.compile("macos_extra");
    }

    cargo_emit::rerun_if_env_changed!("FLUTTER_PROFILE");
    if Flutter::build_mode() == "profile" {
        cargo_emit::rustc_cfg!("flutter_profile");
    }

    let target_system = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    gen_keyboard_map::generate_keyboard_map(&target_system).unwrap();
}
