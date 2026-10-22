//! The on-screen keyboard button. Linux: Mantel's keyboard (hosts/linux/desktop/shell/Keyboard.qml), asked over
//! Quickshell's IPC like the rest of Mantel does (`quickshell ipc -p <shell folder> call keyboard open|close`).
//! Windows: the touch keyboard (TabTip, through its COM interface), or the classic On-Screen Keyboard when
//! Explorer isn't running (Ember as the shell), the same as Ember's TouchKeyboard.cs.

/// Shows (`open`) or hides the keyboard. Err says why it couldn't, in words for the user.
pub fn set(open: bool) -> Result<(), String> { platform::set(open) }

#[cfg(unix)]
mod platform {
    pub fn set(open: bool) -> Result<(), String> {
        let share = std::env::var("NIMBUS_SHARE").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "/usr/share/nimbus".into());
        let shell = format!("{share}/desktop/shell");
        let out = std::process::Command::new("quickshell")
            .args(["ipc", "-p", &shell, "call", "keyboard", if open { "open" } else { "close" }])
            .output();
        match out {
            Ok(o) if o.status.success() => Ok(()),
            Ok(_) => Err("The on-screen keyboard is part of Hearth's desktop, which isn't running.".into()),
            Err(_) => Err("No on-screen keyboard here (it comes with Hearth's desktop).".into()),
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;
    use std::os::windows::process::CommandExt;
    use windows_sys::core::GUID;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetDesktopWindow;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const TIP_CLSID: GUID = GUID::from_u128(0x4ce576fa_83dc_4f88_951c_9d0782b4e376);
    const TIP_IID: GUID = GUID::from_u128(0x37c994e7_432b_4834_a2f7_dce1f13b834b);

    fn running(exe: &str) -> bool {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {exe}"), "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_ascii_lowercase().contains(&exe.to_ascii_lowercase()))
            .unwrap_or(false)
    }

    /// ITipInvocation::Toggle on the touch keyboard (TabTip must be running for it to answer).
    fn toggle_touch_keyboard() -> bool {
        // COM wants a thread of its own here (the app's threads may already be set up differently).
        std::thread::spawn(|| unsafe {
            CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as _);
            let mut p: *mut c_void = std::ptr::null_mut();
            let hr = CoCreateInstance(&TIP_CLSID, std::ptr::null_mut(), (CLSCTX_LOCAL_SERVER | CLSCTX_INPROC_SERVER) as _, &TIP_IID, &mut p);
            if hr < 0 || p.is_null() { return false; }
            // The interface's table: QueryInterface, AddRef, Release, then Toggle(HWND).
            let vtbl = *(p as *const *const usize);
            let release: unsafe extern "system" fn(*mut c_void) -> u32 = std::mem::transmute(*vtbl.add(2));
            let toggle: unsafe extern "system" fn(*mut c_void, HWND) -> i32 = std::mem::transmute(*vtbl.add(3));
            let ok = toggle(p, GetDesktopWindow()) >= 0;
            release(p);
            ok
        })
        .join()
        .unwrap_or(false)
    }

    fn touch_keyboard() -> bool {
        if !running("TabTip.exe") {
            let common = std::env::var("CommonProgramFiles").unwrap_or_else(|_| r"C:\Program Files\Common Files".into());
            let tabtip = std::path::Path::new(&common).join("microsoft shared").join("ink").join("TabTip.exe");
            if !tabtip.is_file() { return false; }
            // TabTip asks for more rights than ours (uiAccess): `start` opens it the way Explorer would.
            let _ = std::process::Command::new("cmd").raw_arg(format!("/c start \"\" \"{}\"", tabtip.display())).creation_flags(CREATE_NO_WINDOW).status();
            std::thread::sleep(std::time::Duration::from_millis(400));
        }
        toggle_touch_keyboard()
    }

    pub fn set(open: bool) -> Result<(), String> {
        // The touch keyboard only has a toggle; the page only asks to open it when it thinks it's closed.
        if running("explorer.exe") && touch_keyboard() { return Ok(()); }
        if !open { return Ok(()); }   // the classic keyboard closes with its own X
        if running("osk.exe") { return Ok(()); }
        std::process::Command::new("cmd")
            .raw_arg("/c start \"\" osk.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map(|_| ())
            .map_err(|e| format!("Couldn't open the on-screen keyboard ({e})."))
    }
}
