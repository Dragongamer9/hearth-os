// Fallback lake water for the OpenGL (Compatibility) renderer, which can't read
// the depth buffer. Water depth is baked into the mesh's vertex color instead.
shader_type spatial;
render_mode blend_mix, depth_draw_always, cull_back, diffuse_burley, specular_schlick_ggx;

uniform sampler2D wave_nrm : filter_linear_mipmap, repeat_enable;
uniform vec3 shallow_color : source_color = vec3(0.17, 0.2, 0.15);
uniform vec3 deep_color : source_color = vec3(0.02, 0.05, 0.055);
uniform float clarity = 2.2;
uniform float wave_strength = 0.35;
uniform vec3 foam_color : source_color = vec3(0.85, 0.88, 0.85);

varying vec3 wp;
varying float depth;

void vertex() {
	wp = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	depth = COLOR.r * 10.0;
	VERTEX.y += sin(wp.x * 0.11 + TIME * 0.7) * 0.04 + sin(wp.z * 0.17 - TIME * 0.9) * 0.03;
}

void fragment() {
	vec2 n1 = texture(wave_nrm, wp.xz * 0.045 + TIME * vec2(0.012, 0.007)).rg * 2.0 - 1.0;
	vec2 n2 = texture(wave_nrm, wp.xz * 0.11 - TIME * vec2(0.009, 0.016)).rg * 2.0 - 1.0;
	vec2 n3 = texture(wave_nrm, wp.xz * 0.31 + TIME * vec2(-0.02, 0.011)).rg * 2.0 - 1.0;
	vec2 nxy = (n1 * 0.5 + n2 * 0.35 + n3 * 0.25) * wave_strength;
	NORMAL = normalize((VIEW_MATRIX * vec4(normalize(vec3(nxy.x, 1.0, nxy.y)), 0.0)).xyz);

	float absorb = 1.0 - exp(-depth / clarity);
	float foam_n = texture(wave_nrm, wp.xz * 0.6 + TIME * 0.03).r;
	float foam = (1.0 - smoothstep(0.0, 0.35, depth)) * smoothstep(0.35, 0.6, foam_n + 0.25);
	ALBEDO = mix(mix(shallow_color, deep_color, absorb), foam_color, foam);
	ROUGHNESS = mix(0.05, 0.6, foam);
	SPECULAR = 0.55;
	ALPHA = clamp(depth * 3.0, 0.0, 1.0) * mix(0.6, 0.96, absorb);
}
