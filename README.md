# zombie-gunship
A simple game where you orbit a battlefield in a gunship, with the aim of preventing zombies from capturing the objective.

![Screenshot](docs/screenshot.png)

## Running

```sh
cargo run --release
```

Needs a GPU/driver with OpenGL 3.3. Left-drag to orbit, scroll to zoom, F1 toggles the debug window.

## Tech

- **winit + glutin + glow** for the window and OpenGL 3.3 core context.
- **egui** (via `egui_glow`) for the immediate-mode debug UI. It plays the role Dear ImGui plays in C++, but is pure Rust, so there's no C++ toolchain or bindings to keep in sync.
- **hecs** stores entities and components; systems are our own small trait.
- **resvg** rasterises `assets/favicon.svg` into the window icon.

## Architecture

The code is split into an engine layer and game content:

| Module | What lives there |
| --- | --- |
| `engine/` | `Engine` (world + resources + ordered systems), the `System` trait, `Time`, `Input`, `Viewport`, `CameraMatrices` |
| `components/` | Plain data: `Transform`, `Camera`, `OrbitController`, `Skybox`, `MeshRenderer`, `Name` |
| `systems/` | Behaviour: `OrbitCameraSystem`, `CameraSystem`, `ClearSystem`, `SkyboxRendererSystem`, `MeshRendererSystem`, `DebugUiSystem` |
| `render/` | Small OpenGL helpers: `ShaderProgram`, `Texture`, `GpuMesh` |
| `platform/` | Window + GL context creation, window icon |
| `scene.rs` | Spawns the starting entities and registers systems in order |
| `app.rs` | The winit event loop; pumps events into the engine |

Each frame the engine runs every system's `update`, then every `render`, then every `ui`.

### Adding a feature

1. Add a component in `components/` (plain data, no GL handles).
2. Add a system in `systems/` that queries for it, implementing only the `System` hooks it needs. Systems own their GPU resources, create them lazily in `render` and free them in `destroy`.
3. Register the system and spawn entities in `scene.rs`.

For example, a gunship would be an entity with `Transform` + `OrbitController` + `MeshRenderer`, with the camera following it; zombies would get a `Zombie` component and a `ZombieAiSystem` that walks them towards the `Objective`.
