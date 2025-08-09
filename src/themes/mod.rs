use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

pub fn interpolate_rgb(start: &Rgb, end: &Rgb, fraction: f32) -> Rgb {
    let r = (start.0 as f32 + (end.0 as f32 - start.0 as f32) * fraction) as u8;
    let g = (start.1 as f32 + (end.1 as f32 - start.1 as f32) * fraction) as u8;
    let b = (start.2 as f32 + (end.2 as f32 - start.2 as f32) * fraction) as u8;
    Rgb(r, g, b)
}

pub fn rgb_to_color(rgb: &Rgb) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum ThemeName {
    Default,
    Oceanic,
    Forest,
    Monochrome,
    CatppuccinMocha,
    Dracula,
    SolarizedDark,
    GruvboxDark,
    Nord,
    Cyberpunk,
    AutumnLeaves,
    HighContrastLight,
    Amethyst,
}

impl ThemeName {
    pub fn icon(&self) -> &str {
        match self {
            ThemeName::Default => "󰎓 ",
            ThemeName::Oceanic => "󰥛 ",
            ThemeName::Forest => "󰔱 ",
            ThemeName::Monochrome => "󰋰 ",
            ThemeName::CatppuccinMocha => "󰄛 ",
            ThemeName::Dracula => "󰭟 ",
            ThemeName::SolarizedDark => "󰓠 ",
            ThemeName::GruvboxDark => "󰟾 ",
            ThemeName::Nord => " ",
            ThemeName::Cyberpunk => "󰹫 ",
            ThemeName::AutumnLeaves => " ",
            ThemeName::HighContrastLight => " ",
            ThemeName::Amethyst => "󰮊 ",
        }
    }

    pub fn all_themes() -> Vec<ThemeName> {
        vec![
            ThemeName::Default,
            ThemeName::Oceanic,
            ThemeName::Forest,
            ThemeName::Monochrome,
            ThemeName::CatppuccinMocha,
            ThemeName::Dracula,
            ThemeName::SolarizedDark,
            ThemeName::GruvboxDark,
            ThemeName::Nord,
            ThemeName::Cyberpunk,
            ThemeName::AutumnLeaves,
            ThemeName::HighContrastLight,
            ThemeName::Amethyst,
        ]
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeColors {
    pub background: Rgb,
    pub border: Rgb,
    pub border_focus: Rgb,
    pub button: Rgb,
    pub text: Rgb,
    pub error: Rgb,
    pub dim: Rgb,
    pub accent: Rgb,
    pub title_gradient_start: Rgb,
    pub title_gradient_end: Rgb,
    pub input_border_active: Rgb,
    pub input_border_inactive: Rgb,
    pub input_text_active: Rgb,
    pub input_text_inactive: Rgb,
    pub placeholder_text: Rgb,
    pub selected_icon: Rgb,
    pub dimmed_icon: Rgb,
    pub button_text_active: Rgb,
    pub button_text_inactive: Rgb,
    pub button_border_active: Rgb,
    pub button_border_inactive: Rgb,
    pub button_bg_active: Rgb,
    pub help_text: Rgb,
    pub instructions_text: Rgb,
    pub popup_border: Rgb,
    pub popup_text: Rgb,
    pub mention_bg: Rgb,
    pub success_color: Rgb,
    pub warning_color: Rgb,
    pub info_color: Rgb,
    pub loading_color: Rgb,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Theme {
    pub name: ThemeName,
    pub icon: String,
    pub colors: ThemeColors,
}

#[derive(Debug, Deserialize)]
pub struct ThemesConfig {
    pub themes: Vec<Theme>,
}

impl ThemesConfig {
    pub fn get_all_themes() -> Result<HashMap<ThemeName, Theme>, Box<dyn std::error::Error>> {
        let config_str = include_str!("themes.json");
        let config: ThemesConfig = serde_json::from_str(config_str)?;
        let themes_map = config.themes.into_iter().map(|t| (t.name, t)).collect();
        Ok(themes_map)
    }
}

pub fn get_contrasting_text_color(bg_color: &Rgb) -> Color {
    let brightness =
        (bg_color.0 as f32 * 299.0 + bg_color.1 as f32 * 587.0 + bg_color.2 as f32 * 114.0)
            / 1000.0;
    if brightness > 128.0 {
        Color::Black
    } else {
        Color::White
    }
}
