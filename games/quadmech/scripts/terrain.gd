extends StaticBody3D
## Procedural rolling terrain with flattened building pads, a ring of hills,
## and sculpted boulders. Grass, trees and structures live in world_dressing.gd.

const U = preload("res://scripts/util.gd")
const TERRAIN_SHADER = preload("res://shaders/terrain.gdshader")

@export var size := 500.0
@export var cell := 1.6
@export var height_scale := 20.0
@export var noise_seed := 1337

## Flattened areas for outposts: (x, radius, z)
const PADS := [Vector3(70, 40, -95), Vector3(-120, 34, 45), Vector3(30, 28, 125)]
## Lake basin: (x, radius, z) and depth below the shoreline.
const LAKE := Vector3(-58, 50, -132)
const LAKE_DEPTH := 6.5
const WATER_SHADER = preload("res://shaders/water.gdshader")

var water_level := 0.0

var _noise := FastNoiseLite.new()
var _detail := FastNoiseLite.new()
var _ridge := FastNoiseLite.new()
var _pad_h: Array[float] = []
var ground_material: ShaderMaterial
var rock_material: ShaderMaterial


func _init() -> void:
	_noise.seed = noise_seed
	_noise.frequency = 0.0055
	_noise.fractal_octaves = 5
	_detail.seed = noise_seed + 1
	_detail.frequency = 0.05
	_detail.fractal_octaves = 3
	_ridge.seed = noise_seed + 2
	_ridge.frequency = 0.011
	_ridge.fractal_type = FastNoiseLite.FRACTAL_RIDGED
	_ridge.fractal_octaves = 3
	for p in PADS:
		_pad_h.append(_raw_height(p.x, p.z))
	# Water sits a little below the lowest ground around the basin rim.
	var rim := INF
	for k in 16:
		var a := TAU * k / 16.0
		rim = minf(rim, _raw_height(LAKE.x + cos(a) * LAKE.y, LAKE.z + sin(a) * LAKE.y))
	water_level = rim - 0.6


func _ready() -> void:
	_make_materials()
	_build_ground()
	_build_water()
	_scatter_rocks()


func _raw_height(x: float, z: float) -> float:
	var h := _noise.get_noise_2d(x, z) * height_scale
	h += maxf(_ridge.get_noise_2d(x, z), 0.0) * 9.0
	h += _detail.get_noise_2d(x, z) * 1.4
	var d := Vector2(x, z).length()
	h *= clampf((d - 14.0) / 40.0, 0.0, 1.0)  # flat spawn
	var edge := clampf((d - size * 0.34) / (size * 0.13), 0.0, 1.0)
	h += edge * edge * 55.0  # ring of hills keeps you in the map
	return h


func height_at(x: float, z: float) -> float:
	var h := _raw_height(x, z)
	for i in PADS.size():
		var p: Vector3 = PADS[i]
		var d := Vector2(x - p.x, z - p.z).length()
		var w := 1.0 - smoothstep(p.y * 0.65, p.y, d)
		if w > 0.0:
			h = lerpf(h, _pad_h[i], w)
	# Lake basin: a smooth bowl below the waterline with gently shelving shores.
	var ld := Vector2(x - LAKE.x, z - LAKE.z).length()
	if ld < LAKE.y * 1.25:
		var t := clampf(ld / LAKE.y, 0.0, 1.25)
		var bowl := water_level + 0.8 - LAKE_DEPTH * maxf(1.0 - t * t, 0.0) - (1.0 - t) * 0.4
		var wgt := 1.0 - smoothstep(0.75, 1.25, t)
		h = lerpf(h, minf(h, bowl), wgt)
	return h


func in_lake(x: float, z: float, margin: float = 1.0) -> bool:
	return Vector2(x - LAKE.x, z - LAKE.z).length() < LAKE.y * margin


func _build_water() -> void:
	var plane := PlaneMesh.new()
	plane.size = Vector2(LAKE.y * 2.8, LAKE.y * 2.8)
	plane.subdivide_width = 96
	plane.subdivide_depth = 96
	var wn := NoiseTexture2D.new()
	var n := FastNoiseLite.new()
	n.seed = 77
	n.frequency = 0.03
	n.fractal_octaves = 4
	wn.noise = n
	wn.width = 512
	wn.height = 512
	wn.seamless = true
	wn.as_normal_map = true
	wn.bump_strength = 4.0
	# Bake water depth into vertex color (used by the OpenGL fallback shader).
	var arrays := plane.get_mesh_arrays()
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var cols := PackedColorArray()
	cols.resize(verts.size())
	for i in verts.size():
		var v := verts[i]
		var dd := clampf((water_level - height_at(LAKE.x + v.x, LAKE.z + v.z)) / 10.0, 0.0, 1.0)
		cols[i] = Color(dd, 0, 0)
	arrays[Mesh.ARRAY_COLOR] = cols
	var water_mesh := ArrayMesh.new()
	water_mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)

	var opengl := RenderingServer.get_rendering_device() == null
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/water_simple.gdshader") if opengl else WATER_SHADER
	mat.set_shader_parameter("wave_nrm", wn)
	var mi := MeshInstance3D.new()
	mi.mesh = water_mesh
	mi.material_override = mat
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(mi)
	mi.position = Vector3(LAKE.x, water_level, LAKE.z)


func _noise_tex(freq: float, octaves: int, cellular: bool, normal_map: bool = false) -> NoiseTexture2D:
	var n := FastNoiseLite.new()
	n.seed = noise_seed + 10
	n.frequency = freq
	n.fractal_octaves = octaves
	if cellular:
		n.noise_type = FastNoiseLite.TYPE_CELLULAR
		n.cellular_return_type = FastNoiseLite.RETURN_DISTANCE2_SUB
	var t := NoiseTexture2D.new()
	t.width = 512
	t.height = 512
	t.seamless = true
	t.noise = n
	if normal_map:
		t.as_normal_map = true
		t.bump_strength = 6.0
	return t


func _make_materials() -> void:
	ground_material = ShaderMaterial.new()
	ground_material.shader = TERRAIN_SHADER
	for layer in ["grass", "dirt", "rock"]:
		ground_material.set_shader_parameter(layer + "_alb", U.tex(layer + "_alb"))
		ground_material.set_shader_parameter(layer + "_nrm", U.tex(layer + "_nrm"))
	ground_material.set_shader_parameter("mask_tex", _noise_tex(0.012, 5, false))
	ground_material.set_shader_parameter("rock_tint", Color(0.78, 0.76, 0.74))
	ground_material.set_shader_parameter("cloud_tex", U.tex("clouds"))
	rock_material = ground_material.duplicate()
	rock_material.set_shader_parameter("force_rock", 1.0)
	rock_material.set_shader_parameter("rock_scale", 0.12)


func _build_ground() -> void:
	var n := int(size / cell)
	var half := size * 0.5
	var w := n + 1
	var hs := PackedFloat32Array()
	hs.resize(w * w)
	for iz in w:
		for ix in w:
			hs[iz * w + ix] = height_at(-half + ix * cell, -half + iz * cell)

	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for iz in w:
		for ix in w:
			var hl := hs[iz * w + maxi(ix - 1, 0)]
			var hr := hs[iz * w + mini(ix + 1, n)]
			var hd := hs[maxi(iz - 1, 0) * w + ix]
			var hu := hs[mini(iz + 1, n) * w + ix]
			st.set_normal(Vector3(hl - hr, 2.0 * cell, hd - hu).normalized())
			st.set_uv(Vector2(ix, iz) / float(n))
			st.add_vertex(Vector3(-half + ix * cell, hs[iz * w + ix], -half + iz * cell))
	for iz in n:
		for ix in n:
			var i00 := iz * w + ix
			var i10 := i00 + 1
			var i01 := i00 + w
			var i11 := i01 + 1
			st.add_index(i00)
			st.add_index(i10)
			st.add_index(i01)
			st.add_index(i10)
			st.add_index(i11)
			st.add_index(i01)
	var mesh := st.commit()

	var mi := MeshInstance3D.new()
	mi.mesh = mesh
	mi.material_override = ground_material
	add_child(mi)

	var cs := CollisionShape3D.new()
	cs.shape = mesh.create_trimesh_shape()
	add_child(cs)


func _scatter_rocks() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = noise_seed
	var half := size * 0.5
	# A handful of shared rock shapes, reused with random scale/rotation.
	var shapes := []
	for k in 8:
		shapes.append(U.rock_mesh(Vector3.ONE, noise_seed + k * 31))
	var placed := 0
	var tries := 0
	while placed < 140 and tries < 600:
		tries += 1
		var p := Vector2(rng.randf_range(-half * 0.62, half * 0.62), rng.randf_range(-half * 0.62, half * 0.62))
		if p.length() < 18.0 or _on_pad(p.x, p.y, 1.0) or in_lake(p.x, p.y, 0.8):
			continue
		var sz := Vector3(rng.randf_range(1.2, 4.0), rng.randf_range(0.8, 2.6), rng.randf_range(1.2, 4.0))
		if rng.randf() < 0.14:
			sz *= rng.randf_range(2.0, 3.2)  # boulders you have to walk around
		var shape: Array = shapes[rng.randi() % shapes.size()]
		var body := StaticBody3D.new()
		add_child(body)
		body.position = Vector3(p.x, height_at(p.x, p.y) - sz.y * 0.12, p.y)
		body.rotation = Vector3(rng.randf_range(-0.2, 0.2), rng.randf() * TAU, rng.randf_range(-0.2, 0.2))
		var mi := U.mesh_node(shape[0], rock_material)
		mi.scale = sz
		body.add_child(mi)
		var pts := PackedVector3Array()
		for v in (shape[1] as PackedVector3Array):
			pts.append(v * sz)
		var cs := CollisionShape3D.new()
		var cv := ConvexPolygonShape3D.new()
		cv.points = pts
		cs.shape = cv
		body.add_child(cs)
		placed += 1


func _on_pad(x: float, z: float, margin: float) -> bool:
	for p in PADS:
		if Vector2(x - p.x, z - p.z).length() < p.y * margin:
			return true
	return false
