extends RefCounted
## Visual effects built at runtime: smoke/dust billboards, sparks, muzzle
## flashes, explosions, scorch marks and footprints.

static var _smoke_tex: ImageTexture
static var _flash_tex: ImageTexture
static var _scorch_tex: ImageTexture
static var _smoke_mat: StandardMaterial3D
static var _fire_mat: StandardMaterial3D
static var _flash_mat: StandardMaterial3D
static var _spark_mat: StandardMaterial3D
static var _decals: Array = []
const MAX_DECALS := 70


# ---------------------------------------------------------------- textures

static func _make_tex(size: int, fn: Callable) -> ImageTexture:
	var img := Image.create(size, size, true, Image.FORMAT_RGBA8)
	for y in size:
		for x in size:
			var uv := Vector2((x + 0.5) / size, (y + 0.5) / size) * 2.0 - Vector2.ONE
			img.set_pixel(x, y, fn.call(uv))
	img.generate_mipmaps()
	return ImageTexture.create_from_image(img)


static func smoke_tex() -> ImageTexture:
	if _smoke_tex == null:
		var n := FastNoiseLite.new()
		n.frequency = 2.2
		n.fractal_octaves = 4
		_smoke_tex = _make_tex(128, func(uv: Vector2) -> Color:
			var r := uv.length()
			var base := clampf(1.0 - r, 0.0, 1.0)
			var nz := n.get_noise_2d(uv.x * 40.0, uv.y * 40.0) * 0.5 + 0.5
			var a := clampf(base * base * (0.55 + nz * 0.9) * 1.4 - 0.05, 0.0, 1.0)
			var shade := 0.75 + 0.25 * nz
			return Color(shade, shade, shade, a))
	return _smoke_tex


static func flash_tex() -> ImageTexture:
	if _flash_tex == null:
		_flash_tex = _make_tex(128, func(uv: Vector2) -> Color:
			var r := uv.length()
			var core := exp(-r * r * 18.0)
			var ang := atan2(uv.y, uv.x)
			var rays := pow(absf(cos(ang * 3.0)), 24.0) * clampf(1.0 - r, 0.0, 1.0)
			var glow := clampf(1.0 - r, 0.0, 1.0)
			var v := clampf(core + rays * 0.8 + glow * glow * 0.35, 0.0, 1.0)
			return Color(v, v, v, v))
	return _flash_tex


static func scorch_tex() -> ImageTexture:
	if _scorch_tex == null:
		var n := FastNoiseLite.new()
		n.frequency = 3.0
		n.fractal_octaves = 3
		_scorch_tex = _make_tex(128, func(uv: Vector2) -> Color:
			var r := uv.length() + n.get_noise_2d(uv.x * 30.0, uv.y * 30.0) * 0.25
			var a := clampf(1.0 - smoothstep(0.35, 0.95, r), 0.0, 1.0)
			return Color(0.03, 0.025, 0.02, a * 0.9))
	return _scorch_tex


# ---------------------------------------------------------------- materials

static func smoke_mat() -> StandardMaterial3D:
	if _smoke_mat == null:
		var m := StandardMaterial3D.new()
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		m.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
		m.vertex_color_use_as_albedo = true
		m.albedo_texture = smoke_tex()
		m.roughness = 1.0
		m.proximity_fade_enabled = true
		m.proximity_fade_distance = 1.5
		_smoke_mat = m
	return _smoke_mat


static func fire_mat() -> StandardMaterial3D:
	if _fire_mat == null:
		var m := StandardMaterial3D.new()
		m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		m.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		m.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
		m.vertex_color_use_as_albedo = true
		m.albedo_texture = smoke_tex()
		_fire_mat = m
	return _fire_mat


static func flash_mat() -> StandardMaterial3D:
	if _flash_mat == null:
		var m := StandardMaterial3D.new()
		m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		m.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		m.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
		m.albedo_texture = flash_tex()
		m.albedo_color = Color(1.0, 0.7, 0.35) * 3.0
		_flash_mat = m
	return _flash_mat


static func spark_mat() -> StandardMaterial3D:
	if _spark_mat == null:
		var m := StandardMaterial3D.new()
		m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		m.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		m.vertex_color_use_as_albedo = true
		_spark_mat = m
	return _spark_mat


# ---------------------------------------------------------------- building blocks

static func _particles(parent: Node, pos: Vector3, mesh: Mesh, amount: int, lifetime: float) -> CPUParticles3D:
	var p := CPUParticles3D.new()
	p.one_shot = true
	p.amount = amount
	p.lifetime = lifetime
	p.explosiveness = 0.92
	p.mesh = mesh
	p.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	p.local_coords = false
	parent.add_child(p)
	p.global_position = pos
	p.emitting = true
	parent.get_tree().create_timer(lifetime + 0.5).timeout.connect(p.queue_free)
	return p


static func _quad(size: float, material: Material) -> QuadMesh:
	var q := QuadMesh.new()
	q.size = Vector2(size, size)
	q.material = material
	return q


static func _ramp(colors: Array, offsets: Array) -> Gradient:
	var g := Gradient.new()
	g.offsets = PackedFloat32Array(offsets)
	g.colors = PackedColorArray(colors)
	return g


static func _grow_curve(a: float, b: float) -> Curve:
	var c := Curve.new()
	c.min_value = 0.0
	c.max_value = 3.0
	c.add_point(Vector2(0.0, a))
	c.add_point(Vector2(1.0, b))
	return c


static func _light(parent: Node, pos: Vector3, color: Color, energy: float, light_range: float, duration: float) -> void:
	var l := OmniLight3D.new()
	l.light_color = color
	l.light_energy = energy
	l.omni_range = light_range
	l.shadow_enabled = false
	parent.add_child(l)
	l.global_position = pos
	var tw := l.create_tween()
	tw.tween_property(l, "light_energy", 0.0, duration)
	tw.tween_callback(l.queue_free)


# ---------------------------------------------------------------- effects

static func dust(parent: Node, pos: Vector3, strength: float = 1.0) -> void:
	var p := _particles(parent, pos + Vector3(0, 0.4, 0), _quad(2.6, smoke_mat()), int(9 * strength) + 3, 3.2)
	p.emission_shape = CPUParticles3D.EMISSION_SHAPE_SPHERE
	p.emission_sphere_radius = 0.9
	p.direction = Vector3.UP
	p.spread = 95.0
	p.initial_velocity_min = 1.5 * strength
	p.initial_velocity_max = 4.0 * strength
	p.gravity = Vector3(0, 0.15, 0)
	p.damping_min = 2.0
	p.damping_max = 3.0
	p.angle_min = 0.0
	p.angle_max = 360.0
	p.angular_velocity_min = -25.0
	p.angular_velocity_max = 25.0
	p.scale_amount_min = 0.8 * strength
	p.scale_amount_max = 1.6 * strength
	p.scale_amount_curve = _grow_curve(0.5, 2.2)
	var c := Color(0.62, 0.53, 0.41)
	p.color_ramp = _ramp([Color(c, 0.0), Color(c, 0.55), Color(c, 0.0)], [0.0, 0.08, 1.0])


static func muzzle(parent: Node, pos: Vector3, dir: Vector3) -> void:
	var fl := MeshInstance3D.new()
	fl.mesh = _quad(1.0, flash_mat())
	fl.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	parent.add_child(fl)
	fl.global_position = pos + dir * 0.4
	fl.scale = Vector3.ONE * randf_range(1.8, 2.6)
	fl.rotation.z = randf() * TAU
	# Flame cone pointing down the barrel
	var cone := MeshInstance3D.new()
	var cm := CylinderMesh.new()
	cm.top_radius = 0.02
	cm.bottom_radius = 0.32
	cm.height = 2.2
	cm.radial_segments = 8
	cone.mesh = cm
	cone.material_override = flash_mat_cone()
	cone.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	parent.add_child(cone)
	cone.global_position = pos + dir * 1.1
	if absf(dir.dot(Vector3.UP)) < 0.98:
		cone.look_at(cone.global_position + dir, Vector3.UP)
		cone.rotate_object_local(Vector3.RIGHT, -PI / 2.0)
	parent.get_tree().create_timer(0.05).timeout.connect(fl.queue_free)
	parent.get_tree().create_timer(0.04).timeout.connect(cone.queue_free)
	_light(parent, pos, Color(1.0, 0.7, 0.35), 6.0, 12.0, 0.08)
	var s := _particles(parent, pos + dir * 0.6, _quad(0.9, smoke_mat()), 4, 1.2)
	s.direction = dir
	s.spread = 20.0
	s.initial_velocity_min = 3.0
	s.initial_velocity_max = 6.0
	s.damping_min = 5.0
	s.damping_max = 7.0
	s.gravity = Vector3(0, 0.6, 0)
	s.angle_max = 360.0
	s.scale_amount_curve = _grow_curve(0.6, 2.5)
	s.color_ramp = _ramp([Color(0.5, 0.48, 0.45, 0.35), Color(0.5, 0.48, 0.45, 0.0)], [0.0, 1.0])


static var _cone_mat: StandardMaterial3D
static func flash_mat_cone() -> StandardMaterial3D:
	if _cone_mat == null:
		_cone_mat = StandardMaterial3D.new()
		_cone_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		_cone_mat.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
		_cone_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		_cone_mat.albedo_color = Color(1.0, 0.55, 0.2) * 1.6
		_cone_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	return _cone_mat


static func impact(parent: Node, pos: Vector3, normal: Vector3, scorch: bool = true) -> void:
	var sm := BoxMesh.new()
	sm.size = Vector3(0.05, 0.55, 0.05)
	sm.material = spark_mat()
	var sp := _particles(parent, pos, sm, 14, 0.7)
	sp.particle_flag_align_y = true
	sp.direction = normal
	sp.spread = 65.0
	sp.initial_velocity_min = 7.0
	sp.initial_velocity_max = 18.0
	sp.gravity = Vector3(0, -16, 0)
	sp.color_ramp = _ramp([Color(1.0, 0.85, 0.5) * 3.0, Color(1.0, 0.4, 0.1) * 2.0, Color(0, 0, 0)], [0.0, 0.4, 1.0])

	var fl := _particles(parent, pos + normal * 0.3, _quad(1.6, fire_mat()), 3, 0.25)
	fl.scale_amount_curve = _grow_curve(0.6, 1.6)
	fl.angle_max = 360.0
	fl.color_ramp = _ramp([Color(1.0, 0.7, 0.3), Color(0.6, 0.2, 0.05), Color(0, 0, 0)], [0.0, 0.5, 1.0])

	var s := _particles(parent, pos + normal * 0.4, _quad(1.8, smoke_mat()), 5, 2.2)
	s.direction = normal
	s.spread = 40.0
	s.initial_velocity_min = 1.0
	s.initial_velocity_max = 3.0
	s.damping_min = 1.5
	s.damping_max = 2.5
	s.gravity = Vector3(0, 0.5, 0)
	s.angle_max = 360.0
	s.scale_amount_curve = _grow_curve(0.5, 2.0)
	s.color_ramp = _ramp([Color(0.35, 0.33, 0.3, 0.0), Color(0.35, 0.33, 0.3, 0.6), Color(0.35, 0.33, 0.3, 0.0)], [0.0, 0.1, 1.0])

	_light(parent, pos + normal * 0.5, Color(1.0, 0.6, 0.25), 5.0, 9.0, 0.15)
	if scorch:
		decal(parent, pos, randf_range(1.2, 1.8), 30.0)


static func explosion(parent: Node, pos: Vector3, size: float = 1.0) -> void:
	var fb := _particles(parent, pos, _quad(4.0 * size, fire_mat()), 18, 0.9)
	fb.emission_shape = CPUParticles3D.EMISSION_SHAPE_SPHERE
	fb.emission_sphere_radius = 1.2 * size
	fb.direction = Vector3.UP
	fb.spread = 180.0
	fb.initial_velocity_min = 2.0 * size
	fb.initial_velocity_max = 7.0 * size
	fb.damping_min = 4.0
	fb.damping_max = 6.0
	fb.gravity = Vector3(0, 3.0, 0)
	fb.angle_max = 360.0
	fb.scale_amount_min = 0.7
	fb.scale_amount_max = 1.4
	fb.scale_amount_curve = _grow_curve(0.6, 1.8)
	fb.color_ramp = _ramp([Color(1.0, 0.9, 0.6) * 1.5, Color(1.0, 0.45, 0.1), Color(0.3, 0.08, 0.02), Color(0, 0, 0)], [0.0, 0.2, 0.6, 1.0])

	var sm := _particles(parent, pos + Vector3(0, 1.0, 0), _quad(5.0 * size, smoke_mat()), 16, 6.0)
	sm.explosiveness = 0.6
	sm.emission_shape = CPUParticles3D.EMISSION_SHAPE_SPHERE
	sm.emission_sphere_radius = 1.5 * size
	sm.direction = Vector3.UP
	sm.spread = 35.0
	sm.initial_velocity_min = 2.0
	sm.initial_velocity_max = 5.0 * size
	sm.damping_min = 0.8
	sm.damping_max = 1.4
	sm.gravity = Vector3(0, 0.8, 0)
	sm.angle_max = 360.0
	sm.angular_velocity_min = -15.0
	sm.angular_velocity_max = 15.0
	sm.scale_amount_curve = _grow_curve(0.5, 2.4)
	var dk := Color(0.12, 0.11, 0.1)
	sm.color_ramp = _ramp([Color(dk, 0.0), Color(dk, 0.85), Color(0.3, 0.28, 0.26, 0.0)], [0.0, 0.1, 1.0])

	var dm := BoxMesh.new()
	dm.size = Vector3(0.35, 0.25, 0.3)
	dm.material = mat_debris()
	var db := _particles(parent, pos + Vector3(0, 0.8, 0), dm, 16, 2.5)
	db.direction = Vector3.UP
	db.spread = 70.0
	db.initial_velocity_min = 6.0
	db.initial_velocity_max = 16.0
	db.gravity = Vector3(0, -18, 0)
	db.angular_velocity_min = -360.0
	db.angular_velocity_max = 360.0
	db.scale_amount_min = 0.5
	db.scale_amount_max = 1.6

	impact(parent, pos + Vector3(0, 0.5, 0), Vector3.UP, false)
	_light(parent, pos + Vector3(0, 2, 0), Color(1.0, 0.55, 0.2), 14.0 * size, 30.0 * size, 0.7)
	decal(parent, pos, 5.0 * size, 60.0)


static var _debris_mat: StandardMaterial3D
static func mat_debris() -> StandardMaterial3D:
	if _debris_mat == null:
		_debris_mat = StandardMaterial3D.new()
		_debris_mat.albedo_color = Color(0.15, 0.14, 0.13)
		_debris_mat.roughness = 0.8
	return _debris_mat


## Projected scorch / scuff mark that fades out after `life` seconds.
static func decal(parent: Node, pos: Vector3, size: float, life: float, strength: float = 1.0) -> void:
	var d := Decal.new()
	d.texture_albedo = scorch_tex()
	d.size = Vector3(size, 3.0, size)
	d.modulate = Color(1, 1, 1, strength)
	d.upper_fade = 0.3
	d.lower_fade = 0.3
	d.cull_mask = 1
	parent.add_child(d)
	d.global_position = pos
	d.rotation.y = randf() * TAU
	_decals.append(d)
	while _decals.size() > MAX_DECALS:
		var old = _decals.pop_front()
		if is_instance_valid(old):
			old.queue_free()
	var tw := d.create_tween()
	tw.tween_interval(life)
	tw.tween_property(d, "modulate:a", 0.0, 4.0)
	tw.tween_callback(d.queue_free)


static var _ring_mat: StandardMaterial3D

## Water splash: spray, mist and an expanding ripple ring.
static func splash(parent: Node, pos: Vector3, strength: float = 1.0) -> void:
	var spray := _particles(parent, pos, _quad(1.2, smoke_mat()), 22, 1.4)
	spray.emission_shape = CPUParticles3D.EMISSION_SHAPE_RING
	spray.emission_ring_axis = Vector3.UP
	spray.emission_ring_radius = 1.0
	spray.emission_ring_inner_radius = 0.6
	spray.emission_ring_height = 0.1
	spray.direction = Vector3.UP
	spray.spread = 30.0
	spray.initial_velocity_min = 4.0 * strength
	spray.initial_velocity_max = 8.0 * strength
	spray.gravity = Vector3(0, -14, 0)
	spray.angle_max = 360.0
	spray.scale_amount_min = 0.4
	spray.scale_amount_max = 0.9
	spray.scale_amount_curve = _grow_curve(0.6, 1.6)
	var w := Color(0.85, 0.9, 0.92)
	spray.color_ramp = _ramp([Color(w, 0.0), Color(w, 0.8), Color(w, 0.0)], [0.0, 0.1, 1.0])
	var mist := _particles(parent, pos + Vector3(0, 0.5, 0), _quad(3.0, smoke_mat()), 6, 2.5)
	mist.emission_shape = CPUParticles3D.EMISSION_SHAPE_SPHERE
	mist.emission_sphere_radius = 1.2
	mist.direction = Vector3.UP
	mist.initial_velocity_min = 0.5
	mist.initial_velocity_max = 1.5
	mist.gravity = Vector3.ZERO
	mist.angle_max = 360.0
	mist.scale_amount_curve = _grow_curve(0.6, 2.0)
	mist.color_ramp = _ramp([Color(w, 0.0), Color(w, 0.35), Color(w, 0.0)], [0.0, 0.15, 1.0])
	if _ring_mat == null:
		_ring_mat = StandardMaterial3D.new()
		_ring_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		_ring_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		_ring_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
		_ring_mat.albedo_texture = flash_tex()
	for k in 2:
		var ring := MeshInstance3D.new()
		var torus := TorusMesh.new()
		torus.inner_radius = 0.92
		torus.outer_radius = 1.0
		torus.rings = 32
		torus.ring_segments = 4
		ring.mesh = torus
		var m := _ring_mat.duplicate() as StandardMaterial3D
		m.albedo_color = Color(0.9, 0.95, 1.0, 0.55)
		ring.material_override = m
		ring.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		parent.add_child(ring)
		ring.global_position = pos + Vector3(0, 0.03, 0)
		ring.scale = Vector3(1.0, 0.05, 1.0)
		var tw := ring.create_tween()
		tw.set_parallel(true)
		tw.tween_property(ring, "scale", Vector3(6.0 + k * 3.0, 0.05, 6.0 + k * 3.0), 2.2 + k * 0.6).set_delay(k * 0.25)
		tw.tween_property(m, "albedo_color:a", 0.0, 2.2 + k * 0.6).set_delay(k * 0.25)
		tw.chain().tween_callback(ring.queue_free)
