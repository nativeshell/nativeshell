use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

#[allow(non_camel_case_types)]
pub type size_t = usize;
#[allow(clippy::upper_case_acronyms)]
pub type UINT = u32;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlutterDesktopMessenger {
    _unused: [u8; 0],
}
pub type FlutterDesktopMessengerRef = *mut FlutterDesktopMessenger;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _FlutterPlatformMessageResponseHandle {
    _unused: [u8; 0],
}
pub type FlutterDesktopMessageResponseHandle = _FlutterPlatformMessageResponseHandle;
pub type FlutterDesktopBinaryReply = ::std::option::Option<
    unsafe extern "C" fn(
        data: *const u8,
        data_size: size_t,
        user_data: *mut ::std::os::raw::c_void,
    ),
>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlutterDesktopMessage {
    pub struct_size: size_t,
    pub channel: *const ::std::os::raw::c_char,
    pub message: *const u8,
    pub message_size: size_t,
    pub response_handle: *const FlutterDesktopMessageResponseHandle,
}
pub type FlutterDesktopMessageCallback = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: FlutterDesktopMessengerRef,
        arg2: *const FlutterDesktopMessage,
        arg3: *mut ::std::os::raw::c_void,
    ),
>;
extern "C" {
    pub fn FlutterDesktopMessengerSend(
        messenger: FlutterDesktopMessengerRef,
        channel: *const ::std::os::raw::c_char,
        message: *const u8,
        message_size: size_t,
    ) -> bool;
}
extern "C" {
    pub fn FlutterDesktopMessengerSendWithReply(
        messenger: FlutterDesktopMessengerRef,
        channel: *const ::std::os::raw::c_char,
        message: *const u8,
        message_size: size_t,
        reply: FlutterDesktopBinaryReply,
        user_data: *mut ::std::os::raw::c_void,
    ) -> bool;
}
extern "C" {
    pub fn FlutterDesktopMessengerSendResponse(
        messenger: FlutterDesktopMessengerRef,
        handle: *const FlutterDesktopMessageResponseHandle,
        data: *const u8,
        data_length: size_t,
    );
}
extern "C" {
    pub fn FlutterDesktopMessengerSetCallback(
        messenger: FlutterDesktopMessengerRef,
        channel: *const ::std::os::raw::c_char,
        callback: FlutterDesktopMessageCallback,
        user_data: *mut ::std::os::raw::c_void,
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlutterDesktopPluginRegistrar {
    _unused: [u8; 0],
}
pub type FlutterDesktopPluginRegistrarRef = *mut FlutterDesktopPluginRegistrar;
pub type FlutterDesktopOnPluginRegistrarDestroyed =
    ::std::option::Option<unsafe extern "C" fn(arg1: FlutterDesktopPluginRegistrarRef)>;
extern "C" {
    pub fn FlutterDesktopPluginRegistrarGetMessenger(
        registrar: FlutterDesktopPluginRegistrarRef,
    ) -> FlutterDesktopMessengerRef;
}
extern "C" {
    pub fn FlutterDesktopPluginRegistrarSetDestructionHandler(
        registrar: FlutterDesktopPluginRegistrarRef,
        callback: FlutterDesktopOnPluginRegistrarDestroyed,
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlutterDesktopViewControllerState {
    _unused: [u8; 0],
}
pub type FlutterDesktopViewControllerRef = *mut FlutterDesktopViewControllerState;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlutterDesktopView {
    _unused: [u8; 0],
}
pub type FlutterDesktopViewRef = *mut FlutterDesktopView;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlutterDesktopEngine {
    _unused: [u8; 0],
}
pub type FlutterDesktopEngineRef = *mut FlutterDesktopEngine;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlutterDesktopEngineProperties {
    pub assets_path: *const u16,
    pub icu_data_path: *const u16,
    pub aot_library_path: *const u16,
    pub dart_entrypoint_argc: ::std::os::raw::c_int,
    pub dart_entrypoint_argv: *mut *const ::std::os::raw::c_char,
}
extern "C" {
    pub fn FlutterDesktopViewControllerCreate(
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
        engine: FlutterDesktopEngineRef,
    ) -> FlutterDesktopViewControllerRef;
}
extern "C" {
    pub fn FlutterDesktopViewControllerDestroy(controller: FlutterDesktopViewControllerRef);
}
extern "C" {
    pub fn FlutterDesktopViewControllerGetEngine(
        controller: FlutterDesktopViewControllerRef,
    ) -> FlutterDesktopEngineRef;
}
extern "C" {
    pub fn FlutterDesktopViewControllerGetView(
        controller: FlutterDesktopViewControllerRef,
    ) -> FlutterDesktopViewRef;
}
extern "C" {
    pub fn FlutterDesktopViewControllerForceRedraw(controller: FlutterDesktopViewControllerRef);
}
extern "C" {
    pub fn FlutterDesktopViewControllerEnableDirectComposition(
        controller: FlutterDesktopViewControllerRef,
        use_direct_composition: bool,
    );
}
extern "C" {
    pub fn FlutterDesktopViewControllerHandleTopLevelWindowProc(
        controller: FlutterDesktopViewControllerRef,
        hwnd: HWND,
        message: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
        result: *mut LRESULT,
    ) -> bool;
}
extern "C" {
    pub fn FlutterDesktopEngineCreate(
        engine_properties: *const FlutterDesktopEngineProperties,
    ) -> FlutterDesktopEngineRef;
}
extern "C" {
    pub fn FlutterDesktopEngineDestroy(engine: FlutterDesktopEngineRef) -> bool;
}
extern "C" {
    pub fn FlutterDesktopEngineRun(
        engine: FlutterDesktopEngineRef,
        entry_point: *const ::std::os::raw::c_char,
    ) -> bool;
}
extern "C" {
    pub fn FlutterDesktopEngineProcessMessages(engine: FlutterDesktopEngineRef) -> u64;
}
extern "C" {
    pub fn FlutterDesktopEngineReloadSystemFonts(engine: FlutterDesktopEngineRef);
}
extern "C" {
    pub fn FlutterDesktopEngineGetPluginRegistrar(
        engine: FlutterDesktopEngineRef,
        plugin_name: *const ::std::os::raw::c_char,
    ) -> FlutterDesktopPluginRegistrarRef;
}
extern "C" {
    pub fn FlutterDesktopEngineGetMessenger(
        engine: FlutterDesktopEngineRef,
    ) -> FlutterDesktopMessengerRef;
}
extern "C" {
    pub fn FlutterDesktopViewGetHWND(view: FlutterDesktopViewRef) -> HWND;
}
pub type FlutterDesktopWindowProcCallback = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: HWND,
        arg2: UINT,
        arg3: WPARAM,
        arg4: LPARAM,
        arg5: *mut ::std::os::raw::c_void,
        result: *mut LRESULT,
    ) -> bool,
>;
extern "C" {
    pub fn FlutterDesktopPluginRegistrarGetView(
        registrar: FlutterDesktopPluginRegistrarRef,
    ) -> FlutterDesktopViewRef;
}
extern "C" {
    pub fn FlutterDesktopPluginRegistrarRegisterTopLevelWindowProcDelegate(
        registrar: FlutterDesktopPluginRegistrarRef,
        delegate: FlutterDesktopWindowProcCallback,
        user_data: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    pub fn FlutterDesktopPluginRegistrarUnregisterTopLevelWindowProcDelegate(
        registrar: FlutterDesktopPluginRegistrarRef,
        delegate: FlutterDesktopWindowProcCallback,
    );
}
extern "C" {
    pub fn FlutterDesktopGetDpiForHWND(hwnd: HWND) -> UINT;
}
extern "C" {
    pub fn FlutterDesktopGetDpiForMonitor(monitor: isize) -> UINT;
}
extern "C" {
    pub fn FlutterDesktopResyncOutputStreams();
}
