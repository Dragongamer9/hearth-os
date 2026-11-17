//! wlr-gamma-control-unstable-v1: a screen's colour ramp for Night light (wlsunset) and Hearth's inverted colours
//! (hearth-gamma), as on labwc. One app at a time per screen (a second one is told "failed", exactly as labwc does,
//! so Night light and inverted colours don't fight); when the app ends or lets go, the screen's ramp goes back to
//! normal at once. On real screens the ramp goes to the graphics card (udev.rs); nested in a window there's no ramp
//! to set, so apps are told "failed" and carry on without it.

use std::{collections::HashMap, os::unix::fs::FileExt};

use smithay::{
    output::Output,
    reexports::{
        wayland_protocols_wlr::gamma_control::v1::server::{
            zwlr_gamma_control_manager_v1::{self, ZwlrGammaControlManagerV1},
            zwlr_gamma_control_v1::{self, ZwlrGammaControlV1},
        },
        wayland_server::{
            backend::ClientId, Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
        },
    },
};
use tracing::{info, warn};

use crate::state::{Backend, HearthState};

#[derive(Debug, Default)]
pub struct GammaState {
    /// The app holding each screen's ramp (by screen name).
    active: HashMap<String, ZwlrGammaControlV1>,
}

#[derive(Debug)]
pub struct GammaData {
    output: Option<Output>,
    size: u32,
}

impl GammaState {
    pub fn new<B: Backend + 'static>(dh: &DisplayHandle) -> GammaState {
        dh.create_global::<HearthState<B>, ZwlrGammaControlManagerV1, _>(1, ());
        GammaState::default()
    }

    pub fn holders(&self) -> usize {
        self.active.values().filter(|c| c.is_alive()).count()
    }
}

/// A ramp as the protocol sends it: `size` reds, then greens, then blues, 16 bits each (native byte order).
pub fn parse_ramp(bytes: &[u8], size: usize) -> Option<Vec<u16>> {
    if size == 0 || bytes.len() != size * 3 * 2 {
        return None;
    }
    Some(bytes.chunks_exact(2).map(|c| u16::from_ne_bytes([c[0], c[1]])).collect())
}

/// The normal ramp (what a screen goes back to).
pub fn identity_ramp(size: usize) -> Vec<u16> {
    let one: Vec<u16> = (0..size)
        .map(|i| if size <= 1 { 0xffff } else { ((i as u64 * 0xffff) / (size as u64 - 1)) as u16 })
        .collect();
    [one.clone(), one.clone(), one].concat()
}

impl<B: Backend + 'static> HearthState<B> {
    fn release_gamma(&mut self, output_name: &str, control: &ZwlrGammaControlV1) {
        let holds = self.gamma_state.active.get(output_name).is_some_and(|c| c == control);
        if !holds {
            return;
        }
        self.gamma_state.active.remove(output_name);
        if let Some(o) = self.space.outputs().find(|o| o.name() == output_name).cloned() {
            if let Err(e) = self.backend_data.set_gamma(&o, None) {
                warn!(output = output_name, "couldn't put the screen's colours back: {e}");
            } else {
                info!(output = output_name, "screen colours back to normal (the app let go)");
            }
        }
    }
}

impl<B: Backend + 'static> GlobalDispatch<ZwlrGammaControlManagerV1, ()> for HearthState<B> {
    fn bind(
        _state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<ZwlrGammaControlManagerV1>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(client: Client, _global_data: &()) -> bool {
        client
            .get_data::<crate::ClientState>()
            .map_or(true, |c| c.security_context.is_none())
    }
}

impl<B: Backend + 'static> Dispatch<ZwlrGammaControlManagerV1, ()> for HearthState<B> {
    fn request(
        state: &mut Self,
        _client: &Client,
        _resource: &ZwlrGammaControlManagerV1,
        request: zwlr_gamma_control_manager_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let zwlr_gamma_control_manager_v1::Request::GetGammaControl { id, output } = request {
            let output = Output::from_resource(&output);
            let size = output.as_ref().and_then(|o| state.backend_data.gamma_size(o)).unwrap_or(0);
            let control = data_init.init(id, GammaData { output: output.clone(), size });
            let Some(o) = output else {
                control.failed();
                return;
            };
            let name = o.name();
            let taken = state.gamma_state.active.get(&name).is_some_and(|c| c.is_alive());
            if size == 0 || taken {
                control.failed();
                return;
            }
            state.gamma_state.active.insert(name, control.clone());
            control.gamma_size(size);
        }
    }
}

impl<B: Backend + 'static> Dispatch<ZwlrGammaControlV1, GammaData> for HearthState<B> {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &ZwlrGammaControlV1,
        request: zwlr_gamma_control_v1::Request,
        data: &GammaData,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        if let zwlr_gamma_control_v1::Request::SetGamma { fd } = request {
            let Some(o) = data.output.clone() else { return };
            if !state.gamma_state.active.get(&o.name()).is_some_and(|c| c == resource) {
                return;
            }
            let file = std::fs::File::from(fd);
            let mut bytes = vec![0u8; data.size as usize * 6];
            let ramp = file.read_exact_at(&mut bytes, 0).ok().and_then(|_| parse_ramp(&bytes, data.size as usize));
            let Some(ramp) = ramp else {
                resource.post_error(zwlr_gamma_control_v1::Error::InvalidGamma, "the ramp isn't the size given");
                return;
            };
            if let Err(e) = state.backend_data.set_gamma(&o, Some(&ramp)) {
                warn!(output = %o.name(), "couldn't set the screen's colours: {e}");
                state.release_gamma(&o.name(), resource);
                resource.failed();
            }
        }
    }

    fn destroyed(state: &mut Self, _client: ClientId, resource: &ZwlrGammaControlV1, data: &GammaData) {
        if let Some(o) = &data.output {
            let name = o.name();
            state.release_gamma(&name, resource);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramps() {
        let id = identity_ramp(256);
        assert_eq!(id.len(), 768);
        assert_eq!((id[0], id[255], id[256], id[767]), (0, 0xffff, 0, 0xffff));
        let bytes: Vec<u8> = id.iter().flat_map(|v| v.to_ne_bytes()).collect();
        assert_eq!(parse_ramp(&bytes, 256), Some(id));
        assert_eq!(parse_ramp(&bytes[..10], 256), None);
    }
}
