extends CharacterBody3D
## Slow, heavy four-legged mech.
## - Throttle-based movement (W/S set throttle, A/D turn the legs)
## - Procedural legs: lateral-sequence WALK at low speed, diagonal TROT above trot_above_speed
## - Stabilized 360-degree turret that slews toward where the camera is aiming
## - Heat, overheat shutdown, and a braced "siege mode"

signal footfall(position: Vector3, strength: float)
signal fired(position: Vector3)
signal destroyed(mech)

const U = preload("res://scripts/util.gd")
const FX = preload("res://scripts/fx.gd")
const Leg = preload("res://scripts/leg.gd")
const Projectile = preload("res://scripts/projectile.gd")

# Leg order: 0 front-left, 1 front-right, 2 back-left, 3 back-right. Forward is -Z.
const HIPS := [Vector3(-1.75, -0.15, -1.75), Vector3(1.75, -0.15, -1.75), Vector3(-1.75, -0.15, 1.75), Vector3(1.75, -0.15, 1.75)]
const RESTS := [Vector3(-3.9, 0, -3.4), Vector3(3.9, 0, -3.4), Vector3(-3.9, 0, 3.4), Vector3(3.9, 0, 3.4)]
const WALK_SEQUENCE := [2, 0, 3, 1]  # back-left, front-left, back-right, front-right
const TROT_OVERLAP := 0.8

@export_group("Movement")
@export var max_speed := 7.0          # m/s (~25 km/h). It's a quad - it's slow.
@export var reverse_speed := 2.5
@export var acceleration := 1.6
@export var deceleration := 2.4
@export var turn_rate_deg := 22.0
@export var throttle_rate := 0.6

@export_group("Body")
@export var ride_height := 3.4
@export var braced_drop := 1.1
@export var brace_time := 1.6

@export_group("Torso")
@export var torso_turn_deg := 55.0
@export var gun_pitch_deg := 35.0
@export var mouse_sensitivity := 0.0025
@export var min_pitch_deg := -20.0
@export var max_pitch_deg := 25.0

@export_group("Legs")
@export var upper_len := 3.4
@export var lower_len := 4.0
@export var step_threshold := 1.3
@export var step_height := 1.1
@export var trot_above_speed := 3.2

@export_group("Weapons")
@export var fire_interval := 0.22
@export var heat_per_shot := 5.5
@export var heat_dissipation := 14.0
@export var overheat_lockout := 3.0
@export var projectile_speed := 220.0

var throttle := 0.0
var speed := 0.0
var yaw_rate := 0.0
var aim_yaw := 0.0       # camera/aim yaw in WORLD space (turret is stabilized)
var aim_pitch := 0.0
var aim_target := Vector3.ZERO
var has_aim := false
var heat := 0.0
var overheated := 0.0
var braced := false
var brace_amount := 0.0
var trotting := false

var hull: Node3D
var turret: Node3D
var guns: Node3D
var muzzles: Array[Node3D] = []
var legs: Array = []

# Multiplayer
var mech_id := "1:0"          # "<peer>:<local index>"
var player_name := "Player"
var is_local := true          # false = puppet driven by network updates
var ctl = null                # PlayerInput for local mechs
var armor := 100.0
var dead := false
var respawn_in := 0.0
var _net_pos := Vector3.ZERO
var _net_yaw := 0.0
var _net_turret := 0.0
var _net_pitch := 0.0
var _net_has := false
var _send_t := 0.0

var _fire_cd := 0.0
var _next_muzzle := 0
var _initialized := false
var _seq_pos := 0
var _last_group := -1


func _ready() -> void:
	motion_mode = CharacterBody3D.MOTION_MODE_FLOATING
	_build_body()
	aim_yaw = rotation.y


# ---------------------------------------------------------------- building

var _m: Dictionary
var _barrels: Array[Node3D] = []
var _recoil := [0.0, 0.0]


func _build_body() -> void:
	_m = U.mech_palette()
	var m := _m

	var col := CollisionShape3D.new()
	col.name = "CollisionShape3D"
	var shape := BoxShape3D.new()
	shape.size = Vector3(3.6, 1.4, 5.6)
	col.shape = shape
	add_child(col)

	hull = Node3D.new()
	hull.name = "Hull"
	add_child(hull)

	# Main hull: sloped glacis at the front, tapered rear.
	hull.add_child(U.prism(PackedVector2Array([
		Vector2(-2.9, 0.0), Vector2(-2.35, 0.72), Vector2(1.9, 0.82), Vector2(2.75, 0.25),
		Vector2(2.55, -0.55), Vector2(-2.3, -0.62)]), 3.1, 0.18, m.armor))
	var keel := U.chamfer_box(Vector3(2.1, 0.55, 3.9), 0.15, m.frame)
	keel.position = Vector3(0, -0.8, 0.1)
	hull.add_child(keel)
	# Side skirts, angled outward.
	for side in [-1.0, 1.0]:
		var skirt := U.prism(PackedVector2Array([
			Vector2(-2.2, 0.45), Vector2(1.6, 0.5), Vector2(2.1, 0.1), Vector2(1.8, -0.6), Vector2(-1.9, -0.6), Vector2(-2.5, -0.05)]),
			0.22, 0.06, m.armor)
		skirt.position = Vector3(side * 1.62, -0.05, 0.0)
		skirt.rotation.z = side * -0.14
		hull.add_child(skirt)
		var trim := U.chamfer_box(Vector3(0.24, 0.12, 3.6), 0.03, m.accent)
		trim.position = Vector3(side * 1.68, 0.33, -0.1)
		trim.rotation.z = side * -0.14
		hull.add_child(trim)
	# Hip housings
	for i in 4:
		var housing := U.chamfer_box(Vector3(1.15, 1.15, 1.35), 0.2, m.frame)
		housing.position = HIPS[i] + Vector3(-signf(HIPS[i].x) * 0.15, 0.1, 0)
		hull.add_child(housing)
	# Front sensor lamps on the glacis
	for side in [-1.0, 1.0]:
		var lamp_box := U.chamfer_box(Vector3(0.5, 0.26, 0.3), 0.05, m.frame)
		lamp_box.position = Vector3(side * 0.9, 0.3, -2.62)
		lamp_box.rotation.x = 0.9
		hull.add_child(lamp_box)
		var lamp := U.box(Vector3(0.38, 0.14, 0.05), m.lamp)
		lamp.position = Vector3(side * 0.9, 0.27, -2.76)
		lamp.rotation.x = 0.9
		hull.add_child(lamp)
	# Engine block, exhaust vents and heat sink fins at the rear
	var engine := U.chamfer_box(Vector3(2.3, 0.95, 1.2), 0.15, m.frame)
	engine.position = Vector3(0, 0.45, 2.35)
	hull.add_child(engine)
	_vent_mat = m.vent.duplicate()
	for side in [-1.0, 1.0]:
		var vent := U.chamfer_box(Vector3(0.8, 0.5, 0.08), 0.02, _vent_mat)
		vent.position = Vector3(side * 0.55, 0.45, 2.97)
		hull.add_child(vent)
	for k in 7:
		var fin := U.chamfer_box(Vector3(0.07, 0.35, 0.95), 0.02, m.metal)
		fin.position = Vector3(-0.9 + k * 0.3, 1.05, 2.3)
		hull.add_child(fin)

	# ---------------- turret / torso
	turret = Node3D.new()
	turret.name = "Turret"
	turret.position = Vector3(0, 0.82, -0.1)
	hull.add_child(turret)
	var ring := U.cylinder(1.45, 0.35, m.frame, 24)
	ring.position.y = 0.1
	turret.add_child(ring)
	turret.add_child(U.prism(PackedVector2Array([
		Vector2(-2.1, 0.2), Vector2(-1.75, 1.1), Vector2(-0.95, 1.55), Vector2(1.35, 1.55),
		Vector2(1.85, 1.05), Vector2(1.75, 0.2)]), 2.5, 0.16, m.armor))
	# Angled cheek plates
	for side in [-1.0, 1.0]:
		var cheek := U.prism(PackedVector2Array([
			Vector2(-1.9, 0.3), Vector2(-1.55, 1.15), Vector2(0.6, 1.25), Vector2(0.9, 0.3)]), 0.3, 0.07, m.armor)
		cheek.position = Vector3(side * 1.3, 0.0, 0.0)
		cheek.rotation.z = side * -0.18
		turret.add_child(cheek)
	# Cockpit canopy on the upper glacis, with frame bars
	var canopy := U.prism(PackedVector2Array([
		Vector2(-1.78, 1.12), Vector2(-1.0, 1.52), Vector2(-0.92, 1.62), Vector2(-1.82, 1.2)]), 1.5, 0.03, m.glass)
	turret.add_child(canopy)
	for x in [-0.75, 0.0, 0.75]:
		var bar := U.chamfer_box(Vector3(0.08, 0.1, 0.95), 0.02, m.frame)
		bar.position = Vector3(x, 1.4, -1.38)
		bar.rotation.x = -0.47
		turret.add_child(bar)
	var brow := U.chamfer_box(Vector3(1.8, 0.16, 0.4), 0.04, m.frame)
	brow.position = Vector3(0, 1.66, -0.95)
	turret.add_child(brow)
	# Missile rack (decorative for now) with tube openings
	var rack := U.chamfer_box(Vector3(1.4, 0.6, 1.3), 0.1, m.armor)
	rack.position = Vector3(0, 1.88, 0.65)
	turret.add_child(rack)
	for r in 2:
		for c in 4:
			var tube := U.tube(0.1, 0.12, m.hole, 8)
			tube.position = Vector3(-0.48 + c * 0.32, 1.78 + r * 0.24, 0.0)
			turret.add_child(tube)
	var rack_stripe := U.chamfer_box(Vector3(1.42, 0.08, 0.5), 0.02, m.accent)
	rack_stripe.position = Vector3(0, 2.13, 0.95)
	turret.add_child(rack_stripe)
	# Antennae and sensor dome
	for side in [-1.0, 1.0]:
		var ant := U.cylinder(0.025, 2.2, m.frame, 6)
		ant.position = Vector3(side * 0.95, 2.6, 1.15)
		ant.rotation.x = 0.12
		turret.add_child(ant)
	var dome := U.sphere(0.28, m.frame, 12)
	dome.position = Vector3(0.6, 1.6, -0.6)
	turret.add_child(dome)
	var eye := U.sphere(0.12, m.redlamp, 8)
	eye.position = Vector3(0.6, 1.6, -0.86)
	turret.add_child(eye)
	# Unit markings
	for side in [-1.0, 1.0]:
		var lbl := Label3D.new()
		lbl.text = "07"
		lbl.font_size = 96
		lbl.pixel_size = 0.006
		lbl.modulate = Color(0.95, 0.9, 0.8, 0.85)
		lbl.outline_size = 0
		lbl.double_sided = false
		lbl.shaded = true
		lbl.position = Vector3(side * 1.47, 0.85, 0.45)
		lbl.rotation.y = side * PI / 2.0
		lbl.rotation.x = 0.0
		lbl.rotate_object_local(Vector3.FORWARD, side * 0.18)
		turret.add_child(lbl)

	# ---------------- weapon pods
	guns = Node3D.new()
	guns.name = "Guns"
	guns.position = Vector3(0, 0.95, -0.4)
	turret.add_child(guns)
	_barrel_mat = m.barrel.duplicate()
	for side in [-1.0, 1.0]:
		var pod := Node3D.new()
		pod.position = Vector3(side * 1.9, 0, 0)
		guns.add_child(pod)
		var arm := U.chamfer_box(Vector3(0.6, 0.55, 0.9), 0.1, m.frame)
		arm.position = Vector3(-side * 0.45, 0, 0.2)
		pod.add_child(arm)
		pod.add_child(U.prism(PackedVector2Array([
			Vector2(-1.35, -0.35), Vector2(-1.2, 0.42), Vector2(0.9, 0.48), Vector2(1.15, 0.1), Vector2(0.95, -0.45)]),
			0.95, 0.12, m.armor))
		var cap := U.chamfer_box(Vector3(0.98, 0.14, 0.5), 0.03, m.orange)
		cap.position = Vector3(0, 0.38, -0.95)
		pod.add_child(cap)
		var recoil := Node3D.new()
		pod.add_child(recoil)
		_barrels.append(recoil)
		var shroud := U.tube(0.26, 1.2, m.frame)
		shroud.position = Vector3(0, -0.02, -1.85)
		recoil.add_child(shroud)
		for k in 3:
			var ringk := U.tube(0.3, 0.08, m.frame)
			ringk.position = Vector3(0, -0.02, -1.4 - k * 0.32)
			recoil.add_child(ringk)
		var barrel := U.tube(0.13, 2.4, _barrel_mat)
		barrel.position = Vector3(0, -0.02, -3.3)
		recoil.add_child(barrel)
		var brake := U.chamfer_box(Vector3(0.36, 0.36, 0.5), 0.06, m.frame)
		brake.position = Vector3(0, -0.02, -4.5)
		recoil.add_child(brake)
		var bore := U.tube(0.1, 0.06, m.hole, 10)
		bore.position = Vector3(0, -0.02, -4.76)
		recoil.add_child(bore)
		var muzzle := Node3D.new()
		muzzle.position = Vector3(0, -0.02, -4.85)
		recoil.add_child(muzzle)
		muzzles.append(muzzle)

	_build_details(m)

	for i in 4:
		var leg = Leg.new()
		leg.setup(i, HIPS[i], RESTS[i], upper_len, lower_len, m)
		leg.group = 0 if (i == 0 or i == 3) else 1
		add_child(leg)
		legs.append(leg)

	# Mech meshes live on render layer 2 so scorch/footprint decals don't paint them.
	_set_layers(self)


var _vent_mat: ShaderMaterial
var _barrel_mat: ShaderMaterial
var _hazes: Array[MeshInstance3D] = []
var _haze_mat: ShaderMaterial
var _haze_barrel_mat: ShaderMaterial


## Greebles: bolt rows, lockers, exhaust stacks, hoses, headlights and heat haze.
func _build_details(m: Dictionary) -> void:
	# Bolt rows along armor edges
	var bolt_mesh := CylinderMesh.new()
	bolt_mesh.top_radius = 0.045
	bolt_mesh.bottom_radius = 0.055
	bolt_mesh.height = 0.06
	bolt_mesh.radial_segments = 6
	bolt_mesh.rings = 1
	var rows := [
		[hull, Vector3(-1.38, 0.8, -2.1), Vector3(-1.38, 0.8, 1.7), 12, Vector3.UP],
		[hull, Vector3(1.38, 0.8, -2.1), Vector3(1.38, 0.8, 1.7), 12, Vector3.UP],
		[turret, Vector3(-1.05, 1.56, -0.8), Vector3(-1.05, 1.56, 1.25), 7, Vector3.UP],
		[turret, Vector3(1.05, 1.56, -0.8), Vector3(1.05, 1.56, 1.25), 7, Vector3.UP],
		[hull, Vector3(-1.74, 0.38, -1.9), Vector3(-1.74, 0.38, 1.4), 9, Vector3.LEFT],
		[hull, Vector3(1.74, 0.38, -1.9), Vector3(1.74, 0.38, 1.4), 9, Vector3.RIGHT],
	]
	for side in [-1.0, 1.0]:
		rows.append([guns, Vector3(side * 2.38, 0.3, -1.1), Vector3(side * 2.38, 0.3, 0.7), 6, Vector3(side, 0, 0)])
		rows.append([guns, Vector3(side * 2.38, -0.3, -1.1), Vector3(side * 2.38, -0.3, 0.7), 6, Vector3(side, 0, 0)])
	for r in rows:
		var parent: Node3D = r[0]
		var a: Vector3 = r[1]
		var b: Vector3 = r[2]
		var count: int = r[3]
		var nrm: Vector3 = r[4]
		for k in count:
			var bolt := U.mesh_node(bolt_mesh, m.metal)
			bolt.position = a.lerp(b, float(k) / float(count - 1))
			if nrm != Vector3.UP:
				bolt.rotation.z = -signf(nrm.x) * PI / 2.0
			parent.add_child(bolt)

	# Storage lockers on the torso flanks, with latches
	for side in [-1.0, 1.0]:
		var locker := U.chamfer_box(Vector3(0.35, 0.55, 1.0), 0.06, m.frame)
		locker.position = Vector3(side * 1.55, 0.75, 1.15)
		turret.add_child(locker)
		for k in 2:
			var latch := U.box(Vector3(0.06, 0.1, 0.12), m.orange)
			latch.position = Vector3(side * 1.74, 0.8, 0.85 + k * 0.6)
			turret.add_child(latch)
		# Hydraulic hoses running along the hull side
		var hose := U.tube(0.06, 3.2, m.frame, 8)
		hose.position = Vector3(side * 1.5, 0.62, -0.2)
		hull.add_child(hose)
		var hose2 := U.tube(0.045, 3.0, m.orange, 8)
		hose2.position = Vector3(side * 1.5, 0.52, -0.1)
		hull.add_child(hose2)

	# Exhaust stacks at the rear, with heat-glowing caps
	for side in [-1.0, 1.0]:
		var stack := U.cylinder(0.16, 1.1, m.frame, 12)
		stack.position = Vector3(side * 0.85, 1.35, 2.75)
		stack.rotation.x = 0.25
		hull.add_child(stack)
		var cap := U.cylinder(0.19, 0.12, _vent_mat, 12)
		cap.position = Vector3(side * 0.85, 1.9, 2.89)
		cap.rotation.x = 0.25
		hull.add_child(cap)
		var tail := U.box(Vector3(0.25, 0.08, 0.04), m.redlamp)
		tail.position = Vector3(side * 1.25, 0.4, 2.98)
		hull.add_child(tail)

	# Headlights that actually light the ground
	for side in [-1.0, 1.0]:
		var spot := SpotLight3D.new()
		spot.light_color = Color(1.0, 0.93, 0.8)
		spot.light_energy = 4.0
		spot.spot_range = 70.0
		spot.spot_angle = 24.0
		spot.spot_attenuation = 0.8
		spot.shadow_enabled = true
		spot.position = Vector3(side * 0.9, 0.25, -2.8)
		spot.rotation_degrees = Vector3(-14, 0, 0)
		hull.add_child(spot)

	# Heat haze above the exhaust stacks and in front of the barrels
	# (Vulkan renderers only: the OpenGL fallback can't refract the scene behind it.)
	if RenderingServer.get_rendering_device() == null:
		return
	var hn := FastNoiseLite.new()
	hn.frequency = 0.02
	var htex := NoiseTexture2D.new()
	htex.noise = hn
	htex.seamless = true
	htex.width = 128
	htex.height = 128
	_haze_mat = ShaderMaterial.new()
	_haze_mat.shader = load("res://shaders/heat_haze.gdshader")
	_haze_mat.set_shader_parameter("noise_tex", htex)
	_haze_barrel_mat = _haze_mat.duplicate()
	var quad := QuadMesh.new()
	quad.size = Vector2(1.0, 2.6)
	for side in [-1.0, 1.0]:
		var hz := MeshInstance3D.new()
		hz.mesh = quad
		hz.material_override = _haze_mat
		hz.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		hz.position = Vector3(side * 0.85, 3.2, 3.1)
		hull.add_child(hz)
		_hazes.append(hz)
	for muzzle in muzzles:
		var hz := MeshInstance3D.new()
		var q2 := QuadMesh.new()
		q2.size = Vector2(0.8, 1.6)
		hz.mesh = q2
		hz.material_override = _haze_barrel_mat
		hz.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		hz.position = Vector3(0, 0.75, 1.0)
		muzzle.add_child(hz)
		_hazes.append(hz)


func _set_layers(n: Node) -> void:
	if n is VisualInstance3D:
		n.layers = 2
	for c in n.get_children():
		_set_layers(c)


func _update_heat_glow(delta: float) -> void:
	var h := clampf(heat / 100.0, 0.0, 1.0)
	_barrel_mat.set_shader_parameter("emission_strength", h * h * 6.0)
	_vent_mat.set_shader_parameter("emission_strength", 0.12 + h * 5.0)
	# Engine always shimmers a little; barrels only once they're hot.
	if _haze_mat != null:
		_haze_mat.set_shader_parameter("strength", 0.25 + h * 0.9)
		_haze_barrel_mat.set_shader_parameter("strength", maxf(h - 0.2, 0.0) * 1.2)
	for i in _barrels.size():
		_recoil[i] = move_toward(_recoil[i], 0.0, delta * 3.0)
		_barrels[i].position.z = _recoil[i]


# ---------------------------------------------------------------- input

func _physics_process(delta: float) -> void:
	if not _initialized:
		for leg in legs:
			var gp := _ground_point(to_global(leg.rest_offset))
			leg.plant(gp, _last_normal)
		_initialized = true
	if dead:
		_update_dead(delta)
		return
	if is_local:
		_update_controls(delta)
		_update_movement(delta)
	else:
		_update_puppet(delta)
	_update_legs(delta)
	_update_body(delta)
	if is_local:
		_update_turret(delta)
		_update_weapons(delta)
		_send_state(delta)
	else:
		heat = maxf(heat - heat_dissipation * delta, 0.0)
	_update_heat_glow(delta)


func _update_controls(delta: float) -> void:
	if ctl == null:
		return
	if ctl.pressed("brace"):
		braced = not braced
	if ctl.pressed("stop"):
		throttle = 0.0
	if ctl.pressed("center"):
		aim_yaw = rotation.y
	aim_yaw += ctl.aim_delta.x
	aim_pitch = clampf(aim_pitch + ctl.aim_delta.y, deg_to_rad(min_pitch_deg), deg_to_rad(max_pitch_deg))
	if braced:
		return
	var axis: float = ctl.throttle_axis
	throttle = clampf(throttle + axis * throttle_rate * delta, -1.0, 1.0)
	if absf(axis) < 0.05 and absf(throttle) < 0.04:
		throttle = 0.0


# ---------------------------------------------------------------- movement

func _update_movement(delta: float) -> void:
	brace_amount = move_toward(brace_amount, 1.0 if braced else 0.0, delta / brace_time)
	var mobile := brace_amount < 0.05 and overheated <= 0.0 and not braced

	var target_speed := 0.0
	if mobile:
		target_speed = throttle * (max_speed if throttle >= 0.0 else reverse_speed)
	var rate := acceleration if absf(target_speed) > absf(speed) else deceleration
	speed = move_toward(speed, target_speed, rate * delta)

	var turn := 0.0
	if mobile and ctl != null:
		turn = ctl.turn_axis
	yaw_rate = move_toward(yaw_rate, deg_to_rad(turn_rate_deg) * turn, deg_to_rad(40.0) * delta)
	rotation.y += yaw_rate * delta

	var fwd := -global_transform.basis.z
	fwd.y = 0.0
	velocity = fwd.normalized() * speed
	var keep_y := global_position.y
	move_and_slide()
	global_position.y = keep_y  # height is driven by the legs, not physics

	# Ran into something solid: bleed off speed instead of grinding forever.
	if get_slide_collision_count() > 0:
		var real := get_real_velocity()
		if Vector2(real.x, real.z).length() < absf(speed) * 0.5:
			speed = move_toward(speed, 0.0, deceleration * 2.0 * delta)


# ---------------------------------------------------------------- legs

func _update_legs(delta: float) -> void:
	trotting = absf(speed) > trot_above_speed
	var step_time := lerpf(0.95, 0.45, clampf(absf(speed) / max_speed, 0.0, 1.0))
	# Each foot lifts once per gait cycle; land it half a cycle ahead so it
	# spends equal time in front of and behind its rest spot.
	var lead := step_time * (TROT_OVERLAP if trotting else 1.0)

	# Predict where the body will be so feet land ahead of it.
	var pred_basis := Basis(Vector3.UP, rotation.y + yaw_rate * lead)
	var pred_origin := global_position + (velocity * lead).limit_length(3.0)
	var spread := 1.0 + 0.18 * brace_amount  # braced = legs splay outward

	for leg in legs:
		var rest_local: Vector3 = leg.rest_offset * Vector3(spread, 1.0, spread)
		leg.target = _ground_point(pred_origin + pred_basis * rest_local)
		leg.target_normal = _last_normal
		leg.error = Vector2(leg.foot.x - leg.target.x, leg.foot.z - leg.target.z).length()
		if leg.stepping:
			leg.step_to = leg.target

	# Emergency: a foot way out of place (sharp turn, shove) steps right away.
	var active := 0
	for leg in legs:
		if leg.stepping:
			active += 1
	for leg in legs:
		if not leg.stepping and leg.error > 4.0 and active < 2:
			leg.start_step()
			active += 1

	if trotting:
		_gait_trot()
	else:
		_gait_walk()

	for leg in legs:
		if leg.update_step(delta / step_time, step_height):
			footfall.emit(leg.foot, 0.8 if trotting else 1.0)
		leg.update_visual(hull.global_transform * leg.hip_offset, global_position, hull.global_transform.basis.y)


## One leg at a time in a fixed order; the next leg may lift once the current one is halfway.
func _gait_walk() -> void:
	var active := 0
	for leg in legs:
		if leg.stepping:
			active += 1
			if leg.step_t < 0.5:
				return
	if active >= 2:
		return
	var moving: bool = absf(speed) > 0.2 or absf(yaw_rate) > 0.05
	for _attempt in 4:
		var leg = legs[WALK_SEQUENCE[_seq_pos]]
		if leg.stepping:
			_seq_pos = (_seq_pos + 1) % 4
			continue
		var need := step_threshold * 0.6 if moving else 0.3
		if leg.error > need:
			leg.start_step()
			_seq_pos = (_seq_pos + 1) % 4
			return
		if moving:
			return  # wait for this leg's turn so the rhythm stays readable
		_seq_pos = (_seq_pos + 1) % 4


## Diagonal pairs (FL+BR, FR+BL) step together, alternating.
## The next pair may lift once the current pair is TROT_OVERLAP through its step.
func _gait_trot() -> void:
	for leg in legs:
		if leg.stepping and (leg.step_t < TROT_OVERLAP or leg.group != _last_group):
			return
	for leg in legs:
		if leg.stepping and leg.group == 1 - _last_group:
			return
	var err := [0.0, 0.0]
	for leg in legs:
		err[leg.group] = maxf(err[leg.group], leg.error)
	var g := 0
	if _last_group >= 0:
		g = 1 - _last_group
	elif err[1] > err[0]:
		g = 1
	if err[g] < step_threshold * 0.5 and err[1 - g] > err[g]:
		g = 1 - g
	if err[g] > step_threshold * 0.5:
		for leg in legs:
			if leg.group == g and not leg.stepping:
				leg.start_step()
		_last_group = g


var _last_normal := Vector3.UP


func _ground_point(p: Vector3) -> Vector3:
	var q := PhysicsRayQueryParameters3D.create(p + Vector3.UP * 10.0, p + Vector3.DOWN * 30.0)
	var ex: Array[RID] = [get_rid()]
	q.exclude = ex
	var hit := get_world_3d().direct_space_state.intersect_ray(q)
	if hit.is_empty():
		_last_normal = Vector3.UP
		return Vector3(p.x, p.y - ride_height, p.z)
	# Soften steep normals so feet don't flip on rock edges.
	_last_normal = (hit.normal as Vector3).lerp(Vector3.UP, 0.4).normalized()
	return hit.position


# ---------------------------------------------------------------- body

func _update_body(delta: float) -> void:
	var ys: Array[float] = []
	for leg in legs:
		ys.append(leg.ground_y())
	var avg := (ys[0] + ys[1] + ys[2] + ys[3]) / 4.0
	var target_y := avg + ride_height - braced_drop * brace_amount

	# Weight shift: body dips and leans away from a lifted leg.
	var roll_sway := 0.0
	for leg in legs:
		if leg.stepping:
			var s := sin(leg.step_t * PI)
			target_y -= s * 0.12
			roll_sway += signf(leg.rest_offset.x) * s * 0.035
	global_position.y = lerpf(global_position.y, target_y, 1.0 - exp(-4.0 * delta))

	var front := (ys[0] + ys[1]) * 0.5
	var back := (ys[2] + ys[3]) * 0.5
	var left := (ys[0] + ys[2]) * 0.5
	var right := (ys[1] + ys[3]) * 0.5
	var pitch := clampf(atan2(front - back, 6.8), -0.35, 0.35)
	var roll := clampf(atan2(right - left, 7.8), -0.35, 0.35) + roll_sway
	var k := 1.0 - exp(-3.0 * delta)
	hull.rotation.x = lerp_angle(hull.rotation.x, pitch, k)
	hull.rotation.z = lerp_angle(hull.rotation.z, roll, k)


# ---------------------------------------------------------------- turret

func _update_turret(delta: float) -> void:
	if not has_aim or overheated > 0.0:
		return
	var local: Vector3 = hull.global_transform.basis.inverse() * (aim_target - turret.global_position)
	var desired_yaw := atan2(-local.x, -local.z)
	var diff := wrapf(desired_yaw - turret.rotation.y, -PI, PI)
	var stp := deg_to_rad(torso_turn_deg) * delta
	turret.rotation.y = wrapf(turret.rotation.y + clampf(diff, -stp, stp), -PI, PI)

	var gl: Vector3 = turret.global_transform.basis.inverse() * (aim_target - guns.global_position)
	var desired_pitch := clampf(atan2(gl.y, Vector2(gl.x, gl.z).length()), deg_to_rad(min_pitch_deg), deg_to_rad(30.0))
	var pstep := deg_to_rad(gun_pitch_deg) * delta
	guns.rotation.x += clampf(desired_pitch - guns.rotation.x, -pstep, pstep)


## Where the guns are actually pointing right now (for the HUD reticle).
func aim_point() -> Vector3:
	if guns == null:
		return global_position
	var from := guns.global_position
	var to := from - guns.global_transform.basis.z * 1000.0
	var q := PhysicsRayQueryParameters3D.create(from, to)
	var ex: Array[RID] = [get_rid()]
	q.exclude = ex
	var hit := get_world_3d().direct_space_state.intersect_ray(q)
	return to if hit.is_empty() else hit.position


# ---------------------------------------------------------------- weapons

func _update_weapons(delta: float) -> void:
	heat = maxf(heat - heat_dissipation * (1.0 + 0.5 * brace_amount) * delta, 0.0)
	if overheated > 0.0:
		overheated -= delta
		return
	_fire_cd -= delta
	if ctl != null and ctl.fire and _fire_cd <= 0.0:
		_fire_cd = fire_interval
		_fire()
		heat += heat_per_shot * (1.0 - 0.45 * brace_amount)
		if heat >= 100.0:
			heat = 100.0
			overheated = overheat_lockout


func _fire() -> void:
	var mi := _next_muzzle
	_next_muzzle = (_next_muzzle + 1) % muzzles.size()
	var m := muzzles[mi]
	var b := m.global_transform.basis
	var spread := deg_to_rad(lerpf(0.8, 0.2, brace_amount))  # bracing = tighter groups
	var dir := (-b.z).rotated(b.x.normalized(), randf_range(-spread, spread))
	dir = dir.rotated(b.y.normalized(), randf_range(-spread, spread)).normalized()
	shoot(mi, m.global_position, dir)
	Net.send_fire(mech_id, mi, m.global_position, dir)


## Spawns the shell and effects. Called locally and for remote players' shots.
## Only the host's copy of a shell can deal damage.
func shoot(mi: int, origin: Vector3, dir: Vector3) -> void:
	_recoil[mi] = 0.45
	var scene := get_tree().current_scene
	var p = Projectile.new()
	scene.add_child(p)
	var ex: Array[RID] = [get_rid()]
	p.shooter_id = mech_id
	p.deals_damage = Net.is_authority()
	p.launch(origin, dir * projectile_speed, ex)
	FX.muzzle(scene, origin, dir)
	fired.emit(origin)


# ---------------------------------------------------------------- multiplayer

func _send_state(delta: float) -> void:
	_send_t -= delta
	if _send_t > 0.0:
		return
	_send_t = 0.05  # 20 Hz
	Net.send_state(mech_id, global_position, rotation.y, speed, yaw_rate, turret.rotation.y, guns.rotation.x, braced, heat)


func apply_net_state(pos: Vector3, yaw: float, spd: float, yr: float, tur: float, pitch: float, brc: bool, ht: float) -> void:
	if not _net_has or pos.distance_to(_net_pos) > 25.0:
		global_position = Vector3(pos.x, pos.y, pos.z)
		rotation.y = yaw
	_net_pos = pos
	_net_yaw = yaw
	speed = spd
	yaw_rate = yr
	_net_turret = tur
	_net_pitch = pitch
	braced = brc
	heat = ht
	_net_has = true


func _update_puppet(delta: float) -> void:
	if not _net_has:
		return
	var k := 1.0 - exp(-10.0 * delta)
	var y := global_position.y
	global_position = global_position.lerp(Vector3(_net_pos.x, y, _net_pos.z), k)
	rotation.y = lerp_angle(rotation.y, _net_yaw, k)
	brace_amount = move_toward(brace_amount, 1.0 if braced else 0.0, delta / brace_time)
	var fwd := -global_transform.basis.z
	fwd.y = 0.0
	velocity = fwd.normalized() * speed
	turret.rotation.y = lerp_angle(turret.rotation.y, _net_turret, k)
	guns.rotation.x = lerpf(guns.rotation.x, _net_pitch, k)


func set_armor(value: float) -> void:
	armor = value


func die() -> void:
	if dead:
		return
	dead = true
	respawn_in = 6.0
	armor = 0.0
	throttle = 0.0
	speed = 0.0
	FX.explosion(get_tree().current_scene, global_position + Vector3(0, 1.5, 0), 1.6)
	visible = false
	$CollisionShape3D.set_deferred("disabled", true)
	destroyed.emit(self)


func respawn_at(pos: Vector3, yaw: float) -> void:
	dead = false
	armor = 100.0
	heat = 0.0
	overheated = 0.0
	braced = false
	brace_amount = 0.0
	visible = true
	$CollisionShape3D.set_deferred("disabled", false)
	global_position = pos
	rotation.y = yaw
	aim_yaw = yaw
	_net_pos = pos
	_net_yaw = yaw
	_initialized = false  # replant the feet


func _update_dead(delta: float) -> void:
	if not is_local:
		return
	respawn_in -= delta
	if respawn_in <= 0.0:
		respawn_in = 1e9  # wait for the respawn message
		var spot: Array = Net.spawn_point_for(mech_id)
		Net.send_respawn(mech_id, spot[0], spot[1])
