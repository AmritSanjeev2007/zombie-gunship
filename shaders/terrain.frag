#version 330 core
in vec3 v_normal;
in vec3 v_world;
out vec4 frag_color;

uniform vec3 u_sun_dir;    // direction towards the sun, normalised
uniform float u_max_height;
uniform vec3 u_low_color;  // valley and crater floors
uniform vec3 u_high_color; // ridges
uniform vec3 u_rock_color; // steep slopes

// --- Fog (same block in terrain.frag and mesh.frag; see render/fog.rs) ---
uniform vec3 u_fog_color;
uniform vec3 u_fog_range;   // start, end (beyond the map edge), outside dim
uniform vec3 u_fog_ground;  // mist density, plain height, mist thickness
uniform vec3 u_map_half;    // xy: half extents of the map

// Distance from the map rectangle on the XZ plane (0 inside).
float outside_distance(vec3 world) {
    return length(max(abs(world.xz) - u_map_half.xy, 0.0));
}

// Land outside the map is a little darker, so the playable edge reads.
vec3 dim_outside(vec3 albedo, vec3 world) {
    return albedo * (1.0 - u_fog_range.z * smoothstep(0.0, 15.0, outside_distance(world)));
}

vec3 apply_fog(vec3 color, vec3 world) {
    float d = outside_distance(world);
    float blanket = smoothstep(u_fog_range.x, u_fog_range.y, d);
    float low = 1.0 - smoothstep(u_fog_ground.y, u_fog_ground.y + u_fog_ground.z, world.y);
    float mist = clamp(u_fog_ground.x * low * smoothstep(0.0, 25.0, d), 0.0, 1.0);
    float f = 1.0 - (1.0 - blanket) * (1.0 - mist);
    return mix(color, u_fog_color, f);
}

void main() {
    vec3 n = normalize(v_normal);
    float h = clamp(v_world.y / max(u_max_height, 0.001), 0.0, 1.0);
    vec3 ground = mix(u_low_color, u_high_color, smoothstep(0.1, 0.8, h));
    float steep = 1.0 - smoothstep(0.45, 0.8, n.y);
    vec3 albedo = dim_outside(mix(ground, u_rock_color, steep), v_world);

    float diffuse = max(dot(n, u_sun_dir), 0.0);
    vec3 color = albedo * (0.35 + 0.65 * diffuse);
    frag_color = vec4(apply_fog(color, v_world), 1.0);
}
