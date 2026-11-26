//! The server's parts. The Home server helper's sidebar (after Server home) and Settings › Home server's tiles are
//! both made from [`PARTS`], in `order`: a new part is one line here, its engine in
//! os/system_files/usr/libexec/nimbus/hearth-server-parts/<engine>.py, and its page in apps/home-server/src/pages.
//!
//! Each part arrives and updates through the Hearth Store (the owner, Nov 26: "All this has to be updated thru the
//! store"): its engine rides in the Home server helper's system part (`store`), its pages in the Home server helper
//! app's part; anything it needs from the system (a service, the firewall) it installs itself, with your password once.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Part {
    /// its id, and its page in the Home server helper
    pub id: &'static str,
    /// where it sits in the sidebar and among the tiles (Server home is 0)
    pub order: u8,
    pub name: &'static str,
    /// its letter mark and colour (the boards' .mark g-xx)
    pub mark: &'static str,
    pub grad: &'static str,
    /// the tile's line under its name
    pub line: &'static str,
    /// the tile's graph colour
    pub color: &'static str,
    /// its engine: hearth-server-parts/<engine>.py ("" none)
    pub engine: &'static str,
    /// the Hearth Store part its engine updates with
    pub store: &'static str,
}

/// The Home server helper app's own Store part (its window and every part's page).
pub const APP_PART: &str = "hearth-home-server-app";
/// The Store system part the engines ride in (root-owned, installed with the admin password, docs/STORE-SYSTEM-PARTS.md).
pub const HELPER_PART: &str = "hearth-home-server-helper";

pub const PARTS: &[Part] = &[
    Part { id: "dash", order: 1, name: "Dashboard", mark: "DB", grad: "or", line: "every service, its health and fixes", color: "#ff8a3d", engine: "dash", store: HELPER_PART },
    Part { id: "vault", order: 2, name: "Model vault", mark: "MV", grad: "pu", line: "your own copy of every AI model", color: "#b9a2ff", engine: "vault", store: HELPER_PART },
    Part { id: "backup", order: 4, name: "Backups", mark: "BK", grad: "bl", line: "every device, every night", color: "#5ec8ff", engine: "backups", store: HELPER_PART },
    // Game servers (3), Game cache (5), Remote play (6), Media (7) and PCs (8) are feature/server-2's: one line each here.
];

/// The parts in their order.
pub fn sorted() -> Vec<&'static Part> {
    let mut v: Vec<&'static Part> = PARTS.iter().collect();
    v.sort_by_key(|p| p.order);
    v
}

pub fn by_id(id: &str) -> Option<&'static Part> {
    PARTS.iter().find(|p| p.id == id)
}
