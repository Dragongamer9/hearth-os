# Hearth Home (Home Assistant, Apache-2.0) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/14-home.html)

Mockups: open-source-apps/14-home.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps/home), made of blocks (studio: Edit layout, Customize, its own
blocks window), built on the open-source project the board credits: Home Assistant on the Nvidia PC (hearth-oss-server,
its own image in rootless Podman, 127.0.0.1 and Tailscale only) through its own REST API.

Board: Hearth Home (Home Assistant, Apache-2.0): a big TV-friendly smart home dashboard.
Pictures: hearth-apps-rust.yml, home.sh (home-1-board, home-2-edit-board, home-0-sign-in).

## Every part of the board

### The top
- [x] The HM letter mark (44 px) and Hearth Home — hosts/hearth-ui/apps/home/src/main.rs: "w::mark(\"HM\", \"gr\", 44.0)"
- [x] Home (in the accent), Rooms, Cameras, Energy, Routines — hosts/hearth-ui/apps/home/src/main.rs: "tab(\"routines\", \"Routines\")"
- [x] The time (1:24 AM, light) and "Thursday · 41° clear · on the TV" (Home Assistant's weather) — hosts/hearth-ui/apps/home/src/main.rs: "format!(\"{}{weather} · on {on}\", DAYS[now.wday])"

### Lights
- [x] LIGHTS · 5 OF 11 ON and the room (Living room; the one you pick, else the one with most on) — hosts/hearth-ui/apps/home/src/main.rs: "format!(\"Lights · {on} of {} on\", all.len())"
- [x] Each light: its glowing square (dark when off), its name, "warm · 60%" / "follows the picture" / "off · on at sunset" / "on · 2 h", its big switch (A turns it on or off) — hosts/hearth-ui/apps/home/src/ha.rs: "format!(\"on · {h} h\")"
- [x] BRIGHTNESS · LIVING ROOM: the big slider and 60% (◀ ▶ on the controller; Home Assistant's light.turn_on for the room) — hosts/hearth-ui/apps/home/src/main.rs: "json!({\"area_id\": room, \"brightness_pct\": v})"
- [x] OTHER ROOMS: Bedroom (1 light on · 20%), Office (2 on · Hearth Lights), Kitchen (all off), Garage (1 on · door shut), each opened with a press — hosts/hearth-ui/apps/home/src/main.rs: "0 => \"all off\".to_string(),"
- [x] The picked tile outlined in the accent (where the controller is) — hosts/hearth-ui/apps/home/src/main.rs: "Some((p.accent, 4.0))"

### Thermostat
- [x] THERMOSTAT · HALLWAY, its dial (the arc to the setting, the white knob), 70° and "Heating · now 67°F" — hosts/hearth-ui/apps/home/src/main.rs: "format!(\"{doing} · now {}{unit}\", now_t.round())"
- [x] − Heat Cool Off ＋ (climate.set_temperature, set_hvac_mode) — hosts/hearth-ui/apps/home/src/main.rs: "a.call(\"climate\", \"set_hvac_mode\""

### Cameras
- [x] CAMERAS and ALL QUIET (its motion sensors) — hosts/hearth-ui/apps/home/src/main.rs: "if quiet { \"All quiet\" } else { \"Movement now\" }"
- [x] Driveway and Front door, each with ● LIVE and its name (Home Assistant's camera pictures, again every 10 s) — hosts/hearth-ui/apps/home/src/main.rs: "format!(\"/api/camera_proxy/{id}\")"
- [x] Moving video in the camera tiles: Home Assistant's camera_proxy_stream (MJPEG) read while the tile is shown, 8 pictures a second (CI pictures home-3-moving-a and -b, a second apart, differ; needs testing on a device) — hosts/hearth-ui/apps/home/src/mjpeg.rs: "pub fn frames(c: &Cam) -> Frames"
- [x] "Last movement: Driveway · 12:52 AM · a cat" — hosts/hearth-ui/apps/home/src/main.rs: "format!(\"Last movement: {} · {}{what}\""

### Plugs and energy
- [x] PLUGS and "412 W now" (their power sensors) — hosts/hearth-ui/apps/home/src/main.rs: "format!(\"{} W now\", total.round())"
- [x] Xbox and TV 96 W, Space heater 300 W, 3D printer "16 W · idle", Coffee maker "on at 5:45 AM", each with its dot and switch — hosts/hearth-ui/apps/home/src/main.rs: "format!(\"{} W · idle\", x.round())"
- [x] ENERGY TODAY · 6.4 KWH and the hour bars (the energy sensor's history), 12 AM … now — hosts/hearth-ui/apps/home/src/main.rs: "format!(\"Energy today · {kwh:.1} kWh\")"

### Scenes and routines
- [x] SCENES: Movie night (the last one used, lit), Game night, Leaving for work, Bedtime, each with its words, pressed to turn it on — hosts/hearth-ui/apps/home/src/main.rs: "a.call(\"scene\", \"turn_on\""
- [x] ROUTINES COMING UP: Coffee maker on 5:45 AM, Heat to 70° before you're home 6:15 AM, Porch light off at sunrise 7:02 AM (each automation's time trigger) — hosts/hearth-ui/apps/home/src/ha.rs: "pub fn next_time(cfg: &Value) -> Option<i64>"

### The foot, the credit, the blocks
- [x] "Hearth Home · runs on the Nvidia PC · on the TV, the Ally and the phone" — hosts/hearth-ui/apps/home/src/main.rs: "· on the TV, the Ally and the phone"
- [x] A Turn on / off, ◀ ▶ Brightness, Y Scenes, B Back (and they work: hearth-pad's keys) — hosts/hearth-ui/apps/home/src/main.rs: "hint(pb(\"Y\", true), \"Scenes\")"
- [x] "Hearth Home · Built on Home Assistant (Apache-2.0)" — hosts/hearth-ui/apps/home/src/main.rs: "w::credit(&p, \"Hearth Home\", \"Built on Home Assistant (Apache-2.0)\")"
- [x] Every part a block (2 pages, 8 blocks), Edit layout in the foot, its own blocks window (9 buttons, 3 panes, 4 info) — hosts/hearth-ui/apps/home/src/main.rs: "name: \"Home\","
- [x] Home Assistant installed and turned on the Hearth way when it isn't there (its image in your Podman, its user service guarded), the token kept in the keyring — os/system_files/usr/lib/systemd/user/hearth-home-assistant.service: "ConditionPathExists=%h/.local/state/nimbus/oss-server/homeassistant.ready"
