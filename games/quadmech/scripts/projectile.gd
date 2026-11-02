extends Node3D
## Autocannon round: bright tracer, slight drop, sparks and scorch on impact.

const U = preload("res://scripts/util.gd")
const FX = preload("res://scripts/fx.gd")

static var _tracer_mesh: BoxMesh

var velocity := Vector3.ZERO
var life := 3.0
var shooter_id := ""
var deals_damage := true   # only the host's shells apply damage
var exclude: Array[RID] = []


func launch(origin: Vector3, vel: Vector3, excl: Array[RID]) -> void:
	global_position = origin
	velocity = vel
	exclude = excl
	if _tracer_mesh == null:
		_tracer_mesh = BoxMesh.new()
		_tracer_mesh.size = Vector3(0.11, 0.11, 3.2)
		_tracer_mesh.material = U.glow_mat(Color(1.0, 0.62, 0.25), 9.0)
	var mi := MeshInstance3D.new()
	mi.mesh = _tracer_mesh
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	mi.layers = 2
	add_child(mi)
	_face()


func _physics_process(delta: float) -> void:
	life -= delta
	if life <= 0.0:
		queue_free()
		return
	velocity.y -= 4.0 * delta
	var from := global_position
	var to := from + velocity * delta
	var q := PhysicsRayQueryParameters3D.create(from, to)
	q.exclude = exclude
	var hit := get_world_3d().direct_space_state.intersect_ray(q)
	if not hit.is_empty():
		var c = hit.collider
		var is_target: bool = c != null and c.has_method("take_hit")
		var is_mech: bool = c != null and "mech_id" in c
		FX.impact(get_tree().current_scene, hit.position, hit.normal, not (is_target or is_mech))
		if deals_damage:
			if is_target:
				Net.target_hit(c.index)
			elif is_mech:
				Net.mech_hit(c.mech_id, shooter_id, 4.0)
		queue_free()
		return
	global_position = to
	_face()


func _face() -> void:
	var d := velocity.normalized()
	if absf(d.dot(Vector3.UP)) < 0.99:
		look_at(global_position + d, Vector3.UP)
