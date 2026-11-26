# Hearth 3D (Blender, GPL-2+) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/17-3d.html)

Mockups: open-source-apps/17-3d.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps), made of blocks (studio: Edit layout, Customize, its own blocks
window), built on the open-source project the board credits.

Board: Hearth 3D (Blender, GPL-2+): modelling with the viewport, scene tree and properties.

## The window
- [x] Title bar: the 3D mark, Hearth 3D, · quad-mech.blend, File / Edit / Add / Object / Render, Send to Godot (glTF), Send to Hearth CAD (STL) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"Send to Hearth CAD\""
- [x] Model / Sculpt / Paint / Animate / Render (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "const MODES: [&str; 5]"
- [x] Sculpt and Paint working: brushes on the picked mesh in the Rust viewport (Draw, Dig, Smooth, Flatten; Paint's colours), Blender writes the points and the paint (its colour attribute) into the .blend (CI pictures oss-a-17-hearth-3d-sculpt, -paint, real strokes; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "fn brush(&mut self, at: [f32; 3], r: f32, look: [f32; 3])"
- [x] Made of blocks (Edit layout), with its own buttons, panes and info (CI picture oss-a-17-hearth-3d-editing; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "const KIT: Kit = Kit {"
- [x] The credit: Hearth 3D · Built on Blender (GPL-2+) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"Built on Blender (GPL-2+)\""
- [x] Blender as the engine in the background (reads and saves the .blend, renders, imports, exports) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/scene.rs: "pub fn script(edits: &str, save: bool)"
- [x] Blender checked for, Get Blender (Flathub) when it isn't here (needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "const BLENDER: Need"

## The viewport
- [x] The tools down the left (select, move, turn, size; box select, the camera's view, frame the picked, the start view) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"Look through the camera\""
- [x] Box select picking by a drawn box (Shift adds; in Tab it picks points) (CI picture oss-a-17-hearth-3d-box, a real drag; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/view3d.rs: "fn objects_in(&self, r: Rectangle, size: Size) -> Vec<String>"
- [x] The scene drawn in Rust: lit faces, the floor's grid, the shadow, the picked one outlined (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/view3d.rs: "impl<'a> Viewport<'a> {"
- [x] Move's arrows on the picked object; G move, R turn, S size, dragging it (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/view3d.rs: "Move's arrows on the picked object"
- [x] User view · perspective (flat with 5) · Collection: Mech › Body (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"User view · perspective\""
- [x] Wire / Solid / Material / Rendered (Blender's render of the frame) · Overlays ▾ (CI picture oss-a-17-hearth-3d, -wire; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "sh(Shade::Rendered, \"Rendered\")"
- [x] The axes in the corner (X, Y, Z) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/view3d.rs: "the axes in the corner"
- [x] Body · 2,412 faces · 4,880 points / Scene: 18,904 faces · objects picked of all (CI picture oss-a-17-hearth-3d; needs testing on a device; the board's "6 objects picked of 23" counts a bigger scene than the test file) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"Scene: {} faces · {} objects picked of {}\""
- [x] "G move · R turn · S size · Tab edit · numbers 1–9 views" (the numbers' views work) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"G move · R turn · S size · Tab edit · numbers 1–9 views\""
- [x] Tab edit (a mesh's points): its own points, edges and faces, picked by click or box, moved (G / drag), sized (S), pulled out (E) and deleted (X) by Blender's bmesh, saved into the .blend (CI picture oss-a-17-hearth-3d-edit; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/scene.rs: "pub fn apply_code(file: &str, then: &str, picked: &[u32], amount: f32) -> String"

## Scene and properties
- [x] SCENE · Find · Scene collection › Mech (Body, Cockpit, Gun, ▸ Legs · 8), Lights (Key light, Rim light), Camera, Ground, each with ◉ ▣ (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "fn tree_rows<"
- [x] Object / Changes / Material / Scene / World (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "[\"Object\", \"Changes\", \"Material\", \"Scene\", \"World\"]"
- [x] WHERE IT IS · BODY: Place, Turn, Size with X Y Z (typed, saved by Blender) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "fn xyz<"
- [x] CHANGES ON TOP · 3: Mirror · X, Bevel · 2 cm · 3 steps, Smooth by angle · 30°, each ◉ ⌄ · + Add a change (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"+ Add a change\""

## Timeline
- [x] ⏮ ▶ ⏭ · Walk cycle · frame 32 of 96 · 24 fps · Keys: Legs · + Key (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "format!(\"{} · frame {} of {} · {} fps\""
- [x] The track: a line a frame, the keys as diamonds, the frame marker with its number; a press sets the frame, ▶ plays it (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "struct Track<"

## The window
- [x] Title bar: the 3D mark, Hearth 3D, · quad-mech.blend, File / Edit / Add / Object / Render, Send to Godot (glTF), Send to Hearth CAD (STL) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"Send to Hearth CAD\""
- [x] Model / Sculpt / Paint / Animate / Render (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "const MODES: [&str; 5]"
- [x] Sculpt and Paint working: brushes on the picked mesh in the Rust viewport (Draw, Dig, Smooth, Flatten; Paint's colours), Blender writes the points and the paint (its colour attribute) into the .blend (CI pictures oss-a-17-hearth-3d-sculpt, -paint, real strokes; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "fn brush(&mut self, at: [f32; 3], r: f32, look: [f32; 3])"
- [x] Made of blocks (Edit layout), with its own buttons, panes and info (CI picture oss-a-17-hearth-3d-editing; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "const KIT: Kit = Kit {"
- [x] The credit: Hearth 3D · Built on Blender (GPL-2+) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"Built on Blender (GPL-2+)\""
- [x] Blender as the engine in the background (reads and saves the .blend, renders, imports, exports) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/scene.rs: "pub fn script(edits: &str, save: bool)"
- [x] Blender checked for, Get Blender (Flathub) when it isn't here (needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "const BLENDER: Need"

## The viewport
- [x] The tools down the left (select, move, turn, size; box select, the camera's view, frame the picked, the start view) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"Look through the camera\""
- [x] Box select picking by a drawn box (Shift adds; in Tab it picks points) (CI picture oss-a-17-hearth-3d-box, a real drag; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/view3d.rs: "fn objects_in(&self, r: Rectangle, size: Size) -> Vec<String>"
- [x] The scene drawn in Rust: lit faces, the floor's grid, the shadow, the picked one outlined (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/view3d.rs: "impl<'a> Viewport<'a> {"
- [x] Move's arrows on the picked object; G move, R turn, S size, dragging it (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/view3d.rs: "Move's arrows on the picked object"
- [x] User view · perspective (flat with 5) · Collection: Mech › Body (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"User view · perspective\""
- [x] Wire / Solid / Material / Rendered (Blender's render of the frame) · Overlays ▾ (CI picture oss-a-17-hearth-3d, -wire; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "sh(Shade::Rendered, \"Rendered\")"
- [x] The axes in the corner (X, Y, Z) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/view3d.rs: "the axes in the corner"
- [x] Body · 2,412 faces · 4,880 points / Scene: 18,904 faces · objects picked of all (CI picture oss-a-17-hearth-3d; needs testing on a device; the board's "6 objects picked of 23" counts a bigger scene than the test file) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"Scene: {} faces · {} objects picked of {}\""
- [x] "G move · R turn · S size · Tab edit · numbers 1–9 views" (the numbers' views work) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"G move · R turn · S size · Tab edit · numbers 1–9 views\""
- [x] Tab edit (a mesh's points): its own points, edges and faces, picked by click or box, moved (G / drag), sized (S), pulled out (E) and deleted (X) by Blender's bmesh, saved into the .blend (CI picture oss-a-17-hearth-3d-edit; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/scene.rs: "pub fn apply_code(file: &str, then: &str, picked: &[u32], amount: f32) -> String"

## Scene and properties
- [x] SCENE · Find · Scene collection › Mech (Body, Cockpit, Gun, ▸ Legs · 8), Lights (Key light, Rim light), Camera, Ground, each with ◉ ▣ (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "fn tree_rows<"
- [x] Object / Changes / Material / Scene / World (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "[\"Object\", \"Changes\", \"Material\", \"Scene\", \"World\"]"
- [x] WHERE IT IS · BODY: Place, Turn, Size with X Y Z (typed, saved by Blender) (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "fn xyz<"
- [x] CHANGES ON TOP · 3: Mirror · X, Bevel · 2 cm · 3 steps, Smooth by angle · 30°, each ◉ ⌄ · + Add a change (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "\"+ Add a change\""

## Timeline
- [x] ⏮ ▶ ⏭ · Walk cycle · frame 32 of 96 · 24 fps · Keys: Legs · + Key (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "format!(\"{} · frame {} of {} · {} fps\""
- [x] The track: a line a frame, the keys as diamonds, the frame marker with its number; a press sets the frame, ▶ plays it (CI picture oss-a-17-hearth-3d; needs testing on a device) — hosts/hearth-ui/apps/hearth-3d/src/main.rs: "struct Track<"
