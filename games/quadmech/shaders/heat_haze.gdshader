// Camera-facing heat shimmer: distorts what's behind it with rising noise.
// UV.y = 0 at the top of the quad.
shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, cull_disabled, shadows_disabled;

uniform sampler2D screen_tex : hint_screen_texture, filter_linear_mipmap;
uniform sampler2D noise_tex : repeat_enable, filter_linear;
uniform float strength = 0.0;

void vertex() {
	// Billboard, keeping the node's scale.
	MODELVIEW_MATRIX = VIEW_MATRIX * mat4(
		INV_VIEW_MATRIX[0] * length(MODEL_MATRIX[0].xyz),
		INV_VIEW_MATRIX[1] * length(MODEL_MATRIX[1].xyz),
		INV_VIEW_MATRIX[2],
		MODEL_MATRIX[3]);
}

void fragment() {
	vec2 n = texture(noise_tex, UV * vec2(1.0, 0.7) + vec2(0.0, TIME * 0.7)).rg - 0.5;
	vec2 n2 = texture(noise_tex, UV * vec2(2.1, 1.3) + vec2(0.37, TIME * 1.3)).rg - 0.5;
	float mask = smoothstep(0.5, 0.05, abs(UV.x - 0.5)) * smoothstep(0.0, 0.45, UV.y) * smoothstep(1.0, 0.8, UV.y);
	vec2 off = (n + n2 * 0.5) * strength * mask * 0.03;
	ALBEDO = texture(screen_tex, SCREEN_UV + off).rgb;
	ALPHA = mask * clamp(strength * 4.0, 0.0, 1.0);
}
