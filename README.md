# Vulkan Terminal

A complete terminal emulator built with Rust and Vulkan for hardware-accelerated text rendering.

## Features

- **Hardware-accelerated rendering** using Vulkan API
- **Font rendering** with glyph caching and texture atlasing
- **Terminal emulation** with character grid, colors, and text formatting
- **Input handling** for keyboard events and commands
- **Window management** with resizing support
- **Cursor animation** with blinking effect
- **Command system** with built-in commands (help, clear, exit)
- **Scrolling and history** support

## Prerequisites

1. **Vulkan SDK** - Install from [LunarG](https://vulkan.lunarg.com/)
2. **Rust** - Install from [rustup.rs](https://rustup.rs/)
3. **Shader compiler** - `glslc` (included with Vulkan SDK)

## Setup

1. Clone or create the project directory
2. Compile the shaders:
   ```bash
   glslc shaders/text.vert -o shaders/text.vert.spv
   glslc shaders/text.frag -o shaders/text.frag.spv
   ```
3. Add a font file named `DejaVuSansMono.ttf` to the `assets/` directory
4. Build and run:
   ```bash
   cargo run
   ```

## Project Structure

```
vulkan-terminal/
├── src/
│   ├── main.rs           # Main application and event loop
│   ├── vulkan.rs         # Vulkan context and rendering setup
│   ├── text_renderer.rs  # Text rendering with font support
│   └── terminal.rs       # Terminal state and command handling
├── shaders/
│   ├── text.vert         # Vertex shader for text rendering
│   ├── text.frag         # Fragment shader for text rendering
│   ├── text.vert.spv     # Compiled vertex shader
│   └── text.frag.spv     # Compiled fragment shader
├── assets/
│   └── DejaVuSansMono.ttf # Font file (user must provide)
├── Cargo.toml            # Project dependencies
└── README.md             # This file
```

## Dependencies

- `ash` - Vulkan bindings for Rust
- `winit` - Cross-platform window creation
- `raw-window-handle` - Window handle abstraction
- `fontdue` - Font rasterization
- `bytemuck` - Safe casting for vertex data
- `memoffset` - Offset calculations for structs

## Usage

Once running, the terminal supports:
- **Text input** - Type normally
- **Commands**:
  - `help` - Show available commands
  - `clear` - Clear the terminal
  - `exit` - Exit the application
- **Keyboard shortcuts**:
  - `Enter` - Execute command
  - `Backspace` - Delete character
  - `Tab` - Tab character

## Architecture

The application is structured in several modules:

1. **Vulkan Context** (`vulkan.rs`) - Manages Vulkan initialization, swapchain, render passes, and frame rendering
2. **Text Renderer** (`text_renderer.rs`) - Handles font loading, glyph caching, and text rendering with Vulkan
3. **Terminal State** (`terminal.rs`) - Manages the terminal grid, input processing, and command execution
4. **Main Application** (`main.rs`) - Window management, event handling, and application lifecycle

The rendering pipeline uses Vulkan for hardware acceleration, with text rendered as textured quads using signed distance field techniques for crisp text at any scale.