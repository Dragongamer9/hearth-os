// Lake water: refraction of the lakebed, depth-based absorption, shoreline foam,
// two scrolling ripple layers and a gentle swell. Reflections come from the sky
// and (on High quality) screen-space reflections via the low roughness.
shader_type spatial;
render_mode blend_mix, depth_draw_always, cull_back, diffuse_burley, specular_schlick_ggx;

uniform sampler2D screen_tex : hint_screen_texture, filter_linear_mipmap;
uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;
uniform sampler2D wave_nrm : filter_linear_mipmap, repeat_enable;

uniform vec3 shallow_tint : source_color = vec3(0.62, 0.7, 0.6);
uniform vec3 deep_color : source_color = vec3(0.02, 0.055, 0.06);
uniform float clarity = 2.2;          // meters until the water is mostly opaque
uniform float wave_strength = 0.35;
uniform float refraction = 0.035;
uniform vec3 foam_color : source_color = vec3(0.85, 0.88, 0.85);

varying vec3 wp;

void vertex() {
	wp = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	VERTEX.y += sin(wp.x * 0.11 + TIME * 0.7) * 0.04 + sin(wp.z * 0.17 - TIME * 0.9) * 0.03;
}

float linear_depth(vec2 uv, mat4 inv_proj) {
	float d = texture(depth_tex, uv).r;
	vec3 ndc = vec3(uv * 2.0 - 1.0, d);
	vec4 v = inv_proj * vec4(ndc, 1.0);
	return -v.z / v.w;
}

void fragment() {
	vec2 uv1 = wp.xz * 0.045 + TIME * vec2(0.012, 0.007);
	vec2 uv2 = wp.xz * 0.11 - TIME * vec2(0.009, 0.016);
	vec2 uv3 = wp.xz * 0.31 + TIME * vec2(-0.02, 0.011);
	vec2 n1 = texture(wave_nrm, uv1).rg * 2.0 - 1.0;
	vec2 n2 = texture(wave_nrm, uv2).rg * 2.0 - 1.0;
	vec2 n3 = texture(wave_nrm, uv3).rg * 2.0 - 1.0;
	vec2 nxy = (n1 * 0.5 + n2 * 0.35 + n3 * 0.25) * wave_strength;
	vec3 nw = normalize(vec3(nxy.x, 1.0, nxy.y));
	NORMAL = normalize((VIEW_MATRIX * vec4(nw, 0.0)).xyz);

	// How much water lies between the surface and the lakebed along this pixel.
	float surface_depth = -VERTEX.z;
	float scene_depth = linear_depth(SCREEN_UV, INV_PROJECTION_MATRIX);
	float water_depth = max(scene_depth - surface_depth, 0.0);

	// Refract, but never sample something in front of the water.
	vec2 ruv = SCREEN_UV + nxy * refraction * clamp(water_depth * 0.5, 0.0, 1.0);
	if (linear_depth(ruv, INV_PROJECTION_MATRIX) < surface_depth) {
		ruv = SCREEN_UV;
	}
	vec3 bed = texture(screen_tex, ruv).rgb;

	float absorb = 1.0 - exp(-water_depth / clarity);
	vec3 transmitted = mix(bed * shallow_tint, deep_color, absorb);

	// Foam where the water is very shallow, broken up by the ripples.
	float foam_n = texture(wave_nrm, wp.xz * 0.6 + TIME * 0.03).r;
	float foam = (1.0 - smoothstep(0.0, 0.35, water_depth)) * smoothstep(0.35, 0.6, foam_n + 0.25);

	ALBEDO = mix(vec3(0.0), foam_color, foam);
	EMISSION = transmitted * (1.0 - foam);
	ROUGHNESS = mix(0.04, 0.6, foam);
	METALLIC = 0.0;
	SPECULAR = 0.55;
	// Fade in at the very edge so the shoreline isn't a hard line.
	ALPHA = clamp(water_depth * 6.0, 0.0, 1.0);
}
