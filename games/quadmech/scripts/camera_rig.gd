extends Node3D
## Chase / cockpit camera. Mouse steers the camera; the mech's turret slews
## toward whatever is under the screen center, so heavy torsos lag behind your aim.

const U = preload("res://scripts/util.gd")

var mech
var camera: Camera3D
var distance := 17.0
var cockpit := false
var trauma := 0.0

var _noise := FastNoiseLite.new()
var _time := 0.0
var _frame: Node3D


func _ready() -> void:
	camera = Camera3D.new()
	camera.fov = 70.0
	camera.far = 2000.0
	camera.near = 0.05
	add_child(camera)
	camera.make_current()
	_noise.frequency = 0.9
	_build_cockpit_frame()


func add_trauma(amount: float) -> void:
	trauma = minf(trauma + amount, 1.0)


func zoom(direction: int) -> void:
	distance = clampf(distance - direction * 1.5, 8.0, 40.0)


func _process(delta: float) -> void:
	if mech == null or not is_instance_valid(mech) or mech.turret == null:
		return
	_time += delta
	var yaw: float = mech.aim_yaw
	var pitch: float = mech.aim_pitch
	var look_dir := (Basis(Vector3.UP, yaw) * Basis(Vector3.RIGHT, pitch)) * Vector3.FORWARD
	var ex: Array[RID] = [mech.get_rid()]
	var space := get_world_3d().direct_space_state

	_frame.visible = cockpit
	if cockpit:
		var cam_pos: Vector3 = mech.turret.to_global(Vector3(0, 1.0, -2.05))
		camera.global_position = cam_pos
		# Cockpit rolls a little with the hull so you feel the terrain.
		var up: Vector3 = Vector3.UP.lerp(mech.hull.global_transform.basis.y, 0.5).normalized()
		camera.look_at(cam_pos + look_dir * 50.0, up)
	else:
		var pivot: Vector3 = mech.global_position + Vector3(0, 4.0, 0)
		var orbit := Basis(Vector3.UP, yaw) * Basis(Vector3.RIGHT, pitch * 0.6 - 0.22)
		var cam_pos := pivot + orbit * Vector3(0, 0, distance)
		var q := PhysicsRayQueryParameters3D.create(pivot, cam_pos)
		q.exclude = ex
		var hit := space.intersect_ray(q)
		if not hit.is_empty():
			cam_pos = hit.position + (pivot - cam_pos).normalized() * 0.6
		camera.global_position = cam_pos
		camera.look_at(pivot + look_dir * 60.0, Vector3.UP)

	# Whatever is under the crosshair becomes the turret's target.
	var from := camera.global_position
	var to := from - camera.global_transform.basis.z * 1200.0
	var aq := PhysicsRayQueryParameters3D.create(from, to)
	aq.exclude = ex
	var ah := space.intersect_ray(aq)
	mech.aim_target = to if ah.is_empty() else ah.position
	mech.has_aim = true

	# Screen shake (applied after aiming so it doesn't throw your aim off).
	trauma = maxf(trauma - delta * 1.5, 0.0)
	var s := trauma * trauma
	if s > 0.0:
		var t := _time * 25.0
		camera.rotate_object_local(Vector3.RIGHT, _noise.get_noise_2d(t, 0.0) * 0.05 * s)
		camera.rotate_object_local(Vector3.UP, _noise.get_noise_2d(0.0, t) * 0.04 * s)
		camera.global_position += Vector3(0, _noise.get_noise_2d(t, 50.0) * 0.6 * s, 0)


func _build_cockpit_frame() -> void:
	var m: Dictionary = U.mech_palette()
	_frame = Node3D.new()
	_frame.visible = false
	camera.add_child(_frame)
	# Canopy struts framing the view (in camera space: -Z forward).
	for side in [-1.0, 1.0]:
		var pillar := U.chamfer_box(Vector3(0.06, 1.6, 0.07), 0.015, m.frame)
		pillar.position = Vector3(side * 0.66, 0.05, -0.7)
		pillar.rotation = Vector3(0.0, 0.0, side * 0.32)
		_frame.add_child(pillar)
		var corner := U.chamfer_box(Vector3(0.5, 0.12, 0.12), 0.02, m.frame)
		corner.position = Vector3(side * 0.46, 0.4, -0.72)
		corner.rotation.z = side * -0.45
		_frame.add_child(corner)
	var top := U.chamfer_box(Vector3(1.6, 0.1, 0.3), 0.02, m.frame)
	top.position = Vector3(0, 0.5, -0.7)
	top.rotation.x = 0.35
	_frame.add_child(top)
	# Dashboard with glowing displays
	var dash := U.prism(PackedVector2Array([Vector2(-0.95, -0.55), Vector2(-0.75, -0.32), Vector2(-0.4, -0.3), Vector2(-0.3, -0.6)]), 2.2, 0.03, m.frame)
	_frame.add_child(dash)
	var screen_mat := U.glow_mat(Color(0.12, 0.4, 0.45), 0.6)
	var amber_mat := U.glow_mat(Color(0.6, 0.35, 0.08), 0.7)
	for k in 3:
		var scr := U.box(Vector3(0.16, 0.08, 0.01), screen_mat if k != 1 else amber_mat)
		scr.position = Vector3(-0.3 + k * 0.3, -0.335, -0.74)
		scr.rotation.x = -1.0
		_frame.add_child(scr)
	for k in 6:
		var btn := U.box(Vector3(0.03, 0.015, 0.03), amber_mat if k % 2 == 0 else U.glow_mat(Color(0.3, 1.0, 0.4), 2.0))
		btn.position = Vector3(-0.55 + k * 0.05, -0.36, -0.62)
		_frame.add_child(btn)
	for c in _frame.get_children():
		if c is GeometryInstance3D:
			c.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
