#version 330 core
in vec3 v_normal;
in float v_height;
out vec4 frag_color;

uniform vec3 u_sun_dir;    // direction towards the sun, normalised
uniform float u_max_height;
uniform vec3 u_low_color;  // valley and crater floors
uniform vec3 u_high_color; // ridges
uniform vec3 u_rock_color; // steep slopes

void main() {
    vec3 n = normalize(v_normal);
    float h = clamp(v_height / max(u_max_height, 0.001), 0.0, 1.0);
    vec3 ground = mix(u_low_color, u_high_color, smoothstep(0.1, 0.8, h));
    float steep = 1.0 - smoothstep(0.45, 0.8, n.y);
    vec3 albedo = mix(ground, u_rock_color, steep);

    float diffuse = max(dot(n, u_sun_dir), 0.0);
    frag_color = vec4(albedo * (0.35 + 0.65 * diffuse), 1.0);
}
