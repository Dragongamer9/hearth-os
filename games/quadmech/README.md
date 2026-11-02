# Quad Mech (Godot 4.3+)

MechWarrior-style prototype built around a slow, heavy, four-legged mech, pushed
as far as Godot's Forward+ renderer goes.

## Running it

- **Play:** run `QuadMech.exe` (Windows) or `QuadMech.x86_64` (Linux).
- **Edit:** open `project.godot` in Godot 4.3+ and press F5.

## Multiplayer (Hearth OS multi-seat / couch co-op test)

From the menu, each player presses **A** (controller) or **Enter** (keyboard) to join,
up to 4 per copy of the game. Then:

- **Play Local**: split-screen on this screen (1 = full, 2 = side by side, 3-4 = grid).
- **Host LAN Game**: starts a match other seats and machines can join. Players can drop in
  mid-match.
- **Join LAN Game**: lists games found on the network *and on other seats of this same
  machine*. You can also type an address.
- **Mode**: Co-op (team vs the turrets, no friendly fire) or PvP deathmatch.

LAN play and split-screen combine: two seats with two players each makes a 4-mech battle.
Ports: **7777** (game, UDP) and **7778** (discovery, UDP).

### Launch options (for Hearth)

Add these after `--`, e.g. `QuadMech.x86_64 -- --host --mode=pvp`:

| Option | What it does |
|---|---|
| `--local` | Start split-screen right away |
| `--host` | Host a LAN match right away |
| `--join=auto` / `--join=192.168.1.20` | Join the first game found, or a given address |
| `--players=auto` / `--players=kbm,0,1` | Who plays on this copy: all connected controllers (default), or a list of devices (`kbm` = keyboard & mouse, numbers = controller ids) |
| `--mode=coop` / `--mode=pvp` | Game mode (when hosting or local) |
| `--name=Seat1` | Player name (extra local players get "Seat1 2", ...) |
| `--quality=low` / `--quality=high` | Graphics preset (3-4 local players default to Low) |
| `--selftest=30` | Automated test: mechs walk, aim and fire by themselves; prints what this seat sees every 2 s, then quits |

Example multi-seat test on one machine: seat 1 runs `-- --host --name=Seat1`, seat 2 runs
`-- --join=auto --name=Seat2`.

**Seat Info** (F3, the Xbox button, or the menu) shows what this copy of the game sees:
renderer and GPU, window and screen, seat/session environment variables, every controller
it can see (and which player uses it), and the network roster. That makes it easy to check
that each seat only gets its own controller.

## Controls

| Keyboard & mouse | Controller | Action |
|---|---|---|
| W / S | Left stick up / down | Throttle up / down (it stays where you set it) |
| A / D | Left stick left / right | Turn the legs |
| Mouse | Right stick (LT = precision) | Aim the torso |
| Left mouse | RT or RB | Fire autocannons |
| B | B | Brace / siege mode |
| X | X | Full stop |
| C | L3 (click left stick) | Point the view back along the legs |
| V | Y | Cockpit / chase camera |
| Mouse wheel | D-pad up / down | Zoom chase camera |
| H | View / Back | Show / hide controls |
| F3 | Xbox button | Seat info |
| Esc | Menu / Start | Pause menu |
| F2 | | Graphics quality High / Low |

## What's in it

**Mech:** procedural IK legs (one-at-a-time walk, diagonal trot), hull that tilts with the
ground, hydraulic pistons, three-toed feet, stabilized 360° turret, heat with overheat
shutdown, recoiling twin autocannons, bolt rows, lockers, exhaust stacks, working headlights,
glowing vents and barrels, and heat shimmer over the exhausts and hot barrels.

**World:** textured terrain (grass, soil with pebbles, cracked lichen-covered rock) blended
by height with anti-tiling, a lake with refraction, depth color, foam and ripples (mech feet
splash), branch-card pine forests, wind-blown grass that takes its color from the ground,
reeds along the shore, sculpted boulders, a fuel depot, a comm station and ruins.

**Lighting and camera:** golden-hour sun with soft shadows, cloudy sky with drifting cloud
shadows, SDFGI global illumination, SSAO, SSIL, screen-space reflections, volumetric fog with
god rays and low mist banks, ACES tonemapping, auto exposure, distance blur, temporal AA,
film grain, lens fringing and vignette.

**Effects:** textured dust and smoke, sparks, muzzle flashes and smoke, explosions with
debris, scorch marks and footprints.

**F2 / Low quality** turns off GI, SSAO, SSIL, reflections and volumetric fog and shortens
shadow and grass distance, for slower GPUs. On machines without Vulkan, Godot falls back to
OpenGL automatically; the game still runs, with simpler water and no heat shimmer.

## Files

| Path | What it does |
|---|---|
| `scripts/quad_mech.gd` | Movement, gaits, body tilt, turret, heat, weapons, mech model |
| `scripts/leg.gd` | One leg: step animation and IK visuals |
| `scripts/terrain.gd` | Terrain, lake basin, water, boulders |
| `scripts/world_dressing.gd` | Outposts, pine trees, grass |
| `scripts/main.gd` | Sky, lighting, post-processing, mist, split-screen views, spawning, launch options |
| `scripts/net.gd` | LAN host/join, discovery, roster, sync of mechs, shots, damage and scores |
| `scripts/player_input.gd` | One player's controls from one device (keyboard/mouse or a controller) |
| `scripts/menu.gd` | Join screen, LAN browser, pause menu, seat diagnostics |
| `scripts/fx.gd` | Dust, smoke, sparks, splashes, explosions, decals |
| `scripts/hud.gd`, `scripts/camera_rig.gd` | HUD; chase/cockpit camera and shake |
| `shaders/` | Armor, terrain, water, foliage, sky, heat haze, lens post-process |
| `textures/` | Generated PBR texture sets (see `tools/gen_textures.py` to regenerate) |
