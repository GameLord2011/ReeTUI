use crate::themes::ThemeName;
use ratatui::widgets::ListState;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FocusedPane {
    Left,
    Right,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SettingsScreen {
    Themes,
    Help,
    UserSettings,
    Quit,
}

#[derive(Clone, Debug)]
pub struct SettingsState {
    pub screen: SettingsScreen,
    pub focused_pane: FocusedPane,
    pub main_selection: usize,
    pub theme_list_state: ListState,
    pub themes: Vec<ThemeName>,
    pub new_username: String,
    pub new_icon: String,
    pub original_username: String,
    pub original_icon: String,
}

impl SettingsState {
    pub fn new(
        themes: Vec<ThemeName>,
        current_theme_name: ThemeName,
        username: &str,
        icon: &str,
        main_selection: usize,
        focused_pane: FocusedPane,
    ) -> Self {
        let mut theme_list_state = ListState::default();
        let theme_selection = themes
            .iter()
            .position(|&t| t == current_theme_name)
            .unwrap_or(0);
        theme_list_state.select(Some(theme_selection));

        Self {
            screen: SettingsScreen::Themes,
            focused_pane,
            main_selection,
            theme_list_state,
            themes,
            new_username: username.to_string(),
            new_icon: icon.to_string(),
            original_username: username.to_string(),
            original_icon: icon.to_string(),
        }
    }

    pub fn get_selected_theme_index(&self) -> Option<usize> {
        self.theme_list_state.selected()
    }

    pub fn next_main_setting(&mut self) {
        self.main_selection = (self.main_selection + 1) % 4; // 4 settings: Themes, Help, User Settings, Quit
        self.update_screen_from_selection();
    }

    pub fn previous_main_setting(&mut self) {
        self.main_selection = if self.main_selection == 0 {
            3
        } else {
            self.main_selection - 1
        };
        self.update_screen_from_selection();
    }

    fn update_screen_from_selection(&mut self) {
        self.screen = match self.main_selection {
            0 => SettingsScreen::Themes,
            1 => SettingsScreen::Help,
            2 => SettingsScreen::UserSettings,
            3 => SettingsScreen::Quit,
            _ => unreachable!(),
        };
    }

    pub fn next_theme(&mut self) {
        let i = match self.get_selected_theme_index() {
            Some(i) => {
                if i >= self.themes.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.theme_list_state.select(Some(i));
    }

    pub fn previous_theme(&mut self) {
        let i = match self.get_selected_theme_index() {
            Some(i) => {
                if i == 0 {
                    self.themes.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.theme_list_state.select(Some(i));
    }

    pub fn is_user_logged_in(&self) -> bool {
        !self.original_username.is_empty()
    }
}
