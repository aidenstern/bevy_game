# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Bevy-based first-person controller game built in Rust. The project uses the Bevy 0.14 game engine with Avian3D physics and leafwing-input-manager for input handling.

## Development Commands

### Building and Running
- `cargo run` - Build and run the game
- `cargo build` - Compile the project
- `cargo check` - Check for compilation errors without building
- `cargo test` - Run tests

### Build Targets
- **macOS ARM**: `cargo run` (auto-detects `aarch64-apple-darwin`)
- **WSL2 Linux (fast dev)**: `cargo run` (uses dynamic linking for fast builds)
- **WSL2 → Windows**: `cargo build --target x86_64-pc-windows-gnu && cp target/x86_64-pc-windows-gnu/debug/bevy_game.exe . && ./bevy_game.exe`

### WSL2 Graphics Setup
**Primary method (Vulkan with NVIDIA GPU):**
```bash
WGPU_BACKEND=vulkan cargo run
```

**Fallback methods:**
```bash
__NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia WGPU_BACKEND=gl cargo run
WINIT_UNIX_BACKEND=x11 WGPU_BACKEND=gl cargo run
```

### Development Profile
The project is configured with optimized debug builds:
- Debug mode has opt-level = 1 for faster iteration
- Dependencies (including Bevy) are compiled with opt-level = 3 for performance

## Architecture Overview

### Core Design Pattern
The project uses a dual-entity architecture for the player:
- **Logical Player**: Handles physics, collision detection, and game logic
- **Render Player**: Camera entity that follows the logical player for smooth rendering

### Module Structure
- `main.rs` - Application entry point, scene setup, and main game loop systems
- `plugin.rs` - FpsControllerPlugin that orchestrates the FPS controller systems
- `components.rs` - All game components, including FpsController, player markers, and input actions
- `input.rs` - Input handling and mouse/keyboard processing
- `movement.rs` - Physics-based movement, ground detection, and controller logic
- `render.rs` - Camera positioning and rendering logic
- `util.rs` - Utility functions and helper methods

### Key Components
- `FpsController` - Main controller component with movement parameters (speeds, acceleration, etc.)
- `FpsControllerInput` - Input state component (movement, look direction, action flags)
- `LogicalPlayer` / `RenderPlayer` - Marker components for the dual-entity system
- `Grounded` - Sparse component indicating ground contact
- `CameraConfig` - Camera offset and scaling configuration

### Physics Integration
- Uses Avian3D for physics simulation
- Capsule collider for player with shape casting for ground detection
- Ground detection uses angled normal checking (max 0.5 radians from vertical)
- Respawn system triggers when player falls below Y = -50

### Input System
Uses leafwing-input-manager with these controls:
- WASD - Movement
- Mouse - Look around
- Space - Jump
- Shift - Sprint
- Ctrl - Crouch
- Alt - Fly mode

## Assets Structure
- `assets/playground.glb` - Main scene file
- `assets/fira_mono.ttf` - UI font
- `assets/texture.png` - Texture asset

## System Execution Order
The FPS controller systems run in a specific chain:
1. `fps_controller_grounded` - Update ground detection
2. `fps_controller_input` - Process input
3. `fps_controller_move` - Apply movement physics
4. `fps_controller_look` - Update look direction
5. `fps_controller_render` - Position camera

## Development Notes
- The project uses optimized dependency compilation for faster debug builds
- Ground detection uses shape casting rather than ray casting for better edge detection
- Movement supports both ground-based and noclip/fly modes
- Physics uses locked rotation on XZ axes to prevent player tumbling