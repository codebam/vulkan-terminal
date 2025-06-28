use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl TerminalColor {
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const YELLOW: Self = Self::new(1.0, 1.0, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const MAGENTA: Self = Self::new(1.0, 0.0, 1.0, 1.0);
    pub const CYAN: Self = Self::new(0.0, 1.0, 1.0, 1.0);
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const GRAY: Self = Self::new(0.5, 0.5, 0.5, 1.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn as_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TerminalCell {
    pub character: char,
    pub fg_color: TerminalColor,
    pub bg_color: TerminalColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            character: ' ',
            fg_color: TerminalColor::WHITE,
            bg_color: TerminalColor::BLACK,
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

pub struct TerminalState {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<TerminalCell>>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub current_fg_color: TerminalColor,
    pub current_bg_color: TerminalColor,
    pub current_bold: bool,
    pub current_italic: bool,
    pub current_underline: bool,
    pub scroll_offset: usize,
    pub history: VecDeque<Vec<TerminalCell>>,
    pub max_history: usize,
    pub input_buffer: String,
    pub prompt: String,
}

impl TerminalState {
    pub fn new(width: usize, height: usize) -> Self {
        let mut cells = Vec::new();
        for _ in 0..height {
            cells.push(vec![TerminalCell::default(); width]);
        }

        Self {
            width,
            height,
            cells,
            cursor_x: 0,
            cursor_y: 0,
            current_fg_color: TerminalColor::WHITE,
            current_bg_color: TerminalColor::BLACK,
            current_bold: false,
            current_italic: false,
            current_underline: false,
            scroll_offset: 0,
            history: VecDeque::new(),
            max_history: 1000,
            input_buffer: String::new(),
            prompt: "$ ".to_string(),
        }
    }

    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        self.width = new_width;
        self.height = new_height;
        
        self.cells.resize(new_height, Vec::new());
        for row in &mut self.cells {
            row.resize(new_width, TerminalCell::default());
        }

        if self.cursor_x >= new_width {
            self.cursor_x = new_width.saturating_sub(1);
        }
        if self.cursor_y >= new_height {
            self.cursor_y = new_height.saturating_sub(1);
        }
    }

    pub fn put_char(&mut self, ch: char) {
        if ch == '\n' {
            self.newline();
            return;
        }

        if ch == '\r' {
            self.cursor_x = 0;
            return;
        }

        if ch == '\t' {
            let tab_stop = 8;
            let spaces = tab_stop - (self.cursor_x % tab_stop);
            for _ in 0..spaces {
                self.put_char(' ');
            }
            return;
        }

        if ch == '\x08' {
            if self.cursor_x > 0 {
                self.cursor_x -= 1;
                self.cells[self.cursor_y][self.cursor_x] = TerminalCell::default();
            }
            return;
        }

        if self.cursor_x >= self.width {
            self.newline();
        }

        self.cells[self.cursor_y][self.cursor_x] = TerminalCell {
            character: ch,
            fg_color: self.current_fg_color,
            bg_color: self.current_bg_color,
            bold: self.current_bold,
            italic: self.current_italic,
            underline: self.current_underline,
        };

        self.cursor_x += 1;
    }

    pub fn write_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.put_char(ch);
        }
    }

    pub fn newline(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += 1;

        if self.cursor_y >= self.height {
            self.scroll_up();
            self.cursor_y = self.height - 1;
        }
    }

    pub fn scroll_up(&mut self) {
        if let Some(first_row) = self.cells.first().cloned() {
            self.history.push_back(first_row);
            if self.history.len() > self.max_history {
                self.history.pop_front();
            }
        }

        self.cells.remove(0);
        self.cells.push(vec![TerminalCell::default(); self.width]);
    }

    pub fn clear(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                *cell = TerminalCell::default();
            }
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn set_fg_color(&mut self, color: TerminalColor) {
        self.current_fg_color = color;
    }

    pub fn set_bg_color(&mut self, color: TerminalColor) {
        self.current_bg_color = color;
    }

    pub fn set_bold(&mut self, bold: bool) {
        self.current_bold = bold;
    }

    pub fn set_italic(&mut self, italic: bool) {
        self.current_italic = italic;
    }

    pub fn set_underline(&mut self, underline: bool) {
        self.current_underline = underline;
    }

    pub fn reset_formatting(&mut self) {
        self.current_fg_color = TerminalColor::WHITE;
        self.current_bg_color = TerminalColor::BLACK;
        self.current_bold = false;
        self.current_italic = false;
        self.current_underline = false;
    }

    pub fn handle_input(&mut self, ch: char) {
        match ch {
            '\x08' => {
                if !self.input_buffer.is_empty() {
                    self.input_buffer.pop();
                    self.put_char('\x08');
                }
            }
            '\r' | '\n' => {
                self.newline();
                self.execute_command();
                self.write_str(&self.prompt);
            }
            ch if ch.is_control() => {
            }
            ch => {
                self.input_buffer.push(ch);
                self.put_char(ch);
            }
        }
    }

    fn execute_command(&mut self) {
        let command = self.input_buffer.trim();
        self.input_buffer.clear();

        match command {
            "" => {}
            "clear" => self.clear(),
            "help" => {
                self.write_str("Available commands:\n");
                self.write_str("  clear - Clear the terminal\n");
                self.write_str("  help  - Show this help message\n");
                self.write_str("  exit  - Exit the terminal\n");
            }
            "exit" => {
                self.write_str("Goodbye!\n");
            }
            _ => {
                self.write_str(&format!("Unknown command: {}\n", command));
            }
        }
    }

    pub fn get_visible_cells(&self) -> &[Vec<TerminalCell>] {
        &self.cells
    }

    pub fn get_cursor_position(&self) -> (usize, usize) {
        (self.cursor_x, self.cursor_y)
    }
}