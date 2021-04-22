// Only available on MacOS, on other platforms it's a no-op.
//
// If running unbundled and project was compiled with MacOSBundle, it will
// re-run current process from within the bundle.
//
// This should be first thing called after application startup since it
// essentially restarts the application.
//
// Generally running unbundled macOS GUI application causes various issues
// (i.e. focus, unresponsive menu-bar, etc). It also means there is no access
// to bundle resources and Info.plist contents.
//
// Calling MacOSBundle::build(options) in build.rs and exec_bundle() at
// application startup allows you to run application as bundled using cargo run.
//
// Note: When debugging LLDB will pause on exec, to disable this you can add
// "settings set target.process.stop-on-exec false" to LLDB configuration
pub fn exec_bundle() {
    #[cfg(target_os = "macos")]
    {
        use super::platform::bundle::macos_exec_bundle;
        macos_exec_bundle();
    }
}
