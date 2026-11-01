//! All Win32 calls. Kept small: process start, XInput for the settings
//! window's gamepad, the folder picker, message boxes.

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::Registry::HKEY_CURRENT_USER;
use windows_sys::Win32::System::Threading::{
    CreateProcessW, GetExitCodeProcess, INFINITE, PROCESS_INFORMATION, STARTUPINFOW,
    WaitForSingleObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MessageBoxW,
};

fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}

fn last_error() -> std::io::Error {
    std::io::Error::last_os_error()
}

pub fn message(title: &str, text: &str, error: bool) {
    let icon = if error {
        MB_ICONERROR
    } else {
        MB_ICONINFORMATION
    };
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(text).as_ptr(),
            wide(title).as_ptr(),
            MB_OK | icon,
        );
    }
}

/// An XInput function from the newest XInput DLL present.
fn xinput_proc(name: &std::ffi::CStr) -> Option<unsafe extern "system" fn() -> isize> {
    for dll in ["xinput1_4.dll", "xinput1_3.dll", "xinput9_1_0.dll"] {
        // SAFETY: plain loader calls with NUL-terminated names.
        unsafe {
            let module = LoadLibraryW(wide(dll).as_ptr());
            if !module.is_null()
                && let Some(proc) = GetProcAddress(module, name.as_ptr().cast())
            {
                return Some(proc);
            }
        }
    }
    None
}

/// The state (16 bytes: a packet number, then the gamepad, `wButtons`
/// first) of each XInput pad connected now, by slot 0..=3.
fn xinput_states() -> Vec<(u32, [u8; 16])> {
    type GetState = unsafe extern "system" fn(u32, *mut [u8; 16]) -> u32;
    let Some(get) = xinput_proc(c"XInputGetState") else {
        return Vec::new();
    };
    // SAFETY: XInputGetState's signature; the state is 16 bytes.
    let get: GetState = unsafe { std::mem::transmute(get) };
    (0..4)
        .filter_map(|i| {
            let mut state = [0u8; 16];
            (unsafe { get(i, &mut state) } == 0).then_some((i, state))
        })
        .collect()
}

/// The XInput (Xbox-style) pads connected now, for the settings window.
pub fn xinput_pads() -> Vec<u32> {
    xinput_states().into_iter().map(|(i, _)| i).collect()
}

/// The buttons (`wButtons` bits) held on any XInput pad now, for the
/// settings window's gamepad navigation; 0 without XInput.
pub fn xinput_buttons() -> u16 {
    xinput_states()
        .into_iter()
        .fold(0, |all, (_, s)| all | u16::from_le_bytes([s[4], s[5]]))
}

/// The main screen's size in pixels, if known.
pub fn screen_size() -> Option<(u32, u32)> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    // SAFETY: plain metric queries.
    let (w, h) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    (w > 0 && h > 0).then_some((w as u32, h as u32))
}

/// Asks for a folder with the system's folder picker.
pub fn pick_folder(title: &str) -> Option<std::path::PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Com::{
        COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree,
    };
    use windows_sys::Win32::UI::Shell::{
        BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW, SHBrowseForFolderW,
        SHGetPathFromIDListW,
    };
    let title = wide(title);
    let mut name = [0u16; 260];
    // SAFETY: the structure and buffers live across the modal call; the
    // returned item list is freed with the COM allocator.
    unsafe {
        CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        let info = BROWSEINFOW {
            hwndOwner: std::ptr::null_mut(),
            pidlRoot: std::ptr::null_mut(),
            pszDisplayName: name.as_mut_ptr(),
            lpszTitle: title.as_ptr(),
            ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
            lpfn: None,
            lParam: 0,
            iImage: 0,
        };
        let list = SHBrowseForFolderW(&info);
        if list.is_null() {
            return None;
        }
        let mut path = [0u16; 1024];
        let ok = SHGetPathFromIDListW(list, path.as_mut_ptr()) != 0;
        CoTaskMemFree(list as *const _);
        let n = path.iter().position(|&c| c == 0).unwrap_or(0);
        (ok && n > 0).then(|| std::ffi::OsString::from_wide(&path[..n]).into())
    }
}

/// True when running under Wine or Proton.
pub fn is_wine() -> bool {
    unsafe {
        let ntdll = GetModuleHandleW(wide("ntdll.dll").as_ptr());
        !ntdll.is_null() && GetProcAddress(ntdll, c"wine_get_version".as_ptr().cast()).is_some()
    }
}

/// Under Wine: draw OpenGL through GLX (`UseEGL` = N). Wine 11 draws
/// through EGL by default, and where the GPU driver has no EGL for X11
/// (NVIDIA without egl-x11) that falls back to software rendering; GLX
/// reaches the GPU. The settings window is the only OpenGL thing here
/// (the game draws through DXVK's Vulkan), but a software window is
/// visibly sluggish.
pub fn prefer_gpu_opengl() -> Result<bool, String> {
    set_user_value_if_missing(r"Software\Wine\X11 Driver", "UseEGL", "N")
}

/// On Windows with two GPUs (a laptop's integrated and discrete ones): the
/// high-performance one for the game (the per-program choice of Settings >
/// Display > Graphics). Under Wine this is cmr3-play's job instead.
pub fn prefer_fast_gpu(exe: &Path) -> Result<bool, String> {
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
        RegCreateKeyExW, RegQueryValueExW, RegSetValueExW,
    };
    let path = r"Software\Microsoft\DirectX\UserGpuPreferences";
    let mut key = std::ptr::null_mut();
    let rc = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide(path).as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if rc != ERROR_SUCCESS {
        return Err(format!("cannot open HKCU\\{path} (error {rc})"));
    }
    let result = (|| {
        let name = wide(exe.display().to_string());
        // SAFETY: an open key and a NUL-terminated name; nothing is read
        // back on the query.
        let present = unsafe {
            RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if present != ERROR_FILE_NOT_FOUND {
            return Ok(false);
        }
        let data = wide("GpuPreference=2;");
        // SAFETY: an open key and NUL-terminated strings.
        let rc = unsafe {
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            )
        };
        if rc == ERROR_SUCCESS {
            Ok(true)
        } else {
            Err(format!("cannot write the GPU preference (error {rc})"))
        }
    })();
    // SAFETY: the key was opened above.
    unsafe { RegCloseKey(key) };
    result
}

fn set_user_value_if_missing(path: &str, name: &str, data: &str) -> Result<bool, String> {
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
        RegCreateKeyExW, RegQueryValueExW, RegSetValueExW,
    };
    let mut key = std::ptr::null_mut();
    let rc = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide(path).as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if rc != ERROR_SUCCESS {
        return Err(format!("cannot open HKCU\\{path} (error {rc})"));
    }
    let result = (|| {
        let vname = wide(name);
        // SAFETY: an open key and a NUL-terminated name; nothing is read
        // back on the query.
        let present = unsafe {
            RegQueryValueExW(
                key,
                vname.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if present != ERROR_FILE_NOT_FOUND {
            return Ok(false);
        }
        let data = wide(data);
        // SAFETY: an open key and NUL-terminated strings.
        let rc = unsafe {
            RegSetValueExW(
                key,
                vname.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            )
        };
        if rc == ERROR_SUCCESS {
            Ok(true)
        } else {
            Err(format!("cannot write {name} (error {rc})"))
        }
    })();
    // SAFETY: the key was opened above.
    unsafe { RegCloseKey(key) };
    result
}

struct Child {
    info: PROCESS_INFORMATION,
}

impl Drop for Child {
    fn drop(&mut self) {
        // SAFETY: both handles were opened for this child.
        unsafe {
            CloseHandle(self.info.hThread);
            CloseHandle(self.info.hProcess);
        }
    }
}

/// Starts `exe` (with `args`, working in `workdir`) and waits for it.
/// `hidden` starts it without a console window (the bundled patcher).
/// Returns the exit code.
fn run_process(exe: &Path, args: &[OsString], workdir: &Path, hidden: bool) -> Result<u32, String> {
    let mut cmdline = wide(format!(
        "\"{}\"{}",
        exe.display(),
        args.iter()
            .map(|a| format!(" {}", a.to_string_lossy()))
            .collect::<String>()
    ));
    let dir = wide(workdir);
    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = size_of::<STARTUPINFOW>() as u32;
    let mut info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let flags = if hidden {
        windows_sys::Win32::System::Threading::CREATE_NO_WINDOW
    } else {
        0
    };
    // SAFETY: the strings live across the call; `cmdline` is writable.
    let ok = unsafe {
        CreateProcessW(
            wide(exe).as_ptr(),
            cmdline.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            flags,
            std::ptr::null(),
            dir.as_ptr(),
            &si,
            &mut info,
        )
    };
    if ok == 0 {
        return Err(format!("cannot start {}: {}", exe.display(), last_error()));
    }
    let child = Child { info };
    let mut code = u32::MAX;
    // SAFETY: the handle is owned by `child`.
    unsafe {
        WaitForSingleObject(child.info.hProcess, INFINITE);
        GetExitCodeProcess(child.info.hProcess, &mut code);
    }
    Ok(code)
}

/// Runs the game and waits for it to exit, returning its exit code. This
/// is how the game starts: no memory patching (the fixes live in the
/// bundled SilentPatch), so the process runs as it is.
pub fn run(exe: &Path, workdir: &Path) -> Result<u32, String> {
    run_process(exe, &[], workdir, false)
}

/// Runs the bundled xdelta patcher hidden (no console window flashes);
/// false when the patch did not apply (a mismatched source file).
pub fn run_hidden(exe: &Path, args: &[OsString], workdir: &Path) -> Result<bool, String> {
    Ok(run_process(exe, args, workdir, true)? == 0)
}
