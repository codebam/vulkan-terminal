mod vulkan;
mod text_renderer;
mod terminal;

use std::time::Instant;
use vulkan::VulkanContext;
use text_renderer::TextRenderer;
use terminal::{TerminalState, TerminalColor};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

struct VulkanTerminalApp {
    window: Option<Window>,
    vulkan_context: Option<VulkanContext>,
    text_renderer: Option<TextRenderer>,
    terminal_state: TerminalState,
    last_frame_time: Instant,
    cursor_blink_timer: f32,
    cursor_visible: bool,
}

impl VulkanTerminalApp {
    fn new() -> Self {
        let terminal_state = TerminalState::new(80, 24);
        
        Self {
            window: None,
            vulkan_context: None,
            text_renderer: None,
            terminal_state,
            last_frame_time: Instant::now(),
            cursor_blink_timer: 0.0,
            cursor_visible: true,
        }
    }

    fn init_vulkan(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(window) = &self.window {
            let vulkan_context = VulkanContext::new(window)?;
            
            let text_renderer = TextRenderer::new(
                vulkan_context.device.clone(),
                vulkan_context.render_pass,
                vulkan_context.swapchain_extent,
                vulkan_context.physical_device,
                &vulkan_context.instance,
            )?;

            self.vulkan_context = Some(vulkan_context);
            self.text_renderer = Some(text_renderer);
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let (Some(vulkan_context), Some(text_renderer)) = 
            (&mut self.vulkan_context, &mut self.text_renderer) {
            
            vulkan_context.draw_frame()?;
            
            let command_buffer = vulkan_context.command_buffers[vulkan_context.current_frame];
            
            let char_width = 8.0;
            let char_height = 16.0;
            let margin_x = 10.0;
            let margin_y = 10.0;

            for (y, row) in self.terminal_state.get_visible_cells().iter().enumerate() {
                for (x, cell) in row.iter().enumerate() {
                    if cell.character != ' ' {
                        let screen_x = margin_x + (x as f32 * char_width);
                        let screen_y = margin_y + (y as f32 * char_height);
                        
                        let color = if cell.bold {
                            [cell.fg_color.r * 1.5, cell.fg_color.g * 1.5, cell.fg_color.b * 1.5, cell.fg_color.a]
                        } else {
                            cell.fg_color.as_array()
                        };

                        text_renderer.render_text(
                            command_buffer,
                            &cell.character.to_string(),
                            screen_x,
                            screen_y,
                            color,
                        )?;
                    }
                }
            }

            let now = Instant::now();
            let delta_time = now.duration_since(self.last_frame_time).as_secs_f32();
            self.last_frame_time = now;

            self.cursor_blink_timer += delta_time;
            if self.cursor_blink_timer >= 0.5 {
                self.cursor_visible = !self.cursor_visible;
                self.cursor_blink_timer = 0.0;
            }

            if self.cursor_visible {
                let (cursor_x, cursor_y) = self.terminal_state.get_cursor_position();
                let screen_x = margin_x + (cursor_x as f32 * char_width);
                let screen_y = margin_y + (cursor_y as f32 * char_height);
                
                text_renderer.render_text(
                    command_buffer,
                    "_",
                    screen_x,
                    screen_y,
                    TerminalColor::WHITE.as_array(),
                )?;
            }
        }
        Ok(())
    }

    fn handle_keyboard_input(&mut self, key_event: KeyEvent) {
        if key_event.state == ElementState::Pressed {
            match key_event.physical_key {
                PhysicalKey::Code(KeyCode::Enter) => {
                    self.terminal_state.handle_input('\n');
                }
                PhysicalKey::Code(KeyCode::Backspace) => {
                    self.terminal_state.handle_input('\x08');
                }
                PhysicalKey::Code(KeyCode::Tab) => {
                    self.terminal_state.handle_input('\t');
                }
                PhysicalKey::Code(KeyCode::Space) => {
                    self.terminal_state.handle_input(' ');
                }
                PhysicalKey::Code(code) => {
                    if let Some(ch) = self.keycode_to_char(code) {
                        self.terminal_state.handle_input(ch);
                    }
                }
                _ => {}
            }
        }
    }

    fn keycode_to_char(&self, keycode: KeyCode) -> Option<char> {
        match keycode {
            KeyCode::KeyA => Some('a'),
            KeyCode::KeyB => Some('b'),
            KeyCode::KeyC => Some('c'),
            KeyCode::KeyD => Some('d'),
            KeyCode::KeyE => Some('e'),
            KeyCode::KeyF => Some('f'),
            KeyCode::KeyG => Some('g'),
            KeyCode::KeyH => Some('h'),
            KeyCode::KeyI => Some('i'),
            KeyCode::KeyJ => Some('j'),
            KeyCode::KeyK => Some('k'),
            KeyCode::KeyL => Some('l'),
            KeyCode::KeyM => Some('m'),
            KeyCode::KeyN => Some('n'),
            KeyCode::KeyO => Some('o'),
            KeyCode::KeyP => Some('p'),
            KeyCode::KeyQ => Some('q'),
            KeyCode::KeyR => Some('r'),
            KeyCode::KeyS => Some('s'),
            KeyCode::KeyT => Some('t'),
            KeyCode::KeyU => Some('u'),
            KeyCode::KeyV => Some('v'),
            KeyCode::KeyW => Some('w'),
            KeyCode::KeyX => Some('x'),
            KeyCode::KeyY => Some('y'),
            KeyCode::KeyZ => Some('z'),
            KeyCode::Digit0 => Some('0'),
            KeyCode::Digit1 => Some('1'),
            KeyCode::Digit2 => Some('2'),
            KeyCode::Digit3 => Some('3'),
            KeyCode::Digit4 => Some('4'),
            KeyCode::Digit5 => Some('5'),
            KeyCode::Digit6 => Some('6'),
            KeyCode::Digit7 => Some('7'),
            KeyCode::Digit8 => Some('8'),
            KeyCode::Digit9 => Some('9'),
            KeyCode::Minus => Some('-'),
            KeyCode::Equal => Some('='),
            KeyCode::BracketLeft => Some('['),
            KeyCode::BracketRight => Some(']'),
            KeyCode::Backslash => Some('\\'),
            KeyCode::Semicolon => Some(';'),
            KeyCode::Quote => Some('\''),
            KeyCode::Comma => Some(','),
            KeyCode::Period => Some('.'),
            KeyCode::Slash => Some('/'),
            _ => None,
        }
    }

    fn resize_terminal(&mut self, width: u32, height: u32) {
        let char_width = 8.0;
        let char_height = 16.0;
        let margin_x = 20.0;
        let margin_y = 20.0;

        let terminal_width = ((width as f32 - margin_x) / char_width) as usize;
        let terminal_height = ((height as f32 - margin_y) / char_height) as usize;

        self.terminal_state.resize(terminal_width.max(1), terminal_height.max(1));
    }
}

impl ApplicationHandler for VulkanTerminalApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowBuilder::new()
            .with_title("Vulkan Terminal")
            .with_inner_size(LogicalSize::new(800, 600))
            .with_resizable(true);

        let window = event_loop.create_window(window_attributes).unwrap();
        
        let window_size = window.inner_size();
        self.resize_terminal(window_size.width, window_size.height);
        
        self.terminal_state.write_str("Welcome to Vulkan Terminal!\n");
        self.terminal_state.write_str("Type 'help' for available commands.\n");
        self.terminal_state.write_str("$ ");
        
        self.window = Some(window);
        
        if let Err(e) = self.init_vulkan() {
            eprintln!("Failed to initialize Vulkan: {}", e);
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                self.resize_terminal(new_size.width, new_size.height);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(event);
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.draw() {
                    eprintln!("Draw error: {}", e);
                }
                
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = VulkanTerminalApp::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
