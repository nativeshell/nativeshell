use std::os::raw::c_int;

extern "C" {
    #[cfg(not(target_os = "dragonfly"))]
    #[cfg_attr(
        any(target_os = "macos", target_os = "ios", target_os = "freebsd"),
        link_name = "__error"
    )]
    #[cfg_attr(
        any(target_os = "openbsd", target_os = "netbsd", target_os = "android"),
        link_name = "__errno"
    )]
    #[cfg_attr(
        any(target_os = "solaris", target_os = "illumos"),
        link_name = "___errno"
    )]
    #[cfg_attr(target_os = "linux", link_name = "__errno_location")]
    #[cfg_attr(target_os = "windows", link_name = "_errno")]
    fn errno_location() -> *mut c_int;
}

pub fn errno() -> c_int {
    unsafe { *errno_location() }
}

pub fn set_errno(errno: c_int) {
    unsafe {
        *errno_location() = errno;
    }
}
