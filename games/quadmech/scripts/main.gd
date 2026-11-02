extends Node3D
## App entry. Builds the world (sky, terrain, outposts, turrets), shows the menu
## over a slow flyover, then runs a match: one mech per player in the roster, and
## a split-screen view (own camera, HUD and lens pass) for every player on this
## machine. Local, LAN host and LAN client all use the same code via Net.

const FX = preload("res://scripts/fx.gd")
const Terrain = preload("res://scripts/terrain.gd")
const Dressing = preload("res://scripts/world_dressing.gd")
const QuadMech = preload("res://scripts/quad_mech.gd")
const CameraRig = preload("res://scripts/camera_rig.gd")
const Hud = preload("res://scripts/hud.gd")
const Target = preload("res://scripts/target.gd")
const PlayerInput = preload("res://scripts/player_input.gd")
const Menu = preload("res://scripts/menu.gd")

var terrain
var dressing
var env: Environment
var sun: DirectionalLight3D
var high_quality := true
var targets: Array = []
var mechs := {}          # mech_id -> mech
var views: Array = []    # one per local player: {container, viewport, rig, hud, input}
var menu
var in_match := false
var args := {}

var _menu_cam: Camera3D
var _menu_t := 0.0
var _view_layer: CanvasLayer
var _grid: GridContainer


func _ready() -> void:
	_setup_input()
	_setup_environment()

	terrain = Terrain.new()
	add_child(terrain)
	dressing = Dressing.new()
	add_child(dressing)
	dressing.build(terrain)
	_add_mist()
	_spawn_targets()

	_menu_cam = Camera3D.new()
	_menu_cam.fov = 60.0
	_menu_cam.far = 2000.0
	add_child(_menu_cam)
	_menu_cam.make_current()

	_view_layer = CanvasLayer.new()
	_view_layer.layer = 0
	add_child(_view_layer)
	_grid = GridContainer.new()
	_grid.set_anchors_preset(Control.PRESET_FULL_RECT)
	_grid.add_theme_constant_override("h_separation", 4)
	_grid.add_theme_constant_override("v_separation", 4)
	_grid.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_view_layer.add_child(_grid)

	Net.game = self
	Net.roster_changed.connect(_sync_mechs)
	Net.disconnected.connect(_on_disconnected)

	menu = Menu.new()
	menu.main = self
	add_child(menu)

	_parse_args()
	if args.has("quality"):
		set_quality(args.quality != "low")
	if args.has("host") or args.has("join") or args.has("local"):
		menu.join_from_args(args)
		if args.has("local"):
			start_match("local")
		elif args.has("host"):
			start_match("host")
		elif args.join == "auto":
			menu.show_browser(true)
		else:
			start_match("join", args.join)
	else:
		menu.show_main()
	if args.has("selftest"):
		_selftest(float(args.selftest) if args.selftest.is_valid_float() else 20.0)


## --selftest[=seconds]: drives this seat's mechs automatically (walk, aim at the
## nearest other mech, fire) and prints what this copy of the game sees. Handy
## for automated Hearth OS seat tests. Quits when done.
func _selftest(secs: float) -> void:
	var t := 0.0
	var report := 0.0
	var tag: String = args.get("name", "seat")
	while t < secs:
		await get_tree().physics_frame
		var dt := get_physics_process_delta_time()
		t += dt
		report -= dt
		for id in mechs:
			var m = mechs[id]
			if not m.is_local or m.dead:
				continue
			m.throttle = 0.35
			var best = null
			var bd := INF
			for oid in mechs:
				var o = mechs[oid]
				if o != m and not o.dead:
					var d: float = o.global_position.distance_to(m.global_position)
					if d < bd:
						bd = d
						best = o
			if best != null:
				m.aim_target = best.global_position + Vector3(0, 1.0, 0)
				m.has_aim = true
				if fmod(t, 0.25) < dt and bd < 120.0:
					m._fire()
		if report <= 0.0:
			report = 2.0
			var parts: PackedStringArray = []
			for id in mechs:
				var m = mechs[id]
				parts.append("%s[%s]%s armor=%d%s" % [m.player_name, id, "L" if m.is_local else "R", int(m.armor), " DEAD" if m.dead else ""])
			print("[%s t=%.0f] %s peer=%d roster=%d mechs=%d team=%d scores=%s | %s" % [tag, t, Net.role_label(), Net.my_peer(), Net.roster.size(), mechs.size(), Net.team_score, str(Net.scores), ", ".join(parts)])
	print("[%s] selftest done" % tag)
	get_tree().quit()


func _parse_args() -> void:
	var all := OS.get_cmdline_args() + OS.get_cmdline_user_args()
	for a in all:
		if not a.begins_with("--"):
			continue
		var kv: PackedStringArray = a.substr(2).split("=", true, 1)
		args[kv[0]] = kv[1] if kv.size() > 1 else ""


func _process(delta: float) -> void:
	if in_match:
		return
	# Slow cinematic flyover behind the menu.
	_menu_t += delta * 0.025
	var r := 95.0
	var p := Vector3(cos(_menu_t) * r, 0.0, sin(_menu_t) * r)
	p.y = maxf(terrain.height_at(p.x, p.z), terrain.water_level) + 22.0
	_menu_cam.global_position = p
	_menu_cam.look_at(Vector3(0, terrain.height_at(0, 0) + 6.0, 0), Vector3.UP)


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("quality"):
		set_quality(not high_quality)
	elif in_match and event is InputEventMouseButton and event.pressed and not menu.is_open() \
			and Input.mouse_mode != Input.MOUSE_MODE_CAPTURED and _has_kbm():
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
		get_viewport().set_input_as_handled()


# ---------------------------------------------------------------- match

## kind: "local", "host" or "join". Net.local_players must already be filled in.
func start_match(kind: String, ip: String = "") -> void:
	var ok := true
	match kind:
		"local":
			Net.start_local()
		"host":
			ok = Net.host_game()
		"join":
			ok = Net.join_game(ip)
	if not ok:
		menu.show_main()
		return
	in_match = true
	if Net.local_players.size() >= 3 and not args.has("quality"):
		set_quality(false)  # 3-4 views at once: lighter settings
	_build_views()
	_sync_mechs()
	menu.hide_all()
	_capture_mouse(true)


func leave_match() -> void:
	Net.leave()
	get_tree().reload_current_scene()


func _on_disconnected() -> void:
	if in_match:
		menu.show_message("Disconnected from host", 2.5)
		await get_tree().create_timer(2.5).timeout
		leave_match()


func _has_kbm() -> bool:
	for v in views:
		if v.input.is_kbm():
			return true
	return false


func _capture_mouse(on: bool) -> void:
	if on and _has_kbm():
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
	else:
		Input.mouse_mode = Input.MOUSE_MODE_VISIBLE


func set_inputs_enabled(on: bool) -> void:
	for v in views:
		v.input.enabled = on
	_capture_mouse(on)


func _build_views() -> void:
	for v in views:
		v.container.queue_free()
		v.input.queue_free()
	views.clear()
	var n: int = Net.local_players.size()
	_grid.columns = 1 if n <= 1 else 2
	for i in n:
		var lp: Dictionary = Net.local_players[i]
		var input = PlayerInput.new()
		input.device = lp.device
		input.player_name = lp.name
		add_child(input)

		var container := SubViewportContainer.new()
		container.stretch = true
		container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		container.size_flags_vertical = Control.SIZE_EXPAND_FILL
		container.mouse_filter = Control.MOUSE_FILTER_IGNORE
		_grid.add_child(container)
		var vp := SubViewport.new()
		vp.use_taa = high_quality
		vp.positional_shadow_atlas_size = 2048
		container.add_child(vp)

		var rig = CameraRig.new()
		vp.add_child(rig)
		var post := CanvasLayer.new()
		post.layer = 0
		vp.add_child(post)
		post.add_child(_post_process())
		var layer := CanvasLayer.new()
		layer.layer = 1
		vp.add_child(layer)
		var hud = Hud.new()
		hud.rig = rig
		hud.game = self
		hud.player_input = input
		layer.add_child(hud)

		input.view_toggled.connect(func(): rig.cockpit = not rig.cockpit)
		input.zoom_changed.connect(func(d: int): rig.zoom(d))
		input.help_toggled.connect(func(): hud.show_help = not hud.show_help)
		input.pause_pressed.connect(func(): menu.show_pause())
		input.info_pressed.connect(func(): menu.toggle_info())
		views.append({"container": container, "viewport": vp, "rig": rig, "hud": hud, "input": input})
	if n == 3:
		var filler := ColorRect.new()
		filler.color = Color(0.02, 0.03, 0.035)
		filler.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		filler.size_flags_vertical = Control.SIZE_EXPAND_FILL
		filler.mouse_filter = Control.MOUSE_FILTER_IGNORE
		_grid.add_child(filler)
	_menu_cam.current = false


## Spawns / removes mechs so they match Net.roster.
func _sync_mechs() -> void:
	if not in_match:
		return
	var wanted := {}
	for e in Net.roster:
		wanted[e.id] = e
	for id in mechs.keys():
		if not wanted.has(id):
			mechs[id].queue_free()
			mechs.erase(id)
	var me: int = Net.my_peer()
	for id in wanted.keys():
		if mechs.has(id):
			continue
		var e: Dictionary = wanted[id]
		var m = QuadMech.new()
		m.mech_id = id
		m.player_name = e.name
		m.is_local = int(e.peer) == me
		add_child(m)
		var spot: Array = Net.spawn_point_for(id)
		m.global_position = spot[0]
		m.rotation.y = spot[1]
		m.aim_yaw = spot[1]
		mechs[id] = m
		m.footfall.connect(_on_footfall.bind(m))
		if m.is_local and int(e.idx) < views.size():
			var v: Dictionary = views[int(e.idx)]
			m.ctl = v.input
			v.rig.mech = m
			v.hud.mech = m
			m.fired.connect(func(_p): v.rig.add_trauma(0.07))


func _on_footfall(pos: Vector3, strength: float, m) -> void:
	for v in views:
		if v.rig.mech == m:
			v.rig.add_trauma(0.2 * strength)
		elif v.rig.mech != null:
			var d: float = v.rig.mech.global_position.distance_to(pos)
			if d < 30.0:
				v.rig.add_trauma(0.12 * strength * (1.0 - d / 30.0))
	if terrain.in_lake(pos.x, pos.z, 1.0) and pos.y < terrain.water_level + 0.3:
		FX.splash(self, Vector3(pos.x, terrain.water_level, pos.z), strength)
		return
	FX.dust(self, pos, strength)
	FX.decal(self, pos + Vector3(0, 0.3, 0), 2.4, 25.0, 0.55)


func _spawn_targets() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	for i in 10:
		var ang := TAU * i / 10.0 + rng.randf() * 0.3
		var r := rng.randf_range(45.0, 140.0)
		var p := Vector3(cos(ang) * r, 0.0, sin(ang) * r)
		if terrain._on_pad(p.x, p.z, 1.1) or terrain.in_lake(p.x, p.z, 1.15):
			p *= 0.6
		p.y = terrain.height_at(p.x, p.z) - 0.2
		var t = Target.new()
		t.index = targets.size()
		add_child(t)
		t.global_position = p
		t.rotation.y = rng.randf() * TAU
		targets.append(t)


# ---------------------------------------------------------------- look

func _setup_environment() -> void:
	var sky_mat := ShaderMaterial.new()
	sky_mat.shader = load("res://shaders/sky.gdshader")
	sky_mat.set_shader_parameter("cloud_tex", load("res://textures/clouds.png"))
	var sky := Sky.new()
	sky.sky_material = sky_mat
	sky.radiance_size = Sky.RADIANCE_SIZE_256

	env = Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_energy = 0.75
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	env.tonemap_mode = Environment.TONE_MAPPER_ACES
	env.tonemap_exposure = 1.1
	env.tonemap_white = 6.0

	env.ssao_enabled = true
	env.ssao_radius = 1.6
	env.ssao_intensity = 2.2
	env.ssao_power = 1.6
	env.ssao_detail = 0.6

	env.glow_enabled = true
	env.glow_intensity = 0.7
	env.glow_bloom = 0.04
	env.glow_hdr_threshold = 1.1
	env.glow_blend_mode = Environment.GLOW_BLEND_MODE_ADDITIVE
	env.set_glow_level(0, 0.0)
	env.set_glow_level(1, 0.6)
	env.set_glow_level(2, 1.0)
	env.set_glow_level(3, 0.8)
	env.set_glow_level(4, 0.5)

	env.fog_enabled = true
	env.fog_light_color = Color(0.78, 0.66, 0.52)
	env.fog_light_energy = 1.0
	env.fog_sun_scatter = 0.35
	env.fog_density = 0.0011
	env.fog_aerial_perspective = 0.3
	env.fog_height = 2.0
	env.fog_height_density = 0.02

	env.volumetric_fog_enabled = true
	env.volumetric_fog_density = 0.0045
	env.volumetric_fog_albedo = Color(0.9, 0.82, 0.7)
	env.volumetric_fog_anisotropy = 0.6
	env.volumetric_fog_length = 160.0
	env.volumetric_fog_sky_affect = 0.0

	# Global illumination and reflections (Forward+ only; F2 toggles them off)
	env.sdfgi_enabled = true
	env.sdfgi_use_occlusion = true
	env.sdfgi_cascades = 4
	env.sdfgi_min_cell_size = 0.4
	env.sdfgi_energy = 0.9
	env.ssil_enabled = true
	env.ssil_radius = 4.0
	env.ssil_intensity = 0.8
	env.ssr_enabled = true
	env.ssr_max_steps = 48
	env.ssr_fade_in = 0.15
	env.ssr_fade_out = 2.0

	env.adjustment_enabled = true
	env.adjustment_contrast = 1.05
	env.adjustment_saturation = 0.95
	var we := WorldEnvironment.new()
	we.environment = env
	# Camera: auto exposure (adapts when you look toward / away from the sun) and a
	# very slight blur on the far distance for depth.
	var cam_attr := CameraAttributesPractical.new()
	cam_attr.auto_exposure_enabled = true
	cam_attr.auto_exposure_scale = 0.35
	cam_attr.auto_exposure_speed = 0.6
	cam_attr.auto_exposure_min_sensitivity = 80.0
	cam_attr.auto_exposure_max_sensitivity = 400.0
	cam_attr.dof_blur_far_enabled = true
	cam_attr.dof_blur_far_distance = 320.0
	cam_attr.dof_blur_far_transition = 220.0
	cam_attr.dof_blur_amount = 0.04
	we.camera_attributes = cam_attr
	add_child(we)

	# Low golden-hour sun behind the spawn view for long shadows across the field.
	sun = DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-24, -32, 0)
	sun.light_color = Color(1.0, 0.84, 0.66)
	sun.light_energy = 1.85
	sun.light_angular_distance = 0.6
	sun.shadow_enabled = true
	sun.shadow_bias = 0.04
	sun.shadow_normal_bias = 1.2
	sun.shadow_blur = 1.2
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	sun.directional_shadow_max_distance = 260.0
	sun.light_volumetric_fog_energy = 1.4
	add_child(sun)


func set_quality(high: bool) -> void:
	high_quality = high
	env.ssao_enabled = high
	env.volumetric_fog_enabled = high
	env.sdfgi_enabled = high
	env.ssil_enabled = high
	env.ssr_enabled = high
	sun.directional_shadow_max_distance = 260.0 if high else 120.0
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS if high else DirectionalLight3D.SHADOW_PARALLEL_2_SPLITS
	for g in dressing.grass_nodes:
		g.visibility_range_end = 120.0 if high else 55.0
	for v in views:
		v.viewport.use_taa = high
		v.hud.flash_message("GRAPHICS: " + ("HIGH" if high else "LOW"))


func _post_process() -> ColorRect:
	var r := ColorRect.new()
	var m := ShaderMaterial.new()
	m.shader = load("res://shaders/post.gdshader")
	r.material = m
	r.set_anchors_preset(Control.PRESET_FULL_RECT)
	r.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return r


## Low mist banks over the lake and in the lowest valleys (volumetric fog only).
func _add_mist() -> void:
	var fm := FogMaterial.new()
	fm.density = 0.06
	fm.albedo = Color(0.9, 0.88, 0.84)
	fm.height_falloff = 0.25
	fm.edge_fade = 0.6
	var spots: Array[Vector3] = [Vector3(terrain.LAKE.x, terrain.water_level + 2.5, terrain.LAKE.z)]
	var samples: Array = []
	var step := 20.0
	var x := -170.0
	while x <= 170.0:
		var z := -170.0
		while z <= 170.0:
			if Vector2(x, z).length() > 40.0 and not terrain._on_pad(x, z, 1.2):
				samples.append(Vector3(x, terrain.height_at(x, z), z))
			z += step
		x += step
	samples.sort_custom(func(a, b): return a.y < b.y)
	for p in samples:
		if spots.size() >= 5:
			break
		var ok := true
		for q in spots:
			if Vector2(p.x - q.x, p.z - q.z).length() < 70.0:
				ok = false
				break
		if ok:
			spots.append(p + Vector3(0, 3.0, 0))
	for i in spots.size():
		var fv := FogVolume.new()
		fv.shape = RenderingServer.FOG_VOLUME_SHAPE_ELLIPSOID
		fv.size = Vector3(130, 9, 130) if i == 0 else Vector3(80, 8, 80)
		fv.material = fm
		add_child(fv)
		fv.global_position = spots[i]


func _setup_input() -> void:
	_key("throttle_up", KEY_W)
	_key("throttle_down", KEY_S)
	_key("turn_left", KEY_A)
	_key("turn_right", KEY_D)
	_key("full_stop", KEY_X)
	_key("brace", KEY_B)
	_key("center_torso", KEY_C)
	_key("toggle_view", KEY_V)
	_key("toggle_help", KEY_H)
	_key("quality", KEY_F2)
	_key("seat_info", KEY_F3)
	if not InputMap.has_action("fire"):
		InputMap.add_action("fire")
		var mb := InputEventMouseButton.new()
		mb.button_index = MOUSE_BUTTON_LEFT
		InputMap.action_add_event("fire", mb)


func _key(action: String, keycode: Key) -> void:
	if InputMap.has_action(action):
		return
	InputMap.add_action(action)
	var ev := InputEventKey.new()
	ev.physical_keycode = keycode
	InputMap.action_add_event(action, ev)
