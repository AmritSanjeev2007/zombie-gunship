#version 330 core
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;

uniform mat4 u_model;
uniform mat4 u_view_proj;

out vec3 v_normal;
out vec3 v_world;

void main() {
    v_normal = mat3(transpose(inverse(u_model))) * a_normal;
    vec4 world = u_model * vec4(a_position, 1.0);
    v_world = world.xyz;
    gl_Position = u_view_proj * world;
}
