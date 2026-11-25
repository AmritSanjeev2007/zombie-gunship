#version 330 core
// Full-screen triangle generated from the vertex index; no buffers needed.
out vec2 v_ndc;
void main() {
    vec2 pos = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2)) * 2.0 - 1.0;
    v_ndc = pos;
    gl_Position = vec4(pos, 1.0, 1.0);
}
