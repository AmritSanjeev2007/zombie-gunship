#version 330 core
in vec2 v_ndc;
out vec4 frag_color;

uniform mat4 u_inv_view_proj; // inverse of (projection * rotation-only view)
uniform sampler2D u_panorama;
uniform vec3 u_tint;
uniform float u_rotation;
uniform vec3 u_fog_color;
uniform float u_fog_blend; // sin of the angle above the horizon where fog ends

const float PI = 3.14159265359;

void main() {
    vec4 world = u_inv_view_proj * vec4(v_ndc, 1.0, 1.0);
    vec3 dir = normalize(world.xyz / world.w);

    // Equirectangular lookup: longitude → u, latitude → v (v = 0 at the top).
    float u = atan(dir.z, dir.x) / (2.0 * PI) + 0.5 + u_rotation / (2.0 * PI);
    float v = acos(clamp(dir.y, -1.0, 1.0)) / PI;

    // Explicit LOD avoids a seam where atan wraps from +PI to -PI.
    vec3 color = textureLod(u_panorama, vec2(u, v), 0.0).rgb;
    // Below the horizon the sky is hidden by the fog blanket; it fades back
    // in just above it, so the fogged ground meets the sky without a line.
    float fog = 1.0 - smoothstep(-0.02, max(u_fog_blend, 0.0), dir.y);
    frag_color = vec4(mix(color * u_tint, u_fog_color, fog), 1.0);
}
