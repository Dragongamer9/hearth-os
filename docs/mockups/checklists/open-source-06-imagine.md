# Hearth Imagine (ComfyUI, GPL-3) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/06-imagine.html)

Mockups: open-source-apps/06-imagine.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps/imagine), made of blocks (studio: Edit layout, Customize, its own
blocks window), built on the open-source project the board credits: ComfyUI's own local HTTP API and WebSocket, on the
Nvidia PC where the Home server helper runs it (hearth-comfyui.service, port 8188).

Board: Hearth Imagine (ComfyUI, GPL-3): Type and make beside the node graph, results and the queue on the Nvidia PC.
The board's top bar and wallpaper are the desktop's, not this app's. Pictures: hearth-apps-rust.yml, imagine.sh
(imagine-1-board … imagine-6-no-comfyui).

## Every part of the board

### The title bar
- [x] The IM letter mark (purple) and the name Hearth Imagine — hosts/hearth-ui/apps/imagine/src/main.rs: "w::mark(\"IM\", \"pu\", 22.0)"
- [x] Type and make / Both / Node graph: which sides show (kept) — hosts/hearth-ui/apps/imagine/src/main.rs: "(\"Type and make\".to_string(), vm == \"type\", Some(Msg::View(\"type\")))"
- [x] NVIDIA PC · RTX 5070 · 12 GB (ComfyUI's own /system_stats: its graphics card and memory) — hosts/hearth-ui/apps/imagine/src/main.rs: "format!(\"Nvidia PC · {}{}\", a.card"
- [x] – ▢ ✕ — hosts/hearth-ui/apps/imagine/src/main.rs: "TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close }"

### Type and make
- [x] Type and make, and "Say what you want. The graph on the right is what it runs; change it there if you like." — hosts/hearth-ui/apps/imagine/src/main.rs: "Say what you want. The graph on the right is what it runs; change it there if you like."
- [x] WHAT TO MAKE: what you asked for, in a box to write in — hosts/hearth-ui/apps/imagine/src/main.rs: "text_editor(&a.prompt).on_action(Msg::Prompt)"
- [x] LEAVE OUT (people, text, blurry): ComfyUI's negative words — hosts/hearth-ui/apps/imagine/src/comfy.rs: "\"7\": {\"class_type\": \"CLIPTextEncode\", \"inputs\": {\"text\": a.leave_out"
- [x] LOOK: Film, Painting, Game art, Pixel, Wallpaper, Sketch (the picked one in the accent; each adds its own words) — hosts/hearth-ui/apps/imagine/src/comfy.rs: "(\"Film\", \"soft film grain, warm tones, cinematic light\", \"soft grain · warm\")"
- [x] SIZE: 16:9 / 9:16 / 1:1 / 4:3 and 1344 × 768 — hosts/hearth-ui/apps/imagine/src/comfy.rs: "(\"16:9\", 1344, 768), (\"9:16\", 768, 1344)"
- [x] How many: 1 2 4 8 — hosts/hearth-ui/apps/imagine/src/main.rs: "w::kv(&p, \"How many\""
- [x] Model "Hearth XL 1.0 ▾" (ComfyUI's own checkpoints, from /object_info) — hosts/hearth-ui/apps/imagine/src/main.rs: "\"/object_info/CheckpointLoaderSimple\""
- [x] Steps slider and 28 — hosts/hearth-ui/apps/imagine/src/main.rs: "slider(1.0..=60.0, a.steps_live, Msg::Steps)"
- [x] Same result again (seed) "482 113 ↻" (↻ a new one) — hosts/hearth-ui/apps/imagine/src/main.rs: "w::kv(&p, \"Same result again (seed)\""
- [x] ✦ Make 4 pictures: each its own run in ComfyUI's queue, so each comes back as soon as it's done — hosts/hearth-ui/apps/imagine/src/main.rs: "format!(\"✦ Make {n} {}\""
- [x] "About 14 s on the Nvidia PC · nothing leaves your network" (from how long a step took last time) — hosts/hearth-ui/apps/imagine/src/main.rs: "About {secs} s {where_} · nothing leaves your network"
- [x] USED LATELY: what you asked for lately, each opened again with › — hosts/hearth-ui/apps/imagine/src/main.rs: "w::cap(&p, \"Used lately\")"

### The node graph
- [x] The dotted ground — hosts/hearth-ui/apps/imagine/src/graph.rs: "// the dotted ground"
- [x] Load model (Hearth XL 1.0; model, text reader, picture maker with their dots) — hosts/hearth-ui/apps/imagine/src/main.rs: "(\"text reader\".into(), w::yellow())"
- [x] Describe it (your words; words ●) — hosts/hearth-ui/apps/imagine/src/main.rs: "n(\"describe\", \"Describe it\", w::yellow(), 420.0, 290.0, 260.0)"
- [x] Leave out (people, text, blurry) — hosts/hearth-ui/apps/imagine/src/main.rs: "n(\"leave\", \"Leave out\", w::yellow(), 420.0, 440.0, 260.0)"
- [x] Empty picture (1344 × 768 · 4 at once) — hosts/hearth-ui/apps/imagine/src/main.rs: "format!(\"{} × {} · {count} at once\", ask.w, ask.h)"
- [x] Look: Film (soft grain · warm) — hosts/hearth-ui/apps/imagine/src/main.rs: "n.title = format!(\"Look: {look}\");"
- [x] Make, RUNNING, steps 28 · guide 6.5, seed 482 113, its bar and "step 18 of 28" (ComfyUI's /ws progress) — hosts/hearth-ui/apps/imagine/src/graph.rs: "format!(\"step {at} of {of}\")"
- [x] Turn into pictures, then save to ~/Pictures/Imagine — hosts/hearth-ui/apps/imagine/src/main.rs: "(\"~/Pictures/Imagine\".into(), false)"
- [x] The wires in their colours (purple, yellow, blue, orange, pink, green) — hosts/hearth-ui/apps/imagine/src/main.rs: "wv(\"look\", 50.0, \"describe\", 70.0, w::lime())"
- [x] ＋ Add a step (Bigger: ComfyUI's upscaler after Make) — hosts/hearth-ui/apps/imagine/src/main.rs: "\"＋ Add a step\""
- [x] Start from ▾ (Hearth's own, and your own saved ones) — hosts/hearth-ui/apps/imagine/src/main.rs: "\"Hearth's own: words to a picture\".to_string()"
- [x] Save as my own (with its ComfyUI graph) — hosts/hearth-ui/apps/imagine/src/main.rs: "\"comfyui\": comfy::graph(&a.ask(a.seed()), \"Imagine/picture\")"
- [x] − 86 % ＋ (and the wheel), steps dragged to move them, the ground to look around — hosts/hearth-ui/apps/imagine/src/main.rs: "format!(\"{} %\", (a.zoom * 100.0).round())"

### Results
- [x] Results, "this one · 4" — hosts/hearth-ui/apps/imagine/src/main.rs: "format!(\"this one · {n_set}\")"
- [x] The pictures as they're made: "1 · done", "2 · done", "3 · step 18" (and the striped ones still coming) — hosts/hearth-ui/apps/imagine/src/main.rs: "format!(\"{} · step {}\", i + 1, pc.step.0)"
- [x] Make more like 1 (picture to picture from it) — hosts/hearth-ui/apps/imagine/src/main.rs: "format!(\"Make more like {k}\")"
- [x] Bigger (ComfyUI's upscaler on it) — hosts/hearth-ui/apps/imagine/src/comfy.rs: "\"ImageUpscaleWithModel\", \"inputs\": {\"upscale_model\": [\"10\", 0], \"image\": [\"12\", 0]}"
- [x] Set as wallpaper — hosts/hearth-ui/apps/imagine/src/main.rs: "\"plasma-apply-wallpaperimage\""
- [x] WAITING: what else is in ComfyUI's queue, "next", "2nd" — hosts/hearth-ui/apps/imagine/src/main.rs: "0 => \"next\".to_string(),"
- [x] EARLIER: the last six pictures from ComfyUI's history — hosts/hearth-ui/apps/imagine/src/main.rs: "w::cap(&p, \"Earlier\")"
- [x] MODELS ON THE NVIDIA PC with their sizes (Hearth XL 1.0 · 6.9 GB, Fast 1-step · 4.1 GB, Upscaler 4× · 64 MB) — hosts/hearth-ui/apps/imagine/src/main.rs: "w::cap(&p, \"Models on the Nvidia PC\")"

### The credit, the blocks, and ComfyUI itself
- [x] "Hearth Imagine · Built on ComfyUI (GPL-3)" — hosts/hearth-ui/apps/imagine/src/main.rs: "w::credit(&p, \"Hearth Imagine\", \"Built on ComfyUI (GPL-3)\")"
- [x] Every part a block (4 pages, 21 blocks), Edit layout in Type and make, its own blocks window (12 buttons, 3 panes, 4 info) — hosts/hearth-ui/apps/imagine/src/main.rs: "name: \"Imagine\","
- [x] ComfyUI installed and turned on the Hearth way when it isn't there (the Home server helper's pictures-install, picturesOn) — hosts/hearth-ui/apps/imagine/src/main.rs: "\"pictures-install\""
