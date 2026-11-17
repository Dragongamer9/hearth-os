//! wlr-output-management-unstable-v1 (v4): the screens for wlr-randr, so Settings › Display (SetDisplay.qml) and the
//! autostart's saved display.sh work on Albireo as on labwc: each screen's name, make, model and serial number (from
//! its EDID, udev.rs), its modes, the one in use, its position, rotation and scale; and changes back (a mode, a
//! position, a rotation, a scale), tried first with "test" or made with "apply".
//!
//! Safe by design: nothing here is saved (the next start is the same as before; Settings saves display.sh itself),
//! a screen is never turned off from here yet (that would leave nothing to draw on: refused), and a mode the screen
//! doesn't list is refused. Nested in a window, the window's size is the mode and there's no rotation.

use std::sync::{Arc, Mutex};

use smithay::{
    desktop::layer_map_for_output,
    output::{Mode, Output, Scale},
    reexports::{
        wayland_protocols_wlr::output_management::v1::server::{
            zwlr_output_configuration_head_v1::{self, ZwlrOutputConfigurationHeadV1},
            zwlr_output_configuration_v1::{self, ZwlrOutputConfigurationV1},
            zwlr_output_head_v1::{self, ZwlrOutputHeadV1},
            zwlr_output_manager_v1::{self, ZwlrOutputManagerV1},
            zwlr_output_mode_v1::{self, ZwlrOutputModeV1},
        },
        wayland_server::{
            backend::ClientId, protocol::wl_output, Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New,
            Resource, WEnum,
        },
    },
    utils::{Point, Transform},
};
use tracing::{info, warn};

use crate::state::{Backend, HearthState};

/// One screen as the protocol describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct HeadInfo {
    pub name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub physical: (i32, i32),
    pub modes: Vec<Mode>,
    pub current: Option<Mode>,
    pub preferred: Option<Mode>,
    pub position: (i32, i32),
    pub transform: Transform,
    pub scale: f64,
}

/// The screen's serial number, when its EDID had one (udev.rs puts it in the output's user data).
#[derive(Debug, Clone, Default)]
pub struct OutputSerial(pub String);

struct Instance {
    manager: ZwlrOutputManagerV1,
    heads: Vec<(ZwlrOutputHeadV1, Vec<ZwlrOutputModeV1>)>,
}

#[derive(Default)]
pub struct OutputManagementState {
    instances: Vec<Instance>,
    serial: u32,
    last: Vec<HeadInfo>,
    /// Changes asked for (applied after this round of requests, outside the protocol's dispatch).
    pub requests: Vec<(ZwlrOutputConfigurationV1, bool)>,
}

impl std::fmt::Debug for OutputManagementState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputManagementState")
            .field("listeners", &self.instances.len())
            .field("serial", &self.serial)
            .finish()
    }
}

#[derive(Debug, Default, Clone)]
pub struct HeadChanges {
    pub mode: Option<Mode>,
    pub custom: Option<(i32, i32, i32)>,
    pub position: Option<(i32, i32)>,
    pub transform: Option<Transform>,
    pub scale: Option<f64>,
}

#[derive(Debug)]
pub struct ConfigData {
    serial: u32,
    used: bool,
    /// Each screen named in the configuration: its changes, or None to turn it off.
    heads: Vec<(String, Option<Arc<Mutex<HeadChanges>>>)>,
}

fn transform_from(t: wl_output::Transform) -> Transform {
    use wl_output::Transform as W;
    match t {
        W::_90 => Transform::_90,
        W::_180 => Transform::_180,
        W::_270 => Transform::_270,
        W::Flipped => Transform::Flipped,
        W::Flipped90 => Transform::Flipped90,
        W::Flipped180 => Transform::Flipped180,
        W::Flipped270 => Transform::Flipped270,
        _ => Transform::Normal,
    }
}

fn transform_to(t: Transform) -> wl_output::Transform {
    use wl_output::Transform as W;
    match t {
        Transform::Normal => W::Normal,
        Transform::_90 => W::_90,
        Transform::_180 => W::_180,
        Transform::_270 => W::_270,
        Transform::Flipped => W::Flipped,
        Transform::Flipped90 => W::Flipped90,
        Transform::Flipped180 => W::Flipped180,
        Transform::Flipped270 => W::Flipped270,
    }
}

/// The listed mode a request means: exactly, or the same size within half a hertz (wlr-randr rounds).
pub fn match_mode(modes: &[Mode], w: i32, h: i32, refresh: i32) -> Option<Mode> {
    let same = modes.iter().filter(|m| m.size.w == w && m.size.h == h);
    if refresh <= 0 {
        return same.max_by_key(|m| m.refresh).copied();
    }
    same.filter(|m| (m.refresh - refresh).abs() <= 500)
        .min_by_key(|m| (m.refresh - refresh).abs())
        .copied()
}

impl OutputManagementState {
    pub fn new<B: Backend + 'static>(dh: &DisplayHandle) -> OutputManagementState {
        dh.create_global::<HearthState<B>, ZwlrOutputManagerV1, _>(4, ());
        OutputManagementState::default()
    }

    fn announce<B: Backend + 'static>(dh: &DisplayHandle, inst: &mut Instance, heads: &[HeadInfo], serial: u32) {
        let Some(client) = inst.manager.client() else { return };
        let version = inst.manager.version();
        for info in heads {
            let Ok(head) =
                client.create_resource::<ZwlrOutputHeadV1, String, HearthState<B>>(dh, version, info.name.clone())
            else {
                continue;
            };
            inst.manager.head(&head);
            head.name(info.name.clone());
            head.description(info.description.clone());
            if info.physical.0 > 0 && info.physical.1 > 0 {
                head.physical_size(info.physical.0, info.physical.1);
            }
            let mut modes = Vec::new();
            for m in &info.modes {
                let Ok(mr) =
                    client.create_resource::<ZwlrOutputModeV1, Mode, HearthState<B>>(dh, version.min(3), *m)
                else {
                    continue;
                };
                head.mode(&mr);
                mr.size(m.size.w, m.size.h);
                if m.refresh > 0 {
                    mr.refresh(m.refresh);
                }
                if info.preferred == Some(*m) {
                    mr.preferred();
                }
                modes.push(mr);
            }
            head.enabled(1);
            if let Some(cur) = info.current {
                if let Some(i) = info.modes.iter().position(|m| *m == cur) {
                    head.current_mode(&modes[i]);
                }
            }
            head.position(info.position.0, info.position.1);
            head.transform(transform_to(info.transform));
            head.scale(info.scale);
            if version >= 2 {
                head.make(info.make.clone());
                head.model(info.model.clone());
                if !info.serial.is_empty() {
                    head.serial_number(info.serial.clone());
                }
            }
            if version >= 4 {
                head.adaptive_sync(zwlr_output_head_v1::AdaptiveSyncState::Disabled);
            }
            inst.heads.push((head, modes));
        }
        inst.manager.done(serial);
    }

    /// The screens now; when anything changed, every listener is told again (old heads finished, new ones sent).
    pub fn refresh<B: Backend + 'static>(&mut self, dh: &DisplayHandle, heads: Vec<HeadInfo>) {
        self.instances.retain(|i| i.manager.is_alive());
        if heads == self.last {
            return;
        }
        self.serial = self.serial.wrapping_add(1);
        for inst in &mut self.instances {
            for (h, modes) in inst.heads.drain(..) {
                for m in modes {
                    if m.is_alive() {
                        m.finished();
                    }
                }
                if h.is_alive() {
                    h.finished();
                }
            }
            Self::announce::<B>(dh, inst, &heads, self.serial);
        }
        self.last = heads;
    }
}

impl<B: Backend + 'static> HearthState<B> {
    pub fn head_infos(&self) -> Vec<HeadInfo> {
        self.space
            .outputs()
            .map(|o| {
                let p = o.physical_properties();
                let geo = self.space.output_geometry(o).unwrap_or_default();
                let mut modes = o.modes();
                if let Some(c) = o.current_mode() {
                    if !modes.contains(&c) {
                        modes.push(c);
                    }
                }
                let serial = o.user_data().get::<OutputSerial>().map(|s| s.0.clone()).unwrap_or_default();
                // Nested, the window's GL flip isn't a rotation anyone asked for.
                let transform = if B::NESTED { Transform::Normal } else { o.current_transform() };
                HeadInfo {
                    name: o.name(),
                    description: format!("{} {} ({})", p.make, p.model, o.name()),
                    make: p.make.clone(),
                    model: p.model.clone(),
                    serial,
                    physical: (p.size.w, p.size.h),
                    modes,
                    current: o.current_mode(),
                    preferred: o.preferred_mode(),
                    position: (geo.loc.x, geo.loc.y),
                    transform,
                    scale: o.current_scale().fractional_scale(),
                }
            })
            .collect()
    }

    /// Checks (and, when `apply`, makes) a configuration. Err: why it can't be done.
    fn configure_outputs(&mut self, data: &ConfigData, apply: bool) -> Result<(), String> {
        if data.serial != self.output_management_state.serial {
            return Err("cancelled".into());
        }
        let outputs: Vec<Output> = self.space.outputs().cloned().collect();
        let mut plan: Vec<(Output, HeadChanges)> = Vec::new();
        for (name, changes) in &data.heads {
            let Some(o) = outputs.iter().find(|o| &o.name() == name) else {
                return Err(format!("no screen {name}"));
            };
            let Some(changes) = changes else {
                return Err(format!("turning {name} off isn't supported yet"));
            };
            let mut c = changes.lock().unwrap().clone();
            if let Some((w, h, r)) = c.custom.take() {
                let m = match_mode(&o.modes(), w, h, r).ok_or_else(|| format!("{name} doesn't list {w}x{h}@{r}"))?;
                c.mode = Some(m);
            }
            if let Some(m) = c.mode {
                let current = o.current_mode();
                if B::NESTED && Some(m) != current {
                    return Err("nested, the window's size is the screen's mode".into());
                }
                if !B::NESTED && !o.modes().contains(&m) && Some(m) != current {
                    return Err(format!("{name} doesn't list that mode"));
                }
            }
            if let Some(t) = c.transform {
                if B::NESTED && t != Transform::Normal {
                    return Err("nested, the screen can't be rotated".into());
                }
            }
            if let Some(s) = c.scale {
                if !(0.25..=8.0).contains(&s) || !s.is_finite() {
                    return Err(format!("scale {s} is out of range"));
                }
            }
            plan.push((o.clone(), c));
        }
        if !apply {
            return Ok(());
        }
        for (o, c) in plan {
            if let Some(m) = c.mode {
                if Some(m) != o.current_mode() {
                    self.backend_data.set_output_mode(&o, m)?;
                    o.change_current_state(Some(m), None, None, None);
                }
            }
            let transform = if B::NESTED { None } else { c.transform };
            let pos = c.position.map(|(x, y)| Point::from((x, y)));
            o.change_current_state(None, transform, c.scale.map(Scale::Fractional), pos);
            if let Some(p) = pos {
                self.space.map_output(&o, p);
            }
            layer_map_for_output(&o).arrange();
            self.backend_data.reset_buffers(&o);
            self.lock_output_changed(&o);
            info!(output = %o.name(), mode = ?o.current_mode(), scale = o.current_scale().fractional_scale(),
                  transform = ?o.current_transform(), "screen changed (output management)");
        }
        crate::shell::fixup_positions(&mut self.space, self.pointer.current_location());
        Ok(())
    }

    /// After every round of requests (wm.rs): changes asked for are made, then every listener hears what's on.
    pub fn output_management_tick(&mut self) {
        let requests = std::mem::take(&mut self.output_management_state.requests);
        for (config, apply) in requests {
            if !config.is_alive() {
                continue;
            }
            let Some(data) = config.data::<Mutex<ConfigData>>() else { continue };
            let snapshot = {
                let d = data.lock().unwrap();
                ConfigData { serial: d.serial, used: d.used, heads: d.heads.clone() }
            };
            match self.configure_outputs(&snapshot, apply) {
                Ok(()) => config.succeeded(),
                Err(e) if e == "cancelled" => config.cancelled(),
                Err(e) => {
                    warn!("screen change refused: {e}");
                    config.failed();
                }
            }
        }
        let heads = self.head_infos();
        let dh = self.display_handle.clone();
        self.output_management_state.refresh::<B>(&dh, heads);
    }
}

impl<B: Backend + 'static> GlobalDispatch<ZwlrOutputManagerV1, ()> for HearthState<B> {
    fn bind(
        state: &mut Self,
        dh: &DisplayHandle,
        _client: &Client,
        resource: New<ZwlrOutputManagerV1>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        let manager = data_init.init(resource, ());
        let mut inst = Instance { manager, heads: Vec::new() };
        let heads = state.head_infos();
        let serial = state.output_management_state.serial;
        OutputManagementState::announce::<B>(dh, &mut inst, &heads, serial);
        state.output_management_state.instances.push(inst);
    }

    fn can_view(client: Client, _global_data: &()) -> bool {
        client
            .get_data::<crate::ClientState>()
            .map_or(true, |c| c.security_context.is_none())
    }
}

impl<B: Backend + 'static> Dispatch<ZwlrOutputManagerV1, ()> for HearthState<B> {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &ZwlrOutputManagerV1,
        request: zwlr_output_manager_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            zwlr_output_manager_v1::Request::CreateConfiguration { id, serial } => {
                data_init.init(id, Mutex::new(ConfigData { serial, used: false, heads: Vec::new() }));
            }
            zwlr_output_manager_v1::Request::Stop => {
                state.output_management_state.instances.retain(|i| &i.manager != resource);
                resource.finished();
            }
            _ => {}
        }
    }

    fn destroyed(state: &mut Self, _client: ClientId, resource: &ZwlrOutputManagerV1, _data: &()) {
        state.output_management_state.instances.retain(|i| &i.manager != resource);
    }
}

impl<B: Backend + 'static> Dispatch<ZwlrOutputHeadV1, String> for HearthState<B> {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &ZwlrOutputHeadV1,
        _request: zwlr_output_head_v1::Request,
        _data: &String,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
    }
}

impl<B: Backend + 'static> Dispatch<ZwlrOutputModeV1, Mode> for HearthState<B> {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &ZwlrOutputModeV1,
        _request: zwlr_output_mode_v1::Request,
        _data: &Mode,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
    }
}

impl<B: Backend + 'static> Dispatch<ZwlrOutputConfigurationV1, Mutex<ConfigData>> for HearthState<B> {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &ZwlrOutputConfigurationV1,
        request: zwlr_output_configuration_v1::Request,
        data: &Mutex<ConfigData>,
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        use zwlr_output_configuration_v1::{Error, Request};
        let mut d = data.lock().unwrap();
        match request {
            Request::EnableHead { id, head } => {
                let name = head.data::<String>().cloned().unwrap_or_default();
                if d.heads.iter().any(|(n, _)| *n == name) {
                    resource.post_error(Error::AlreadyConfiguredHead, "that screen is already in this configuration");
                    return;
                }
                let changes = Arc::new(Mutex::new(HeadChanges::default()));
                data_init.init(id, changes.clone());
                d.heads.push((name, Some(changes)));
            }
            Request::DisableHead { head } => {
                let name = head.data::<String>().cloned().unwrap_or_default();
                if d.heads.iter().any(|(n, _)| *n == name) {
                    resource.post_error(Error::AlreadyConfiguredHead, "that screen is already in this configuration");
                    return;
                }
                d.heads.push((name, None));
            }
            Request::Apply | Request::Test => {
                if d.used {
                    resource.post_error(Error::AlreadyUsed, "this configuration was already used");
                    return;
                }
                d.used = true;
                let apply = matches!(request, Request::Apply);
                state.output_management_state.requests.push((resource.clone(), apply));
            }
            _ => {}
        }
    }
}

impl<B: Backend + 'static> Dispatch<ZwlrOutputConfigurationHeadV1, Arc<Mutex<HeadChanges>>> for HearthState<B> {
    fn request(
        _state: &mut Self,
        _client: &Client,
        resource: &ZwlrOutputConfigurationHeadV1,
        request: zwlr_output_configuration_head_v1::Request,
        data: &Arc<Mutex<HeadChanges>>,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        use zwlr_output_configuration_head_v1::{Error, Request};
        let mut c = data.lock().unwrap();
        match request {
            Request::SetMode { mode } => {
                if c.mode.is_some() || c.custom.is_some() {
                    resource.post_error(Error::AlreadySet, "the mode was already set");
                    return;
                }
                match mode.data::<Mode>() {
                    Some(m) => c.mode = Some(*m),
                    None => resource.post_error(Error::InvalidMode, "not a mode of this screen"),
                }
            }
            Request::SetCustomMode { width, height, refresh } => {
                if c.mode.is_some() || c.custom.is_some() {
                    resource.post_error(Error::AlreadySet, "the mode was already set");
                    return;
                }
                if width <= 0 || height <= 0 || refresh < 0 {
                    resource.post_error(Error::InvalidCustomMode, "that mode can't be");
                    return;
                }
                c.custom = Some((width, height, refresh));
            }
            Request::SetPosition { x, y } => {
                if c.position.is_some() {
                    resource.post_error(Error::AlreadySet, "the position was already set");
                    return;
                }
                c.position = Some((x, y));
            }
            Request::SetTransform { transform } => {
                if c.transform.is_some() {
                    resource.post_error(Error::AlreadySet, "the rotation was already set");
                    return;
                }
                match transform {
                    WEnum::Value(t) => c.transform = Some(transform_from(t)),
                    WEnum::Unknown(_) => resource.post_error(Error::InvalidTransform, "not a rotation"),
                }
            }
            Request::SetScale { scale } => {
                if c.scale.is_some() {
                    resource.post_error(Error::AlreadySet, "the scale was already set");
                    return;
                }
                if !(scale > 0.0) || !scale.is_finite() {
                    resource.post_error(Error::InvalidScale, "not a scale");
                    return;
                }
                c.scale = Some(scale);
            }
            Request::SetAdaptiveSync { .. } => {
                // VRR comes with stage 3: asked-for "on" is ignored rather than refused (the mode still applies).
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(w: i32, h: i32, r: i32) -> Mode {
        Mode { size: (w, h).into(), refresh: r }
    }

    #[test]
    fn modes_match_like_wlr_randr_asks() {
        let modes = vec![m(1920, 1080, 60000), m(1920, 1080, 143981), m(1280, 720, 60000)];
        assert_eq!(match_mode(&modes, 1920, 1080, 144000), Some(m(1920, 1080, 143981)));
        assert_eq!(match_mode(&modes, 1920, 1080, 0), Some(m(1920, 1080, 143981)));
        assert_eq!(match_mode(&modes, 1280, 720, 60000), Some(m(1280, 720, 60000)));
        assert_eq!(match_mode(&modes, 1280, 720, 75000), None);
        assert_eq!(match_mode(&modes, 800, 600, 60000), None);
    }

    #[test]
    fn rotations_go_both_ways() {
        for t in [Transform::Normal, Transform::_90, Transform::Flipped270] {
            assert_eq!(transform_from(transform_to(t)), t);
        }
    }
}
