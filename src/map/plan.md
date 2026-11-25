# Map generation plan

A map is a rectangle on the XZ plane, centred on the origin, with Y up and a
maximum terrain height. Generation is a pure CPU function:

```rust
let map = map::generate(&MapConfig { seed, .. });
```

The same config and seed always give the same map. Generation knows nothing
about OpenGL or the ECS, so it can be unit tested. The result is stored as
`Resources::map`, because the terrain renderer, the debug UI and (later) zombie
AI all read it. It is shared state, not a per-entity component.

```
Map
├── heights: Heightfield       final terrain (world units)
├── objectives: Vec<Objective>
├── spawns: Vec<SpawnPoint>    { pos, edge }
├── objective_ring: Vec<usize> objectives sorted by angle
├── ring_road: Vec<Vec2>       smooth loop through the objectives (carved)
├── paths: Vec<PathTree>       one per edge that has spawns
│   └── routes: Vec<Route>     smooth polyline, spawn → objective
└── apron: Apron               land around the map (rings of vertices)
```

## Modules

| File                          | Job                                                     |
|-------------------------------|---------------------------------------------------------|
| `map/mod.rs`                  | `MapConfig`, `Map`, `Edge`, `generate()`                |
| `map/terrain.rs`              | Seeded Perlin noise, 4-octave fBm, `Heightfield`        |
| `map/points.rs`               | Objective and spawn placement                           |
| `map/pathing.rs`              | Target choice, path trees, Catmull-Rom routes           |
| `map/carve.rs`                | Crater and valley profiles, carving into the heightfield |
| `map/apron.rs`                | Land around the map, flattening into a plain            |
| `render/fog.rs`               | Fog uniforms shared by the terrain and mesh shaders     |
| `systems/map.rs`              | Generates on start / on request, marker entities, Map UI |
| `systems/terrain_renderer.rs` | Heightfield as an indexed mesh, path overlay            |

Randomness comes from one `ChaCha8Rng` seeded from `MapConfig::seed`. Perlin
noise is hand-written (a seeded permutation table), so no noise crate is needed.

## Pipeline

1. **Base terrain.** fBm of Perlin noise with 4 octaves (lacunarity 2,
   persistence 0.5). Also keep a *smooth* layer (the first octave only), used to
   flatten crater and valley floors. Height range is `[0, max_height]`.
2. **Objectives.** Random angle, radius `R·uᵏ` (k ≈ 2) so they cluster toward
   the centre. `R` is a fraction of the half-extents, so the shape follows the
   rectangle. Rejection sampling keeps a minimum spacing between all points.
3. **Spawns.** Pick the edge first, round-robin from a random starting edge so
   every edge gets a fair share. Then pick a spot along that edge, inset so the
   crater stays on the map, and keep the minimum spacing. The edge is stored
   with the spawn.
4. **Objective ring.** Objectives sorted by `atan2` angle from +X, joined in
   order (and back to the first) into a closed loop. The loop is a cubic
   Hermite curve with Catmull-Rom tangents, so it passes through every
   objective, with the same meander as the routes. Two objectives give a
   single curve; one gives none. It is carved as a valley in step 8 (toggle
   and size in `RingConfig`), so zombies can move on after a capture.
5. **Target per edge.** The map is split into four triangles by its
   diagonals, one per edge. In normalised map space (divide by the
   half-extents) an objective is a candidate for an edge when
   `dot(dir_to_objective, edge_normal) >= threshold` (default cos 45°, which is
   exactly the triangle). One candidate is picked at random. If none
   qualifies, the objective with the highest dot product is used. Edges with
   no spawns are skipped.
6. **Path tree, built from the spawns inward.** The edge's spawns are sorted
   along the edge. The root is the target objective. The sorted list is split
   into 2–3 neighbouring groups, and each group with more than one spawn gets a
   branch node partway from its parent toward the group's centre, then recurses.
   A group of one spawn becomes a leaf, *at the spawn*. So the leaf count always
   equals the spawn count and branches never cross. `max_depth` caps the
   branch levels: the split factor grows when needed so the cap is never
   exceeded, and at the last level every spawn connects directly. With
   `root_trunk` on (the default), the root first gets a single trunk node, so
   paths merge before they reach the objective instead of fanning out from
   the crater.
   Branch nodes are placed at the lowest of a few random candidates on the base
   terrain, so paths follow existing low ground.
7. **Routes.** For each spawn, the chain of nodes from spawn to objective is
   joined with a centripetal Catmull-Rom spline (it passes through every node;
   no extra control points to invent), sampled to a polyline, and given a
   small low-frequency meander that fades to zero at both ends.
8. **Carving.** Each feature has a smooth, flat-bottomed profile
   (`1 - smoothstep` between the floor radius and the outer radius):
   * Craters at objectives and spawns, with an optional raised rim. The
     radius wobbles with angle (noise), so craters aren't perfect circles.
   * Valleys along routes, widening from the spawn toward the objective.
   * A constant-width valley along the objective ring.

   Overlapping features take the **maximum** carve, never the sum, so shared
   trunks aren't dug twice. Floors blend toward the smooth layer, so they
   look like natural valley floors instead of noisy trenches. Rims are faded
   out where a valley passes, so paths break through crater walls. All widths
   are in world units, not grid cells.

## Beyond the map

9. **Apron.** Rings of vertices grow outward from the map's edge. Ring 0 is
   the map's own edge (same positions, heights and normals), so the seam is
   watertight. Rings are rounded at the corners and their spacing grows
   geometrically, so detail is fine near the map and coarse far away. Heights
   continue the same base noise; any crater cut by the edge fades out over a
   few units, and the hills settle into a flat plain (at the terrain floor
   height) between `flatten_start` and `flatten_end`. One last flat ring runs
   out to the horizon.
10. **Fog.** `Resources::fog`. Fog is measured from the map's edge, not from
   the camera, so the battlefield stays clear at any zoom while the land
   beyond fades into a blanket. Its colour matches the sky's horizon, and the
   skybox fades to the same colour below the horizon, so ground and sky meet
   without a line. Optional ground mist pools on the low plain, and land
   outside the map is slightly dimmed so the playable edge reads clearly.

## Rendering and tools

* `GpuMesh` gains index buffer support. The heightfield becomes an indexed
  triangle mesh with smooth normals and height/slope colouring.
* Marker entities (`MapMarker`) mark objectives and spawns with the existing
  `MeshRenderer`, and are respawned on every regeneration.
* The debug UI gets a **Map** window: seed, counts, size, noise, crater and
  valley settings, Regenerate and Random seed buttons, and a toggle for the
  path overlay (routes, tree nodes, objective ring).

## Tests

* Same seed gives the same map; a different seed gives a different one.
* Every spawn has exactly one route, which starts at the spawn and ends at an
  objective.
* Each spawn lies near its own edge; objectives lie inside the map.
* Minimum spacing is kept; tree depth never exceeds `max_depth`.
* Terrain is lower at objectives than the uncarved base.
