//! The headless backend (`firehold-comp --headless`): no screen, no window, no input devices. The desktop is drawn into
//! memory on one made-up screen (HEADLESS-1, --size), with the GPU's EGL or Mesa's software one, 60 times a second
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

pub struct HeadlessData {
    renderer: GlesRenderer,
    damage_tracker: OutputDamageTracker,
    target: Option<GlesRenderbuffer>,
    full_redraw: u8,
}

impl Backend for HeadlessData {
    const KIND: &'static str = "headless";
    fn seat_name(&self) -> String {
        String::from("headless")
    }
    fn reset_buffers(&mut self, _output: &Output) {
        self.full_redraw = 2;
        self.target = None;
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
    let (w, h) = crate::options::get().size;
    let mode = Mode { size: (w, h).into(), refresh: 60_000 };
    let output = Output::new(
        OUTPUT_NAME.to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Hearth".into(),
            model: "Headless".into(),
        },
    );
    let _global = output.create_global::<HearthState<HeadlessData>>(&display.handle());
    output.change_current_state(Some(mode), Some(Transform::Normal), None, Some((0, 0).into()));
    output.set_preferred(mode);

    let data = HeadlessData {
        renderer,
        damage_tracker: OutputDamageTracker::from_output(&output),
        target: None,
        full_redraw: 2,
    };
    let mut state = HearthState::init(display, event_loop.handle(), data, true);
    let formats: Vec<_> = state.backend_data.renderer.shm_formats().collect();
    state.shm_state.update_formats(formats);
    state.space.map_output(&output, (0, 0));

    #[cfg(feature = "xwayland")]
    state.start_xwayland();

    info!(w, h, "headless: drawing into memory");
    let frame = Duration::from_micros(16_667);
    while state.running.load(Ordering::SeqCst) {
        let now = state.clock.now();
        let frame_target = now + frame;
        state.pre_repaint(&output, frame_target);
        let lock_view = state.lock.view(&output);
        let locked = lock_view.is_some();

        let size = output
            .current_mode()
            .map(|m| output.current_transform().transform_size(m.size))
            .unwrap_or((w, h).into());
        let data = &mut state.backend_data;
        if data.target.is_none() {
            data.damage_tracker = OutputDamageTracker::from_output(&output);
            data.target = data
                .renderer
                .create_buffer(Fourcc::Abgr8888, size.to_logical(1).to_buffer(1, Transform::Normal))
                .map_err(|e| warn!("headless: no buffer to draw in: {e}"))
                .ok();
        }
        let age = if data.full_redraw > 0 { 0 } else { 1 };
        data.full_redraw = data.full_redraw.saturating_sub(1);
        let rendered = match data.target.as_mut() {
            Some(target) => match data.renderer.bind(target) {
                Ok(mut fb) => render_output(
                    &output,
                    &state.space,
                    Vec::<CustomRenderElements<GlesRenderer>>::new(),
                    &mut data.renderer,
                    &mut fb,
                    &mut data.damage_tracker,
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
                state.post_repaint(&output, frame_target, None, &states);
            }
            Err(e) => warn!("headless: drawing failed: {e}"),
        }
        if state.captures.wants(&output) {
            crate::screencopy::run_captures(
                &mut state.captures,
                &mut state.backend_data.renderer,
                &output,
                &state.space,
                lock_view.as_ref(),
                state.clock.now().into(),
            );
        }

        let result = event_loop.dispatch(Some(frame), &mut state);
        if result.is_err() {
            state.running.store(false, Ordering::SeqCst);
        } else {
            state.space.refresh();
            state.popups.cleanup();
            state.hearth_tick();
            let _ = display_handle.flush_clients();
        }
    }
    state.cleanup();
}
