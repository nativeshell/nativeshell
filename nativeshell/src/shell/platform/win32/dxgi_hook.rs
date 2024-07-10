use crate::util::OkLog;
use detour::RawDetour;
use once_cell::sync::Lazy;
use std::{
    cell::Cell,
    ffi::c_void,
    mem::{self, ManuallyDrop},
    slice,
    sync::Mutex,
};
use windows::{
    core::{IUnknown, Interface, RawPtr, GUID, HRESULT},
    Win32::{
        Foundation::{BOOL, HWND},
        Graphics::Dxgi::{
            Common::DXGI_FORMAT, IDXGIAdapter, IDXGIDevice, IDXGIFactory, IDXGIFactory2,
            IDXGISwapChain1, DXGI_PRESENT_PARAMETERS, DXGI_SWAP_CHAIN_DESC1,
            DXGI_SWAP_CHAIN_FULLSCREEN_DESC,
        },
        System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
        UI::WindowsAndMessaging::GetParent,
    },
};

type CreateTargetForHwndT = unsafe extern "system" fn(
    this: RawPtr,
    hwnd: HWND,
    topmost: BOOL,
    target: *mut ::std::option::Option<IUnknown>,
) -> HRESULT;

type DCompositionCreateDeviceT = fn(usize, *const GUID, *mut *mut ::std::ffi::c_void) -> HRESULT;

type D3D11CreateDeviceT = fn(
    usize,
    i32,
    isize,
    u32,
    *const c_void,
    u32,
    u32,
    *mut ::std::option::Option<IUnknown>,
    *mut c_void,
    *mut ::std::option::Option<IUnknown>,
) -> HRESULT;

type CreateSwapChainForHwndT = unsafe extern "system" fn(
    RawPtr,
    RawPtr,
    HWND,
    *const DXGI_SWAP_CHAIN_DESC1,
    *const DXGI_SWAP_CHAIN_FULLSCREEN_DESC,
    RawPtr,
    *mut ::std::option::Option<IDXGISwapChain1>,
) -> HRESULT;

type CreateSwapChainForCompositionT = fn(
    RawPtr,
    RawPtr,
    *const DXGI_SWAP_CHAIN_DESC1,
    RawPtr,
    *mut Option<IDXGISwapChain1>,
) -> HRESULT;

type Present1T = fn(RawPtr, u32, u32, *const DXGI_PRESENT_PARAMETERS) -> HRESULT;

type ResizeBuffersT = fn(RawPtr, u32, u32, u32, DXGI_FORMAT, u32) -> HRESULT;

unsafe extern "system" fn create_target_for_hwnd(
    this: RawPtr,
    mut hwnd: HWND,
    topmost: BOOL,
    target: *mut ::std::option::Option<IUnknown>,
) -> HRESULT {
    // Use parent HWND as direct composition target; This significantly reduces transparency
    // artifacts along right and bottom edge during resizing
    let mut parent = take_override_parent_hwnd();
    if parent.0 == 0 {
        parent = GetParent(hwnd);
    }
    if parent.0 != 0 {
        hwnd = parent;
    }
    let global = GLOBAL.lock().unwrap();
    return global.create_target_for_hwnd.as_ref().unwrap()(this, hwnd, topmost, target);
}

unsafe extern "system" fn dcomposition_create_device(
    dxgi_device: usize,
    iid: *const GUID,
    dcomposition_device: *mut *mut ::std::ffi::c_void,
) -> HRESULT {
    let mut global = GLOBAL.lock().unwrap();
    let res =
        global.dcomposition_create_device.as_ref().unwrap()(dxgi_device, iid, dcomposition_device);
    if global.create_target_for_hwnd.is_none() {
        let device = &*(dcomposition_device as *const Option<IUnknown>);
        let vtable = ::windows::core::Interface::vtable(device.as_ref().unwrap());
        let myvtable: &[usize] = std::slice::from_raw_parts(
            vtable as *const windows::core::IUnknown_abi as *const usize,
            7,
        );

        let dt = ManuallyDrop::new(Box::new(
            RawDetour::new(
                myvtable[6] as *const (),
                create_target_for_hwnd as *const (),
            )
            .unwrap(),
        ));
        dt.enable().ok();

        #[allow(clippy::missing_transmute_annotations)]
        global
            .create_target_for_hwnd
            .replace(mem::transmute(dt.trampoline()));
    }
    res
}

thread_local! {
    static IGNORE_NEXT_PRESENT : Cell<bool> = const { Cell::new(false) };
    static OVERRIDE_PARENT_HWND: Cell<HWND> = const { Cell::new(HWND(0)) };
}

pub(super) fn set_override_parent_hwnd(hwnd: HWND) {
    OVERRIDE_PARENT_HWND.with(|v| {
        v.replace(hwnd);
    })
}

pub(super) fn take_override_parent_hwnd() -> HWND {
    OVERRIDE_PARENT_HWND.with(|v| v.take())
}

unsafe fn resize_buffers(
    this: RawPtr,
    buffer_count: u32,
    width: u32,
    height: u32,
    new_format: DXGI_FORMAT,
    swap_chain_flags: u32,
) -> HRESULT {
    let global = GLOBAL.lock().unwrap();
    // No need to do anything now, leaving this here in case things change in future
    global.resize_buffers.unwrap()(
        this,
        buffer_count,
        width,
        height,
        new_format,
        swap_chain_flags,
    )
}

unsafe fn present1(
    this: RawPtr,
    sync_interval: u32,
    present_flags: u32,
    p_present_parameters: *const DXGI_PRESENT_PARAMETERS,
) -> HRESULT {
    let parameters = &*p_present_parameters;
    if parameters.DirtyRectsCount == 1 {
        let rects =
            slice::from_raw_parts(parameters.pDirtyRects, parameters.DirtyRectsCount as usize);
        // rect.left == 1 means Present was driggered from eglPostSubBufferNV
        // inside AngleSurfaceManager during resizing. If this is the case,
        // ignore this Present call and the next one
        if rects.get_unchecked(0).left == 1 {
            IGNORE_NEXT_PRESENT.with(|v| {
                v.replace(true);
            });
            return HRESULT(0);
        }
    }

    let ignore = IGNORE_NEXT_PRESENT.with(|v| v.replace(false));
    if ignore {
        return HRESULT(0);
    }

    // No dirty rect means first angle flip after resizing surface; However during normal surface
    // resize, triggered from AngleSurfaceManager::ResizeSurface, this should be ignored by the
    // code above; So if we get here, it means Angle resized surface on it's own, as a result of
    // previous frame SwapBuffer call, where it noticed change in window dimentions; As such we
    // need to ignore it, otherwise it causes glitches
    if parameters.DirtyRectsCount == 0 {
        return HRESULT(0);
    }

    let global = GLOBAL.lock().unwrap();
    global.present1.unwrap()(this, sync_interval, present_flags, p_present_parameters)
}

unsafe fn hook_swap_chain(swap_chain: IDXGISwapChain1) {
    let mut global = GLOBAL.lock().unwrap();
    if global.present1.is_none() {
        let vtable = ::windows::core::Interface::vtable(&swap_chain);
        let dt = ManuallyDrop::new(Box::new(
            RawDetour::new(vtable.22 as *const (), present1 as *const ()).unwrap(),
        ));
        dt.enable().ok();

        #[allow(clippy::missing_transmute_annotations)]
        global.present1.replace(mem::transmute(dt.trampoline()));
    }
    if global.resize_buffers.is_none() {
        let vtable = ::windows::core::Interface::vtable(&swap_chain);
        let dt = ManuallyDrop::new(Box::new(
            RawDetour::new(vtable.13 as *const (), resize_buffers as *const ()).unwrap(),
        ));
        dt.enable().ok();

        #[allow(clippy::missing_transmute_annotations)]
        global
            .resize_buffers
            .replace(mem::transmute(dt.trampoline()));
    }
}

unsafe extern "system" fn create_swap_chain_for_hwnd(
    this: RawPtr,
    p_device: RawPtr,
    h_wnd: HWND,
    p_desc: *const DXGI_SWAP_CHAIN_DESC1,
    p_fullscreen_desc: *const DXGI_SWAP_CHAIN_FULLSCREEN_DESC,
    p_restrict_to_output: RawPtr,
    pp_swap_chain: *mut Option<IDXGISwapChain1>,
) -> HRESULT {
    let res = {
        let global = GLOBAL.lock().unwrap();
        global.create_swap_chain_for_hwnd.unwrap()(
            this,
            p_device,
            h_wnd,
            p_desc,
            p_fullscreen_desc,
            p_restrict_to_output,
            pp_swap_chain,
        )
    };
    if let Some(swap_chain) = &(*pp_swap_chain) {
        hook_swap_chain(swap_chain.clone())
    }
    res
}

unsafe extern "system" fn create_swap_chain_for_composition(
    this: RawPtr,
    p_device: RawPtr,
    p_desc: *const DXGI_SWAP_CHAIN_DESC1,
    p_restrict_to_output: RawPtr,
    pp_swap_chain: *mut Option<IDXGISwapChain1>,
) -> HRESULT {
    let res = {
        let global = GLOBAL.lock().unwrap();
        global.create_swap_chain_for_composition.unwrap()(
            this,
            p_device,
            p_desc,
            p_restrict_to_output,
            pp_swap_chain,
        )
    };
    if let Some(swap_chain) = &(*pp_swap_chain) {
        hook_swap_chain(swap_chain.clone())
    }
    res
}

unsafe extern "system" fn d3d11_create_device(
    p_adapter: usize,
    driver_type: i32,
    software: isize,
    flags: u32,
    p_feature_levels: *const c_void,
    feature_levels: u32,
    sdk_version: u32,
    pp_device: *mut ::std::option::Option<IUnknown>,
    p_feature_level: *mut c_void,
    pp_immediate_context: *mut ::std::option::Option<IUnknown>,
) -> windows::core::HRESULT {
    let mut global = GLOBAL.lock().unwrap();
    let res = global.d3d11_create_device.as_ref().unwrap()(
        p_adapter,
        driver_type,
        software,
        flags,
        p_feature_levels,
        feature_levels,
        sdk_version,
        pp_device,
        p_feature_level,
        pp_immediate_context,
    );

    if let Some(device) = (*pp_device).clone() {
        let device = device.cast::<IDXGIDevice>().unwrap();
        let adapter: Option<IDXGIAdapter> = device.GetAdapter().ok_log();
        if let Some(adapter) = adapter {
            let factory = adapter
                .GetParent::<IDXGIFactory>()
                .ok_log()
                .and_then(|f| f.cast::<IDXGIFactory2>().ok());

            if let Some(factory) = factory {
                let vtable = ::windows::core::Interface::vtable(&factory);

                let dt = ManuallyDrop::new(Box::new(
                    RawDetour::new(
                        vtable.15 as *const (),
                        create_swap_chain_for_hwnd as *const (),
                    )
                    .unwrap(),
                ));
                #[allow(clippy::missing_transmute_annotations)]
                global
                    .create_swap_chain_for_hwnd
                    .replace(mem::transmute(dt.trampoline()));

                dt.enable().ok();

                let dt = ManuallyDrop::new(Box::new(
                    RawDetour::new(
                        vtable.24 as *const (),
                        create_swap_chain_for_composition as *const (),
                    )
                    .unwrap(),
                ));
                #[allow(clippy::missing_transmute_annotations)]
                global
                    .create_swap_chain_for_composition
                    .replace(mem::transmute(dt.trampoline()));

                dt.enable().ok();
            }
        }
    }

    res
}

struct Global {
    create_target_for_hwnd: Option<CreateTargetForHwndT>,
    dcomposition_create_device: Option<DCompositionCreateDeviceT>,
    d3d11_create_device: Option<D3D11CreateDeviceT>,
    create_swap_chain_for_hwnd: Option<CreateSwapChainForHwndT>,
    create_swap_chain_for_composition: Option<CreateSwapChainForCompositionT>,
    present1: Option<Present1T>,
    resize_buffers: Option<ResizeBuffersT>,
}

static GLOBAL: Lazy<Mutex<Global>> = Lazy::new(|| {
    Mutex::new(Global {
        create_target_for_hwnd: None,
        dcomposition_create_device: None,
        d3d11_create_device: None,
        create_swap_chain_for_hwnd: None,
        create_swap_chain_for_composition: None,
        present1: None,
        resize_buffers: None,
    })
});

pub(super) fn init_dxgi_hook() {
    let mut global = GLOBAL.lock().unwrap();
    let address = get_module_symbol_address("Dcomp.dll", "DCompositionCreateDevice");
    if let Some(address) = address {
        unsafe {
            let detour = ManuallyDrop::new(Box::new(
                RawDetour::new(
                    address as *const (),
                    dcomposition_create_device as *const (),
                )
                .unwrap(),
            ));
            #[allow(clippy::missing_transmute_annotations)]
            global
                .dcomposition_create_device
                .replace(mem::transmute(detour.trampoline()));
            detour.enable().ok();
        }
    }

    if false {
        // disable for now
        let address = get_module_symbol_address("d3d11.dll", "D3D11CreateDevice");
        if let Some(address) = address {
            unsafe {
                let detour = ManuallyDrop::new(Box::new(
                    RawDetour::new(address as *const (), d3d11_create_device as *const ()).unwrap(),
                ));
                #[allow(clippy::missing_transmute_annotations)]
                global
                    .d3d11_create_device
                    .replace(mem::transmute(detour.trampoline()));
                detour.enable().ok();
            }
        }
    }
}

fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
    unsafe {
        let mut handle = GetModuleHandleW(module);
        if handle.0 == 0 {
            handle = LoadLibraryW(module);
        }
        GetProcAddress(handle, symbol).map(|addr| addr as usize)
    }
}
