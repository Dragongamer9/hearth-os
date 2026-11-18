//! firehold-comp: Hearth's own Wayland compositor (preview), Mantel's window manager for 2.0 (docs/FIREHOLD.md).
//!
//!   firehold-comp [--winit | --tty-udev | --headless | --auto] [--socket NAME] [--config FILE] [--labwc-rc FILE]
//!                     [--size WxH] [--no-xwayland] [-- COMMAND ARGS...]
//!   firehold-comp ctl COMMAND       one command to the running compositor's window IPC (ipc.rs), its answer printed
//!
//! --winit runs it in a window (nested, for tests and development), --tty-udev on the screens and input devices
//! themselves (DRM/KMS + libinput through libseat), --headless with no screen at all (drawn into memory, for CI),
//! --auto picks winit when it's started inside another desktop
//! (WAYLAND_DISPLAY or DISPLAY set) and the screens otherwise. COMMAND is started once the compositor is up (with
//! WAYLAND_DISPLAY, and DISPLAY once XWayland is ready): Desktop Mode's autostart, as KWin and labwc are given theirs.
//! It reads Hearth's settings and never writes them; it ends cleanly on SIGTERM, SIGINT or SIGHUP.

use firehold_comp::options::{Backend, Options};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("ctl") {
        std::process::exit(firehold_comp::ipc::ctl(&args[1..]));
    }
    let filter = std::env::var("FIREHOLD_LOG")
        .ok()
        .or_else(|| std::env::var("RUST_LOG").ok())
        .unwrap_or_else(|| "info".into());
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
        .init();

    let opts = match Options::parse(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("firehold-comp: {e}");
            eprintln!("{}", firehold_comp::options::USAGE);
            std::process::exit(2);
        }
    };
    if opts.help {
        println!("{}", firehold_comp::options::USAGE);
        return;
    }
    let backend = opts.backend();
    firehold_comp::options::set(opts);
    match backend {
        #[cfg(feature = "winit")]
        Backend::Winit => {
            tracing::info!("Firehold: nested (winit)");
            firehold_comp::winit::run_winit();
        }
        #[cfg(feature = "udev")]
        Backend::Udev => {
            tracing::info!("Firehold: on the screens (DRM/KMS, libinput)");
            firehold_comp::udev::run_udev();
        }
        #[cfg(feature = "headless")]
        Backend::Headless => {
            tracing::info!("Firehold: headless (drawn into memory)");
            firehold_comp::headless::run_headless();
        }
        #[allow(unreachable_patterns)]
        other => {
            tracing::error!("This build has no {:?} backend", other);
            std::process::exit(2);
        }
    }
    tracing::info!("Firehold ended");
}
