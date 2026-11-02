//! Launcher logic that runs anywhere; the Windows-only parts live in the
//! binary's `ui` and `app` modules.
//!
//! Unlike the 1998 game's launcher (rea-colin-mcrae), which applies its
//! fixes in memory to a suspended process, this one assembles open-source
//! components around the game: SilentPatch inside it (an ASI plugin), Xidi
//! and DXVK beside it, and the game's own settings file configured. The
//! user's folder stays read-only; everything happens in the runtime copy
//! (see `runtime`).
//!
//! The generic machinery (runtime sync, whole-file swaps, save mirroring,
//! the component ledger, ini and `launcher.cfg` editing, Wine overrides)
//! is the shared `rea_launcher` SDK; this crate holds the game's own
//! knowledge.

pub mod components;
pub mod config;
pub mod edition;
pub mod form;
pub mod instance;
#[cfg(any(windows, test))]
pub mod look;
pub mod plan;
pub mod profile;
pub mod runtime;
pub mod sp;
#[cfg(any(windows, test))]
pub mod window;
pub mod xidi;
