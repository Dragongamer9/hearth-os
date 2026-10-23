# Hearth Maps and Hearth Contacts

Two of Hearth's own apps, built like Hearth Terminal and Hearth Disks (a Rust crate with a web page, in a Tauri
window: WebKitGTK on Linux). For now they ship in the Linux editions (Anatase, Peppermint) only; Ember gets them
later.

## Hearth Maps (hosts/hearth-maps)

OpenStreetMap maps drawn with MapLibre GL JS. The built files are vendored in `ui/vendor`: run the
**Hearth Maps** workflow by hand with "vendor" ticked to download the pinned version (`vendor.sh`). Every request to a
map service goes through the Rust side (`src/services.rs`), so the services' rules are kept in one place.

| What | Service | Its rules, and how Maps keeps them |
|---|---|---|
| Map pieces (vector tiles, style, fonts, icons) | [OpenFreeMap](https://openfreemap.org) public instance, `tiles.openfreemap.org` | Free, no key, no registration, commercial use allowed. **Attribution required**: the style's own attribution (OpenFreeMap, © OpenMapTiles, data from OpenStreetMap) always shows in full in the corner. **Offline copies** are allowed up to 50 MB / 3,000 tiles, with a User-Agent naming the app and at most 4 requests at once: Maps keeps copies of the pieces you've viewed, capped at 50 MB (least recently used go first), sends `HearthMaps/<version> (Hearth OS; +https://github.com/Dragongamer9/hearth-os)` and never fetches more than 4 at once. Copies are used while the service says they're fresh, and after that only when it can't be reached. Maps never downloads areas in bulk. |
| Search and "what's here" | [Nominatim](https://nominatim.org) on `nominatim.openstreetmap.org` ([usage policy](https://operations.osmfoundation.org/policies/nominatim/)) | At most **1 request a second** for the whole app (a shared clock in `src/search.rs`); the same User-Agent; **no search-as-you-type** (only when Enter / Search is pressed); answers **cached** on disk for 30 days so a repeat isn't sent again; attribution shown with the results; and the app **can be switched off without an update** (below). |
| Directions (car, bike, on foot) | OSRM on FOSSGIS's routing server `routing.openstreetmap.de` (what openstreetmap.org uses) | At most 1 request a second, a valid User-Agent, the attribution with a link to fix the map (shown in the corner and under the directions), no heavy use: a route is only asked when Go is pressed. "Open in browser" always offers the same trip on openstreetmap.org instead, and is what's left if the service is switched off or unreachable. Full terms (German): https://www.fossgis.de/arbeitsgruppen/osm-server/nutzungsbedingungen/ |
| Where am I | GeoClue, the system's location service | Only when the button is pressed; stopped straight after; the location isn't sent anywhere by Maps. The images allow the desktop id `hearth-maps` (`/etc/geoclue/conf.d/90-hearth-maps.conf`); GeoClue still decides where the location comes from (GPS, Wi-Fi). |

**The switch.** Maps reads `config/hearth-maps-services.json` from the main branch at most once a day (the last copy
is used offline). Setting `searchEnabled`, `routingEnabled` or `tilesEnabled` to `false` there turns that service off
in every copy of Maps, with an optional `notice` to show; `nominatim`, `routing` and `tiles` can point at another
`https://` server (for example a self-hosted one). On one computer, `~/.config/hearth-maps/services.json` does the
same. `cacheMb` can lower the offline copies' size (never above 50).

Opening: `hearth-maps geo:52.5163,13.3777?z=16` (the `geo:` link handler), `hearth-maps --search "Berlin"`.

Keyboard: arrows pan and + / − zoom on the map, `/` or Ctrl+F search, Ctrl+D directions, L where am I, S saved
places, M map style, Enter "what's here" at the middle, Esc back. Controller: left stick pans, LT/RT zoom, A what's
here, Y search (on-screen keyboard), X directions, LB map / panel, B back, Menu map style, View saved places.

## Hearth Contacts (hosts/hearth-contacts)

An address book: names, phones, emails, addresses, birthday, photo, notes and groups (vCard CATEGORIES). Each
contact is a `.vcf` file in `~/.local/share/hearth-contacts/books/<book>/`. vCard 2.1, 3.0 and 4.0 are read and
3.0 is written; properties Contacts doesn't edit are kept, so nothing is lost on the way through. Import and export
`.vcf` files (a file can hold many contacts); double-clicking a `.vcf` opens Contacts with it.

**CardDAV sync** (optional, Accounts): Nextcloud, iCloud, Fastmail or any CardDAV server. The address books are found
from the server's address (well-known URL, principal, address book home). Changes go both ways; when a contact
changed on both sides, the server's version is kept and this computer's is saved next to it. The person signs in
themselves; the password goes straight into the system keyring (Secret Service: GNOME Keyring, which Hearth's
session starts) and is only ever sent to that server over HTTPS. iCloud and Fastmail need an **app password**
(iCloud: account.apple.com › Sign-In and Security › App-Specific Passwords; Fastmail: Settings › Privacy & Security
› Manage app passwords). Nothing needs registering by the owner.

Command line: `--import FILE`, `--export FILE`, `--list`.

Keyboard: arrows move, Enter opens, E edits, Ctrl+S saves, Delete deletes, Ctrl+N new, Ctrl+F or `/` search,
Ctrl+I import, Ctrl+E export, F5 sync, Esc back. Controller: A choose, B back, X new, Y search, Menu import /
export / accounts; an on-screen keyboard comes up for typing.

## CI

`.github/workflows/contacts.yml`: unit tests, vCard round trips (Google, iCloud, Android 2.1, RFC 6350 files), sync
against Radicale started in the run, passwords in a real GNOME Keyring, build, screenshots.
`.github/workflows/maps.yml`: unit tests, a light check against the real services (the styles, a tile, one search,
one route), build, screenshots. The images build both apps (base-anatase.yml, base-peppermint.yml).
