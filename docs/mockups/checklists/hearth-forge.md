# Hearth Forge (approved Nov 26, 2026; sources: hearth-workspace hearth/mockups/2026-11-26-hearth-forge/01-make.html, 02-models.html)

Mockups: hearth-forge/01-make.png, hearth-forge/02-models.png
Release: next

The owner, Nov 26: "Can we Build a Generator that Generats things like Tripod3d does? Would need to be able to do
realistic items and would not be public on the store", then "Make the mockups also include every model", "Hearth
Forge looks good" and "Work on Hearth Forge". A pure-Rust Hearth app (hosts/hearth-ui/apps/forge) made of blocks;
its own service on the Nvidia PC (helper/hearth-forge.py, built into the app); public in the Hearth Store (it started private; the owner, Nov 26: "push it to the public").
It ships as a Hearth Store update of the app (hearth-forge-app), not a Hearth OS release.

The boards' top bar and wallpaper are the desktop's, not this app's. Pictures: hearth-apps-rust.yml, forge.sh
(forge-1-make … forge-7-no-service).

## Every part of the boards

### Board 1 (01-make): the title bar
- [x] The HF letter mark (orange) and the name Hearth Forge — hosts/hearth-ui/apps/forge/src/main.rs: "w::mark(\"HF\", \"or\", 22.0)"
- [x] · real 3D models from a photo or a few words — hosts/hearth-ui/apps/forge/src/main.rs: "\"· real 3D models from a photo or a few words\""
- [x] ~~PRIVATE · ONLY YOUR DEVICES~~ taken off: the owner made it public (Nov 26, 23:45: "Ehh push it to the public") — hosts/hearth-ui/apps/forge/src/main.rs: "gpu_tag"
- [x] NVIDIA PC · RTX 5070 · 12 GB (the service's own nvidia-smi: its card and memory) — hosts/hearth-ui/apps/forge/src/main.rs: "format!(\"Nvidia PC · {} · {} GB\", a.card(), a.vram_gb())"
- [x] – ▢ ✕ — hosts/hearth-ui/apps/forge/src/main.rs: "TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close }"

### Board 1: Make a model
- [x] Make a model, and Photo / Words / Several photos / Words → picture (the picked one in the accent) — hosts/hearth-ui/apps/forge/src/main.rs: "(\"picture\", \"Words → picture\")"
- [x] PHOTO: the drop area with the photo, its name (old-lantern.jpg), "Background taken out by itself" and "Drop a photo · or paste · or Take one" (a dropped file, Ctrl+V, the Open window, the camera) — hosts/hearth-ui/apps/forge/src/main.rs: "\"Drop a photo · or paste · or \""
- [x] The background really taken out by itself before the model is made (each model's own cut-out: hy3dgen's BackgroundRemover, rembg) (needs testing on a device) — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "img = BackgroundRemover()(img.convert('RGB'))"
- [x] TELL IT MORE (OPTIONAL): the words, in a box to write in (kept as a draft) — hosts/hearth-ui/apps/forge/src/main.rs: "\"Tell it more (optional)\""
- [x] MODEL: "Hunyuan3D 2.0 · shape and real textures · low-memory ▾" (the installed models to pick from) — hosts/hearth-ui/apps/forge/helper/models.json: "\"pick\": \"Hunyuan3D 2.0 · shape and real textures · low-memory\""
- [x] "Fits your 12 GB by moving parts to memory · All 22 models ›" (low-memory: its painting moved to memory on a card under 20 GB; the link opens board 2) — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "paint.enable_model_cpu_offload()"
- [x] HOW REAL › Detail: Draft / Good / High / Highest (the shape's resolution and steps) — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "DETAIL = {'draft': (192, 20), 'good': (256, 30), 'high': (320, 40), 'highest': (384, 50)}"
- [x] Textures: None / Colour / Real materials (PBR) — hosts/hearth-ui/apps/forge/src/main.rs: "(\"pbr\", \"Real materials (PBR)\")"
- [x] Texture size: 1K / 2K / 4K — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "TEXSIZE = {'1k': 1024, '2k': 2048, '4k': 4096}"
- [x] Real size: 30 cm tall (the exports are made that size) — hosts/hearth-ui/apps/forge/src/main.rs: "w::input(&p, \"30 cm tall\", &real_size, 12.0, Msg::RealSize)"
- [x] Same result again (seed): 71 204 ↻ — hosts/hearth-ui/apps/forge/src/main.rs: "format!(\"{} ↻\", forge::spaced(a.seed()))"
- [x] ✦ Make the model (sent to the service on the Nvidia PC, made one at a time in a program of its own) (needs testing on a device) — hosts/hearth-ui/apps/forge/src/main.rs: "\"✦ Make the model\""
- [x] On the Nvidia PC · nothing leaves your network · free — hosts/hearth-ui/apps/forge/src/main.rs: "\"On the Nvidia PC · nothing leaves your network · free\""
- [x] Or online with Tripo3D (your key) · 2,275 credits · this one 30 (Tripo3D's own balance; its cost shown and asked before it's made; the key only in the password store) (needs testing on a device) — hosts/hearth-ui/apps/forge/src/main.rs: "format!(\"{} credits · this one {}\", forge::commas(b), a.tripo_cost())"

### Board 1: the model
- [x] Shape / Clay / Real / Wire / Textures — hosts/hearth-ui/apps/forge/src/main.rs: "(\"textures\", \"Textures\")"
- [x] ↻ Turn it — hosts/hearth-ui/apps/forge/src/main.rs: "\"↻ Turn it\""
- [x] Light: workshop ▾ (workshop, daylight, studio, candle) — hosts/hearth-ui/apps/forge/src/view3d.rs: "(\"workshop\", [255, 214, 160], [255, 160, 70], [43, 35, 32], 1.0)"
- [x] Floor — hosts/hearth-ui/apps/forge/src/main.rs: "\"Floor\""
- [x] Make again (a new seed, the same choices) — hosts/hearth-ui/apps/forge/src/main.rs: "\"Make again\""
- [x] Change it with words… (its photo changed by your words, then made again) — hosts/hearth-ui/apps/forge/src/main.rs: "\"Change it with words…\""
- [x] The model turning in the viewport, drawn in Rust: drag to turn it, scroll to zoom, right-drag to move, and the note saying so — hosts/hearth-ui/apps/forge/src/main.rs: "\"Drag to turn it · scroll to zoom · right-drag to move\""
- [x] Its card: Old iron lantern · 48 210 faces · 24 120 points · Real materials: colour, roughness, metal, bumps · 30.0 × 17.4 × 17.4 cm · closed (prints solid) · Made in 3 min 12 s · Hunyuan3D 2.0 — hosts/hearth-ui/apps/forge/src/main.rs: "\"closed (prints solid)\""
- [x] DONE · Shape 0:48 · textures 2:24 (MAKING with how far, while it's being made) — hosts/hearth-ui/apps/forge/src/main.rs: "format!(\"Shape {}\", forge::mmss(x))"
- [x] Compare with your photo (a switch) — hosts/hearth-ui/apps/forge/src/main.rs: "\"Compare with your photo\""
- [x] ITS MATERIALS (PBR): Colour, Roughness, Metal, Bumps (the model's own maps; the three worked out from its colour when it has only that) — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "out['roughness'] = Image.merge('RGB', [ImageOps.invert(grey)] * 3)"
- [x] READY IT FOR › For a game: Phone · 5K faces / Laptop · 15K / PC · 50K / Keep all (fewer faces when it's sent) (needs testing on a device: Blender) — hosts/hearth-ui/apps/forge/src/main.rs: "(\"phone\", \"Phone · 5K faces\")"
- [x] Textures for a small laptop: 1024 px / 2048 px / Keep (needs testing on a device: Blender) — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "img.scale(a[\"texture\"], a[\"texture\"])"
- [x] For printing: Make it solid / Flat bottom / Hollow, 2 mm walls (needs testing on a device: Blender) — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "mod(\"SOLIDIFY\", thickness=a[\"hollow\"] / 1000.0"

### Board 1: Send it to
- [x] Hearth 3D · open and keep working (a .blend it opens) — hosts/hearth-ui/apps/forge/src/main.rs: "run::detached(&[\"hearth-3d\", &out.to_string_lossy()]);"
- [x] Godot · glTF (.glb) · Gravesleep (into its project's assets/forge) — hosts/hearth-ui/apps/forge/src/main.rs: "forge::find_project(\"project.godot\", \"gravesleep\")"
- [x] Noclip Protocol (O3DE) · .fbx (into its project's Assets/Forge) — hosts/hearth-ui/apps/forge/src/main.rs: "forge::find_project(\"project.json\", \"noclip\")"
- [x] Hearth CAD · STL · Anycubic Chiron (ready for printing) — hosts/hearth-ui/apps/forge/src/main.rs: "target(&p, \"Hearth CAD · STL · Anycubic Chiron\""
- [x] Save as… .glb .obj .fbx .stl .usdz — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "bpy.ops.wm.usd_export(filepath=out)"
- [x] WAITING: Knight's helmet, dented · from words — next; Backrooms chair, 3 photos — 2nd (the service's queue) — hosts/hearth-ui/apps/forge/src/main.rs: "format!(\"{title} · from words\")"
- [x] MADE LATELY: Old lantern, Plague doctor mask, Yellow wall lamp, Wooden cart, Mech foot, Truck console knob (open one to see it) — hosts/hearth-ui/apps/forge/src/main.rs: "\"Made lately\""
- [x] Every model kept in ~/Hearth/Forge with its photo, words and settings · Edit layout — hosts/hearth-ui/apps/forge/src/main.rs: "\"Every model kept in ~/Hearth/Forge with its photo, words and settings · \""
- [x] Hearth Forge · Built on Hunyuan3D, TRELLIS, TripoSR and the other open models (each model's own licence) (· in the Hearth Store: public now, the owner, Nov 26) — hosts/hearth-ui/apps/forge/src/main.rs: "Built on Hunyuan3D, TRELLIS, TripoSR and the other open models (each model's own licence)"

### Board 2 (02-models): the side
- [x] · models — hosts/hearth-ui/apps/forge/src/main.rs: "\"models\" => \"· models\""
- [x] Make a model / Models 22 / Made 38 / Waiting 2 / Settings (each a page; the counts are the real ones) — hosts/hearth-ui/apps/forge/src/models.rs: "nav(&p, \"Waiting\", waiting(a).len().to_string()"
- [x] SHOW: Every model 22 / Fit my 12 GB / Installed 4 / Real materials (PBR) 4 / Free for paid work 12 (counted from the list; the board's "14" for Fit my 12 GB is one more than its own rows, 13) — hosts/hearth-ui/apps/forge/src/models.rs: "format!(\"Fit my {} GB\", a.vram_gb())"
- [x] ONLINE (YOUR KEY): Tripo3D · tripo3d.ai with your own key, kept in the system's password store · 2,275 credits left · each model shows its cost before it's made · Use it — hosts/hearth-ui/apps/forge/src/models.rs: "\"tripo3d.ai with your own key, kept in the system's password store\\n{credits}\""
- [x] Models are kept in ~/Hearth/Forge/models (31 GB used · 412 GB free) (the service's own figures) — hosts/hearth-ui/apps/forge/src/models.rs: "format!(\"Models are kept in ~/Hearth/Forge/models ({} used · {} free)\""

### Board 2: every model
- [x] Every model Hearth Forge can use — hosts/hearth-ui/apps/forge/src/models.rs: "\"Every model Hearth Forge can use\""
- [x] Find a model — hosts/hearth-ui/apps/forge/src/models.rs: "w::search(&p, \"Find a model\", &a.search, Msg::Search)"
- [x] Sort: Fits first ▾ (Fits first, By name, By maker, Installed first) — hosts/hearth-ui/apps/forge/src/models.rs: "format!(\"Sort: {sort_words} ▾\")"
- [x] Get the best 4 for my PC — hosts/hearth-ui/apps/forge/src/main.rs: "Msg::GetBest => {"
- [x] Each runs on your Nvidia PC. "Fits" is for your RTX 5070's 12 GB; a model too big here can run on a bigger PC of yours over Hearth Link. Each has its own licence, shown here. — hosts/hearth-ui/apps/forge/src/models.rs: "a model too big here can run on a bigger PC of yours over Hearth Link. Each has its own licence, shown here."
- [x] MODEL / MAKES / MEMORY IT NEEDS / ON YOUR 12 GB / LICENCE — hosts/hearth-ui/apps/forge/src/models.rs: "[\"Model\", \"Makes\", \"Memory it needs\", &format!(\"On your {vram} GB\"), \"Licence\"]"
- [x] OBJECTS · SHAPE AND TEXTURES / OBJECTS · QUICK DRAFTS AND SHAPE ONLY / PEOPLE / PLACES AND WORLDS (SPLATS) — hosts/hearth-ui/apps/forge/helper/models.json: "{\"id\": \"worlds\", \"label\": \"Places and worlds (splats)\"}"
- [x] INSTALLED · DEFAULT / INSTALLED / Get (Get: only its own files from its own Hugging Face repo at the pinned revision, each checked against its SHA-256, only when pressed; how far it is while it gets) (needs testing on a device) — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "raise Fail('%s from %s didn\\'t match its SHA-256: not kept' % (f['path'], repo))"
- [x] Hunyuan3D 2.0 · Tencent: Photo, words → mesh with textures · 6 GB shape · 16 GB with textures · Fits · low-memory, slower · Tencent community · not EU, UK, S. Korea · Get (tencent/Hunyuan3D-2) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"Hunyuan3D 2.0\", \"maker\": \"Tencent\""
- [x] Hunyuan3D 2.0 mini · Tencent: Photo → mesh with textures · smaller (0.6B shape) · Fits · Tencent community licence · Get (tencent/Hunyuan3D-2mini) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"Hunyuan3D 2.0 mini\", \"maker\": \"Tencent\""
- [x] Hunyuan3D 2.0 Turbo · Tencent: Photo → mesh, fewer steps · like 2.0 · Fits · low-memory · Tencent community licence · Get (tencent/Hunyuan3D-2) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"Hunyuan3D 2.0 Turbo\", \"maker\": \"Tencent\""
- [x] Hunyuan3D 2.1 · Tencent: Photo → mesh with real materials (PBR) · 10 GB shape · 29 GB with PBR · Shape fits · its PBR textures don't: 2.0 paints them · Tencent community licence · Get (tencent/Hunyuan3D-2.1) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"Hunyuan3D 2.1\", \"maker\": \"Tencent\""
- [x] TRELLIS.2 · Microsoft: Photo → mesh with real materials (PBR) · 24 GB · ~10 GB at 1024 · Fits at 1024 only · MIT · Get (microsoft/TRELLIS.2-4B) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"TRELLIS.2\", \"maker\": \"Microsoft\""
- [x] TRELLIS · Microsoft: Photo, words → mesh with textures · ~16 GB · Fits · low-memory, slower · MIT · Get (microsoft/TRELLIS-image-large) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"TRELLIS\", \"maker\": \"Microsoft\""
- [x] Stable Fast 3D · Stability AI: Photo → mesh with textures · ~6 GB · Fits · Stability community (free under $1M a year) · Get (Hugging Face asks you to accept Stability's licence first; your Hugging Face key in Settings) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"Stable Fast 3D\", \"maker\": \"Stability AI\""
- [x] SPAR3D · Stability AI: Photo → mesh with textures, editable points · 10.5 GB · ~7 GB low-memory · Fits · Stability community (free under $1M a year) · Get (Hugging Face asks you to accept Stability's licence first; your Hugging Face key in Settings) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"SPAR3D\", \"maker\": \"Stability AI\""
- [x] Pixal3D · Tencent ARC: Photo → mesh with real materials (PBR) · low-memory mode at 1024 · Fits · low-memory · MIT · Get (TencentARC/Pixal3D) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"Pixal3D\", \"maker\": \"Tencent ARC\""
- [x] Step1X-3D · StepFun: Photo → mesh with textures · 27 GB · Too big for 12 GB · on another PC over Hearth Link · Apache-2.0 · Get (stepfun-ai/Step1X-3D) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"Step1X-3D\", \"maker\": \"StepFun\""
- [x] TripoSR · Tripo + Stability AI: Photo → rough mesh · 6-8 GB · Fits · MIT · Get (stabilityai/TripoSR) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"TripoSR\", \"maker\": \"Tripo + Stability AI\""
- [x] TripoSG · VAST: Photo → detailed shape, no textures · over 8 GB · Fits · MIT · Get (VAST-AI/TripoSG) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"TripoSG\", \"maker\": \"VAST\""
- [x] Direct3D-S2 · Direct3D-S2 team: Photo → very fine shape, no textures · 10 GB at 512 · ~24 GB at 1024 · Fits at 512 only · MIT · Get (wushuang98/Direct3D-S2) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"Direct3D-S2\", \"maker\": \"Direct3D-S2 team\""
- [x] Hi3DGen · Hi3DGen team: Photo → sharp details (normals), no textures · not stated · Not tried on 12 GB yet · MIT · Get (Stable-X/trellis-normal-v0-1) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"Hi3DGen\", \"maker\": \"Hi3DGen team\""
- [x] Hi3D 1.0 · Hi3D: Makes another model's shape finer · not stated · Not tried on 12 GB yet · Hi3D open licence · Get (its maker's own release, hi3d-official/Hi3Dv1.0 on GitHub and Hugging Face) — hosts/hearth-ui/apps/forge/helper/models.json: "https://github.com/hi3d-official/Hi3Dv1.0.git"
- [x] UltraShape · UltraShape team: Photo → shape, no textures · not stated · Not tried on 12 GB yet · Apache-2.0 · Get (infinith/UltraShape) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"UltraShape\", \"maker\": \"UltraShape team\""
- [x] SAM 3D Objects · Meta: Pick an object in any photo → mesh · not stated · Needs Meta's OK to download · SAM licence (ask Meta for access) · Get (Meta has to say yes on Hugging Face first; your Hugging Face key in Settings) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"SAM 3D Objects\", \"maker\": \"Meta\""
- [x] SAM 3D Body · Meta: Photo of a person → posed body mesh · not stated · Needs Meta's OK to download · SAM licence (ask Meta for access) · Get (Meta has to say yes on Hugging Face first; your Hugging Face key in Settings) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"SAM 3D Body\", \"maker\": \"Meta\""
- [x] TripoSplat · VAST: Photo → splat (points, not a mesh) · not stated · Not tried on 12 GB yet · MIT · Get (VAST-AI/TripoSplat) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"TripoSplat\", \"maker\": \"VAST\""
- [x] HY-World 2.0 · Tencent: Words, photo, video → a world · not stated · Not tried on 12 GB yet · Tencent community licence · Get (tencent/HY-World-2.0) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"HY-World 2.0\", \"maker\": \"Tencent\""
- [x] Lyra 2.0 · NVIDIA: Photo → a world as splats · not stated · Not tried on 12 GB yet · NVIDIA research licence (no paid work) · Get (nvidia/Lyra-2.0) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"Lyra 2.0\", \"maker\": \"NVIDIA\""
- [x] SHARP · Apple: One photo → a scene as splats · runs on CPU or Nvidia · Fits · Research only (no commercial use) · Get (apple/Sharp) — hosts/hearth-ui/apps/forge/helper/models.json: "\"name\": \"SHARP\", \"maker\": \"Apple\""
- [x] Hearth Forge · each model credited with its own licence · numbers are each maker's own — hosts/hearth-ui/apps/forge/src/main.rs: "\"each model credited with its own licence\", \" · numbers are each maker's own\""

### Under the boards: what they promise
- [x] Made on the Nvidia PC by Hearth Forge's own user service: off until set up, no [Install], started by its timer only while on, low priority, waits while you play, 127.0.0.1 and Tailscale only (it can never stop the PC or Game Mode from starting) — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "Description=Hearth Forge: 3D models on this PC"
- [x] Making with every model: all 22, each through its maker's own code at a pinned commit with the maker's own pinned requirements, in its own Python environment (~/Hearth/Forge/runtimes/<runner>, the Python its maker names, PyTorch's CUDA 12.8 build, its parts built for the card), installed on Get with how far it is shown (needs testing on a device) — hosts/hearth-ui/apps/forge/helper/forge_runners.py: "RUNNERS = {'hunyuan3d21': hunyuan3d21, 'trellis2': trellis2"
- [x] On 12 GB with each maker's own low-memory or offload setting where it has one: Hunyuan3D 2.1's model offload, TRELLIS.2's low_vram at 1024, SPAR3D's --low-vram-mode, Pixal3D's --low_vram, Step1X-3D's geometry offload, Direct3D-S2 at 512, Hi3D's --sequential-cpu-offload, UltraShape's low-VRAM settings, WorldMirror in bf16, Lyra 2.0's --offload and --use_dmd (needs testing on a device) — hosts/hearth-ui/apps/forge/helper/forge_runners.py: "args += ['--offload', '--offload_when_prompt', '--offload_da3_diffusion', '--use_dmd']"
- [x] A model too big for this card and with no low-memory mode (SAM 3D Objects: Meta's 32 GB) isn't tried: Hearth Forge says so and sends you to your bigger PC (Settings › Where models are made); Step1X-3D makes its shape on 12 GB with its offload, and whole on a bigger PC — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "make it on a bigger PC of yours (Settings › Where models are made)"
- [x] Shape-only models painted by Hunyuan3D 2.0's painter when Textures isn't None (Hunyuan3D 2.1, TripoSG, Direct3D-S2, Hi3DGen, Hi3D 1.0, UltraShape, Step1X-3D on 12 GB), their shape kept beside it; without the painter it says to Get Hunyuan3D 2.0 (needs testing on a device) — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "if spawn(d, m['paint'], ['paint', d], cat) == 0 and os.path.isfile(glb):"
- [x] Refiners (Hi3D 1.0, UltraShape) start from another model's shape: the one you picked, or made first by the model each maker names (Hunyuan3D 2.0 mini, 2.0, 2.1) (needs testing on a device) — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "code = spawn(d, c['runner'], ['run', d, '--as', cid, '--out', 'coarse.glb', '--textures', 'none'], cat)"
- [x] Splat models (TripoSplat, HY-World 2.0, Lyra 2.0, SHARP; TRELLIS and SAM 3D Objects keep theirs too): the splat kept (splat.ply), a closed mesh with its colours made from it so Send it to works, and the splat's own points in the viewport (Splat) — hosts/hearth-ui/apps/forge/src/view3d.rs: "if self.look == Look::Splat && !m.s.is_empty()"
- [x] A model too big for 12 GB made on a bigger PC of yours over Hearth Link (each PC with Hearth Forge set up is reached by its Tailscale name in Settings › Where models are made) — hosts/hearth-ui/apps/forge/src/models.rs: "a model too big for one PC is made on a bigger PC of yours the same way."
- [x] Every model made kept in ~/Hearth/Forge/<its name> with its photo, its words and its settings (ask.json) — hosts/hearth-ui/apps/forge/helper/hearth-forge.py: "write_json(os.path.join(d, 'ask.json'), ask)"
- [x] Your choices remembered and never changed by themselves (only what you change, through settings_file: atomic, .bak, merged) — hosts/hearth-ui/apps/forge/src/main.rs: "let prefs = Prefs::load(\"forge\");"
- [x] Public in the Hearth Store (the owner, Nov 26: "push it to the public"; it started private) — hosts/linux/test/test_hearth_forge.py: "def test_in_the_public_list(self):"
- [x] Made of blocks: Edit layout, its own blocks window, Customize (hearth_ui studio) — hosts/hearth-ui/apps/forge/src/main.rs: ".with_pages(BLOCKS).with_kit(&KIT)"

## Notes (not parts of the boards)
- Weights: every model's code and git requirements are pinned to a commit; the Hugging Face weights are pinned at the first Get (huggingface.co can't be reached from where Forge is built). The boards don't show pins, so this isn't an open part.
