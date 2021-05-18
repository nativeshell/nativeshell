fn main() -> () {
    #[cfg(target_os = "windows")]
    {
        windows::build!(
            Windows::Win32::SystemServices::{
                // Methods
                CreateEventW, SetEvent, WaitForSingleObject,
                MsgWaitForMultipleObjects, LoadLibraryW,
                FreeLibrary, GetProcAddress, GetModuleHandleW, GetCurrentThreadId,
                GlobalSize, GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock, LocalFree,
                // Constants
                S_OK, S_FALSE, E_NOINTERFACE, E_NOTIMPL,
                TRUE, FALSE,
                BOOL,
                DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS,
                CLIPBOARD_FORMATS,
            },
            Windows::Win32::Gdi::{
                EnumDisplayMonitors, ClientToScreen, ScreenToClient, CreateSolidBrush, GetDC, ReleaseDC,
                CreateDIBSection, DeleteObject, RedrawWindow, GetDCEx, ExcludeClipRect,
                FillRect, PAINTSTRUCT, BeginPaint, EndPaint, BI_RGB,
            },
            Windows::Win32::KeyboardAndMouseInput::{
                SetFocus, EnableWindow, IsWindowEnabled, SetActiveWindow, ReleaseCapture, SetCapture,
                GetCapture, GetAsyncKeyState, GetKeyboardState, GetKeyState, TrackMouseEvent, ToUnicode,
            },
            Windows::Win32::Debug::{
                IsDebuggerPresent, FlashWindowEx, GetLastError, FormatMessageW, FACILITY_CODE,
            },
            Windows::Win32::Dwm:: {
                DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DwmFlush,
                DWMWINDOWATTRIBUTE, DWMNCRENDERINGPOLICY,
            },
            Windows::Win32::WindowsProgramming::CloseHandle,
            Windows::Win32::WindowsAndMessaging::{
                // Messages
                WM_DPICHANGED, WM_DESTROY, WM_SIZE, WM_ACTIVATE, WM_NCCREATE, WM_NCDESTROY, WM_ENTERMENULOOP,
                WM_QUIT, WM_DISPLAYCHANGE, WM_SHOWWINDOW, WM_CLOSE, WM_PAINT, WM_GETMINMAXINFO,
                WM_WINDOWPOSCHANGING, WM_NCCALCSIZE, WM_MOUSEMOVE, WM_NCMOUSEMOVE, WM_NCHITTEST, WM_NCMOUSEHOVER, WM_NCPAINT,
                WM_MOUSEFIRST, WM_MOUSELAST, WM_LBUTTONDOWN, WM_RBUTTONDOWN, WM_MBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONUP,
                WM_MBUTTONUP, WM_XBUTTONUP,
                WM_TIMER, WM_MENUCOMMAND, WM_COMMAND, WM_USER, WM_CANCELMODE, WM_MENUSELECT,
                WM_CHANGEUISTATE, WM_UPDATEUISTATE, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYUP, WM_SETFOCUS, WM_DWMCOMPOSITIONCHANGED,
                WM_NCLBUTTONDOWN, WM_ERASEBKGND, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE,
                WM_QUERYUISTATE, WM_SYSCOMMAND,
                MK_LBUTTON,
                // Methods
                GetSystemMenu, EnableMenuItem, CreatePopupMenu, DestroyMenu, AppendMenuW,
                TrackPopupMenuEx, InsertMenuItemW, RemoveMenu, SetMenuItemInfoW, SetMenuInfo, GetMenuInfo,
                GetMenuItemInfoW, GetCursorPos, EndMenu, GetSubMenu, GetMenuItemCount, HiliteMenuItem,
                RegisterClassW, UnregisterClassW, PostMessageW, SendMessageW,
                GetMessageW, PeekMessageW, TranslateMessage, DispatchMessageW, DestroyWindow, CreateWindowExW,
                DefWindowProcW, SetWindowLongW, GetWindowLongW, ShowWindow, SetProcessDPIAware,
                SetWindowPos, GetWindowRect, GetClientRect, SetParent, GetParent, MoveWindow, SetForegroundWindow,
                SetTimer, SetWindowsHookExW, UnhookWindowsHookEx, CallNextHookEx, FindWindowW,
                GetGUIThreadInfo, WindowFromPoint, LoadCursorW,
                // Structures
                CREATESTRUCTW, MSG, WINDOWPOS, NCCALCSIZE_PARAMS,
                // Constants
                TRACK_POPUP_MENU_FLAGS, WINDOW_LONG_PTR_INDEX,
                VK_SHIFT, WNDCLASS_STYLES, IDC_ARROW, SC_CLOSE, HTCAPTION, HTTOPLEFT,
                HTTOPRIGHT, HTTOP, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTBOTTOM, HTLEFT, HTRIGHT, HTCLIENT, HTTRANSPARENT,
                MSGF_MENU, VK_DOWN, VK_RIGHT, VK_LEFT,
            },
            Windows::Win32::DisplayDevices::{
                POINTL
            },
            Windows::Win32::Controls:: {
                WM_MOUSELEAVE,
            },
            Windows::Win32::StructuredStorage::{
                IStream, STREAM_SEEK,
            },
            Windows::Win32::Shell::{
                SetWindowSubclass, RemoveWindowSubclass, DefSubclassProc, IDropTargetHelper, IDragSourceHelper,
                DragQueryFileW, DROPFILES, SHCreateMemStream, SHDRAGIMAGE,
            },
            Windows::Win32::Com::{
                CoInitializeEx, CoInitializeSecurity, CoUninitialize, COINIT,
                IDataObject, IDropSource, IDropTarget, RevokeDragDrop, OleInitialize, DVASPECT, TYMED,
                ReleaseStgMedium, DATADIR, EOLE_AUTHENTICATION_CAPABILITIES, FORMATETC, IEnumFORMATETC, IEnumSTATDATA,
                IAdviseSink, RegisterDragDrop, DoDragDrop,
            },
            Windows::Win32::DataExchange::{
                RegisterClipboardFormatW, GetClipboardFormatNameW
            },
            Windows::Win32::Dxgi::{
                IDXGIDevice, IDXGIFactory, IDXGIFactory2, IDXGISwapChain1, IDXGIAdapter,
                DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FULLSCREEN_DESC, DXGI_PRESENT_PARAMETERS
            },
            Windows::Win32::HiDpi::EnableNonClientDpiScaling
        );
        // cargo_emit::rustc_link_lib! {
        // "flutter_windows.dll",
        // }
    }

    #[cfg(target_os = "linux")]
    {
        cargo_emit::rustc_link_lib! {
            "flutter_linux_gtk",
        };
    }
}
