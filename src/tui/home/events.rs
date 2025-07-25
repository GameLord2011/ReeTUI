use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crate::tui::TuiPage;
use std::io;

pub fn handle_home_event(wait_time: std::time::Duration) -> io::Result<Option<TuiPage>> {
    if event::poll(wait_time)? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(Some(TuiPage::Exit)),
                    _ => return Ok(Some(TuiPage::Chat)),
                }
            }
        }
    }
    Ok(None)
}
