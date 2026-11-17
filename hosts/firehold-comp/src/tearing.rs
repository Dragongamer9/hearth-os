//! tearing-control-v1: games say they'd rather tear than wait for the screen (no vsync). The hint is kept per
//! surface (`wants_tearing`), so games that ask for it find the protocol as on KWin; the frames themselves still wait
//! for the screen until Smithay can flip asynchronously (it can't in 0.7: docs/FIREHOLD.md, stage 3).

use std::sync::Mutex;

use smithay::{
    reexports::{
        wayland_protocols::wp::tearing_control::v1::server::{
            wp_tearing_control_manager_v1::{self, WpTearingControlManagerV1},
            wp_tearing_control_v1::{self, WpTearingControlV1},
        },
        wayland_server::{
            backend::ClientId, protocol::wl_surface::WlSurface, Client, DataInit, Dispatch, DisplayHandle,
            GlobalDispatch, New, Resource, WEnum,
        },
    },
    wayland::compositor::with_states,
};

use crate::state::{Backend, HearthState};

/// On a surface: whether it asked to tear, and whether it has a tearing-control object.
#[derive(Debug, Default)]
struct TearingHint {
    tear: bool,
    taken: bool,
}

pub fn init<B: Backend + 'static>(dh: &DisplayHandle) {
    dh.create_global::<HearthState<B>, WpTearingControlManagerV1, _>(1, ());
}

/// Whether the surface asked for tearing (the game's "no vsync").
pub fn wants_tearing(surface: &WlSurface) -> bool {
    with_states(surface, |s| s.data_map.get::<Mutex<TearingHint>>().is_some_and(|h| h.lock().unwrap().tear))
}

fn with_hint<T>(surface: &WlSurface, f: impl FnOnce(&mut TearingHint) -> T) -> T {
    with_states(surface, |s| {
        s.data_map.insert_if_missing_threadsafe(|| Mutex::new(TearingHint::default()));
        f(&mut s.data_map.get::<Mutex<TearingHint>>().unwrap().lock().unwrap())
    })
}

impl<B: Backend + 'static> GlobalDispatch<WpTearingControlManagerV1, ()> for HearthState<B> {
    fn bind(
        _state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<WpTearingControlManagerV1>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }
}

impl<B: Backend + 'static> Dispatch<WpTearingControlManagerV1, ()> for HearthState<B> {
    fn request(
        _state: &mut Self,
        _client: &Client,
        resource: &WpTearingControlManagerV1,
        request: wp_tearing_control_manager_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let wp_tearing_control_manager_v1::Request::GetTearingControl { id, surface } = request {
            let taken = with_hint(&surface, |h| std::mem::replace(&mut h.taken, true));
            if taken {
                resource.post_error(
                    wp_tearing_control_manager_v1::Error::TearingControlExists,
                    "this surface already has a tearing control",
                );
                return;
            }
            data_init.init(id, surface.downgrade());
        }
    }
}

impl<B: Backend + 'static> Dispatch<WpTearingControlV1, smithay::reexports::wayland_server::Weak<WlSurface>>
    for HearthState<B>
{
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &WpTearingControlV1,
        request: wp_tearing_control_v1::Request,
        surface: &smithay::reexports::wayland_server::Weak<WlSurface>,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        if let wp_tearing_control_v1::Request::SetPresentationHint { hint } = request {
            if let Ok(s) = surface.upgrade() {
                let tear = matches!(hint, WEnum::Value(wp_tearing_control_v1::PresentationHint::Async));
                with_hint(&s, |h| h.tear = tear);
            }
        }
    }

    fn destroyed(
        _state: &mut Self,
        _client: ClientId,
        _resource: &WpTearingControlV1,
        surface: &smithay::reexports::wayland_server::Weak<WlSurface>,
    ) {
        if let Ok(s) = surface.upgrade() {
            with_hint(&s, |h| *h = TearingHint::default());
        }
    }
}
