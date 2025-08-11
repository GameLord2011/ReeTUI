use crate::app::AppState;
use crate::themes::rgb_to_color;
use ansi_to_tui::IntoText;
use chrono::{DateTime, Local};
use crossterm::event::{KeyCode, KeyEvent};
use devicons::{icon_for_file, FileIcon, Theme};
use image::{GenericImageView, ImageReader};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Padding, Paragraph},
};
use std::{
    collections::HashMap,
    env, fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style as SyntectStyle, ThemeSet},
    parsing::SyntaxSet,
};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio::io::AsyncWriteExt;
use tokio::process::Command as TokioCommand;

pub enum FileManagerEvent {
    FileSelectedForUpload(PathBuf),
    CloseFileManager,
    None,
}

#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: PathBuf,
    pub is_dir: bool,
    pub icon: FileIcon,
    pub children: Vec<FileItem>,
    pub expanded: bool,
    pub is_parent_nav: bool,
}

impl FileItem {
    fn new(path: PathBuf, is_dir: bool, is_parent_nav: bool) -> Self {
        let icon = icon_for_file(path.as_path(), &Some(Theme::Dark));
        Self {
            path,
            is_dir,
            icon,
            children: Vec::new(),
            expanded: false,
            is_parent_nav,
        }
    }
}

#[derive(Debug)]
pub struct FileManager {
    pub tree: FileItem,
    pub selected_index: usize,
    pub displayed_items: Vec<PathBuf>,
    pub redraw_tx: mpsc::UnboundedSender<String>,
    pub app_state: Arc<tokio::sync::Mutex<AppState>>,
    preview_tx: mpsc::UnboundedSender<(PathBuf, Result<Text<'static>, String>)>,
    preview_rx: mpsc::UnboundedReceiver<(PathBuf, Result<Text<'static>, String>)>,
    preview_cache: HashMap<PathBuf, Result<Text<'static>, String>>,
    generating_preview_for: Option<PathBuf>,
    syntax_set: Arc<SyntaxSet>,
    theme_set: Arc<ThemeSet>,
    metadata_tx: mpsc::UnboundedSender<(PathBuf, Result<String, String>)>,
    metadata_rx: mpsc::UnboundedReceiver<(PathBuf, Result<String, String>)>,
    metadata_cache: HashMap<PathBuf, Result<String, String>>,
    generating_metadata_for: Option<PathBuf>,
    gif_tx: mpsc::UnboundedSender<(PathBuf, Result<Vec<(Text<'static>, u32)>, String>)>,
    gif_rx: mpsc::UnboundedReceiver<(PathBuf, Result<Vec<(Text<'static>, u32)>, String>)>,
    gif_cache: HashMap<PathBuf, Result<Vec<(Text<'static>, u32)>, String>>,
    current_gif_frame: HashMap<PathBuf, usize>,
    last_rendered_height: u16,
}

impl FileManager {
    const MAX_UPLOAD_SIZE_MB: u64 = 25; // 25 MB
    const MAX_UPLOAD_SIZE_BYTES: u64 = Self::MAX_UPLOAD_SIZE_MB * 1024 * 1024;

    pub fn new(
        redraw_tx: mpsc::UnboundedSender<String>,
        app_state_param: Arc<tokio::sync::Mutex<AppState>>,
    ) -> Self {
        let current_path = dirs::home_dir().unwrap_or_else(|| {
            log::warn!("Could not determine home directory, falling back to current directory.");
            env::current_dir().unwrap_or_else(|e| {
                log::error!("Could not determine current directory: {}", e);
                PathBuf::from("/")
            })
        });

        let mut root = FileItem::new(current_path, true, false);
        Self::read_dir(&mut root);
        root.expanded = true;

        let (preview_tx, preview_rx) = mpsc::unbounded_channel();
        let (metadata_tx, metadata_rx) = mpsc::unbounded_channel();
        let (gif_tx, gif_rx) = mpsc::unbounded_channel::<(PathBuf, Result<Vec<(Text<'static>, u32)>, String>)>();

        Self {
            tree: root,
            selected_index: 0,
            displayed_items: Vec::new(),
            redraw_tx,
            app_state: app_state_param,
            preview_tx,
            preview_rx,
            preview_cache: HashMap::new(),
            generating_preview_for: None,
            syntax_set: Arc::new(SyntaxSet::load_defaults_newlines()),
            theme_set: Arc::new(ThemeSet::load_defaults()),
            metadata_tx,
            metadata_rx,
            metadata_cache: HashMap::new(),
            generating_metadata_for: None,
            gif_tx,
            gif_rx,
            gif_cache: HashMap::new(),
            current_gif_frame: HashMap::new(),
            last_rendered_height: 0,
        }
    }

    fn read_dir(item: &mut FileItem) {
        if !item.is_dir || !item.children.is_empty() {
            return;
        }

        if let Ok(entries) = fs::read_dir(&item.path) {
            let mut children: Vec<FileItem> = entries
                .filter_map(Result::ok)
                .map(|entry| {
                    let path = entry.path();
                    let is_dir = path.is_dir();
                    FileItem::new(path, is_dir, false)
                })
                .collect();
            children.sort_by(|a, b| (b.is_dir, &a.path).cmp(&(a.is_dir, &b.path)));

            if let Some(parent_path) = item.path.parent() {
                if parent_path != item.path {
                    children.insert(0, FileItem::new(parent_path.to_path_buf(), true, true));
                }
            }
            item.children = children;
        }
    }

    pub fn ui(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.current_theme;

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)].as_ref())
            .split(area);

        let left_area = chunks[0];
        let right_area = chunks[1];

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
            .split(right_area);

        let preview_area = main_chunks[0];
        let metadata_area = main_chunks[1];

        let file_tree_block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(rgb_to_color(&theme.colors.accent)))
            .padding(Padding::new(0, 0, 0, 0));
        f.render_widget(file_tree_block.clone(), left_area);
        let inner_left_area = file_tree_block.inner(left_area);
        self.last_rendered_height = inner_left_area.height;

        let mut lines = Vec::new();
        self.displayed_items.clear();
        let mut displayed_items = Vec::new();
        for child in &self.tree.children {
            self.render_tree(child, &mut lines, 0, &mut displayed_items, theme);
        }
        self.displayed_items = displayed_items;

        let list_height = inner_left_area.height as usize;
        let start_index = self
            .selected_index
            .saturating_sub(list_height / 2)
            .min(self.displayed_items.len().saturating_sub(list_height));

        for (i, line) in lines.iter().enumerate().skip(start_index).take(list_height) {
            let y = inner_left_area.y + (i - start_index) as u16;
            let is_selected = i == self.selected_index;
            let style = if is_selected {
                Style::default()
                    .fg(rgb_to_color(&theme.colors.background))
                    .bg(rgb_to_color(&theme.colors.accent))
            } else {
                Style::default().fg(Color::White)
            };
            f.buffer_mut()
                .set_line(inner_left_area.x, y, line, inner_left_area.width);
            if is_selected {
                f.buffer_mut().set_style(
                    Rect::new(inner_left_area.x, y, inner_left_area.width, 1),
                    style,
                );
            }
        }

        let preview_block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(rgb_to_color(&theme.colors.accent)))
            .padding(Padding::new(1, 1, 1, 1));
        let inner_preview_area = preview_block.inner(preview_area);
        f.render_widget(
            Block::default().bg(rgb_to_color(&theme.colors.background)),
            preview_area,
        );
        f.render_widget(preview_block.clone(), preview_area);

        if let Ok((path, result)) = self.preview_rx.try_recv() {
            self.preview_cache.insert(path, result);
            self.generating_preview_for = None;
        }

        if let Ok((path, result)) = self.gif_rx.try_recv() {
            self.gif_cache.insert(path.clone(), result);
            // When a new GIF is cached, reset its frame to 0
            if let Some(Ok(frames_with_delays)) = self.gif_cache.get(&path) {
                if !frames_with_delays.is_empty() {
                    self.current_gif_frame.insert(path, 0);
                }
            }
        }

        let mut gif_frame_to_update: Option<(PathBuf, usize)> = None;

        if let Some(item) = self.get_selected_item() {
            if item.is_dir {
                f.render_widget(Clear, inner_preview_area);
                let folder_ascii_lines: Vec<&str> = "\n╭───────────╮             \n│           ╰──────────╮  \n│ ╭──────────────────────╮\n│ │                      │\n│ │                      │\n│ │                      │\n│ │                      │\n│ │                      │\n│ │                      │\n│ │                      │\n│ │                      │\n╰─╰──────────────────────╯".lines().collect();
                let ascii_height = folder_ascii_lines.len() as u16;
                let preview_height = inner_preview_area.height;
                let padding_top = preview_height.saturating_sub(ascii_height) / 2;
                let ascii_width = 24; // Max width of folder ASCII art
                let preview_width = inner_preview_area.width;
                let padding_left = preview_width.saturating_sub(ascii_width) / 2;
                let horizontal_padding_str = " ".repeat(padding_left as usize);

                let mut text_lines = Vec::new();
                for _ in 0..padding_top {
                    text_lines.push(Line::raw(""));
                }
                for line_str in folder_ascii_lines {
                    text_lines.push(Line::raw(format!("{}{}", horizontal_padding_str, line_str)));
                }

                let p = Paragraph::new(Text::from(text_lines))
                    .style(Style::default().bg(rgb_to_color(&theme.colors.background)));
                f.render_widget(p, inner_preview_area);
            } else {
                if let Some(cached_gif) = self.gif_cache.get(&item.path) {
                    match cached_gif {
                        Ok(frames_with_delays) => {
                            let current_frame_index =
                                *self.current_gif_frame.get(&item.path).unwrap_or(&0);
                            if let Some((frame_text, delay_ms_ref)) = frames_with_delays.get(current_frame_index) {
                                let delay_ms = *delay_ms_ref; // Copy the u32 value
                                let p = Paragraph::new(frame_text.clone()).style(
                                    Style::default().bg(rgb_to_color(&theme.colors.background)),
                                );
                                f.render_widget(p, inner_preview_area);

                                // Prepare update for next frame
                                let next_frame_index = (current_frame_index + 1) % frames_with_delays.len();
                                gif_frame_to_update = Some((item.path.clone(), next_frame_index));

                                // Schedule redraw for next frame
                                let redraw_tx_clone = self.redraw_tx.clone();
                                tokio::spawn(async move {
                                    sleep(Duration::from_millis(delay_ms as u64)).await;
                                    let _ = redraw_tx_clone.send("redraw".to_string());
                                });
                            } else {
                                let p = Paragraph::new("GIF frame error").style(
                                    Style::default()
                                        .fg(Color::Red)
                                        .bg(rgb_to_color(&theme.colors.background)),
                                );
                                f.render_widget(p, inner_preview_area);
                            }
                        }
                        Err(e) => {
                            let p = Paragraph::new(format!("GIF error: {}", e)).style(
                                Style::default()
                                    .fg(Color::Red)
                                    .bg(rgb_to_color(&theme.colors.background)),
                            );
                            f.render_widget(p, inner_preview_area);
                        }
                    }
                } else if let Some(cached_preview) = self.preview_cache.get(&item.path) {
                    match cached_preview {
                        Ok(text) => {
                            let p = Paragraph::new(text.clone())
                                .style(Style::default().bg(rgb_to_color(&theme.colors.background)));
                            f.render_widget(p, inner_preview_area);
                        }
                        Err(e) => {
                            let p = Paragraph::new(format!("Preview error: {}", e)).style(
                                Style::default()
                                    .fg(Color::Red)
                                    .bg(rgb_to_color(&theme.colors.background)),
                            );
                            f.render_widget(p, inner_preview_area);
                        }
                    }
                } else {
                    let path = item.path.clone();
                    if self.generating_preview_for.as_ref() != Some(&path) {
                        self.generating_preview_for = Some(path.clone());
                        let tx = self.preview_tx.clone();
                        let gif_tx = self.gif_tx.clone();
                        let syntax_set = Arc::clone(&self.syntax_set);
                        let theme_set = Arc::clone(&self.theme_set);
                        let height = inner_preview_area.height;
                        let is_image_file = FileManager::is_image(&path);
                        let is_likely_binary_file = FileManager::is_likely_binary(&path);

                        tokio::spawn(async move {
                            let result = if is_image_file {
                                if path
                                    .extension()
                                    .map_or(false, |ext| ext.to_ascii_lowercase() == "gif")
                                {
                                    // Handle GIF decoding
                                    let frames_result =
                                        decode_gif_frames(&path, inner_preview_area.width, height)
                                            .await;
                                    let _ = gif_tx.send((path.clone(), frames_result));
                                    Ok(Text::raw("Loading GIF...")) // Display loading message in preview
                                } else {
                                    // Handle other images with chafa
                                    let cmd = TokioCommand::new("chafa")
                                        .arg("-f")
                                        .arg("symbols")
                                        .arg(format!(
                                            "--size={}x{}",
                                            inner_preview_area.width, height
                                        ))
                                        .arg(&path)
                                        .output()
                                        .await;

                                    match cmd {
                                        Ok(output) => {
                                            if output.status.success() {
                                                Ok(String::from_utf8_lossy(&output.stdout)
                                                    .to_string()
                                                    .into_text()
                                                    .unwrap())
                                            } else {
                                                Err(String::from_utf8_lossy(&output.stderr)
                                                    .into_owned())
                                            }
                                        }
                                        Err(e) => Err(e.to_string()),
                                    }
                                }
                            } else if is_likely_binary_file {
                                let binary_ascii_lines: Vec<&str> = "\n╭───────────────╮\n│100010110101010│\n│010110101010100│\n│101101010110101│\n│101010101101010│\n│101010101101010│\n│010100010101100│\n│111110010100101│\n│101010101010100│\n├─────╮101011010│\n│     │001001101│\n│.BIN │101010101│\n╰─────┴─────────╯".lines().collect();
                                let ascii_height = binary_ascii_lines.len() as u16;
                                let preview_height = inner_preview_area.height;
                                let padding_top = preview_height.saturating_sub(ascii_height) / 2;
                                let ascii_width = 17; // Max width of binary ASCII art
                                let preview_width = inner_preview_area.width;
                                let padding_left = preview_width.saturating_sub(ascii_width) / 2;
                                let horizontal_padding_str = " ".repeat(padding_left as usize);

                                let mut text_lines = Vec::new();
                                for _ in 0..padding_top {
                                    text_lines.push(Line::raw(""));
                                }
                                for line_str in binary_ascii_lines {
                                    text_lines.push(Line::raw(format!("{}{}", horizontal_padding_str, line_str)));
                                }
                                Ok(Text::from(text_lines))
                            } else {
                                let syntax = syntax_set
                                    .find_syntax_by_extension(
                                        path.extension().and_then(|s| s.to_str()).unwrap_or(""),
                                    )
                                    .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

                                let theme = &theme_set.themes["base16-ocean.dark"];
                                let file = fs::File::open(&path);
                                let mut lines_vec = Vec::new();
                                if let Ok(file) = file {
                                    let reader = BufReader::new(file);
                                    for (i, line) in reader.lines().enumerate() {
                                        if i >= height as usize {
                                            break;
                                        }
                                        if let Ok(line) = line {
                                            let mut h = HighlightLines::new(syntax, theme);
                                            let ranges: Vec<(SyntectStyle, &str)> =
                                                h.highlight_line(&line, &syntax_set).unwrap();
                                            let spans: Vec<Span> = ranges
                                                .iter()
                                                .map(|(style, text)| {
                                                    let color = style.foreground;
                                                    Span::styled(
                                                        text.to_string(),
                                                        Style::default().fg(Color::Rgb(
                                                            color.r, color.g, color.b,
                                                        )),
                                                    )
                                                })
                                                .collect();
                                            lines_vec.push(Line::from(spans));
                                        }
                                    }
                                    Ok(Text::from(lines_vec))
                                } else {
                                    Err("Cannot open file.".to_string())
                                }
                            };
                            let _ = tx.send((path, result));
                        });
                    }
                    let p = Paragraph::new("Generating preview...")
                        .style(Style::default().bg(rgb_to_color(&theme.colors.background)));
                    f.render_widget(p, inner_preview_area);
                }
            }
        } else {
            let p = Paragraph::new("No item selected")
                .style(Style::default().bg(rgb_to_color(&theme.colors.background)));
            f.render_widget(p, inner_preview_area);
        }

        // Apply GIF frame update after rendering
        if let Some((path, next_frame_index)) = gif_frame_to_update {
            self.current_gif_frame.insert(path, next_frame_index);
        }

        // Metadata block
        let metadata_block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(rgb_to_color(&theme.colors.accent)))
            .title(Span::styled(
                "Metadata",
                Style::default().fg(rgb_to_color(&theme.colors.text)),
            ))
            .padding(Padding::new(1, 1, 1, 1));
        let inner_metadata_area = metadata_block.inner(metadata_area);
        f.render_widget(
            Block::default().bg(rgb_to_color(&theme.colors.background)),
            metadata_area,
        );
        f.render_widget(metadata_block.clone(), metadata_area);

        if let Ok((path, result)) = self.metadata_rx.try_recv() {
            self.metadata_cache.insert(path, result);
            self.generating_metadata_for = None;
        }

        if let Some(item) = self.get_selected_item() {
            let metadata_field_constraints: Vec<Constraint> =
                (0..9).map(|_| Constraint::Length(3)).collect();
            let _metadata_display_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(metadata_field_constraints)
                .split(inner_metadata_area);

            let mut metadata_info = HashMap::new();

            if item.is_dir {
                metadata_info.insert("Type".to_string(), "Directory".to_string());
            } else {
                if let Some(cached) = self.metadata_cache.get(&item.path) {
                    match cached {
                        Ok(text) => {
                            let lines: Vec<&str> = text.lines().collect();
                            for line in lines {
                                if let Some((key, value)) = line.split_once(": ") {
                                    metadata_info.insert(key.to_string(), value.to_string());
                                }
                            }
                        }
                        Err(e) => {
                            metadata_info
                                .insert("Error".to_string(), format!("Metadata error: {}", e));
                        }
                    }
                } else {
                    let path = item.path.clone();
                    if self.generating_metadata_for.as_ref() != Some(&path) {
                        self.generating_metadata_for = Some(path.clone());
                        let tx = self.metadata_tx.clone();

                        tokio::spawn(async move {
                            let result = match fs::metadata(&path) {
                                Ok(metadata) => {
                                    let mut info = String::new();
                                    info.push_str(&format!(
                                        "Size: {}\n",
                                        FileManager::format_file_size(metadata.len())
                                    ));
                                    if let Ok(created) = metadata.created() {
                                        info.push_str(&format!(
                                            "Created: {}\n",
                                            DateTime::<Local>::from(created)
                                                .format("%Y-%m-%d %H:%M:%S")
                                        ));
                                    }
                                    if let Ok(modified) = metadata.modified() {
                                        info.push_str(&format!(
                                            "Last Modified: {}\n",
                                            DateTime::<Local>::from(modified)
                                                .format("%Y-%m-%d %H:%M:%S")
                                        ));
                                    }
                                    info.push_str(&format!(
                                        "Type: {}\n",
                                        if metadata.is_file() {
                                            "File"
                                        } else if metadata.is_dir() {
                                            "Directory"
                                        } else {
                                            "Other"
                                        }
                                    ));
                                    info.push_str(&format!(
                                        "Permissions: {:?}\n",
                                        metadata.permissions()
                                    ));

                                    // Resolution for images
                                    if FileManager::is_image(&path) {
                                        if let Ok(reader) = ImageReader::open(&path) {
                                            if let Ok(img) = reader.decode() {
                                                let (width, height) = img.dimensions();
                                                info.push_str(&format!(
                                                    "Resolution: {}x{}\n",
                                                    width, height
                                                ));
                                            }
                                        }
                                    }

                                    // Duration and resolution for videos
                                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                                        if matches!(
                                            ext.to_lowercase().as_str(),
                                            "mp4" | "mkv" | "avi" | "mov"
                                        ) {
                                            let cmd = Command::new("ffprobe")
                                                .args(&[
                                                    "-v",
                                                    "error",
                                                    "-select_streams",
                                                    "v:0", // Select video stream
                                                    "-show_entries",
                                                    "stream=width,height:format=duration", // Get width, height, duration
                                                    "-of",
                                                    "default=noprint_wrappers=1:nokey=1",
                                                    path.to_str().unwrap(),
                                                ])
                                                .output();

                                            if let Ok(output) = cmd {
                                                if output.status.success() {
                                                    let output_str =
                                                        String::from_utf8_lossy(&output.stdout);
                                                    let lines: Vec<&str> =
                                                        output_str.trim().lines().collect();
                                                    let mut resolution_found = false;
                                                    let mut duration_found = false;

                                                    for line in lines {
                                                        if line.contains('x') && !resolution_found {
                                                            // Simple check for resolution
                                                            info.push_str(&format!(
                                                                "Resolution: {}\n",
                                                                line
                                                            ));
                                                            resolution_found = true;
                                                        } else if let Ok(duration) =
                                                            line.parse::<f64>()
                                                        {
                                                            // Check for duration
                                                            let minutes = (duration / 60.0).round();
                                                            info.push_str(&format!(
                                                                "Duration: {} minutes\n",
                                                                minutes
                                                            ));
                                                            duration_found = true;
                                                        }
                                                        if resolution_found && duration_found {
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Too big to send
                                    if metadata.len() > FileManager::MAX_UPLOAD_SIZE_BYTES {
                                        info.push_str(&format!(
                                            "Too Big to Send: Yes (> {}\n)",
                                            FileManager::format_file_size(
                                                FileManager::MAX_UPLOAD_SIZE_BYTES
                                            )
                                        ));
                                    } else {
                                        info.push_str("Too Big to Send: No\n");
                                    }

                                    Ok(info)
                                }
                                Err(e) => Err(format!("Failed to get metadata: {}", e)),
                            };
                            let _ = tx.send((path, result));
                        });
                    }
                    metadata_info.insert("Status".to_string(), "Loading metadata...".to_string());
                }
            }

            let size_str = metadata_info
                .get("Size")
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            let created_str = metadata_info
                .get("Created")
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            let last_modified_str = metadata_info
                .get("Last Modified")
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            let type_str = metadata_info
                .get("Type")
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            let permissions_str = metadata_info
                .get("Permissions")
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            let resolution_str = metadata_info
                .get("Resolution")
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            let duration_str = metadata_info
                .get("Duration")
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            let too_big_str = metadata_info
                .get("Too Big to Send")
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            let status_str = metadata_info
                .get("Status")
                .map(|s| s.as_str())
                .unwrap_or("N/A");

            let fields = [
                ("Size", size_str),
                ("Created", created_str),
                ("Last Modified", last_modified_str),
                ("Type", type_str),
                ("Permissions", permissions_str),
                ("Resolution", resolution_str),
                ("Duration", duration_str),
                ("Too Big to Send", too_big_str),
                ("Status", status_str),
            ];

            let metadata_field_constraints: Vec<Constraint> =
                fields.iter().map(|_| Constraint::Length(3)).collect();
            let _metadata_display_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(metadata_field_constraints)
                .split(inner_metadata_area);

            for (i, (label, value)) in fields.iter().enumerate() {
                let chunk = _metadata_display_chunks[i];
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(rgb_to_color(&theme.colors.accent)))
                    .title(Span::styled(
                        *label,
                        Style::default().fg(rgb_to_color(&theme.colors.text)),
                    ));
                let p = Paragraph::new(*value)
                    .style(Style::default().bg(rgb_to_color(&theme.colors.background)));
                f.render_widget(block.clone(), chunk);
                f.render_widget(p, block.inner(chunk));
            }
        } else {
            let p = Paragraph::new("No item selected")
                .style(Style::default().bg(rgb_to_color(&theme.colors.background)));
            f.render_widget(p, inner_metadata_area);
        }
    }

    fn is_likely_binary(path: &Path) -> bool {
        let mut file = match fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return true,
        };
        let mut buffer = [0; 1024];
        let n = match file.read(&mut buffer) {
            Ok(n) => n,
            Err(_) => return true,
        };
        buffer[..n].contains(&0)
    }

    fn is_image(path: &Path) -> bool {
        let extension = path.extension().and_then(|s| s.to_str());
        if let Some(ext) = extension {
            matches!(
                ext.to_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "bmp"
            )
        } else {
            false
        }
    }

    fn render_tree<'a>(
        &self,
        item: &'a FileItem,
        lines: &mut Vec<Line<'a>>,
        depth: usize,
        displayed_items: &mut Vec<PathBuf>,
        theme: &crate::themes::Theme,
    ) {
        let prefix = " ".repeat(depth * 2);
        let icon_span = if item.is_dir {
            let folder_icon = if item.expanded { "" } else { "" };
            let folder_style = Style::default().fg(Color::White);
            Span::styled(folder_icon.to_string(), folder_style)
        } else {
            let icon_str = item.icon.icon.to_string();
            let file_icon = if icon_str == "*" {
                "".to_string()
            } else {
                icon_str
            };
            let color_u32 = u32::from_str_radix(item.icon.color.trim_start_matches('#'), 16)
                .unwrap_or(0xFFFFFF);
            let r = ((color_u32 >> 16) & 0xFF) as u8;
            let g = ((color_u32 >> 8) & 0xFF) as u8;
            let b = (color_u32 & 0xFF) as u8;
            Span::styled(file_icon, Style::default().fg(Color::Rgb(r, g, b)))
        };

        let file_name = if item.is_parent_nav {
            "..".to_string()
        } else {
            item.path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        };
        let file_style = Style::default().fg(rgb_to_color(&theme.colors.text));

        lines.push(Line::from(vec![
            Span::raw(prefix.clone()),
            icon_span.clone(),
            Span::raw(" "),
            Span::styled(file_name.to_string(), file_style),
        ]));
        displayed_items.push(item.path.clone());

        if item.expanded {
            for child in &item.children {
                self.render_tree(child, lines, depth + 1, displayed_items, theme);
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> FileManagerEvent {
        match key.code {
            KeyCode::Up => self.select_previous(),
            KeyCode::Down => self.select_next(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Right => self.expand_dir(),
            KeyCode::Left => self.collapse_dir(),
            KeyCode::Enter => {
                if let Some(item) = self.get_selected_item_mut() {
                    if item.is_dir {
                        let new_root_path = item.path.clone();
                        self.tree = FileItem::new(new_root_path, true, false);
                        Self::read_dir(&mut self.tree);
                        self.tree.expanded = true;
                        self.selected_index = 0;
                    } else {
                        return FileManagerEvent::FileSelectedForUpload(item.path.clone());
                    }
                }
            }
            KeyCode::Esc => return FileManagerEvent::CloseFileManager,
            _ => {}
        }
        FileManagerEvent::None
    }

    fn get_selected_item_mut(&mut self) -> Option<&mut FileItem> {
        if self.selected_index >= self.displayed_items.len() {
            return None;
        }
        let path = &self.displayed_items[self.selected_index].clone();
        Self::find_item_mut(&mut self.tree, path)
    }

    fn get_selected_item(&self) -> Option<&FileItem> {
        if self.selected_index >= self.displayed_items.len() {
            return None;
        }
        let path = &self.displayed_items[self.selected_index];
        Self::find_item(&self.tree, path)
    }

    fn find_item_mut<'a>(item: &'a mut FileItem, path: &Path) -> Option<&'a mut FileItem> {
        if item.path == path {
            return Some(item);
        }
        if item.expanded {
            for child in &mut item.children {
                if let Some(found) = Self::find_item_mut(child, path) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn find_item<'a>(item: &'a FileItem, path: &Path) -> Option<&'a FileItem> {
        if item.path == path {
            return Some(item);
        }
        if item.expanded {
            for child in &item.children {
                if let Some(found) = Self::find_item(child, path) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn select_next(&mut self) {
        if self.selected_index < self.displayed_items.len() - 1 {
            self.selected_index += 1;
        }
    }

    fn expand_dir(&mut self) {
        if let Some(item) = self.get_selected_item_mut() {
            if item.is_dir && !item.expanded {
                Self::read_dir(item);
                item.expanded = true;
            }
        }
    }

    fn collapse_dir(&mut self) {
        if let Some(item) = self.get_selected_item_mut() {
            if item.is_dir && item.expanded {
                item.expanded = false;
            }
        }
    }

    fn page_up(&mut self) {
        let list_height = self.last_rendered_height as usize;
        if self.selected_index > list_height {
            self.selected_index -= list_height;
        } else {
            self.selected_index = 0;
        }
    }

    fn page_down(&mut self) {
        let list_height = self.last_rendered_height as usize;
        let max_index = self.displayed_items.len().saturating_sub(1);
        if self.selected_index + list_height < max_index {
            self.selected_index += list_height;
        } else {
            self.selected_index = max_index;
        }
    }
}

async fn decode_gif_frames(
    path: &Path,
    width: u16,
    height: u16,
) -> Result<Vec<(Text<'static>, u32)>, String> {
    let file = fs::File::open(path).map_err(|e| format!("Failed to open GIF: {}", e))?;
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = decoder
        .read_info(file)
        .map_err(|e| format!("Failed to read GIF info: {}", e))?;

    let mut frames_with_delays = Vec::new();
    // Read all frames first to avoid borrowing issues
    let mut gif_frames_data: Vec<gif::Frame> = Vec::new();
    let (gif_width, gif_height) = (decoder.width() as u32, decoder.height() as u32);

    while let Some(frame) = decoder
        .read_next_frame()
        .map_err(|e| format!("Failed to read GIF frame: {}", e))?
    {
        gif_frames_data.push(frame.to_owned());
    }

    for frame in gif_frames_data {
        let mut image_buffer = image::RgbaImage::new(gif_width, gif_height);
        // Copy the frame data into the correct position within the full buffer
        image::imageops::overlay(
            &mut image_buffer,
            &image::RgbaImage::from_raw(
                frame.width as u32,
                frame.height as u32,
                frame.buffer.to_vec(),
            )
            .unwrap(),
            frame.left as i64,
            frame.top as i64,
        );

        // Encode image_buffer to PNG in memory
        let mut png_bytes = Vec::new();
        image_buffer
            .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode GIF frame to PNG: {}", e))?;

        let mut cmd = TokioCommand::new("chafa");
        cmd.arg("-f")
            .arg("symbols")
            .arg(format!("--size={}x{}", width, height))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn chafa: {}", e))?;

        // Write PNG bytes to chafa's stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&png_bytes).await.map_err(|e| format!("Failed to write to chafa stdin: {}", e))?;
            // Close stdin to signal EOF to chafa
            drop(stdin);
        } else {
            return Err("Failed to get chafa stdin".to_string());
        }

        let output = child.wait_with_output().await.map_err(|e| format!("Failed to wait for chafa: {}", e))?;

        if output.status.success() {
            let text_frame = String::from_utf8_lossy(&output.stdout)
                .to_string()
                .into_text()
                .map_err(|e| format!("Failed to convert ANSI to Text: {:?}", e))?;
            let delay_ms = frame.delay as u32 * 10; // Convert 1/100s to ms
            frames_with_delays.push((text_frame, delay_ms));
        } else {
            return Err(format!(
                "Chafa error: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    Ok(frames_with_delays)
}

impl FileManager {
    fn format_file_size(size: u64) -> String {
        const KIB: u64 = 1024;
        const MIB: u64 = KIB * 1024;
        const GIB: u64 = MIB * 1024;
        const TIB: u64 = GIB * 1024;

        if size < KIB {
            format!("{} B", size)
        } else if size < MIB {
            format!("{:.2} KiB", size as f64 / KIB as f64)
        } else if size < GIB {
            format!("{:.2} MiB", size as f64 / MIB as f64)
        } else {
            format!("{:.2} TiB", size as f64 / TIB as f64)
        }
    }
}
