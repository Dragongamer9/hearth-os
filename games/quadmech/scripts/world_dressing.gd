extends Node3D
## Set dressing for the test map: grass, pine trees and outpost structures.

const U = preload("res://scripts/util.gd")

var terrain
var grass_nodes: Array[MultiMeshInstance3D] = []
var _rng := RandomNumberGenerator.new()

var _concrete: ShaderMaterial
var _concrete_dark: ShaderMaterial
var _tank_paint: ShaderMaterial
var _steel: ShaderMaterial
var _hazard: ShaderMaterial
var _container_mats: Array[ShaderMaterial] = []
var _window: StandardMaterial3D
var _red: StandardMaterial3D


func build(t) -> void:
	terrain = t
	_rng.seed = 4242
	_concrete = U.armor({"surface": "concrete", "tex_scale": 0.2, "base_color": Color(0.5, 0.48, 0.45), "camo_amount": 0.0, "tex_albedo": 1.0, "tex_normal": 0.8,
		"panel_size": 3.2, "panel_line": 0.02, "panel_depth": 0.4, "wear": 0.0, "base_metallic": 0.0, "base_roughness": 0.92,
		"dirt_amount": 0.85, "dust_amount": 0.5})
	_concrete_dark = _concrete.duplicate()
	_concrete_dark.set_shader_parameter("base_color", Color(0.45, 0.44, 0.41))
	_tank_paint = U.armor({"base_color": Color(0.58, 0.56, 0.52), "camo_amount": 0.0, "panel_size": 1.8, "panel_scale": Vector2(2.0, 1.0), "wear": 0.35,
		"base_metallic": 0.2, "base_roughness": 0.65, "dirt_amount": 0.95, "dirt_color": Color(0.35, 0.2, 0.1)})
	_steel = U.armor({"base_color": Color(0.2, 0.21, 0.22), "camo_amount": 0.0, "panel_size": 0.6, "wear": 0.6,
		"base_metallic": 0.7, "base_roughness": 0.5, "dirt_color": Color(0.35, 0.2, 0.1)})
	_hazard = U.armor({"hazard": true, "hazard_scale": 0.9, "wear": 0.35, "panel_size": 3.0})
	for c in [Color(0.5, 0.18, 0.12), Color(0.16, 0.27, 0.38), Color(0.3, 0.34, 0.2), Color(0.62, 0.45, 0.15)]:
		_container_mats.append(U.armor({"base_color": c, "camo_amount": 0.0, "panel_size": 0.3, "panel_line": 0.05,
			"panel_scale": Vector2(1.0, 60.0), "panel_depth": 0.35, "wear": 0.5, "base_metallic": 0.4, "base_roughness": 0.6, "dirt_color": Color(0.3, 0.17, 0.08)}))
	_window = U.glow_mat(Color(1.0, 0.82, 0.55), 3.0)
	_red = U.glow_mat(Color(1.0, 0.08, 0.04), 8.0)

	_build_depot(terrain.PADS[0])
	_build_comm_station(terrain.PADS[1])
	_build_ruins(terrain.PADS[2])
	_build_trees()
	_build_grass()


# ---------------------------------------------------------------- helpers

func _ground(x: float, z: float) -> float:
	return terrain.height_at(x, z)


func _body(pos: Vector3, rot_y: float = 0.0) -> StaticBody3D:
	var b := StaticBody3D.new()
	add_child(b)
	b.position = pos
	b.rotation.y = rot_y
	return b


func _col_box(b: Node3D, size: Vector3, offset: Vector3) -> void:
	var cs := CollisionShape3D.new()
	var bs := BoxShape3D.new()
	bs.size = size
	cs.shape = bs
	cs.position = offset
	b.add_child(cs)


func _add(parent: Node3D, node: Node3D, pos: Vector3, rot: Vector3 = Vector3.ZERO) -> Node3D:
	parent.add_child(node)
	node.position = pos
	node.rotation = rot
	return node


func _container(x: float, z: float, rot: float, stack: int = 0) -> void:
	var y := _ground(x, z) + stack * 2.6
	var b := _body(Vector3(x, y, z), rot)
	var m: ShaderMaterial = _container_mats[_rng.randi() % _container_mats.size()]
	_add(b, U.chamfer_box(Vector3(2.44, 2.6, 6.1), 0.06, m), Vector3(0, 1.3, 0))
	for end in [-1.0, 1.0]:
		_add(b, U.chamfer_box(Vector3(2.5, 2.66, 0.12), 0.03, _steel), Vector3(0, 1.3, end * 3.0))
	_col_box(b, Vector3(2.44, 2.6, 6.1), Vector3(0, 1.3, 0))


func _tank(x: float, z: float, r: float, h: float) -> void:
	var y := _ground(x, z)
	var b := _body(Vector3(x, y, z), _rng.randf() * TAU)
	_add(b, U.cylinder(r + 0.3, 0.6, _concrete_dark, 32), Vector3(0, 0.3, 0))
	_add(b, U.cylinder(r, h, _tank_paint, 32), Vector3(0, 0.6 + h * 0.5, 0))
	var dome := U.sphere(r, _tank_paint, 28)
	dome.scale = Vector3(1, 0.32, 1)
	_add(b, dome, Vector3(0, 0.6 + h, 0))
	for k in 2:
		_add(b, U.cylinder(r + 0.06, 0.35, _hazard, 32), Vector3(0, 1.6 + k * (h - 2.6), 0))
	# Ladder and catwalk
	for side in [-0.3, 0.3]:
		_add(b, U.box(Vector3(0.08, h, 0.08), _steel), Vector3(side, 0.6 + h * 0.5, r + 0.25))
	for k in int(h / 0.5):
		_add(b, U.box(Vector3(0.6, 0.05, 0.05), _steel), Vector3(0, 0.9 + k * 0.5, r + 0.25))
	_add(b, U.cylinder(r + 0.9, 0.12, _steel, 32), Vector3(0, 0.6 + h, 0))
	var cs := CollisionShape3D.new()
	var cy := CylinderShape3D.new()
	cy.radius = r + 0.3
	cy.height = h + 1.5
	cs.shape = cy
	cs.position.y = (h + 1.5) * 0.5
	b.add_child(cs)


func _bunker(x: float, z: float, rot: float, size: Vector3) -> void:
	var y := _ground(x, z)
	var b := _body(Vector3(x, y, z), rot)
	_add(b, U.prism(PackedVector2Array([
		Vector2(-size.z * 0.5, 0.0), Vector2(-size.z * 0.5 + 0.8, size.y), Vector2(size.z * 0.5 - 0.8, size.y), Vector2(size.z * 0.5, 0.0)]),
		size.x, 0.3, _concrete), Vector3(0, -0.3, 0))
	var slit := U.box(Vector3(size.x * 0.6, 0.35, 0.06), _window)
	_add(b, slit, Vector3(0, size.y * 0.62, -size.z * 0.5 + 0.55), Vector3(-0.35, 0, 0))
	_add(b, U.chamfer_box(Vector3(size.x * 0.64, 0.2, 0.5), 0.05, _concrete_dark), Vector3(0, size.y * 0.75, -size.z * 0.5 + 0.7))
	_add(b, U.chamfer_box(Vector3(2.2, 2.6, 0.2), 0.05, _steel), Vector3(size.x * 0.3, 1.0, size.z * 0.5 - 0.25))
	for k in 2:
		_add(b, U.chamfer_box(Vector3(1.0, 0.6, 1.0), 0.1, _steel), Vector3(-size.x * 0.25 + k * 1.4, size.y - 0.1, 0.5))
	_col_box(b, Vector3(size.x, size.y, size.z), Vector3(0, size.y * 0.5 - 0.3, 0))


func _tower(x: float, z: float, h: float) -> void:
	var y := _ground(x, z)
	var b := _body(Vector3(x, y, z), _rng.randf() * TAU)
	var base_w := 3.0
	var top_w := 1.0
	var corners := [Vector2(-1, -1), Vector2(1, -1), Vector2(1, 1), Vector2(-1, 1)]
	for c in corners:
		var p0 := Vector3(c.x * base_w, 0, c.y * base_w)
		var p1 := Vector3(c.x * top_w, h, c.y * top_w)
		_beam(b, p0, p1, 0.16)
	var levels := int(h / 4.0)
	for k in levels + 1:
		var t := float(k) / float(levels)
		var w := lerpf(base_w, top_w, t)
		var ly := h * t
		for i in 4:
			var a: Vector2 = corners[i]
			var c2: Vector2 = corners[(i + 1) % 4]
			_beam(b, Vector3(a.x * w, ly, a.y * w), Vector3(c2.x * w, ly, c2.y * w), 0.08)
			if k < levels:
				var w2 := lerpf(base_w, top_w, float(k + 1) / float(levels))
				_beam(b, Vector3(a.x * w, ly, a.y * w), Vector3(c2.x * w2, ly + h / levels, c2.y * w2), 0.06)
	_add(b, U.chamfer_box(Vector3(3.0, 0.3, 3.0), 0.05, _steel), Vector3(0, h, 0))
	_add(b, U.cylinder(0.08, 6.0, _steel, 6), Vector3(0, h + 3.0, 0))
	_add(b, U.sphere(0.3, _red, 8), Vector3(0, h + 6.1, 0))
	for k in 2:
		var dish := U.cylinder(0.9, 0.25, _tank_paint, 16)
		_add(b, dish, Vector3(0.6 - k * 1.2, h - 2.5 - k * 3.0, 1.0), Vector3(PI / 2.0, k * 1.4, 0))
	_add(b, U.chamfer_box(Vector3(6.5, 0.4, 6.5), 0.1, _concrete), Vector3(0, 0.0, 0))
	_col_box(b, Vector3(5.0, h, 5.0), Vector3(0, h * 0.5, 0))


func _beam(parent: Node3D, a: Vector3, c: Vector3, t: float) -> void:
	var mi := U.box(Vector3(t, t, 1.0), _steel)
	parent.add_child(mi)
	var tr := Transform3D(Basis(), a)
	var up := Vector3.UP if absf((c - a).normalized().y) < 0.95 else Vector3.FORWARD
	tr = tr.looking_at(c, up)
	tr.basis = tr.basis * Basis.from_scale(Vector3(1, 1, (c - a).length()))
	tr.origin = (a + c) * 0.5
	mi.transform = tr


func _wall(x: float, z: float, rot: float, length: float, broken: bool = false) -> void:
	var y := _ground(x, z)
	var b := _body(Vector3(x, y - 0.3, z), rot)
	var h := 3.6
	var segs := int(length / 4.0)
	for k in segs:
		var sh := h * (_rng.randf_range(0.3, 1.0) if broken and _rng.randf() < 0.5 else 1.0)
		if broken and _rng.randf() < 0.2:
			continue
		var seg := U.prism(PackedVector2Array([Vector2(-0.9, 0.0), Vector2(-0.35, sh), Vector2(0.35, sh), Vector2(0.9, 0.0)]), 3.9, 0.1, _concrete)
		_add(b, seg, Vector3(-length * 0.5 + 2.0 + k * 4.0, 0, 0))
	_col_box(b, Vector3(length, h, 1.4), Vector3(0, h * 0.5, 0))


func _rubble(x: float, z: float, n: int) -> void:
	for k in n:
		var px := x + _rng.randf_range(-6, 6)
		var pz := z + _rng.randf_range(-6, 6)
		var s := Vector3(_rng.randf_range(0.6, 2.2), _rng.randf_range(0.4, 1.2), _rng.randf_range(0.6, 2.2))
		var b := _body(Vector3(px, _ground(px, pz) - 0.1, pz), _rng.randf() * TAU)
		var mi := U.chamfer_box(s, 0.08, _concrete if _rng.randf() < 0.7 else _concrete_dark)
		mi.rotation = Vector3(_rng.randf_range(-0.4, 0.4), 0, _rng.randf_range(-0.4, 0.4))
		_add(b, mi, Vector3(0, s.y * 0.4, 0), mi.rotation)
		_col_box(b, s, Vector3(0, s.y * 0.4, 0))


func _pipe(a: Vector3, c: Vector3, r: float) -> void:
	var mi := U.cylinder(r, (c - a).length(), _tank_paint, 12)
	add_child(mi)
	var dir := (c - a).normalized()
	var up := Vector3.UP if absf(dir.y) < 0.95 else Vector3.FORWARD
	var tr := Transform3D(Basis(), a).looking_at(c, up)
	tr.basis = tr.basis * Basis(Vector3.RIGHT, PI / 2.0)
	tr.origin = (a + c) * 0.5
	mi.transform = tr


# ---------------------------------------------------------------- outposts

func _build_depot(pad: Vector3) -> void:
	var cx := pad.x
	var cz := pad.z
	var tanks := [Vector2(-12, -8), Vector2(0, -12), Vector2(12, -8)]
	for t in tanks:
		_tank(cx + t.x, cz + t.y, 4.2, 8.0)
	var gy := _ground(cx, cz)
	for i in 2:
		var a: Vector2 = tanks[i]
		var c: Vector2 = tanks[i + 1]
		_pipe(Vector3(cx + a.x, gy + 1.2, cz + a.y), Vector3(cx + c.x, gy + 1.2, cz + c.y), 0.35)
	_pipe(Vector3(cx, gy + 1.2, cz - 12), Vector3(cx, gy + 1.2, cz + 4), 0.35)
	_bunker(cx - 2, cz + 10, 0.1, Vector3(12, 4.2, 7))
	for k in 4:
		_container(cx + 16 + (k % 2) * 3.0, cz + 6 + (k / 2) * 7.0, 0.05)
	_container(cx + 16, cz + 6, 0.05, 1)
	_container(cx - 18, cz + 4, 1.4)
	_container(cx - 20, cz + 10, 1.2)
	_wall(cx, cz + 24, 0.0, 30.0)
	_wall(cx - 26, cz, PI / 2.0, 22.0)
	_tower(cx + 22, cz - 18, 22.0)


func _build_comm_station(pad: Vector3) -> void:
	var cx := pad.x
	var cz := pad.z
	_tower(cx, cz, 34.0)
	_bunker(cx + 11, cz + 6, -0.6, Vector3(9, 3.8, 6))
	_bunker(cx - 10, cz - 7, 2.4, Vector3(8, 3.4, 6))
	_container(cx + 4, cz - 14, 0.3)
	_container(cx + 7, cz - 13, 0.3)
	_container(cx + 5.5, cz - 13.5, 0.3, 1)
	_tank(cx - 14, cz + 10, 2.6, 5.0)
	_wall(cx + 2, cz + 20, 0.2, 26.0)


func _build_ruins(pad: Vector3) -> void:
	var cx := pad.x
	var cz := pad.z
	_wall(cx - 6, cz - 8, 0.3, 24.0, true)
	_wall(cx + 8, cz + 2, 1.8, 20.0, true)
	_wall(cx - 2, cz + 12, -0.2, 16.0, true)
	_bunker(cx + 2, cz - 2, 0.9, Vector3(7, 2.2, 5))
	_rubble(cx, cz, 18)
	_rubble(cx + 10, cz - 10, 10)
	_container(cx - 12, cz + 4, 2.1)


# ---------------------------------------------------------------- foliage

func _tree_mesh() -> ArrayMesh:
	## Pine: tapered bark trunk + whorls of drooping needle-frond cards.
	var mesh := ArrayMesh.new()
	var trng := RandomNumberGenerator.new()
	trng.seed = 11
	var height := 11.0

	var trunk_st := SurfaceTool.new()
	trunk_st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var trunk := CylinderMesh.new()
	trunk.top_radius = 0.05
	trunk.bottom_radius = 0.34
	trunk.height = height
	trunk.radial_segments = 8
	trunk.rings = 4
	trunk_st.append_from(trunk, 0, Transform3D(Basis(), Vector3(0, height * 0.5 - 0.3, 0)))
	trunk_st.commit(mesh)

	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var y := 1.6
	while y < height - 0.6:
		var t := (y - 1.6) / (height - 2.2)
		var count := 6 if t < 0.7 else 5
		var spin := trng.randf() * TAU
		for b in count:
			var az := spin + TAU * b / count + trng.randf_range(-0.25, 0.25)
			var dir := Vector3(cos(az), 0.0, sin(az))
			var length := lerpf(3.3, 0.7, pow(t, 0.9)) * trng.randf_range(0.85, 1.15)
			var droop := trng.randf_range(0.25, 0.45)
			var base := Vector3(0, y, 0) + dir * 0.15
			var tip := base + dir * length + Vector3(0, -droop * length + 0.25, 0)
			var width := length * 0.62
			for roll in [0.0, 1.0]:
				var side := Vector3(-dir.z, 0, dir.x)
				if roll > 0.5:
					side = (side * 0.55 + Vector3.UP * 0.8).normalized()
				_card(st, base, tip, side * width * 0.5, dir, length / 3.3)
		y += trng.randf_range(0.6, 0.8)
	# leader at the top
	for k in 2:
		var az := k * PI * 0.5
		var side := Vector3(cos(az), 0, sin(az)) * 0.35
		_card(st, Vector3(0, height - 1.6, 0), Vector3(0, height + 0.3, 0), side, Vector3.UP, 0.3)
	st.commit(mesh)

	var bark := StandardMaterial3D.new()
	bark.albedo_texture = U.tex("bark_alb")
	bark.normal_enabled = true
	bark.normal_texture = U.tex("bark_nrm")
	bark.roughness = 0.95
	bark.uv1_scale = Vector3(3.0, 6.0, 1.0)
	var needles := ShaderMaterial.new()
	needles.shader = load("res://shaders/foliage.gdshader")
	needles.set_shader_parameter("albedo_tex", U.tex("needles_card"))
	needles.set_shader_parameter("wind", 0.18)
	needles.set_shader_parameter("tint", Color(0.8, 0.86, 0.78))
	needles.set_shader_parameter("cloud_tex", U.tex("clouds"))
	mesh.surface_set_material(0, bark)
	mesh.surface_set_material(1, needles)
	return mesh


## One frond card from base to tip. U runs along the branch, V across it.
func _card(st: SurfaceTool, base: Vector3, tip: Vector3, half_side: Vector3, out_dir: Vector3, sway: float) -> void:
	var nrm := (out_dir * 0.55 + Vector3.UP * 0.85).normalized()
	var v := [base - half_side, base + half_side, tip + half_side, tip - half_side]
	var uvs := [Vector2(0, 0), Vector2(0, 1), Vector2(1, 1), Vector2(1, 0)]
	var sw := [0.0, 0.0, sway, sway]
	for idx in [0, 1, 2, 0, 2, 3]:
		st.set_normal(nrm)
		st.set_uv(uvs[idx])
		st.set_uv2(Vector2(sw[idx], 0))
		st.add_vertex(v[idx])


func _build_trees() -> void:
	var dens := FastNoiseLite.new()
	dens.seed = 99
	dens.frequency = 0.012
	var xforms: Array[Transform3D] = []
	var colors: Array[Color] = []
	var half: float = terrain.size * 0.5 * 0.9
	var tries := 0
	while xforms.size() < 1500 and tries < 20000:
		tries += 1
		var x := _rng.randf_range(-half, half)
		var z := _rng.randf_range(-half, half)
		if Vector2(x, z).length() < 28.0 or terrain._on_pad(x, z, 1.15) or terrain.in_lake(x, z, 1.05):
			continue
		var hill := clampf((Vector2(x, z).length() - 150.0) / 40.0, 0.0, 1.0)
		if dens.get_noise_2d(x, z) < 0.12 - hill * 0.3:
			continue
		var y := _ground(x, z)
		var slope := absf(_ground(x + 1.0, z) - y) + absf(_ground(x, z + 1.0) - y)
		if slope > 1.8:
			continue
		var s := _rng.randf_range(0.7, 1.35)
		var tb := Basis(Vector3.UP, _rng.randf() * TAU).scaled(Vector3(s, s * _rng.randf_range(0.9, 1.2), s))
		xforms.append(Transform3D(tb, Vector3(x, y - 0.2, z)))
		var v := _rng.randf_range(0.8, 1.15)
		colors.append(Color(v * _rng.randf_range(0.92, 1.05), v, v * _rng.randf_range(0.85, 1.0)))
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.mesh = _tree_mesh()
	mm.instance_count = xforms.size()
	for i in xforms.size():
		mm.set_instance_transform(i, xforms[i])
		mm.set_instance_color(i, colors[i])
	var mmi := MultiMeshInstance3D.new()
	mmi.multimesh = mm
	mmi.layers = 4
	add_child(mmi)


func _grass_mesh() -> ArrayMesh:
	## Three crossed alpha cards per tuft. UV.y = 0 at the tips.
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var w := 0.5
	var h := 0.55
	for k in 3:
		var az := k * PI / 3.0 + 0.2
		var side := Vector3(cos(az), 0, sin(az)) * w
		var v := [-side + Vector3(0, -0.05, 0), side + Vector3(0, -0.05, 0), side + Vector3(0, h, 0), -side + Vector3(0, h, 0)]
		var uvs := [Vector2(0, 1), Vector2(1, 1), Vector2(1, 0), Vector2(0, 0)]
		for idx in [0, 1, 2, 0, 2, 3]:
			st.set_normal(Vector3.UP)
			st.set_uv(uvs[idx])
			st.add_vertex(v[idx])
	return st.commit()


func _build_grass() -> void:
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/foliage.gdshader")
	mat.set_shader_parameter("albedo_tex", U.tex("grass_card"))
	mat.set_shader_parameter("grass_mode", true)
	mat.set_shader_parameter("wind", 0.1)
	mat.set_shader_parameter("alpha_cut", 0.4)
	mat.set_shader_parameter("tint", Color(0.66, 0.74, 0.5))
	mat.set_shader_parameter("translucency", Color(0.08, 0.1, 0.03))
	mat.set_shader_parameter("ground_tex", U.tex("grass_alb"))
	mat.set_shader_parameter("ground_blend", 0.75)
	mat.set_shader_parameter("cloud_tex", U.tex("clouds"))
	var mesh := _grass_mesh()
	mesh.surface_set_material(0, mat)
	var dens := FastNoiseLite.new()
	dens.seed = 7
	dens.frequency = 0.03
	var chunk := 25.0
	var reach := 175.0
	var per_chunk := 480
	var cz := -reach
	while cz < reach:
		var cx := -reach
		while cx < reach:
			var xforms: Array[Transform3D] = []
			var colors: Array[Color] = []
			for i in per_chunk:
				var x := cx + _rng.randf() * chunk
				var z := cz + _rng.randf() * chunk
				if terrain.in_lake(x, z, 0.86) or terrain._on_pad(x, z, 0.8):
					continue
				var shore: bool = terrain.in_lake(x, z, 1.08)
				if not shore and dens.get_noise_2d(x, z) < -0.25:
					continue
				var y := _ground(x, z)
				var slope := absf(_ground(x + 1.0, z) - y) + absf(_ground(x, z + 1.0) - y)
				if slope > 0.55:
					continue
				var s := _rng.randf_range(0.7, 1.15) * (1.9 if shore else 1.0)  # tall reeds along the shore
				var center := Vector3(cx + chunk * 0.5, 0.0, cz + chunk * 0.5)
				xforms.append(Transform3D(Basis(Vector3.UP, _rng.randf() * TAU).scaled(Vector3(s, s, s)), Vector3(x, y - 0.05, z) - center))
				var v := _rng.randf_range(0.7, 1.1)
				var dry := clampf(dens.get_noise_2d(x * 0.3, z * 0.3) + 0.5, 0.0, 1.0)
				colors.append(Color(v * lerpf(0.85, 1.15, dry), v, v * lerpf(0.9, 0.75, dry)))
			if xforms.size() > 0:
				var mm := MultiMesh.new()
				mm.transform_format = MultiMesh.TRANSFORM_3D
				mm.use_colors = true
				mm.mesh = mesh
				mm.instance_count = xforms.size()
				for i in xforms.size():
					mm.set_instance_transform(i, xforms[i])
					mm.set_instance_color(i, colors[i])
				var mmi := MultiMeshInstance3D.new()
				mmi.multimesh = mm
				mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
				mmi.layers = 4
				mmi.visibility_range_end = 120.0
				mmi.visibility_range_end_margin = 20.0
				mmi.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
				add_child(mmi)
				mmi.position = Vector3(cx + chunk * 0.5, 0.0, cz + chunk * 0.5)
				grass_nodes.append(mmi)
			cx += chunk
		cz += chunk
