#version 330 core
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;

uniform mat4 u_view_proj;

out vec3 v_normal;
out vec3 v_world;

void main() {
    v_normal = a_normal;
    v_world = a_position;
    gl_Position = u_view_proj * vec4(a_position, 1.0);
}
