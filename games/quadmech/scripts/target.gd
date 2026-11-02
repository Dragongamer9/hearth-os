extends StaticBody3D
## Destructible defense turret used as a practice target. Respawns after a while.

const U = preload("res://scripts/util.gd")
const FX = preload("res://scripts/fx.gd")

@export var max_hp := 14
var hp := 14
var index := 0   # stable id shared by every player (spawn order)

var _visual: Node3D
var _wreck: Node3D
var _shape: CollisionShape3D
var _flash_mat: ShaderMaterial
var _head: Node3D
var _t := 0.0


func _ready() -> void:
	hp = max_hp
	_t = randf() * 10.0
	var concrete := U.armor({"base_color": Color(0.52, 0.5, 0.46), "camo_amount": 0.0, "panel_size": 1.6,
		"wear": 0.15, "base_metallic": 0.0, "base_roughness": 0.9, "dirt_amount": 0.8})
	_flash_mat = U.armor({"base_color": Color(0.45, 0.16, 0.12), "camo_color": Color(0.3, 0.1, 0.08), "wear": 0.5})
	var frame := U.armor({"base_color": Color(0.12, 0.12, 0.13), "camo_amount": 0.0, "panel_size": 0.5,
		"base_metallic": 0.6, "base_roughness": 0.45})
	var red := U.glow_mat(Color(1.0, 0.1, 0.05), 6.0)

	_visual = Node3D.new()
	add_child(_visual)
	var base := U.prism(PackedVector2Array([Vector2(-2.4, 0.0), Vector2(-1.9, 1.4), Vector2(1.9, 1.4), Vector2(2.4, 0.0)]), 4.8, 0.2, concrete)
	_visual.add_child(base)
	var collar := U.cylinder(1.4, 0.5, frame, 20)
	collar.position.y = 1.6
	_visual.add_child(collar)
	_head = Node3D.new()
	_head.position.y = 1.85
	_visual.add_child(_head)
	_head.add_child(U.prism(PackedVector2Array([Vector2(-1.5, 0.0), Vector2(-1.1, 1.3), Vector2(1.0, 1.4), Vector2(1.4, 0.0)]), 2.2, 0.15, _flash_mat))
	for side in [-0.45, 0.45]:
		var gun := U.tube(0.14, 2.6, frame)
		gun.position = Vector3(side, 0.7, -2.1)
		_head.add_child(gun)
	var eye := U.box(Vector3(0.5, 0.18, 0.05), red)
	eye.position = Vector3(0, 1.0, -1.25)
	eye.rotation.x = 0.3
	_head.add_child(eye)

	_wreck = Node3D.new()
	_wreck.visible = false
	add_child(_wreck)
	var wbase := U.prism(PackedVector2Array([Vector2(-2.4, 0.0), Vector2(-1.9, 1.2), Vector2(1.6, 1.0), Vector2(2.4, 0.0)]), 4.8, 0.2, concrete)
	_wreck.add_child(wbase)
	var chunk := U.chamfer_box(Vector3(1.6, 0.8, 1.4), 0.15, frame)
	chunk.position = Vector3(0.4, 1.4, 0.2)
	chunk.rotation = Vector3(0.4, 0.7, 0.3)
	_wreck.add_child(chunk)

	_shape = CollisionShape3D.new()
	var bs := BoxShape3D.new()
	bs.size = Vector3(4.6, 3.4, 4.6)
	_shape.shape = bs
	_shape.position.y = 1.7
	add_child(_shape)


func _process(delta: float) -> void:
	if hp > 0:
		_t += delta
		_head.rotation.y = sin(_t * 0.3) * 1.2


## Kept so shells recognise targets; damage itself goes through Net.target_hit().
func take_hit(_pos: Vector3) -> void:
	pass


## Called on every player when the host reports a new hp value.
func apply_hp(value: int) -> void:
	if hp <= 0 or value >= hp:
		return
	hp = value
	_flash_mat.set_shader_parameter("emission_color", Color(1.0, 0.4, 0.1))
	_flash_mat.set_shader_parameter("emission_strength", 1.5)
	var tw := create_tween()
	tw.tween_method(func(v: float): _flash_mat.set_shader_parameter("emission_strength", v), 1.5, 0.0, 0.2)
	if hp <= 0:
		_destroy()


func _destroy() -> void:
	FX.explosion(get_parent(), global_position + Vector3(0, 2.0, 0), 1.0)
	_visual.visible = false
	_wreck.visible = true
	_shape.set_deferred("disabled", true)
	await get_tree().create_timer(12.0).timeout
	hp = max_hp
	_visual.visible = true
	_wreck.visible = false
	_shape.set_deferred("disabled", false)
