extends Node3D
## One procedural mech leg: owns its foot position, step animation and
## two-bone IK visuals. The mech decides WHEN a leg steps; the leg handles HOW.

const U = preload("res://scripts/util.gd")

var index := 0
var group := 0                  # diagonal pair (used by the trot gait)
var hip_offset := Vector3.ZERO  # hip position in hull-local space
var rest_offset := Vector3.ZERO # neutral foot position relative to the mech root
var upper_len := 3.4
var lower_len := 4.0

var foot := Vector3.ZERO        # current foot position (world)
var target := Vector3.ZERO      # where the foot wants to be (world)
var target_normal := Vector3.UP
var foot_normal := Vector3.UP
var error := 0.0                # horizontal distance foot -> target
var stepping := false
var step_t := 0.0
var step_from := Vector3.ZERO
var step_to := Vector3.ZERO
var _normal_from := Vector3.UP

var _upper: Node3D
var _lower: Node3D
var _hip: Node3D
var _knee: Node3D
var _ankle: Node3D
var _foot: Node3D
var _sleeve: Node3D
var _rod: Node3D


func setup(i: int, hip: Vector3, rest: Vector3, ul: float, ll: float, m: Dictionary) -> void:
	index = i
	hip_offset = hip
	rest_offset = rest
	upper_len = ul
	lower_len = ll
	top_level = true  # visuals are placed in world space

	# Thigh: heavy armored plate over an inner frame. Built 1 unit long along -Z,
	# scaled to the bone length every frame. +Y points to the outside of the bend.
	_upper = _node()
	_upper.add_child(U.prism(PackedVector2Array([
		Vector2(0.05, -0.4), Vector2(0.05, 0.42), Vector2(-0.12, 0.62), Vector2(-0.8, 0.5),
		Vector2(-0.98, 0.3), Vector2(-0.98, -0.25), Vector2(-0.85, -0.38)]), 1.0, 0.12, m.armor))
	var tframe := U.chamfer_box(Vector3(0.7, 0.55, 1.0), 0.08, m.frame)
	tframe.position = Vector3(0, -0.3, -0.5)
	_upper.add_child(tframe)
	for side in [-1.0, 1.0]:
		var rib := U.chamfer_box(Vector3(0.08, 0.3, 0.7), 0.03, m.frame)
		rib.position = Vector3(side * 0.53, 0.25, -0.45)
		_upper.add_child(rib)

	# Shin: narrower armored plate with an orange stripe near the ankle.
	_lower = _node()
	_lower.add_child(U.prism(PackedVector2Array([
		Vector2(0.0, -0.3), Vector2(0.0, 0.35), Vector2(-0.12, 0.45), Vector2(-0.75, 0.32),
		Vector2(-1.0, 0.16), Vector2(-1.0, -0.18), Vector2(-0.85, -0.28)]), 0.72, 0.08, m.armor))
	var sframe := U.chamfer_box(Vector3(0.5, 0.36, 1.0), 0.06, m.frame)
	sframe.position = Vector3(0, -0.2, -0.5)
	_lower.add_child(sframe)
	var stripe := U.chamfer_box(Vector3(0.76, 0.1, 0.12), 0.02, m.orange)
	stripe.position = Vector3(0, 0.2, -0.82)
	_lower.add_child(stripe)

	_hip = _node()
	var hip_ball := U.sphere(0.62, m.frame, 14)
	_hip.add_child(hip_ball)
	var hip_cap := U.chamfer_box(Vector3(1.1, 0.5, 0.9), 0.12, m.armor)
	hip_cap.position = Vector3(0, 0.45, 0)
	_hip.add_child(hip_cap)

	_knee = _node()
	_knee.add_child(U.sphere(0.5, m.frame, 12))
	var guard := U.prism(PackedVector2Array([
		Vector2(0.45, 0.0), Vector2(0.3, 0.55), Vector2(-0.35, 0.6), Vector2(-0.5, 0.1)]), 0.85, 0.1, m.armor)
	_knee.add_child(guard)
	var spike := U.prism(PackedVector2Array([Vector2(0.15, 0.55), Vector2(-0.2, 0.58), Vector2(0.0, 0.85)]), 0.18, 0.03, m.frame)
	_knee.add_child(spike)

	_ankle = _node()
	_ankle.add_child(U.sphere(0.38, m.frame, 12))

	# Foot: oriented each frame so +Y = ground normal, -Z = away from the body.
	_foot = _node()
	var pad := U.chamfer_box(Vector3(1.3, 0.38, 1.3), 0.12, m.frame)
	pad.position.y = 0.19
	_foot.add_child(pad)
	var top := U.chamfer_box(Vector3(0.9, 0.3, 0.9), 0.1, m.armor)
	top.position.y = 0.5
	_foot.add_child(top)
	for k in 3:
		var toe := Node3D.new()
		toe.rotation.y = deg_to_rad(-38.0 + 38.0 * k)
		_foot.add_child(toe)
		var claw := U.prism(PackedVector2Array([
			Vector2(-0.4, 0.0), Vector2(-0.4, 0.42), Vector2(-0.9, 0.3), Vector2(-1.35, 0.0)]), 0.34, 0.06, m.armor)
		toe.add_child(claw)
		var tip := U.prism(PackedVector2Array([Vector2(-1.15, 0.0), Vector2(-1.15, 0.16), Vector2(-1.55, 0.0)]), 0.26, 0.03, m.frame)
		toe.add_child(tip)
	var spur := U.prism(PackedVector2Array([Vector2(0.4, 0.0), Vector2(0.4, 0.3), Vector2(0.95, 0.0)]), 0.3, 0.05, m.frame)
	_foot.add_child(spur)

	# Hydraulic piston from the hull to the thigh.
	_sleeve = _node()
	var sl := U.tube(0.17, 1.7, m.frame)
	sl.position.z = -0.85
	_sleeve.add_child(sl)
	var collar := U.tube(0.22, 0.25, m.orange)
	collar.position.z = -1.6
	_sleeve.add_child(collar)
	_rod = _node()
	var rod := U.tube(0.09, 1.8, m.metal)
	rod.position.z = -0.9
	_rod.add_child(rod)


func _node() -> Node3D:
	var n := Node3D.new()
	add_child(n)
	return n


func plant(p: Vector3, nrm: Vector3 = Vector3.UP) -> void:
	foot = p
	target = p
	step_from = p
	step_to = p
	foot_normal = nrm
	target_normal = nrm
	stepping = false


func start_step() -> void:
	stepping = true
	step_t = 0.0
	step_from = foot
	step_to = target
	_normal_from = foot_normal


## Advances the step. Returns true on the frame the foot lands.
func update_step(dt_norm: float, height: float) -> bool:
	if not stepping:
		return false
	step_t = minf(step_t + dt_norm, 1.0)
	var e := step_t * step_t * (3.0 - 2.0 * step_t)  # smoothstep
	foot = step_from.lerp(step_to, e)
	var climb := maxf(step_to.y - step_from.y, 0.0)
	foot.y += sin(step_t * PI) * (height + climb * 0.5)
	foot_normal = _normal_from.slerp(target_normal, e) if _normal_from.dot(target_normal) < 0.9999 else target_normal
	if step_t >= 1.0:
		foot = step_to
		foot_normal = target_normal
		stepping = false
		return true
	return false


## Ground height this leg is (or will be) supporting.
func ground_y() -> float:
	return step_to.y if stepping else foot.y


func update_visual(hip_world: Vector3, body_center: Vector3, body_up: Vector3) -> void:
	var outward := hip_world - body_center
	outward.y = 0.0
	outward = outward.normalized()
	# The leg hangs from the hip; the foot attaches at the ankle above the pad.
	var ankle := foot + foot_normal * 0.75
	var to_foot := ankle - hip_world
	var d := to_foot.length()
	var dir := to_foot / maxf(d, 0.001)
	var reach := upper_len + lower_len - 0.05
	var dc := clampf(d, absf(upper_len - lower_len) + 0.05, reach)

	# Two-bone IK (law of cosines). Knee bends up and outward, spider-style.
	var a := (upper_len * upper_len - lower_len * lower_len + dc * dc) / (2.0 * dc)
	var h := sqrt(maxf(upper_len * upper_len - a * a, 0.0))
	var pole := outward * 0.6 + Vector3.UP
	var perp := pole - dir * pole.dot(dir)
	if perp.length() < 0.001:
		perp = Vector3.UP
	perp = perp.normalized()

	var knee := hip_world + dir * a + perp * h
	var ankle_vis := hip_world + dir * dc
	_stretch(_upper, hip_world, knee, perp)
	_stretch(_lower, knee, ankle_vis, perp)

	_hip.global_transform = _look(hip_world, hip_world + outward, body_up)
	var knee_dir := (perp + (knee - hip_world).normalized() * 0.3).normalized()
	_knee.global_transform = _look(knee, knee + (ankle_vis - knee), knee_dir)
	_ankle.global_position = ankle_vis

	var fwd := outward - foot_normal * outward.dot(foot_normal)
	if fwd.length() < 0.01:
		fwd = Vector3.FORWARD
	_foot.global_transform = _look(ankle_vis - foot_normal * 0.75, ankle_vis - foot_normal * 0.75 + fwd, foot_normal)

	# Piston: anchored on the hull above the hip, pushing on the thigh.
	var anchor := hip_world + body_up * 1.1 - outward * 0.5
	var mount := hip_world.lerp(knee, 0.55) + perp * 0.55
	_sleeve.global_transform = _look(anchor, mount, perp)
	_rod.global_transform = _look(mount, anchor, perp)


func _look(from: Vector3, to: Vector3, up: Vector3) -> Transform3D:
	var v := to - from
	if v.length_squared() < 1e-6:
		return Transform3D(Basis(), from)
	if absf(v.normalized().dot(up.normalized())) > 0.98:
		up = Vector3.FORWARD if absf(v.normalized().dot(Vector3.FORWARD)) < 0.9 else Vector3.RIGHT
	return Transform3D(Basis(), from).looking_at(to, up)


func _stretch(seg: Node3D, from: Vector3, to: Vector3, up_hint: Vector3) -> void:
	var t := _look(from, to, up_hint)
	t.basis = t.basis * Basis.from_scale(Vector3(1.0, 1.0, (to - from).length()))
	seg.global_transform = t
