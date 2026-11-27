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
    // Game servers, Game cache, Remote play, Media and PCs (feature/server-2): their work is Mantel's hearth-server-hub
    // (it comes with Mantel's Store part), their pages hearth_server_more's.
    Part { id: "games", order: 3, name: "Game servers", mark: "GS", grad: "gr", line: "worlds for you and your friends", color: "#9be36b", engine: "", store: "mantel" },
    Part { id: "cache", order: 5, name: "Game cache", mark: "GC", grad: "bl", line: "every game downloaded once", color: "#5ec8ff", engine: "", store: "mantel" },
    Part { id: "play", order: 6, name: "Remote play", mark: "RP", grad: "or", line: "play the PC's games anywhere", color: "#ff8a3d", engine: "", store: "mantel" },
    Part { id: "media", order: 7, name: "Media", mark: "ME", grad: "pu", line: "TV, movies, music and photos", color: "#b9a2ff", engine: "", store: "mantel" },
    Part { id: "pcs", order: 8, name: "PCs", mark: "PC", grad: "gy", line: "more PCs as one server", color: "#c9c1ba", engine: "", store: "mantel" },
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
