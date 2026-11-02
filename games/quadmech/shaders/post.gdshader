// Final lens pass: subtle chromatic aberration toward the edges, animated film
// grain and a soft vignette. Drawn under the HUD.
shader_type canvas_item;

uniform sampler2D screen_tex : hint_screen_texture, filter_linear_mipmap;
uniform float grain = 0.03;
uniform float aberration = 0.0022;
uniform float vignette = 0.42;
uniform vec3 vignette_tint : source_color = vec3(0.06, 0.045, 0.03);

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

void fragment() {
	vec2 uv = SCREEN_UV;
	vec2 d = uv - 0.5;
	float r2 = dot(d, d);
	vec2 off = d * aberration * r2 * 4.0;
	vec3 c;
	c.r = texture(screen_tex, uv + off).r;
	c.g = texture(screen_tex, uv).g;
	c.b = texture(screen_tex, uv - off).b;
	float n = hash(floor(uv / SCREEN_PIXEL_SIZE) + fract(TIME * 7.13) * 113.0);
	float lum = dot(c, vec3(0.299, 0.587, 0.114));
	c += (n - 0.5) * grain * (1.0 - lum * 0.6);  // grain shows more in the shadows
	float v = smoothstep(0.12, 0.75, r2 * 2.0) * vignette;
	c = mix(c, c * vignette_tint * 4.0, v * 0.35);
	c *= 1.0 - v;
	COLOR = vec4(c, 1.0);
}
