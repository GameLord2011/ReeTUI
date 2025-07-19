use crate::tui::themes::ThemeName;
use ratatui::widgets::ListState;

pub struct ThemeSettingsForm {
    pub selected_theme_index: usize,
    pub themes: Vec<ThemeName>,
    pub list_state: ListState,
    pub original_theme: ThemeName,
}

impl ThemeSettingsForm {
    pub fn new(current_theme: ThemeName) -> Self {
        let themes = ThemeName::all_themes();
        let selected_theme_index = themes
            .iter()
            .position(|&t| t == current_theme)
            .unwrap_or(0);

        let mut list_state = ListState::default();
        list_state.select(Some(selected_theme_index));

        Self {
            selected_theme_index,
            themes,
            list_state,
            original_theme: current_theme,
        }
    }

    pub fn next_theme(&mut self) {
        self.selected_theme_index = (self.selected_theme_index + 1) % self.themes.len();
        self.list_state.select(Some(self.selected_theme_index));
    }

    pub fn previous_theme(&mut self) {
        self.selected_theme_index =
            (self.selected_theme_index + self.themes.len() - 1) % self.themes.len();
        self.list_state.select(Some(self.selected_theme_index));
    }

    pub fn get_selected_theme(&self) -> ThemeName {
        self.themes[self.selected_theme_index]
    }
}