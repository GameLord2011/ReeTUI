#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct HelpState {
    pub current_page: usize,
    pub total_pages: usize,
}

impl HelpState {
    pub fn new(total_pages: usize) -> Self {
        Self {
            current_page: 0,
            total_pages,
        }
    }

    pub fn next_page(&mut self) -> Option<crate::app::TuiPage> {
        if self.current_page < self.total_pages - 1 {
            self.current_page += 1;
            None
        } else {
            self.current_page = 0; // Reset for next time
            Some(crate::app::TuiPage::Auth)
        }
    }

    pub fn previous_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
        } else {
            self.current_page = self.total_pages - 1; // Loop back to the last page
        }
    }
}
