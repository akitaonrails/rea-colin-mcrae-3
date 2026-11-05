//! The Windows executable's version information, icon and manifest
//! (rea-launcher-build): a named, versioned program with the launcher's
//! icon, instead of an anonymous one that antivirus heuristics distrust.
fn main() {
    rea_launcher_build::Resources::new(
        "CMR3 Launcher",
        "Colin McRae Rally 3 launcher for modern Windows and Linux",
    )
    .company("AkitaOnRails")
    .file_name("CMR3Launcher.exe")
    .icon("../../packaging/appimage/cmr3-launcher.png")
    .embed()
    .unwrap();
}
