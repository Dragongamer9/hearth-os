extends RefCounted
## Geometry and material helpers. Everything in the prototype is built from
## code: beveled armor plates, chamfered boxes, sculpted rocks.

const ARMOR_SHADER = preload("res://shaders/armor.gdshader")


# ---------------------------------------------------------------- materials

static func mat(color: Color, metallic: float = 0.5, roughness: float = 0.6, emission: Color = Color.BLACK, emission_energy: float = 2.0) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.albedo_color = color
	m.metallic = metallic
	m.roughness = roughness
	if emission != Color.BLACK:
		m.emission_enabled = true
		m.emission = emission
		m.emission_energy_multiplier = emission_energy
	return m


static func glow_mat(color: Color, energy: float = 4.0, additive: bool = false) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.albedo_color = color
	if additive:
		m.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		m.albedo_color = color * energy
	else:
		m.emission_enabled = true
		m.emission = color
		m.emission_energy_multiplier = energy
	return m


static func tex(name: String) -> Texture2D:
	return load("res://textures/%s.png" % name)


## Surface material. params["surface"] picks the texture set ("metal" default, or "concrete").
static func armor(params: Dictionary = {}) -> ShaderMaterial:
	var m := ShaderMaterial.new()
	m.shader = ARMOR_SHADER
	var surface: String = params.get("surface", "metal")
	m.set_shader_parameter("surf_alb", tex(surface + "_alb"))
	m.set_shader_parameter("surf_nrm", tex(surface + "_nrm"))
	for k in params:
		if k != "surface":
			m.set_shader_parameter(k, params[k])
	return m


## Mech paint scheme. Returns a dictionary of shared materials.
static func mech_palette() -> Dictionary:
	return {
		"armor": armor({"base_color": Color(0.3, 0.32, 0.3), "camo_color": Color(0.17, 0.185, 0.175), "camo_scale": 0.22,
			"panel_size": 1.25, "panel_line": 0.016, "panel_depth": 0.5, "wear": 0.28}),
		"frame": armor({"base_color": Color(0.13, 0.135, 0.14), "camo_amount": 0.0, "panel_size": 0.7, "wear": 0.22,
			"base_metallic": 0.65, "base_roughness": 0.45, "dust_amount": 0.25, "metal_color": Color(0.55, 0.54, 0.52)}),
		"accent": armor({"hazard": true, "wear": 0.3, "panel_size": 2.0, "dust_amount": 0.2}),
		"orange": armor({"base_color": Color(0.8, 0.38, 0.07), "camo_amount": 0.0, "wear": 0.3, "panel_size": 1.5}),
		"metal": mat(Color(0.62, 0.61, 0.6), 0.95, 0.28),
		"barrel": armor({"base_color": Color(0.09, 0.09, 0.095), "camo_amount": 0.0, "panel_size": 5.0, "panel_line": 0.0,
			"wear": 0.04, "base_metallic": 0.8, "base_roughness": 0.4, "dust_amount": 0.1, "dirt_amount": 0.2}),
		"vent": armor({"base_color": Color(0.05, 0.05, 0.05), "camo_amount": 0.0, "panel_size": 0.12, "panel_line": 0.03,
			"panel_depth": 0.9, "wear": 0.0, "dust_amount": 0.0, "dirt_amount": 0.0}),
		"glass": mat(Color(0.03, 0.05, 0.07), 0.95, 0.04, Color(0.2, 0.45, 0.6), 0.25),
		"lamp": glow_mat(Color(1.0, 0.92, 0.75), 6.0),
		"redlamp": glow_mat(Color(1.0, 0.12, 0.05), 5.0),
		"hole": mat(Color(0.02, 0.02, 0.02), 0.2, 0.9),
	}


# ---------------------------------------------------------------- meshes

static func mesh_node(mesh: Mesh, material: Material) -> MeshInstance3D:
	var mi := MeshInstance3D.new()
	mi.mesh = mesh
	mi.material_override = material
	return mi


static func box(size: Vector3, material: Material) -> MeshInstance3D:
	var b := BoxMesh.new()
	b.size = size
	return mesh_node(b, material)


static func sphere(radius: float, material: Material, segments: int = 16) -> MeshInstance3D:
	var s := SphereMesh.new()
	s.radius = radius
	s.height = radius * 2.0
	s.radial_segments = segments
	s.rings = maxi(segments / 2, 4)
	return mesh_node(s, material)


static func cylinder(radius: float, height: float, material: Material, segments: int = 16, top_radius: float = -1.0) -> MeshInstance3D:
	var c := CylinderMesh.new()
	c.top_radius = radius if top_radius < 0.0 else top_radius
	c.bottom_radius = radius
	c.height = height
	c.radial_segments = segments
	c.rings = 1
	return mesh_node(c, material)


## Cylinder lying along Z (barrels, tubes, pistons).
static func tube(radius: float, length: float, material: Material, segments: int = 14) -> MeshInstance3D:
	var c := cylinder(radius, length, material, segments)
	c.rotation.x = PI / 2.0
	return c


## Beveled prism: a convex side profile given as (z, y) points, extruded along X.
## This is the workhorse for angular armor plates.
static func prism(profile: PackedVector2Array, width: float, bevel: float, material: Material) -> MeshInstance3D:
	return mesh_node(prism_mesh(profile, width, bevel), material)


static func chamfer_box(size: Vector3, bevel: float, material: Material) -> MeshInstance3D:
	return mesh_node(chamfer_box_mesh(size, bevel), material)


static func chamfer_box_mesh(size: Vector3, bevel: float) -> ArrayMesh:
	var hz := size.z * 0.5
	var hy := size.y * 0.5
	var b := minf(bevel, minf(hz, hy) * 0.45)
	var prof := PackedVector2Array([
		Vector2(-hz, -hy + b), Vector2(-hz, hy - b), Vector2(-hz + b, hy), Vector2(hz - b, hy),
		Vector2(hz, hy - b), Vector2(hz, -hy + b), Vector2(hz - b, -hy), Vector2(-hz + b, -hy),
	])
	return prism_mesh(prof, size.x, b)


static func prism_mesh(profile: PackedVector2Array, width: float, bevel: float) -> ArrayMesh:
	var n := profile.size()
	var c2 := Vector2.ZERO
	for p in profile:
		c2 += p
	c2 /= float(n)
	var hx := width * 0.5
	var b := minf(bevel, hx * 0.45)

	# Inset the profile (proper polygon offset) for the beveled caps.
	var area := 0.0
	for i in n:
		var p0 := profile[i]
		var p1 := profile[(i + 1) % n]
		area += p0.x * p1.y - p1.x * p0.y
	var s := 1.0 if area > 0.0 else -1.0
	var inset := PackedVector2Array()
	for i in n:
		var prev := profile[(i - 1 + n) % n]
		var cur := profile[i]
		var nxt := profile[(i + 1) % n]
		var e1 := (cur - prev).normalized()
		var e2 := (nxt - cur).normalized()
		var n1 := Vector2(-e1.y, e1.x) * s
		var n2 := Vector2(-e2.y, e2.x) * s
		var nn := (n1 + n2).normalized()
		var k := maxf(nn.dot(n1), 0.35)
		inset.append(cur + nn * (b / k))

	var rings := [
		_ring(inset if b > 0.0 else profile, -hx),
		_ring(profile, -hx + b),
		_ring(profile, hx - b),
		_ring(inset if b > 0.0 else profile, hx),
	]
	var center := Vector3(0.0, c2.y, c2.x)
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var cap0: PackedVector3Array = rings[0]
	var cap3: PackedVector3Array = rings[3]
	for i in range(1, n - 1):
		_tri(st, cap0[0], cap0[i], cap0[i + 1], Vector3.LEFT)
		_tri(st, cap3[0], cap3[i], cap3[i + 1], Vector3.RIGHT)
	for k in 3:
		if b <= 0.0 and k != 1:
			continue
		var r0: PackedVector3Array = rings[k]
		var r1: PackedVector3Array = rings[k + 1]
		for i in n:
			var j := (i + 1) % n
			var mid := (r0[i] + r0[j] + r1[i] + r1[j]) * 0.25
			var out := mid - Vector3(0.0, center.y, center.z)
			out.x = mid.x * 0.6 if k != 1 else 0.0
			_tri(st, r0[i], r0[j], r1[j], out)
			_tri(st, r0[i], r1[j], r1[i], out)
	return st.commit()


static func _ring(profile: PackedVector2Array, x: float) -> PackedVector3Array:
	var r := PackedVector3Array()
	for p in profile:
		r.append(Vector3(x, p.y, p.x))
	return r


## Adds a flat-shaded triangle, wound so its front face points along `outward`.
static func _tri(st: SurfaceTool, a: Vector3, b: Vector3, c: Vector3, outward: Vector3) -> void:
	var cr := (b - a).cross(c - a)
	if cr.length_squared() < 1e-12:
		return
	# Godot front faces are clockwise, i.e. (b-a)x(c-a) points inward.
	if cr.dot(outward) > 0.0:
		var t := b
		b = c
		c = t
		cr = -cr
	var nrm := -cr.normalized()
	st.set_normal(nrm)
	st.add_vertex(a)
	st.set_normal(nrm)
	st.add_vertex(b)
	st.set_normal(nrm)
	st.add_vertex(c)


## Noise-sculpted, flat-shaded boulder. Returns [mesh, collision_points].
static func rock_mesh(size: Vector3, rng_seed: int) -> Array:
	var sm := SphereMesh.new()
	sm.radius = 0.5
	sm.height = 1.0
	sm.radial_segments = 20
	sm.rings = 12
	var arrays := sm.get_mesh_arrays()
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var idx: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
	var noise := FastNoiseLite.new()
	noise.seed = rng_seed
	noise.frequency = 1.4
	noise.fractal_octaves = 4
	var facets := FastNoiseLite.new()
	facets.seed = rng_seed + 5
	facets.noise_type = FastNoiseLite.TYPE_CELLULAR
	facets.frequency = 2.2
	var pts := PackedVector3Array()
	pts.resize(verts.size())
	for i in verts.size():
		var v := verts[i]
		var d := 1.0 + noise.get_noise_3dv(v * 2.0) * 0.42 + facets.get_noise_3dv(v * 2.0) * 0.1
		var p := v * d
		p.y = maxf(p.y, -0.3)  # flat-ish bottom so it sits in the ground
		pts[i] = p * size
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for t in range(0, idx.size(), 3):
		var a := pts[idx[t]]
		var b := pts[idx[t + 1]]
		var c := pts[idx[t + 2]]
		_tri(st, a, b, c, (a + b + c) / 3.0 - Vector3(0, -0.1 * size.y, 0))
	return [st.commit(), pts]
