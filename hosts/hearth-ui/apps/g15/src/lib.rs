//! Hearth's Logitech G15 support (docs/G15.md). Linux's own driver (hid-lg-g15, kernel 5.5+) already does the hard
//! part: the G-keys come as KEY_MACRO1… events on an input device named "Logitech Gaming Keyboard Gaming Keys", the
//! M-keys as KEY_MACRO_PRESET1–3, MR as KEY_MACRO_RECORD_START, the keys under the screen as KEY_KBD_LCD_MENU1–5,
//! and the lights are LEDs (/sys/class/leds/g15::kbd_backlight, g15::lcd_backlight, g15::macro_preset1–3,
//! g15::macro_record). The screen is drawn through the G-key part's hidraw node (USB 046d:c222 on the first G15;
//! 046d:c221 is its ordinary keyboard and 046d:c223 the hub inside it): output report 3, as lcdproc and libg15 do.
//!
//! * [`root`]: where /sys and /dev are (HEARTH_G15_FIXTURE: a pretend keyboard for CI and the pictures);
//! * [`device`]: finding the keyboard's parts; [`leds`]: its lights;
//! * [`keys`]: key codes, names and the US layout for typing; [`input`]: reading events and the uinput keyboard;
//! * [`actions`]: what a G-key does, playing it and recording a macro (MR);
//! * [`config`]: g15.json (only through settings_file.rs: red flag 9);
//! * [`lcd`]: the 160x43 screen: frames, the 5x7 font, the screens made of elements, the report sent to the keyboard;
//! * [`values`]: what screens can show (CPU, graphics card, memory, drive, network, fans, the song…);
//! * [`running`]: which games run (window lists, Steam, Hearth's Game Mode: told, not polled);
//! * [`lights_auto`]: Night light's schedule and logind's sleep / lock / idle signals for the lights;
//! * [`service`]: the user service that ties it together.

pub mod actions;
pub mod config;
pub mod device;
pub mod input;
pub mod keys;
pub mod lcd;
pub mod leds;
pub mod lights_auto;
pub mod root;
pub mod running;
pub mod service;
pub mod values;

/// Runs a program with a time limit (coreutils' timeout) and returns its stdout, or None. Nothing here can hang.
pub fn run_out(cmd: &[&str], secs: u32) -> Option<String> {
    if cmd.is_empty() {
        return None;
    }
    let o = std::process::Command::new("timeout")
        .arg("-k")
        .arg("2")
        .arg(secs.to_string())
        .args(cmd)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if o.status.success() {
        Some(String::from_utf8_lossy(&o.stdout).to_string())
    } else {
        None
    }
}

/// Starts a program and doesn't wait for it (a thread reaps it, so nothing is left behind as a zombie).
pub fn spawn_detached(cmd: &[String]) -> bool {
    if cmd.is_empty() {
        return false;
    }
    match std::process::Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(mut ch) => {
            let _ = std::thread::Builder::new().name("hearth-g15-reap".into()).stack_size(64 * 1024).spawn(move || {
                let _ = ch.wait();
            });
            true
        }
        Err(_) => false,
    }
}

/// A program on PATH?
pub fn which(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(name).is_file()))
        .unwrap_or(false)
}

/// One line in the journal (stderr of the user service).
pub fn log(s: &str) {
    eprintln!("hearth-g15: {s}");
}
