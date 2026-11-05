//! Embed the application icon and version metadata in Windows executables.
//!
//! The window icon is set from the embedded PNG at runtime, but Explorer,
//! shortcuts and the taskbar read the executable's icon resource, which has to
//! be linked in. `winresource` needs `rc.exe` from the Windows SDK (or
//! `windres` for GNU targets), so it only runs when the target is Windows.
//!
//! `CARGO_CFG_TARGET_OS` is used instead of `cfg(windows)` because build
//! scripts run on the host, not on the target.

#![allow(clippy::expect_used)] // a failed resource compile must fail the build

fn main() {
    println!("cargo:rerun-if-changed=assets/jadeforge.ico");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS");
    if target_os.as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("assets/jadeforge.ico");
    resource
        .set("ProductName", "JadeForge")
        .set("FileDescription", "JadeForge - Jade Empire savegame editor")
        .set("OriginalFilename", "jadeforge.exe");
    resource
        .compile()
        .expect("failed to compile the Windows icon resource");
}
