// Sky with atmospheric gradient, sun glow, and a layer of cumulus clouds
// lit from the sun. Static (no TIME) so the radiance map isn't re-rendered every frame.
shader_type sky;

uniform sampler2D cloud_tex : filter_linear_mipmap, repeat_enable;
uniform vec3 zenith : source_color = vec3(0.2, 0.34, 0.56);
uniform vec3 horizon : source_color = vec3(0.74, 0.72, 0.68);
uniform vec3 ground : source_color = vec3(0.28, 0.25, 0.21);
uniform float cloud_cover : hint_range(0.0, 1.0) = 0.48;
uniform float cloud_scale = 0.22;
uniform vec3 cloud_light : source_color = vec3(1.0, 0.95, 0.88);
uniform vec3 cloud_shadow : source_color = vec3(0.42, 0.45, 0.52);
uniform float brightness = 1.0;

void sky() {
	vec3 d = EYEDIR;
	float h = d.y;
	vec3 sun_dir = LIGHT0_ENABLED ? LIGHT0_DIRECTION : vec3(0.0, 1.0, 0.0);
	vec3 sun_col = LIGHT0_ENABLED ? LIGHT0_COLOR * LIGHT0_ENERGY : vec3(1.0);
	float sd = max(dot(d, sun_dir), 0.0);

	vec3 col = mix(horizon, zenith, pow(clamp(h, 0.0, 1.0), 0.42));
	// Mie haze around the sun, warmer near the horizon
	col += sun_col * (pow(sd, 6.0) * 0.18 + pow(sd, 48.0) * 0.35) * mix(1.0, 0.6, clamp(h, 0.0, 1.0));
	if (h < 0.0) {
		col = mix(horizon * 0.85, ground, clamp(-h * 5.0, 0.0, 1.0));
	}

	float dens = 0.0;
	if (h > 0.005) {
		vec2 uv = d.xz / (h + 0.06) * cloud_scale;
		float c = texture(cloud_tex, uv).r * 0.65 + texture(cloud_tex, uv * 2.7 + vec2(0.37, 0.13)).r * 0.35;
		float edge = 1.0 - cloud_cover;
		dens = smoothstep(edge, edge + 0.22, c) * smoothstep(0.005, 0.14, h);
		// Thicker cores are darker underneath; edges facing the sun glow.
		float core = smoothstep(edge + 0.1, edge + 0.45, c);
		vec3 lit = cloud_light * (0.85 + 0.6 * pow(sd, 4.0));
		vec3 ccol = mix(lit, cloud_shadow, core * 0.75);
		ccol = mix(ccol, horizon, smoothstep(0.25, 0.0, h) * 0.6);  // aerial perspective
		col = mix(col, ccol, dens * 0.95);
	}

	// Sun disk
	col += sun_col * smoothstep(0.99955, 0.99985, sd) * 18.0 * (1.0 - dens);
	COLOR = col * brightness;
}
