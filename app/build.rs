//! Puts Pencraft's icon and name into `pencraft.exe` when it is built on Windows; does nothing elsewhere.

// Human: the icon inside the program is what the Explorer, the Start menu, shortcuts and `.pen` files show, and
// the name is what the Task Manager calls it. `design/icon/build.sh` draws the icon file.
// Agent: `winresource` is a Windows-only build dependency, so every use sits behind `cfg(windows)` (the system
// the build runs on); CARGO_CFG_TARGET_OS is the system it builds for.
use std::io::Write as _;

/// The icon file, from this package's folder.
const ICON: &str = "../installer/pencraft.ico";

fn main() -> std::io::Result<()> {
    // Not `println!`: the workspace's lints keep that out of the code.
    let mut out = std::io::stdout();
    writeln!(out, "cargo:rerun-if-changed=build.rs")?;
    writeln!(out, "cargo:rerun-if-changed={ICON}")?;
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(ICON).set("ProductName", "Pencraft").set("FileDescription", "Pencraft");
        resource.compile()?;
    }
    Ok(())
}
