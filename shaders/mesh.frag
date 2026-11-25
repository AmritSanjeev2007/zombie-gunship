#version 330 core
in vec3 v_normal;
out vec4 frag_color;

uniform vec3 u_color;
uniform vec3 u_sun_dir;   // direction the light travels *from*, normalised
uniform float u_lit;

void main() {
    float diffuse = max(dot(normalize(v_normal), u_sun_dir), 0.0);
    float light = mix(1.0, 0.35 + 0.65 * diffuse, u_lit);
    frag_color = vec4(u_color * light, 1.0);
}
