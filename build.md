# Build Instructions

Due to API compatibility issues between different versions of ash/winit/vulkan, here are the instructions to fix and build the project:

## Quick Fix for Current Issues:

1. **Use compatible dependency versions**:
```toml
[dependencies]
ash = "0.37"
ash-window = "0.12" 
winit = "0.27"  # Use older version for compatibility
raw-window-handle = "0.5"
fontdue = "0.8"
bytemuck = { version = "1.0", features = ["derive"] }
memoffset = "0.9"
```

2. **Key API changes needed**:
   - Replace builder pattern with struct initialization for Vulkan structs
   - Fix window handle usage for ash-window
   - Update event handling for winit 0.27

3. **Missing components to add**:
   - Compile shaders: `glslc shaders/text.vert -o shaders/text.vert.spv`
   - Add DejaVuSansMono.ttf font to assets/
   - Fix struct field assignments instead of method calls

## Alternative Approach:

For a working version, consider using:
- `wgpu` instead of raw Vulkan (more stable API)
- `egui` for text rendering 
- Or use established terminal libraries like `crossterm` + `tui-rs`

The project structure is complete but needs API compatibility fixes for the specific versions used.