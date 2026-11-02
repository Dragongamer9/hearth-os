// Ground and boulders. Texture splatting of grass / soil / rock with
// height-based blending, anti-tiling, triplanar rock and per-layer normals.
// *_alb: RGB albedo, A height (32..255).  *_nrm: RG normal, B roughness.
shader_type spatial;
render_mode diffuse_burley, specular_schlick_ggx;

uniform sampler2D grass_alb : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D grass_nrm : filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D dirt_alb : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D dirt_nrm : filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D rock_alb : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D rock_nrm : filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D mask_tex : filter_linear_mipmap, repeat_enable;

uniform float grass_scale = 0.17;
uniform float dirt_scale = 0.21;
uniform float rock_scale = 0.055;
uniform vec3 grass_tint : source_color = vec3(1.0);
uniform vec3 dirt_tint : source_color = vec3(1.0);
uniform vec3 rock_tint : source_color = vec3(1.0);
uniform float force_rock : hint_range(0.0, 1.0) = 0.0;
uniform float normal_strength = 1.0;
uniform sampler2D cloud_tex : filter_linear_mipmap, repeat_enable;
uniform float cloud_shadow = 0.38;
uniform vec2 cloud_wind = vec2(0.0035, 0.0018);

varying vec3 wp;
varying vec3 wn;

float hdec(float a) { return clamp((a - 0.1255) / 0.8745, 0.0, 1.0); }

vec2 tn(vec4 n) { return n.rg * 2.0 - 1.0; }

// Two samples at different scale/rotation, picked by a noise mask, to hide tiling.
vec4 at(sampler2D t, vec2 uv, float m) {
	vec2 uv2 = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8)) * uv * 0.43 + vec2(0.37, 0.11);
	return mix(texture(t, uv), texture(t, uv2), m);
}

// Height-aware blend: w = how much of B we want, hA/hB = layer heights.
float hblend(float w, float hA, float hB) {
	return smoothstep(0.0, 1.0, clamp((w - 0.5) * 3.0 + (hB - hA) * 1.6 + 0.5, 0.0, 1.0));
}

void vertex() {
	wp = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	wn = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
	vec3 n = normalize(wn);
	float macro = texture(mask_tex, wp.xz * 0.0021).r;
	float macro2 = texture(mask_tex, wp.xz * 0.0063 + vec2(0.3, 0.7)).r;
	float mtile = smoothstep(0.4, 0.6, texture(mask_tex, wp.xz * 0.011).r);
	float d1 = texture(mask_tex, wp.xz * 0.045).r;

	// Ground layers (planar)
	vec2 guv = wp.xz * grass_scale;
	vec4 ga = at(grass_alb, guv, mtile);
	vec4 gn = at(grass_nrm, guv, mtile);
	vec2 duv = wp.xz * dirt_scale;
	vec4 da = at(dirt_alb, duv, 1.0 - mtile);
	vec4 dn = at(dirt_nrm, duv, 1.0 - mtile);

	// Rock (triplanar, whiteout normal blend)
	vec3 bw = pow(abs(n), vec3(6.0));
	bw /= (bw.x + bw.y + bw.z);
	vec2 uvx = wp.zy * rock_scale;
	vec2 uvy = wp.xz * rock_scale * 0.8;
	vec2 uvz = wp.xy * rock_scale;
	vec4 rax = texture(rock_alb, uvx);
	vec4 ray = texture(rock_alb, uvy);
	vec4 raz = texture(rock_alb, uvz);
	vec4 rnx = texture(rock_nrm, uvx);
	vec4 rny = texture(rock_nrm, uvy);
	vec4 rnz = texture(rock_nrm, uvz);
	vec4 ra = rax * bw.x + ray * bw.y + raz * bw.z;
	float rrough = rnx.b * bw.x + rny.b * bw.y + rnz.b * bw.z;
	vec2 tx = tn(rnx) * normal_strength * 1.3;
	vec2 ty = tn(rny) * normal_strength * 1.3;
	vec2 tz = tn(rnz) * normal_strength * 1.3;
	vec3 nX = vec3(tx + n.zy, n.x);
	vec3 nY = vec3(ty + n.xz, n.y);
	vec3 nZ = vec3(tz + n.xy, n.z);
	vec3 rock_n = normalize(nX.zyx * bw.x + nY.xzy * bw.y + nZ.xyz * bw.z);

	// Layer weights
	float slope = 1.0 - n.y;
	float hg = hdec(ga.a);
	float hd = hdec(da.a);
	float hr = hdec(ra.a);
	float dirt_w = smoothstep(0.52, 0.68, macro2 * 0.7 + d1 * 0.45);
	dirt_w = max(dirt_w, smoothstep(0.2, 0.3, slope) * 0.8);
	float t_dirt = hblend(dirt_w, hg, hd);
	float rock_w = max(smoothstep(0.3, 0.45, slope + (macro - 0.5) * 0.2), force_rock);
	float h_ground = mix(hg, hd, t_dirt);
	float t_rock = max(hblend(rock_w, h_ground, hr), force_rock);

	vec3 grass_c = ga.rgb * grass_tint * mix(0.9, 1.12, macro);
	grass_c = mix(grass_c, grass_c * vec3(1.15, 1.05, 0.8), smoothstep(0.5, 0.75, macro2) * 0.6);
	vec3 col = mix(grass_c, da.rgb * dirt_tint, t_dirt);
	vec3 rock_c = ra.rgb * rock_tint * mix(0.85, 1.1, macro2);
	col = mix(col, rock_c, t_rock);

	float rough = mix(mix(gn.b, dn.b, t_dirt), rrough, t_rock);
	vec3 ground_n = normalize(n + vec3(tn(mix(gn, dn, t_dirt)).x, 0.0, tn(mix(gn, dn, t_dirt)).y) * normal_strength);
	vec3 nw = normalize(mix(ground_n, rock_n, t_rock));

	// Moss and dust settling on top of boulders
	if (force_rock > 0.5) {
		float top = smoothstep(0.55, 0.92, n.y) * smoothstep(0.35, 0.65, d1 + hr * 0.4);
		col = mix(col, ga.rgb * 0.85, top * 0.8);
		rough = mix(rough, 0.95, top);
	}

	// Drifting cloud shadows
	float cs = smoothstep(0.5, 0.72, texture(cloud_tex, wp.xz * 0.0016 + TIME * cloud_wind).r);
	col *= 1.0 - cs * cloud_shadow;

	NORMAL = normalize((VIEW_MATRIX * vec4(nw, 0.0)).xyz);
	ALBEDO = col;
	ROUGHNESS = rough;
	SPECULAR = 0.35;
	float h = mix(h_ground, hr, t_rock);
	AO = mix(0.55, 1.0, h);
	AO_LIGHT_AFFECT = 0.35;
}
