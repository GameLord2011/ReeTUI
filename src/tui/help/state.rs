#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HelpState {
    pub current_page: usize,
    pub total_pages: usize,
    pub show_font_check_page: bool,
    pub show_chafa_check_page: bool,
    pub info_text_animation_progress: usize,
    pub gauge_animation_start_ratio: f64,
    pub gauge_animation_end_ratio: f64,
    pub gauge_animation_progress: f64,
    pub gauge_animation_active: bool,
}

impl Default for HelpState {
    fn default() -> Self {
        Self {
            current_page: 0,
            total_pages: 0, // Will be calculated dynamically
            show_font_check_page: true,
            show_chafa_check_page: true,
            info_text_animation_progress: 0,
            gauge_animation_start_ratio: 0.0,
            gauge_animation_end_ratio: 0.0,
            gauge_animation_progress: 0.0,
            gauge_animation_active: false,
        }
    }
}

impl HelpState {
    pub fn next_page(&mut self) -> Option<crate::app::TuiPage> {
        self.info_text_animation_progress = 0;
        if self.current_page < self.total_pages - 1 {
            let start_ratio = if self.total_pages > 0 {
                (self.current_page + 1) as f64 / self.total_pages as f64
            } else {
                0.0
            };

            self.current_page += 1;

            let end_ratio = if self.total_pages > 0 {
                (self.current_page + 1) as f64 / self.total_pages as f64
            } else {
                0.0
            };

            self.gauge_animation_start_ratio = start_ratio;
            self.gauge_animation_end_ratio = end_ratio;
            self.gauge_animation_progress = 0.0;
            self.gauge_animation_active = true;

            None
        } else {
            self.current_page = 0; // Reset for next time
            Some(crate::app::TuiPage::Auth)
        }
    }

    pub fn previous_page(&mut self) {
        self.info_text_animation_progress = 0;
        if self.current_page > 0 {
            self.current_page -= 1;
        } else {
            self.current_page = self.total_pages - 1; // Loop back to the last page
        }
    }
}