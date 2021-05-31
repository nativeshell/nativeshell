use nativeshell_build::Flutter;

fn main() -> () {
    #[cfg(target_os = "windows")]
    {
        windows::build!(
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
                FillRect, PAINTSTRUCT, BeginPaint, EndPaint, BI_RGB, DIB_RGB_COLORS,
            },
            Windows::Win32::Storage::StructuredStorage::{
                IStream, STREAM_SEEK, STREAM_SEEK_END,
            },
            Windows::Win32::System::Com::{
                CoInitializeEx, CoInitializeSecurity, CoUninitialize, COINIT,
                IDataObject, IDropSource, IDropTarget, RevokeDragDrop, OleInitialize, DVASPECT, TYMED,
                ReleaseStgMedium, DATADIR, EOLE_AUTHENTICATION_CAPABILITIES, FORMATETC, IEnumFORMATETC, IEnumSTATDATA,
                IAdviseSink, RegisterDragDrop, DoDragDrop,
                // constants
                TYMED_HGLOBAL, TYMED_ISTREAM, DATADIR_GET, DVASPECT_CONTENT, COINIT_APARTMENTTHREADED,
            },
            Windows::Win32::System::DataExchange::{
                RegisterClipboardFormatW, GetClipboardFormatNameW
            },
            Windows::Win32::System::Diagnostics::Debug::{
                IsDebuggerPresent, FlashWindowEx, GetLastError, FormatMessageW, FACILITY_CODE, FACILITY_WIN32,
                FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_IGNORE_INSERTS,
            },
            Windows::Win32::System::Memory::{
                GlobalSize, GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock, LocalFree,
            },
            Windows::Win32::System::SystemServices::{
                // Methods
                LoadLibraryW, MsgWaitForMultipleObjects,
                FreeLibrary, GetProcAddress, GetModuleHandleW,
                // Constants
                S_OK, S_FALSE, E_NOINTERFACE, E_NOTIMPL,
                TRUE, FALSE,
                BOOL,
                DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS,
                CLIPBOARD_FORMATS, CF_HDROP,
            },
            Windows::Win32::System::Threading::{
                CreateEventW, SetEvent, WaitForSingleObject,
                GetCurrentThreadId
            },
            Windows::Win32::System::WindowsProgramming::{
                FORMAT_MESSAGE_MAX_WIDTH_MASK, CloseHandle
            },
            Windows::Win32::UI::Controls:: {
                WM_MOUSELEAVE,
            },
            Windows::Win32::UI::DisplayDevices::{
                POINTL
            },
            Windows::Win32::UI::HiDpi::EnableNonClientDpiScaling,
            Windows::Win32::UI::KeyboardAndMouseInput::{
                SetFocus, EnableWindow, IsWindowEnabled, SetActiveWindow, ReleaseCapture, SetCapture,
                GetCapture, GetAsyncKeyState, GetKeyboardState, GetKeyState, TrackMouseEvent, ToUnicode,
                TME_LEAVE,
            },
            Windows::Win32::UI::Shell::{
                SetWindowSubclass, RemoveWindowSubclass, DefSubclassProc, IDropTargetHelper, IDragSourceHelper,
                DragQueryFileW, DROPFILES, SHCreateMemStream, SHDRAGIMAGE,
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
                WM_QUERYUISTATE, WM_SYSCOMMAND, GWL_EXSTYLE, GWL_STYLE, GWL_HWNDPARENT, GWL_USERDATA, GWLP_USERDATA,
                WS_EX_LAYOUTRTL, MK_LBUTTON, SW_SHOW, SW_HIDE, SWP_NOZORDER, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
                SWP_FRAMECHANGED,
                // Methods
                GetSystemMenu, EnableMenuItem, CreatePopupMenu, DestroyMenu, AppendMenuW,
                TrackPopupMenuEx, InsertMenuItemW, RemoveMenu, SetMenuItemInfoW, SetMenuInfo, GetMenuInfo,
                GetMenuItemInfoW, GetCursorPos, EndMenu, GetSubMenu, GetMenuItemCount, HiliteMenuItem,
                RegisterClassW, UnregisterClassW, PostMessageW, SendMessageW,
                GetMessageW, PeekMessageW, TranslateMessage, DispatchMessageW, DestroyWindow, CreateWindowExW,
                DefWindowProcW, SetWindowLongW, GetWindowLongW, ShowWindow, SetProcessDPIAware,
                SetWindowPos, GetWindowRect, GetClientRect, SetParent, GetParent, MoveWindow, SetForegroundWindow,
                SetTimer, SetWindowsHookExW, UnhookWindowsHookEx, CallNextHookEx, FindWindowW, SetWindowTextW,
                GetGUIThreadInfo, WindowFromPoint, LoadCursorW,
                // Structures
                CREATESTRUCTW, MSG, WINDOWPOS, NCCALCSIZE_PARAMS,
                // Constants
                TRACK_POPUP_MENU_FLAGS, WINDOW_LONG_PTR_INDEX,
                VK_SHIFT, WNDCLASS_STYLES, IDC_ARROW, SC_CLOSE, HTCAPTION, HTTOPLEFT,
                HTTOPRIGHT, HTTOP, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTBOTTOM, HTLEFT, HTRIGHT, HTCLIENT, HTTRANSPARENT,
                MSGF_MENU, VK_DOWN, VK_RIGHT, VK_LEFT, MIM_MENUDATA, MIM_STYLE, MFT_SEPARATOR, MFT_STRING,
                MFS_ENABLED, MFS_DISABLED, MFS_CHECKED, MFT_RADIOCHECK, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING,  MIIM_SUBMENU,
                MF_BYCOMMAND, MF_DISABLED, MF_GRAYED, MF_POPUP, MF_MOUSESELECT, MF_ENABLED,
                WS_OVERLAPPEDWINDOW, WS_DLGFRAME, WS_CAPTION, WS_THICKFRAME, WS_BORDER, WS_POPUP, WS_SYSMENU,
                WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
                WS_EX_NOREDIRECTIONBITMAP, WS_EX_APPWINDOW,
                CS_HREDRAW, CS_VREDRAW,
                WH_MSGFILTER,
                TPM_LEFTALIGN, TPM_TOPALIGN, TPM_VERTICAL, TPM_RETURNCMD,
            },
        );
    }

    #[cfg(target_os = "linux")]
    {
        cargo_emit::rustc_link_lib! {
            "flutter_linux_gtk",
        };
    }

    cargo_emit::rerun_if_env_changed!("FLUTTER_PROFILE");
    if Flutter::build_mode() == "profile" {
        cargo_emit::rustc_cfg!("flutter_profile");
    }
}
