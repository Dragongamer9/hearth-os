// Hearth Maps: OpenStreetMap maps for Hearth OS. The page (ui/) draws the map with MapLibre GL; everything that
// goes out to the internet goes through here so the services' rules are kept in one place (services.rs): map
// pieces and their offline copies (tiles.rs), search (search.rs), directions (route.rs). geo: links (geo.rs),
// saved places (places.rs) and "Where am I" through GeoClue (locate.rs).
pub mod geo;
pub mod locate;
pub mod places;
pub mod route;
pub mod search;
pub mod services;
pub mod tiles;
