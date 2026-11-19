//! The headless backend (`firehold-comp --headless`): no screen, no window, no input devices. The desktop is drawn into
//! memory on one made-up screen (HEADLESS-1, --size), or on several side by side (`--outputs HEADLESS-1,HDMI-A-1`:
//! a monitor and a TV, for Firehold's per-screen posture), with the GPU's EGL or Mesa's software one, 60 times a second
//! when something changed. For tests and CI: everything else is as on real screens (Mantel, apps, XWayland, the lock,
//! screen pictures through screencopy, wlr-randr), so a test needs no virtual X screen; its pictures come from grim.
//! Input comes from the protocols apps use for it (wtype's virtual keyboard, wlrctl).

use std::{sync::atomic::Ordering, time::Duration};

use smithay::{
    backend::{
        allocator::Fourcc,
        egl::{native::EGLSurfacelessDisplay, EGLContext, EGLDevice, EGLDisplay},
        renderer::{
            damage::OutputDamageTracker,
            gles::{GlesRenderbuffer, GlesRenderer},
            Bind, ImportMemWl, Offscreen,
        },
    },
    input::keyboard::LedState,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::EventLoop,
        wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
        wayland_server::{protocol::wl_surface, Display},
    },
    utils::Transform,
    wayland::presentation::Refresh,
};
use tracing::{error, info, warn};

use crate::{
    render::{render_output, CustomRenderElements},
    state::{take_presentation_feedback, Backend, HearthState},
};

pub const OUTPUT_NAME: &str = "HEADLESS-1";

/// One made-up screen: what it's drawn into.
struct Screen {
    output: Output,
    damage_tracker: OutputDamageTracker,
    target: Option<GlesRenderbuffer>,
    full_redraw: u8,
}

pub struct HeadlessData {
    renderer: GlesRenderer,
    screens: Vec<Screen>,
}

impl Backend for HeadlessData {
    const KIND: &'static str = "headless";
    const SCREEN_PACES: bool = false;
    fn seat_name(&self) -> String {
        String::from("headless")
    }
    fn reset_buffers(&mut self, output: &Output) {
        if let Some(s) = self.screens.iter_mut().find(|s| &s.output == output) {
            s.full_redraw = 2;
            s.target = None;
        }
    }
    fn early_import(&mut self, _surface: &wl_surface::WlSurface) {}
    fn update_led_state(&mut self, _led_state: LedState) {}
}

/// A GL renderer without any screen: Mesa's surfaceless platform first, then any EGL device (software last).
fn make_renderer() -> Result<GlesRenderer, String> {
    let mut tried = Vec::new();
    match unsafe { EGLDisplay::new(EGLSurfacelessDisplay) } {
        Ok(display) => match EGLContext::new(&display).map(|c| unsafe { GlesRenderer::new(c) }) {
            Ok(Ok(r)) => return Ok(r),
            Ok(Err(e)) => tried.push(format!("surfaceless renderer: {e}")),
            Err(e) => tried.push(format!("surfaceless context: {e}")),
        },
        Err(e) => tried.push(format!("surfaceless display: {e}")),
    }
    let mut devices: Vec<EGLDevice> = EGLDevice::enumerate().map_err(|e| format!("no EGL devices: {e}"))?.collect();
    devices.sort_by_key(|d| d.is_software());
    for d in devices {
        match unsafe { EGLDisplay::new(d) } {
            Ok(display) => match EGLContext::new(&display).map(|c| unsafe { GlesRenderer::new(c) }) {
                Ok(Ok(r)) => return Ok(r),
                Ok(Err(e)) => tried.push(format!("device renderer: {e}")),
                Err(e) => tried.push(format!("device context: {e}")),
            },
            Err(e) => tried.push(format!("device display: {e}")),
        }
    }
    Err(tried.join("; "))
}

pub fn run_headless() {
    let mut event_loop = EventLoop::try_new().unwrap();
    let display = Display::new().unwrap();
    let mut display_handle = display.handle();

    let renderer = match make_renderer() {
        Ok(r) => r,
        Err(e) => {
            error!("headless: no renderer ({e})");
            std::process::exit(1);
        }
    };
    let opts = crate::options::get();
    let list: Vec<(String, (i32, i32))> =
        if opts.outputs.is_empty() { vec![(OUTPUT_NAME.to_string(), opts.size)] } else { opts.outputs.clone() };
    let mut screens = Vec::new();
    let mut globals = Vec::new();
    let mut x = 0;
    let mut placed = Vec::new();
    for (name, (w, h)) in &list {
        let mode = Mode { size: (*w, *h).into(), refresh: 60_000 };
        // An HDMI screen is a TV (a big one, as Firehold's Settings calls it); the others are monitors.
        let tv = name.to_ascii_uppercase().starts_with("HDMI");
        let output = Output::new(
            name.clone(),
            PhysicalProperties {
                size: if tv { (1210, 680).into() } else { (0, 0).into() },
                subpixel: Subpixel::Unknown,
                make: "Hearth".into(),
                model: if tv { "TV".into() } else { "Headless".into() },
            },
        );
        globals.push(output.create_global::<HearthState<HeadlessData>>(&display.handle()));
        output.change_current_state(Some(mode), Some(Transform::Normal), None, Some((x, 0).into()));
        output.set_preferred(mode);
        placed.push((output.clone(), x));
        x += *w;
        screens.push(Screen { damage_tracker: OutputDamageTracker::from_output(&output), output, target: None, full_redraw: 2 });
    }

    let data = HeadlessData { renderer, screens };
    let mut state = HearthState::init(display, event_loop.handle(), data, true);
    let formats: Vec<_> = state.backend_data.renderer.shm_formats().collect();
    state.shm_state.update_formats(formats);
    for (output, x) in &placed {
        state.space.map_output(output, (*x, 0));
    }

    #[cfg(feature = "xwayland")]
    state.start_xwayland();

    info!(screens = list.len(), "headless: drawing into memory ({})", list.iter().map(|(n, (w, h))| format!("{n} {w}x{h}")).collect::<Vec<_>>().join(", "));
    let frame = Duration::from_micros(16_667);
    // FIREHOLD_HEADLESS_IDLE=1 (the at-rest checks, firehold-rust-idle.sh): after a few frames with nothing new, wait for
    // a client instead of drawing 60 times a second, as a real screen does (udev draws when something changed). Off by
    // default: the game tests count frame callbacks at the headless screen's steady 60.
    let idle_ok = std::env::var_os("FIREHOLD_HEADLESS_IDLE").is_some();
    let mut quiet: u32 = 0;
    while state.running.load(Ordering::SeqCst) {
        let mut any_damage = false;
        let mut any_wants = false;
        for i in 0..state.backend_data.screens.len() {
            let output = state.backend_data.screens[i].output.clone();
            let now = state.clock.now();
            let frame_target = now + frame;
            state.pre_repaint(&output, frame_target);
            let lock_view = state.lock.view(&output);
            let locked = lock_view.is_some();

            let (w, h) = list.get(i).map(|l| l.1).unwrap_or(opts.size);
            let size = output
                .current_mode()
                .map(|m| output.current_transform().transform_size(m.size))
                .unwrap_or((w, h).into());
            let HeadlessData { renderer, screens } = &mut state.backend_data;
            let scr = &mut screens[i];
            if scr.target.is_none() {
                scr.damage_tracker = OutputDamageTracker::from_output(&output);
                scr.target = renderer
                    .create_buffer(Fourcc::Abgr8888, size.to_logical(1).to_buffer(1, Transform::Normal))
                    .map_err(|e| warn!("headless: no buffer to draw in: {e}"))
                    .ok();
            }
            let age = if scr.full_redraw > 0 { 0 } else { 1 };
            scr.full_redraw = scr.full_redraw.saturating_sub(1);
            let rendered = match scr.target.as_mut() {
                Some(target) => match renderer.bind(target) {
                    Ok(mut fb) => render_output(
                        &output,
                        &state.space,
                        Vec::<CustomRenderElements<GlesRenderer>>::new(),
                        renderer,
                        &mut fb,
                        &mut scr.damage_tracker,
                        age,
                        false,
                        lock_view.as_ref(),
                    )
                    .map(|r| (r.damage.is_some(), r.states))
                    .map_err(|e| format!("{e:?}")),
                    Err(e) => Err(format!("{e}")),
                },
                None => Err("no buffer".into()),
            };
            match rendered {
                Ok((damaged, states)) => {
                    any_damage |= damaged;
                    if damaged {
                        state.captures.damaged(&output);
                        let mut feedback = take_presentation_feedback(&output, &state.space, &states);
                        feedback.presented(
                            frame_target,
                            Refresh::fixed(frame),
                            0,
                            wp_presentation_feedback::Kind::empty(),
                        );
                    }
                    if locked {
                        state.lock_frame_drawn(&output);
                    }
                    crate::state::update_primary_scanout_output(
                        &state.space,
                        &output,
                        &state.dnd_icon,
                        &state.cursor_status,
                        &states,
                    );
                    state.post_repaint(&output, frame_target, None, &states);
                }
                Err(e) => warn!("headless: drawing failed on {}: {e}", output.name()),
            }
            if state.captures.wants(&output) {
                any_wants = true;
                crate::screencopy::run_captures(
                    &mut state.captures,
                    &mut state.backend_data.renderer,
                    &output,
                    &state.space,
                    lock_view.as_ref(),
                    state.clock.now().into(),
                );
            }
        }
        quiet = if any_damage { 0 } else { quiet.saturating_add(1) };

        let wait = if idle_ok && quiet >= 3 && !any_wants { Duration::from_secs(1) } else { frame };
        let result = event_loop.dispatch(Some(wait), &mut state);
        if result.is_err() {
            state.running.store(false, Ordering::SeqCst);
        } else {
            state.space.refresh();
            state.popups.cleanup();
            state.hearth_tick();
            let _ = display_handle.flush_clients();
        }
    }
    drop(globals);
    state.cleanup();
}
