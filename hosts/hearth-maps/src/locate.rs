// "Where am I": asks GeoClue (the system's location service) once, only when the person presses the button, and
// stops it straight after. GeoClue decides where the location comes from (GPS, Wi-Fi, ...) and whether Hearth Maps
// may have it; the images allow the desktop id "hearth-maps" (/etc/geoclue/conf.d/90-hearth-maps.conf).
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Fix {
    pub lat: f64,
    pub lon: f64,
    /// Metres.
    pub accuracy: f64,
}

#[cfg(target_os = "linux")]
mod gc {
    use zbus::zvariant::OwnedObjectPath;

    #[zbus::proxy(interface = "org.freedesktop.GeoClue2.Manager", default_service = "org.freedesktop.GeoClue2", default_path = "/org/freedesktop/GeoClue2/Manager")]
    pub trait Manager {
        fn get_client(&self) -> zbus::Result<OwnedObjectPath>;
        fn delete_client(&self, client: &zbus::zvariant::ObjectPath<'_>) -> zbus::Result<()>;
    }

    #[zbus::proxy(interface = "org.freedesktop.GeoClue2.Client", default_service = "org.freedesktop.GeoClue2")]
    pub trait Client {
        fn start(&self) -> zbus::Result<()>;
        fn stop(&self) -> zbus::Result<()>;
        #[zbus(property)]
        fn location(&self) -> zbus::Result<OwnedObjectPath>;
        #[zbus(property)]
        fn desktop_id(&self) -> zbus::Result<String>;
        #[zbus(property)]
        fn set_desktop_id(&self, id: &str) -> zbus::Result<()>;
        #[zbus(property)]
        fn requested_accuracy_level(&self) -> zbus::Result<u32>;
        #[zbus(property)]
        fn set_requested_accuracy_level(&self, level: u32) -> zbus::Result<()>;
    }

    #[zbus::proxy(interface = "org.freedesktop.GeoClue2.Location", default_service = "org.freedesktop.GeoClue2")]
    pub trait Location {
        #[zbus(property)]
        fn latitude(&self) -> zbus::Result<f64>;
        #[zbus(property)]
        fn longitude(&self) -> zbus::Result<f64>;
        #[zbus(property)]
        fn accuracy(&self) -> zbus::Result<f64>;
    }
}

#[cfg(target_os = "linux")]
pub fn locate(timeout_secs: u64) -> Result<Fix, String> {
    use zbus::proxy::CacheProperties;
    let none = |e: zbus::Error| format!("The system's location service isn't available ({e}).");
    let conn = zbus::blocking::Connection::system().map_err(none)?;
    let mgr = gc::ManagerProxyBlocking::new(&conn).map_err(none)?;
    let path = mgr.get_client().map_err(none)?;
    let client = gc::ClientProxyBlocking::builder(&conn).path(path.clone()).map_err(none)?.cache_properties(CacheProperties::No).build().map_err(none)?;
    client.set_desktop_id("hearth-maps").map_err(none)?;
    // 8: exact (GPS when there is one); GeoClue gives less if that's all it has or all Hearth Maps is allowed.
    client.set_requested_accuracy_level(8).map_err(none)?;
    client.start().map_err(|e| format!("The location service said no: {e}. (Settings › Privacy may have location turned off.)"))?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let result = loop {
        match client.location() {
            Ok(p) if p.as_str() != "/" => {
                let loc = gc::LocationProxyBlocking::builder(&conn).path(p).map_err(none)?.cache_properties(CacheProperties::No).build().map_err(none)?;
                break Ok(Fix { lat: loc.latitude().map_err(none)?, lon: loc.longitude().map_err(none)?, accuracy: loc.accuracy().unwrap_or(0.0) });
            }
            _ if std::time::Instant::now() > deadline => break Err("The system didn't find where you are in time. It needs Wi-Fi turned on or a GPS.".to_string()),
            _ => std::thread::sleep(std::time::Duration::from_millis(400)),
        }
    };
    let _ = client.stop();
    let _ = mgr.delete_client(&path);
    result
}

#[cfg(not(target_os = "linux"))]
pub fn locate(_timeout_secs: u64) -> Result<Fix, String> {
    Err("Where am I isn't available on this system yet.".into())
}
