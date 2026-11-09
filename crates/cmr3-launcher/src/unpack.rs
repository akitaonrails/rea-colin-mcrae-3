//! Unpacking the game's installer on Windows (and in Wine): the SDK's
//! innoextract flow (`rea_launcher_win32::innoextract::unpack`) with
//! CMR3's folders and game check. The binary hands it the progress
//! window; the env-gated `tests/real_installer.rs` runs the same code with
//! none, on the owner's real repack.

use std::path::{Path, PathBuf};

use rea_launcher::installer::{self, Installer, UnpackJob, UnpackOutcome};
use rea_launcher::paths;
use rea_launcher_win32::innoextract::{self, Unpacked};

use crate::source;

/// Unpacks `found` (in the folder `chosen`) into
/// [`source::unpacked_dir`]`(runtime)` with the innoextract in
/// `components`, or reuses that folder when it already holds this
/// installer's game. `watch` shows the progress (an `Err` waits without a
/// window). Returns the game folder, or `None` when the player stopped it.
pub fn unpack(
    found: &Installer,
    chosen: &Path,
    runtime: &Path,
    components: &Path,
    log: &mut dyn FnMut(&str),
    watch: impl FnOnce(&mut dyn UnpackJob) -> Result<UnpackOutcome, String>,
) -> Result<Option<PathBuf>, String> {
    let dest = source::unpacked_dir(runtime);
    paths::check_separate(chosen, &dest)?;
    let exe = components.join(innoextract::EXE);
    log(&format!(
        "installer: {} ({}, sha256 {})",
        found.setup.display(),
        found.build.name,
        source::sha256_hex(found)
    ));
    if installer::is_unpacked(&dest, found) {
        log(&format!(
            "installer: already unpacked into {}",
            dest.display()
        ));
    } else if exe.is_file() {
        log(&format!("innoextract: {}", exe.display()));
    } else {
        return Err(missing_innoextract(&exe));
    }
    match innoextract::unpack(&exe, found, &dest, source::check_game, log, watch)? {
        Unpacked::Ready(done) => {
            log(&format!("game: {} ({})", dest.display(), done.game));
            Ok(Some(dest))
        }
        Unpacked::Cancelled => Ok(None),
    }
}

/// The error when the package lost its innoextract.
pub fn missing_innoextract(exe: &Path) -> String {
    format!(
        "This folder holds the game's installer, but {}, which unpacks it, is missing.\n\n\
         Reinstall the launcher package (the zip or the AppImage), then try again.",
        exe.display()
    )
}
