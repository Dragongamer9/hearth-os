//! Pictures of the screens for the apps that take them: wlr-screencopy-v1 (v3: grim, wf-recorder, Hearth Capture and
//! Hearth Recorder, xdg-desktop-portal-wlr's screen sharing) and ext-image-copy-capture-v1 with its output sources
//! (newer grim, the portal later). The backends draw each asked-for screen once more, into memory, right after its
//! frame (winit.rs, udev.rs: `run_captures`), exactly as it's shown, the lock screen included when it's locked (so a
//! picture never shows what the lock screen covers), and copy it into the app's shared-memory buffer.
//!
//! Recorders ask "with damage": they get the next picture only once the screen changed, so a still screen costs
//! nothing. The pointer isn't in the pictures yet; apps that want it draw it themselves (as with KWin's portal).
//! Sandboxed apps (Flatpak's security context) don't see these protocols: they go through the portal.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::Duration,
};

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            damage::OutputDamageTracker, gles::GlesRenderbuffer, Bind, ExportMem, ImportAll, ImportMem,
            Offscreen, Renderer, TextureMapping,
        },
    },
    desktop::Space,
    output::Output,
    reexports::{
        wayland_protocols::ext::{
            image_capture_source::v1::server::{
                ext_image_capture_source_v1::{self, ExtImageCaptureSourceV1},
                ext_output_image_capture_source_manager_v1::{self, ExtOutputImageCaptureSourceManagerV1},
            },
            image_copy_capture::v1::server::{
                ext_image_copy_capture_cursor_session_v1::{self, ExtImageCopyCaptureCursorSessionV1},
                ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
                ext_image_copy_capture_manager_v1::{self, ExtImageCopyCaptureManagerV1},
                ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
            },
        },
        wayland_protocols_wlr::screencopy::v1::server::{
            zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
            zwlr_screencopy_manager_v1::{self, ZwlrScreencopyManagerV1},
        },
        wayland_server::{
            backend::ClientId,
            protocol::{wl_buffer::WlBuffer, wl_output, wl_shm},
            Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
        },
    },
    utils::{Physical, Rectangle, Size, Transform},
};
use tracing::{debug, warn};

use crate::{
    lock::LockView,
    render::{output_elements, CustomRenderElements},
    shell::WindowElement,
    state::{Backend, HearthState},
};

/// The only format offered (every app takes it, and it's wl_shm's required one).
const FORMAT: wl_shm::Format = wl_shm::Format::Xrgb8888;

/// Who gets the picture.
#[derive(Debug, Clone)]
pub enum Target {
    Wlr { frame: ZwlrScreencopyFrameV1, with_damage: bool },
    Ext { frame: ExtImageCopyCaptureFrameV1, session: ExtImageCopyCaptureSessionV1 },
}

/// A picture asked for, waiting for its screen's next frame.
#[derive(Debug, Clone)]
pub struct Pending {
    pub output: Output,
    /// The part of the screen, in its pixels (the whole screen for ext and capture_output).
    pub region: Rectangle<i32, Physical>,
    pub buffer: WlBuffer,
    pub target: Target,
    /// Only once the screen changed after this (recorders): the screen's frame count when it was asked.
    pub after: Option<u64>,
}

#[derive(Debug, Default)]
pub struct CaptureState {
    pub pending: Vec<Pending>,
    /// Each screen's count of frames that changed something (by name).
    damage: HashMap<String, u64>,
    /// Pictures taken since the start, through each protocol (the status file: tests check apps really got them).
    pub taken_wlr: u64,
    pub taken_ext: u64,
}

impl CaptureState {
    pub fn new<B: Backend + 'static>(dh: &DisplayHandle) -> CaptureState {
        dh.create_global::<HearthState<B>, ZwlrScreencopyManagerV1, _>(3, ());
        dh.create_global::<HearthState<B>, ExtImageCopyCaptureManagerV1, _>(1, ());
        dh.create_global::<HearthState<B>, ExtOutputImageCaptureSourceManagerV1, _>(1, ());
        CaptureState::default()
    }

    /// A screen's frame changed something (the backends call this).
    pub fn damaged(&mut self, output: &Output) {
        *self.damage.entry(output.name()).or_insert(0) += 1;
    }

    fn seq(&self, output: &Output) -> u64 {
        self.damage.get(&output.name()).copied().unwrap_or(0)
    }

    pub fn wants(&self, output: &Output) -> bool {
        self.pending.iter().any(|p| &p.output == output)
    }
}

/// A screen's size in its own pixels, as it's shown (rotation applied).
fn output_size(output: &Output) -> Option<Size<i32, Physical>> {
    let mode = output.current_mode()?;
    Some(output.current_transform().transform_size(mode.size))
}

fn not_sandboxed(client: &Client) -> bool {
    client
        .get_data::<crate::ClientState>()
        .map_or(true, |c| c.security_context.is_none())
}

fn split_time(t: Duration) -> (u32, u32, u32) {
    let secs = t.as_secs();
    ((secs >> 32) as u32, (secs & 0xffff_ffff) as u32, t.subsec_nanos())
}

/// Draws the screen once into memory: rows top to bottom, 4 bytes a pixel (R, G, B, A).
fn draw_screen<R>(
    renderer: &mut R,
    output: &Output,
    space: &Space<WindowElement>,
    lock: Option<&LockView>,
) -> Result<(Size<i32, Physical>, Vec<u8>), String>
where
    R: Renderer + ImportAll + ImportMem + ExportMem + Offscreen<GlesRenderbuffer> + Bind<GlesRenderbuffer>,
    R::TextureId: Clone + Send + 'static,
{
    let size = output_size(output).ok_or("the screen has no mode")?;
    let scale = output.current_scale().fractional_scale();
    let (elements, clear) =
        output_elements(output, space, Vec::<CustomRenderElements<R>>::new(), renderer, false, lock);
    let buf_size = size.to_logical(1).to_buffer(1, Transform::Normal);
    let mut rb: GlesRenderbuffer = renderer
        .create_buffer(Fourcc::Abgr8888, buf_size)
        .map_err(|e| format!("no buffer to draw in: {e:?}"))?;
    let mut fb = renderer.bind(&mut rb).map_err(|e| format!("couldn't draw in it: {e:?}"))?;
    let mut tracker = OutputDamageTracker::new(size, scale, Transform::Normal);
    tracker
        .render_output(renderer, &mut fb, 0, &elements, clear)
        .map_err(|e| format!("drawing failed: {e:?}"))?;
    let mapping = renderer
        .copy_framebuffer(&fb, Rectangle::from_size(buf_size), Fourcc::Abgr8888)
        .map_err(|e| format!("reading it back failed: {e:?}"))?;
    // Smithay's GLES renderer already draws offscreen targets upside down for GL (its projection flips Y), so
    // glReadPixels hands back the rows top first even though the mapping says "flipped" (that's meant for window
    // surfaces). Taking the flag at its word gave upside-down pictures (grim, wf-recorder): the rows are used as they
    // come, and the frames go out without Y_INVERT. firehold-orientation.py checks it in CI, nested and headless.
    let _ = mapping.flipped();
    let bytes = renderer
        .map_texture(&mapping)
        .map_err(|e| format!("reading it back failed: {e:?}"))?;
    let row = (size.w * 4) as usize;
    if bytes.len() < row * size.h as usize {
        return Err(format!("read back {} bytes, wanted {}", bytes.len(), row * size.h as usize));
    }
    let mut out = vec![0u8; row * size.h as usize];
    for y in 0..size.h as usize {
        out[y * row..(y + 1) * row].copy_from_slice(&bytes[y * row..(y + 1) * row]);
    }
    Ok((size, out))
}

/// Copies part of a drawn screen (R, G, B, A rows) into an app's buffer as XRGB8888 (B, G, R, X in memory).
fn fill_buffer(buffer: &WlBuffer, screen: (Size<i32, Physical>, &[u8]), region: Rectangle<i32, Physical>) -> Result<(), String> {
    let (size, px) = screen;
    smithay::wayland::shm::with_buffer_contents_mut(buffer, |ptr, len, data| {
        if data.width != region.size.w || data.height != region.size.h {
            return Err(format!(
                "the buffer is {}x{}, the picture {}x{}",
                data.width, data.height, region.size.w, region.size.h
            ));
        }
        if !matches!(data.format, wl_shm::Format::Xrgb8888 | wl_shm::Format::Argb8888) {
            return Err(format!("the buffer's format {:?} isn't XRGB8888", data.format));
        }
        let stride = data.stride as usize;
        let need = data.offset as usize + stride * (data.height.max(0) as usize);
        if stride < region.size.w as usize * 4 || need > len {
            return Err("the buffer is too small".into());
        }
        // Shared memory the app could change at any time: written through the pointer, never borrowed as a slice.
        for y in 0..region.size.h {
            let sy = (region.loc.y + y).clamp(0, size.h - 1) as usize;
            let dst_row = unsafe { ptr.add(data.offset as usize + y as usize * stride) };
            for x in 0..region.size.w {
                let sx = (region.loc.x + x).clamp(0, size.w - 1) as usize;
                let i = (sy * size.w as usize + sx) * 4;
                unsafe {
                    let d = dst_row.add(x as usize * 4);
                    *d = px[i + 2];
                    *d.add(1) = px[i + 1];
                    *d.add(2) = px[i];
                    *d.add(3) = 255;
                }
            }
        }
        Ok(())
    })
    .map_err(|e| format!("not a shared-memory buffer ({e:?})"))?
}

fn fail(p: &Pending) {
    match &p.target {
        Target::Wlr { frame, .. } => frame.failed(),
        Target::Ext { frame, .. } => frame.failed(ext_image_copy_capture_frame_v1::FailureReason::BufferConstraints),
    }
}

/// After a screen's frame: the pictures due for it are drawn and sent (the backends call this).
pub fn run_captures<R>(
    captures: &mut CaptureState,
    renderer: &mut R,
    output: &Output,
    space: &Space<WindowElement>,
    lock: Option<&LockView>,
    now: Duration,
) where
    R: Renderer + ImportAll + ImportMem + ExportMem + Offscreen<GlesRenderbuffer> + Bind<GlesRenderbuffer>,
    R::TextureId: Clone + Send + 'static,
{
    if !captures.wants(output) {
        return;
    }
    let seq = captures.seq(output);
    let mut due = Vec::new();
    captures.pending.retain(|p| {
        let alive = match &p.target {
            Target::Wlr { frame, .. } => frame.is_alive(),
            Target::Ext { frame, .. } => frame.is_alive(),
        };
        if !alive {
            return false;
        }
        if &p.output == output && p.after.is_none_or(|a| seq > a) {
            due.push(p.clone());
            return false;
        }
        true
    });
    if due.is_empty() {
        return;
    }
    let screen = match draw_screen(renderer, output, space, lock) {
        Ok(s) => s,
        Err(e) => {
            warn!(output = %output.name(), "screen picture: {e}");
            due.iter().for_each(fail);
            return;
        }
    };
    let (hi, lo, ns) = split_time(now);
    for p in due {
        if let Err(e) = fill_buffer(&p.buffer, (screen.0, &screen.1), p.region) {
            debug!("screen picture for an app: {e}");
            fail(&p);
            if let Target::Ext { session, .. } = &p.target {
                send_constraints(session, Some(output));
            }
            continue;
        }
        let (w, h) = (p.region.size.w, p.region.size.h);
        match &p.target {
            Target::Wlr { frame, with_damage } => {
                captures.taken_wlr += 1;
                frame.flags(zwlr_screencopy_frame_v1::Flags::empty());
                if *with_damage {
                    frame.damage(0, 0, w as u32, h as u32);
                }
                frame.ready(hi, lo, ns);
            }
            Target::Ext { frame, session } => {
                captures.taken_ext += 1;
                frame.transform(wl_output::Transform::Normal);
                frame.damage(0, 0, w, h);
                frame.presentation_time(hi, lo, ns);
                frame.ready();
                if let Some(d) = session.data::<Mutex<SessionData>>() {
                    d.lock().unwrap().last = Some(seq);
                }
            }
        }
    }
}

// ---- wlr-screencopy-unstable-v1

#[derive(Debug)]
pub struct WlrFrameData {
    output: Option<Output>,
    region: Rectangle<i32, Physical>,
    used: bool,
}

impl<B: Backend + 'static> GlobalDispatch<ZwlrScreencopyManagerV1, ()> for HearthState<B> {
    fn bind(
        _state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<ZwlrScreencopyManagerV1>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(client: Client, _global_data: &()) -> bool {
        not_sandboxed(&client)
    }
}

impl<B: Backend + 'static> Dispatch<ZwlrScreencopyManagerV1, ()> for HearthState<B> {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &ZwlrScreencopyManagerV1,
        request: zwlr_screencopy_manager_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        use zwlr_screencopy_manager_v1::Request;
        let (frame, output, region) = match request {
            Request::CaptureOutput { frame, output, .. } => (frame, output, None),
            Request::CaptureOutputRegion { frame, output, x, y, width, height, .. } => {
                (frame, output, Some(Rectangle::<i32, smithay::utils::Logical>::new((x, y).into(), (width, height).into())))
            }
            _ => return,
        };
        let output = Output::from_resource(&output);
        let full = output.as_ref().and_then(output_size);
        let rect = match (&output, full, region) {
            (Some(o), Some(full), Some(r)) => {
                let scale = o.current_scale().fractional_scale();
                r.to_f64()
                    .to_physical(scale)
                    .to_i32_round::<i32>()
                    .intersection(Rectangle::from_size(full))
                    .filter(|r| r.size.w > 0 && r.size.h > 0)
            }
            (Some(_), Some(full), None) => Some(Rectangle::from_size(full)),
            _ => None,
        };
        let frame = data_init.init(
            frame,
            Mutex::new(WlrFrameData {
                output: output.clone(),
                region: rect.unwrap_or_default(),
                used: false,
            }),
        );
        match rect {
            Some(r) => {
                frame.buffer(FORMAT, r.size.w as u32, r.size.h as u32, r.size.w as u32 * 4);
                if frame.version() >= 3 {
                    frame.buffer_done();
                }
            }
            None => frame.failed(),
        }
    }
}

impl<B: Backend + 'static> Dispatch<ZwlrScreencopyFrameV1, Mutex<WlrFrameData>> for HearthState<B> {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &ZwlrScreencopyFrameV1,
        request: zwlr_screencopy_frame_v1::Request,
        data: &Mutex<WlrFrameData>,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        use zwlr_screencopy_frame_v1::Request;
        let (buffer, with_damage) = match request {
            Request::Copy { buffer } => (buffer, false),
            Request::CopyWithDamage { buffer } => (buffer, true),
            _ => return,
        };
        let mut d = data.lock().unwrap();
        if d.used {
            resource.post_error(zwlr_screencopy_frame_v1::Error::AlreadyUsed, "this frame was already copied");
            return;
        }
        d.used = true;
        let Some(output) = d.output.clone() else {
            resource.failed();
            return;
        };
        let ok = smithay::wayland::shm::with_buffer_contents(&buffer, |_, _, b| {
            b.width == d.region.size.w
                && b.height == d.region.size.h
                && b.stride >= d.region.size.w * 4
                && matches!(b.format, wl_shm::Format::Xrgb8888 | wl_shm::Format::Argb8888)
        })
        .unwrap_or(false);
        if !ok {
            resource.post_error(zwlr_screencopy_frame_v1::Error::InvalidBuffer, "the buffer isn't the size and format offered");
            return;
        }
        let after = with_damage.then(|| state.captures.seq(&output));
        state.captures.pending.push(Pending {
            output,
            region: d.region,
            buffer,
            target: Target::Wlr { frame: resource.clone(), with_damage },
            after,
        });
    }

    fn destroyed(state: &mut Self, _client: ClientId, resource: &ZwlrScreencopyFrameV1, _data: &Mutex<WlrFrameData>) {
        state
            .captures
            .pending
            .retain(|p| !matches!(&p.target, Target::Wlr { frame, .. } if frame == resource));
    }
}

// ---- ext-image-capture-source-v1 (screens as sources) and ext-image-copy-capture-v1

#[derive(Debug)]
pub struct SourceData {
    output: Option<Output>,
}

#[derive(Debug)]
pub struct SessionData {
    output: Option<Output>,
    /// The screen's frame count when the last picture was sent (the next waits for a change).
    last: Option<u64>,
}

#[derive(Debug)]
pub struct FrameData {
    session: ExtImageCopyCaptureSessionV1,
    buffer: Option<WlBuffer>,
    captured: bool,
}

fn send_constraints(session: &ExtImageCopyCaptureSessionV1, output: Option<&Output>) {
    match output.and_then(output_size) {
        Some(size) => {
            session.buffer_size(size.w as u32, size.h as u32);
            session.shm_format(FORMAT);
            session.done();
        }
        None => session.stopped(),
    }
}

impl<B: Backend + 'static> GlobalDispatch<ExtOutputImageCaptureSourceManagerV1, ()> for HearthState<B> {
    fn bind(
        _state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<ExtOutputImageCaptureSourceManagerV1>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(client: Client, _global_data: &()) -> bool {
        not_sandboxed(&client)
    }
}

impl<B: Backend + 'static> Dispatch<ExtOutputImageCaptureSourceManagerV1, ()> for HearthState<B> {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &ExtOutputImageCaptureSourceManagerV1,
        request: ext_output_image_capture_source_manager_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let ext_output_image_capture_source_manager_v1::Request::CreateSource { source, output } = request {
            data_init.init(source, SourceData { output: Output::from_resource(&output) });
        }
    }
}

impl<B: Backend + 'static> Dispatch<ExtImageCaptureSourceV1, SourceData> for HearthState<B> {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &ExtImageCaptureSourceV1,
        _request: ext_image_capture_source_v1::Request,
        _data: &SourceData,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
    }
}

impl<B: Backend + 'static> GlobalDispatch<ExtImageCopyCaptureManagerV1, ()> for HearthState<B> {
    fn bind(
        _state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<ExtImageCopyCaptureManagerV1>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(client: Client, _global_data: &()) -> bool {
        not_sandboxed(&client)
    }
}

impl<B: Backend + 'static> Dispatch<ExtImageCopyCaptureManagerV1, ()> for HearthState<B> {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &ExtImageCopyCaptureManagerV1,
        request: ext_image_copy_capture_manager_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        use ext_image_copy_capture_manager_v1::Request;
        match request {
            Request::CreateSession { session, source, .. } => {
                let output = source.data::<SourceData>().and_then(|s| s.output.clone());
                let session = data_init.init(session, Mutex::new(SessionData { output: output.clone(), last: None }));
                send_constraints(&session, output.as_ref());
            }
            Request::CreatePointerCursorSession { session, .. } => {
                // The pointer on its own isn't offered yet: its capture session stops at once.
                data_init.init(session, ());
            }
            _ => {}
        }
    }
}

impl<B: Backend + 'static> Dispatch<ExtImageCopyCaptureCursorSessionV1, ()> for HearthState<B> {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &ExtImageCopyCaptureCursorSessionV1,
        request: ext_image_copy_capture_cursor_session_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let ext_image_copy_capture_cursor_session_v1::Request::GetCaptureSession { session } = request {
            let session = data_init.init(session, Mutex::new(SessionData { output: None, last: None }));
            session.stopped();
        }
    }
}

impl<B: Backend + 'static> Dispatch<ExtImageCopyCaptureSessionV1, Mutex<SessionData>> for HearthState<B> {
    fn request(
        _state: &mut Self,
        _client: &Client,
        resource: &ExtImageCopyCaptureSessionV1,
        request: ext_image_copy_capture_session_v1::Request,
        _data: &Mutex<SessionData>,
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let ext_image_copy_capture_session_v1::Request::CreateFrame { frame } = request {
            data_init.init(frame, Mutex::new(FrameData { session: resource.clone(), buffer: None, captured: false }));
        }
    }

    fn destroyed(state: &mut Self, _client: ClientId, resource: &ExtImageCopyCaptureSessionV1, _data: &Mutex<SessionData>) {
        state
            .captures
            .pending
            .retain(|p| !matches!(&p.target, Target::Ext { session, .. } if session == resource));
    }
}

impl<B: Backend + 'static> Dispatch<ExtImageCopyCaptureFrameV1, Mutex<FrameData>> for HearthState<B> {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &ExtImageCopyCaptureFrameV1,
        request: ext_image_copy_capture_frame_v1::Request,
        data: &Mutex<FrameData>,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        use ext_image_copy_capture_frame_v1::{Error, FailureReason, Request};
        let mut d = data.lock().unwrap();
        match request {
            Request::AttachBuffer { buffer } => d.buffer = Some(buffer),
            Request::DamageBuffer { .. } => {}
            Request::Capture => {
                if d.captured {
                    resource.post_error(Error::AlreadyCaptured, "this frame was already captured");
                    return;
                }
                d.captured = true;
                let Some(buffer) = d.buffer.clone() else {
                    resource.post_error(Error::NoBuffer, "no buffer attached");
                    return;
                };
                let (output, last) = match d.session.data::<Mutex<SessionData>>() {
                    Some(s) => {
                        let s = s.lock().unwrap();
                        (s.output.clone(), s.last)
                    }
                    None => (None, None),
                };
                let Some(output) = output.filter(|o| output_size(o).is_some()) else {
                    resource.failed(FailureReason::Stopped);
                    return;
                };
                let full = output_size(&output).unwrap_or_default();
                state.captures.pending.push(Pending {
                    output,
                    region: Rectangle::from_size(full),
                    buffer,
                    target: Target::Ext { frame: resource.clone(), session: d.session.clone() },
                    // The first picture at once; after that, the next change.
                    after: last,
                });
            }
            _ => {}
        }
    }

    fn destroyed(state: &mut Self, _client: ClientId, resource: &ExtImageCopyCaptureFrameV1, _data: &Mutex<FrameData>) {
        state
            .captures
            .pending
            .retain(|p| !matches!(&p.target, Target::Ext { frame, .. } if frame == resource));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_split_as_the_protocol_wants() {
        let (hi, lo, ns) = split_time(Duration::new((7u64 << 32) + 5, 123));
        assert_eq!((hi, lo, ns), (7, 5, 123));
    }
}
