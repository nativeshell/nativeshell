use std::ptr::null_mut;

use windows::Win32::System::{
    Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
    LibraryLoader::LoadLibraryW,
    Ole::OleInitialize,
};

use super::{
    dpi::become_dpi_aware,
    dxgi_hook::init_dxgi_hook,
    error::{PlatformError, PlatformResult},
    util::direct_composition_supported,
};

pub fn init_platform() -> PlatformResult<()> {
    unsafe {
        // Angle will try opening these with GetModuleHandleEx, which means they need to be
        // loaded first; Otherwise it falls back to d3dcompiler_47, which is not present on
        // some Windows 7 installations.
        #[allow(clippy::collapsible_if)]
        if LoadLibraryW("d3dcompiler_47.dll").0 == 0 {
            if LoadLibraryW("d3dcompiler_46.dll").0 == 0 {
                LoadLibraryW("d3dcompiler_43.dll");
            }
        }

        CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED).map_err(PlatformError::from)?;
        OleInitialize(null_mut()).map_err(PlatformError::from)?;

        // Needed for direct composition check
        LoadLibraryW("dcomp.dll");
    }
    if direct_composition_supported() {
        init_dxgi_hook();
    }
    become_dpi_aware();
    Ok(())
}
