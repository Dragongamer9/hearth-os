// Painted armor / structure surface.
// Texture maps (triplanar, object space) supply paint mottling, micro normals,
// roughness and a wear mask; on top of that: camo, panel seams with beveled
// normals, bare-metal scratches/chips, grime streaks and dust on top faces.
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

uniform sampler2D surf_alb : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D surf_nrm : filter_linear_mipmap_anisotropic, repeat_enable;
uniform float tex_scale = 0.33;
uniform float tex_normal = 0.22;
uniform float tex_albedo : hint_range(0.0, 1.0) = 0.45;

uniform vec3 base_color : source_color = vec3(0.3, 0.32, 0.3);
uniform vec3 camo_color : source_color = vec3(0.17, 0.185, 0.175);
uniform float camo_amount : hint_range(0.0, 1.0) = 1.0;
uniform float camo_scale = 0.22;
uniform vec3 metal_color : source_color = vec3(0.56, 0.55, 0.53);
uniform vec3 dirt_color : source_color = vec3(0.2, 0.16, 0.12);
uniform float dirt_amount : hint_range(0.0, 1.0) = 0.5;
uniform vec3 dust_color : source_color = vec3(0.55, 0.48, 0.37);
uniform float dust_amount : hint_range(0.0, 1.0) = 0.35;
uniform float panel_size = 1.25;
uniform vec2 panel_scale = vec2(1.0, 1.0);  // stretch cells, e.g. (1, 60) = vertical lines only
uniform float panel_line = 0.016;
uniform float panel_depth : hint_range(0.0, 1.0) = 0.5;
uniform float wear : hint_range(0.0, 1.0) = 0.3;
uniform float base_metallic : hint_range(0.0, 1.0) = 0.25;
uniform float base_roughness : hint_range(0.0, 1.0) = 0.6;
uniform bool hazard = false;
uniform vec3 hazard_color : source_color = vec3(0.9, 0.48, 0.06);
uniform float hazard_scale = 1.4;
uniform vec3 emission_color : source_color = vec3(1.0, 0.32, 0.05);
uniform float emission_strength = 0.0;

varying vec3 lp;   // local position with node scale applied (panels stay square on stretched parts)
varying vec3 ln;
varying vec3 vax;
varying vec3 vay;
varying vec3 vaz;

float hash(vec3 p) {
	p = fract(p * 0.3183099 + vec3(0.1, 0.2, 0.3));
	p *= 17.0;
	return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

float vnoise(vec3 x) {
	vec3 i = floor(x);
	vec3 f = fract(x);
	f = f * f * (3.0 - 2.0 * f);
	return mix(
		mix(mix(hash(i), hash(i + vec3(1, 0, 0)), f.x), mix(hash(i + vec3(0, 1, 0)), hash(i + vec3(1, 1, 0)), f.x), f.y),
		mix(mix(hash(i + vec3(0, 0, 1)), hash(i + vec3(1, 0, 1)), f.x), mix(hash(i + vec3(0, 1, 1)), hash(i + vec3(1, 1, 1)), f.x), f.y),
		f.z);
}

float fbm(vec3 p) {
	float a = 0.5;
	float s = 0.0;
	for (int i = 0; i < 3; i++) {
		s += a * vnoise(p);
		p *= 2.03;
		a *= 0.5;
	}
	return s;
}

float edge_dist(vec2 uv) {
	vec2 cell = uv / (panel_size * panel_scale);
	cell.x += floor(cell.y) * 0.37;
	vec2 f = abs(fract(cell) - 0.5);
	vec2 ed = (0.5 - f) * panel_size * panel_scale;
	return min(ed.x, ed.y);
}

void vertex() {
	vec3 sc = vec3(length(MODEL_MATRIX[0].xyz), length(MODEL_MATRIX[1].xyz), length(MODEL_MATRIX[2].xyz));
	lp = VERTEX * sc;
	ln = NORMAL;
	vax = normalize((MODELVIEW_MATRIX * vec4(1.0, 0.0, 0.0, 0.0)).xyz);
	vay = normalize((MODELVIEW_MATRIX * vec4(0.0, 1.0, 0.0, 0.0)).xyz);
	vaz = normalize((MODELVIEW_MATRIX * vec4(0.0, 0.0, 1.0, 0.0)).xyz);
}

void fragment() {
	vec3 p = lp;
	vec3 n = normalize(ln);
	vec3 an = abs(n);

	// --- triplanar texture maps
	vec3 bw = pow(an, vec3(8.0));
	bw /= (bw.x + bw.y + bw.z);
	vec2 uvx = p.zy * tex_scale;
	vec2 uvy = p.xz * tex_scale;
	vec2 uvz = p.xy * tex_scale;
	vec4 ta = texture(surf_alb, uvx) * bw.x + texture(surf_alb, uvy) * bw.y + texture(surf_alb, uvz) * bw.z;
	vec4 nx4 = texture(surf_nrm, uvx);
	vec4 ny4 = texture(surf_nrm, uvy);
	vec4 nz4 = texture(surf_nrm, uvz);
	float t_rough = nx4.b * bw.x + ny4.b * bw.y + nz4.b * bw.z;
	vec2 tx = (nx4.rg * 2.0 - 1.0) * tex_normal;
	vec2 ty = (ny4.rg * 2.0 - 1.0) * tex_normal;
	vec2 tz = (nz4.rg * 2.0 - 1.0) * tex_normal;
	vec3 tnorm = normalize(vec3(tx + n.zy, n.x).zyx * bw.x + vec3(ty + n.xz, n.y).xzy * bw.y + vec3(tz + n.xy, n.z) * bw.z);
	float wear_mask = clamp((ta.a - 0.1255) / 0.8745, 0.0, 1.0);

	// --- panel seams on the dominant plane, with beveled normals
	int axis = an.x > an.y && an.x > an.z ? 0 : (an.y > an.z ? 1 : 2);
	vec2 uv = axis == 0 ? p.zy : (axis == 1 ? p.xz : p.xy);
	float ed = edge_dist(uv);
	float panel = 1.0 - smoothstep(0.0, panel_line, ed);
	float rim = 1.0 - smoothstep(panel_line, panel_line * 4.0, ed);
	float e = 0.004;
	vec2 g = vec2(edge_dist(uv + vec2(e, 0.0)) - edge_dist(uv - vec2(e, 0.0)),
		edge_dist(uv + vec2(0.0, e)) - edge_dist(uv - vec2(0.0, e))) / (2.0 * e);
	vec2 tilt = -g * (1.0 - smoothstep(panel_line * 0.3, panel_line * 1.6, ed)) * 0.9 * panel_depth;
	vec3 seam = axis == 0 ? vec3(0.0, tilt.y, tilt.x) : (axis == 1 ? vec3(tilt.x, 0.0, tilt.y) : vec3(tilt.x, tilt.y, 0.0));
	vec2 cid = floor(uv / (panel_size * panel_scale));
	float panel_id = hash(vec3(cid, float(axis) * 7.0));

	// --- paint
	float camo = smoothstep(0.47, 0.53, fbm(p * camo_scale + vec3(3.1, 7.7, 1.3)));
	vec3 paint = mix(base_color, camo_color, camo * camo_amount);
	if (hazard) {
		float stripe = step(0.5, fract((p.x + p.y + p.z) * hazard_scale));
		paint = mix(hazard_color, vec3(0.04), stripe);
	}
	paint *= mix(vec3(1.0), ta.rgb * 2.0, tex_albedo);  // texture mottling and grime (texture is centered on 0.5)
	paint *= 1.0 + (panel_id - 0.5) * 0.07;

	// --- wear: texture scratches/chips, concentrated on panel rims
	float th = 1.0 - wear * 0.55;
	float bare = smoothstep(th, th + 0.05, wear_mask * (0.75 + rim * 0.35)) * (1.0 - panel);

	// --- grime streaks running down, dust settling on top faces
	float streak = smoothstep(0.5, 0.8, fbm(p * vec3(3.0, 0.4, 3.0) + vec3(9.0))) * dirt_amount;
	vec3 wn = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
	float dust = smoothstep(0.3, 0.92, wn.y) * dust_amount * (0.5 + 0.8 * vnoise(p * 2.3));

	vec3 albedo = mix(paint, metal_color * (0.75 + 0.5 * ta.r), bare);
	albedo = mix(albedo, dirt_color, streak * 0.5);
	albedo = mix(albedo, dust_color, dust);
	albedo *= 1.0 - panel * panel_depth * 0.8;
	albedo = mix(albedo, albedo * 0.7, rim * 0.35 * dirt_amount);  // grime collects along seams

	// --- normals: perturb Godot's geometric normal by our local-space delta
	vec3 nl = normalize(tnorm + seam);
	vec3 d = nl - n;
	NORMAL = normalize(NORMAL + vax * d.x + vay * d.y + vaz * d.z);

	ALBEDO = albedo;
	METALLIC = mix(base_metallic, 0.95, bare) * (1.0 - dust);
	ROUGHNESS = clamp(mix(base_roughness, t_rough, 0.3) - bare * 0.3 + dust * 0.3 + streak * 0.1, 0.06, 1.0);
	AO = 1.0 - panel * 0.6;
	AO_LIGHT_AFFECT = 0.2;
	EMISSION = emission_color * emission_strength;
}
