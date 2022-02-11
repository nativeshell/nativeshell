#[allow(non_camel_case_types)]
pub mod glib {
    use std::os::raw::{c_int, c_uint, c_void};
    pub type gboolean = c_int;
    pub type gpointer = *mut c_void;
    pub type GSourceFunc = Option<unsafe extern "C" fn(gpointer) -> gboolean>;
    pub type GDestroyNotify = Option<unsafe extern "C" fn(gpointer)>;
    pub const GFALSE: c_int = 0;
    pub const G_SOURCE_REMOVE: gboolean = GFALSE;
    pub const G_PRIORITY_DEFAULT: c_int = 0;

    #[repr(C)]
    pub struct GMainContext(c_void);

    #[link(name = "glib-2.0")]
    extern "C" {
        pub fn g_source_remove(tag: c_uint) -> gboolean;
        pub fn g_timeout_add_full(
            priority: c_int,
            interval: c_uint,
            function: GSourceFunc,
            data: gpointer,
            notify: GDestroyNotify,
        ) -> c_uint;
        pub fn g_main_context_invoke_full(
            context: *mut GMainContext,
            priority: c_int,
            function: GSourceFunc,
            data: gpointer,
            notify: GDestroyNotify,
        );
        pub fn g_main_context_default() -> *mut GMainContext;
    }
    #[link(name = "gtk-3")]
    extern "C" {
        pub fn gtk_main();
        pub fn gtk_main_quit();
    }
}
