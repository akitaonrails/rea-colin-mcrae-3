//! Launcher logic that runs anywhere; the Windows-only parts live in
//! `win32` (the binary's `app` module) and `ui`.
//!
//! Unlike the 1998 game's launcher (rea-colin-mcrae), which applies its
//! fixes in memory to a suspended process, this one assembles open-source
//! components around the game: SilentPatch inside it (an ASI plugin), Xidi
//! and DXVK beside it, and the game's own settings file configured. The
//! user's folder stays read-only; everything happens in the runtime copy
//! (see `runtime`).

pub mod args;
pub mod components;
pub mod config;
pub mod edition;
pub mod form;
pub mod runtime;
pub mod sp;
pub mod xidi;
