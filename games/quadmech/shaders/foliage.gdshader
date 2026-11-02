// Alpha-tested foliage cards (pine fronds, grass tufts) with wind sway
// and light transmission. Grass: UV.y = 0 at the tips. Trees: UV2.x = sway weight.
shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_schlick_ggx;

uniform sampler2D albedo_tex : source_color, filter_linear_mipmap, repeat_disable;
uniform vec3 tint : source_color = vec3(1.0);
uniform float wind = 0.12;
uniform float alpha_cut = 0.42;
uniform bool grass_mode = false;
uniform vec3 translucency : source_color = vec3(0.22, 0.26, 0.08);
uniform sampler2D ground_tex : source_color, filter_linear_mipmap, repeat_enable;
uniform float ground_scale = 0.17;
uniform float ground_blend = 0.0;   // grass: how much the roots take the terrain color
uniform sampler2D cloud_tex : filter_linear_mipmap, repeat_enable;
uniform float cloud_shadow = 0.38;
uniform vec2 cloud_wind = vec2(0.0035, 0.0018);

varying float k;
varying vec3 wpos;

void vertex() {
	vec3 w = MODEL_MATRIX[3].xyz;
	float s = sin(TIME * 1.3 + w.x * 0.21 + w.z * 0.17) + 0.45 * sin(TIME * 2.7 + w.x * 0.57 - w.z * 0.33);
	k = grass_mode ? pow(1.0 - UV.y, 2.0) : UV2.x;
	wpos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	VERTEX.x += s * wind * k;
	VERTEX.z += s * wind * 0.55 * k;
	VERTEX.y -= abs(s) * wind * 0.15 * k;
}

void fragment() {
	vec4 t = texture(albedo_tex, UV);
	vec3 c = t.rgb * tint * COLOR.rgb;
	if (grass_mode) {
		vec3 ground = texture(ground_tex, wpos.xz * ground_scale).rgb;
		float tip = smoothstep(1.0, 0.3, UV.y);
		c = mix(ground * 1.1, c, mix(1.0 - ground_blend, 1.0, tip));
		c *= mix(0.5, 1.0, tip);  // darker at the roots
	}
	float cs = smoothstep(0.5, 0.72, texture(cloud_tex, wpos.xz * 0.0016 + TIME * cloud_wind).r);
	c *= 1.0 - cs * cloud_shadow;
	ALBEDO = c;
	ALPHA = t.a;
	ALPHA_SCISSOR_THRESHOLD = alpha_cut;
	ROUGHNESS = 0.82;
	SPECULAR = 0.25;
	BACKLIGHT = translucency;
}
