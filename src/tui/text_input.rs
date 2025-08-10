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
            String::from(char_to_repeat).repeat(self.text.chars().count())
        } else if self.text.chars().count() > text_width {
            let start_char_index = self.text.chars().count() - text_width;
            self.text.chars().skip(start_char_index).collect::<String>()
        } else {
            self.text.clone()
        };
        let input_paragraph = Paragraph::new(display_text)
            .style(Style::default().fg(text_color))
            .block(input_block);

        f.render_widget(input_paragraph, area);

        if self.is_focused {
            let text_before_cursor = &self.text[..self.cursor_position];
            let cursor_char_pos = text_before_cursor.chars().count();
            
            let text_len_chars = self.text.chars().count();
            let scroll_offset = if text_len_chars > text_width {
                text_len_chars - text_width
            } else {
                0
            };

            let final_cursor_pos = (cursor_char_pos as u16).saturating_sub(scroll_offset as u16);

            f.set_cursor(area.x + 1 + final_cursor_pos, area.y + 1);
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            let mut prev_char_boundary = self.cursor_position - 1;
            while prev_char_boundary > 0 && !self.text.is_char_boundary(prev_char_boundary) {
                prev_char_boundary -= 1;
            }
            self.text.drain(prev_char_boundary..self.cursor_position);
            self.cursor_position = prev_char_boundary;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            let mut new_pos = self.cursor_position - 1;
            while new_pos > 0 && !self.text.is_char_boundary(new_pos) {
                new_pos -= 1;
            }
            self.cursor_position = new_pos;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.text.len() {
            let mut new_pos = self.cursor_position + 1;
            while new_pos < self.text.len() && !self.text.is_char_boundary(new_pos) {
                new_pos += 1;
            }
            self.cursor_position = new_pos;
        }
    }

    pub fn reset(&mut self) {
        self.text.clear();
        self.cursor_position = 0;
    }
}