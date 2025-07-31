use crate::themes::{rgb_to_color, Theme};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub struct TextInput {
    pub text: String,
    pub cursor_position: usize,
    pub is_focused: bool,
    pub label: String,
    pub is_password: bool,
    pub password_char: Option<char>,
}

impl TextInput {
    pub fn new(label: String) -> Self {
        Self {
            text: String::new(),
            cursor_position: 0,
            is_focused: false,
            label,
            is_password: false,
            password_char: None,
        }
    }

    pub fn render<B: Backend>(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let border_color = if self.is_focused {
            rgb_to_color(&theme.colors.input_border_active)
        } else {
            rgb_to_color(&theme.colors.input_border_inactive)
        };
        let text_color = if self.is_focused {
            rgb_to_color(&theme.colors.input_text_active)
        } else {
            rgb_to_color(&theme.colors.input_text_inactive)
        };

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(self.label.as_str());

        let text_width = area.width.saturating_sub(2) as usize;
        let display_text = if self.is_password {
            let char_to_repeat = self.password_char.unwrap_or('*');
            String::from(char_to_repeat).repeat(self.text.len())
        } else if self.text.len() > text_width {
            let start = self.text.len().saturating_sub(text_width);
            self.text[start..].to_string()
        } else {
            self.text.clone()
        };
        let input_paragraph = Paragraph::new(display_text)
            .style(Style::default().fg(text_color))
            .block(input_block);

        f.render_widget(input_paragraph, area);

        if self.is_focused {
            f.set_cursor_position((area.x + 1 + self.cursor_position as u16, area.y + 1));
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.text.remove(self.cursor_position);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.text.len() {
            self.cursor_position += 1;
        }
    }

    pub fn reset(&mut self) {
        self.text.clear();
        self.cursor_position = 0;
    }
}
