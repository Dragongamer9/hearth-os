//! hearth-compositor: Hearth's own Wayland compositor (preview), Mantel's window manager for 2.0 (docs/COMPOSITOR.md).
//!
//!   hearth-compositor [--winit | --tty-udev | --auto] [--socket NAME] [--config FILE] [--labwc-rc FILE]
//!                     [--size WxH] [--no-xwayland] [-- COMMAND ARGS...]
//!
//! --winit runs it in a window (nested, for tests and development), --tty-udev on the screens and input devices
//! themselves (DRM/KMS + libinput through libseat), --auto picks winit when it's started inside another desktop
//! (WAYLAND_DISPLAY or DISPLAY set) and the screens otherwise. COMMAND is started once the compositor is up (with
//! WAYLAND_DISPLAY, and DISPLAY once XWayland is ready): Desktop Mode's autostart, as KWin and labwc are given theirs.
//! It reads Hearth's settings and never writes them; it ends cleanly on SIGTERM, SIGINT or SIGHUP.

use hearth_compositor::options::{Backend, Options};

fn main() {
    let filter = std::env::var("HEARTH_COMPOSITOR_LOG")
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
            eprintln!("hearth-compositor: {e}");
            eprintln!("{}", hearth_compositor::options::USAGE);
            std::process::exit(2);
        }
    };
    if opts.help {
        println!("{}", hearth_compositor::options::USAGE);
        return;
    }
    let backend = opts.backend();
    hearth_compositor::options::set(opts);
    match backend {
        #[cfg(feature = "winit")]
        Backend::Winit => {
            tracing::info!("Hearth compositor: nested (winit)");
            hearth_compositor::winit::run_winit();
        }
        #[cfg(feature = "udev")]
        Backend::Udev => {
            tracing::info!("Hearth compositor: on the screens (DRM/KMS, libinput)");
            hearth_compositor::udev::run_udev();
        }
        #[allow(unreachable_patterns)]
        other => {
            tracing::error!("This build has no {:?} backend", other);
            std::process::exit(2);
        }
    }
    tracing::info!("Hearth compositor ended");
}
