use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::colors::ColorPalette;
use crate::display::{PixelatedImage, RatatuiImage};
use crate::lyrics::{parser, ParsedLyrics};
use crate::models::AlbumMetadata;
use crate::player::PlayerMetadata;

pub struct App {
    pub should_quit: bool,
    pub album_art: Option<PixelatedImage>,
    pub album_art_ratatui: Option<RatatuiImage>,
    pub album_metadata: Option<AlbumMetadata>,
    pub player_metadata: PlayerMetadata,
    pub progress: f32,
    pub color_palette: Option<ColorPalette>,
    pub lyrics: Option<ParsedLyrics>,
    pub lyrics_loading: bool,
    pub waiting_for_player: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            album_art: None,
            album_art_ratatui: None,
            album_metadata: None,
            player_metadata: PlayerMetadata {
                artist: String::new(),
                title: String::new(),
                album: None,
                position: None,
                length: None,
                streaming_source: None,
                art_url: None,
                status: None,
            },
            progress: 0.0,
            color_palette: None,
            lyrics: None,
            lyrics_loading: false,
            waiting_for_player: true,
        }
    }

    pub fn update_metadata(
        &mut self,
        album_art: Option<PixelatedImage>,
        album_art_ratatui: Option<RatatuiImage>,
        album_metadata: Option<AlbumMetadata>,
        player_metadata: PlayerMetadata,
        progress: f32,
        color_palette: Option<ColorPalette>,
    ) {
        self.album_art = album_art;
        self.album_art_ratatui = album_art_ratatui;
        self.album_metadata = album_metadata;
        self.player_metadata = player_metadata;
        self.progress = progress;
        self.color_palette = color_palette;
        self.waiting_for_player = false;
    }

    pub fn update_lyrics(&mut self, lyrics: Option<ParsedLyrics>) {
        self.lyrics = lyrics;
        self.lyrics_loading = false;
    }

    pub fn set_lyrics_loading(&mut self, loading: bool) {
        self.lyrics_loading = loading;
    }
}

pub fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: Arc<Mutex<App>>,
) -> Result<()> {
    // Clear the terminal once at the start
    terminal.clear()?;

    loop {
        // Draw UI with current state
        terminal.draw(|f| {
            let app = app.lock().unwrap();
            ui(f, &app)
        })?;

        // Check for input events
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    // Clear the entire area first
    f.render_widget(Clear, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Main content
        ])
        .split(f.area());

    // If waiting for player, show waiting message
    if app.waiting_for_player {
        // Render title bar even when waiting
        let title = Paragraph::new("🎵 trackwatch")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        f.render_widget(title, chunks[0]);

        let waiting_lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from("🎵 Waiting for media player..."),
            Line::from(""),
            Line::from("Start playing music in:"),
            Line::from("  • YouTube (browser)"),
            Line::from("  • Spotify"),
            Line::from("  • Tidal"),
            Line::from("  • VLC"),
            Line::from("  • Any MPRIS2-compatible player"),
            Line::from(""),
            Line::from("Press 'q' or 'Esc' to quit"),
        ];

        let waiting_message = Paragraph::new(waiting_lines)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::NONE));

        f.render_widget(waiting_message, chunks[1]);
        return;
    }

    // Title with streaming source
    let mut title_spans = vec![
        Span::styled("🎵 ", Style::default()),
        Span::styled(
            if !app.player_metadata.artist.is_empty() {
                format!(
                    "Now Playing: {} - {}",
                    app.player_metadata.artist, app.player_metadata.title
                )
            } else {
                format!("Now Playing: {}", app.player_metadata.title)
            },
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];

    // Add streaming source if available
    if let Some(ref source) = app.player_metadata.streaming_source {
        title_spans.push(Span::styled(
            format!(" ({source})"),
            Style::default().fg(Color::Cyan),
        ));
    }

    let title = Paragraph::new(Line::from(title_spans))
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(title, chunks[0]);

    // Create a container that limits height to match album art + progress bar
    let content_height = 35; // 32 (album art) + 3 (progress bar)
    let content_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(content_height),
            Constraint::Min(0), // Remaining space (unused)
        ])
        .split(chunks[1])[0]; // Take only the first chunk

    // Main content area - split into 3 columns
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(64), // Album art + progress (original size)
            Constraint::Length(46), // Metadata (increased by 4 for breathing room)
            Constraint::Min(26),    // Lyrics (reduced by 4)
        ])
        .split(content_area);

    // Left column - Album art and progress bar
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(32), // Album art (30 + 2 border)
            Constraint::Length(1),  // Progress bar
        ])
        .split(main_chunks[0]);

    // Album art
    let album_art_block = Block::default()
        // .title("Album Art")
        .borders(Borders::NONE)
        .style(Style::default().fg(Color::Cyan));

    // Render album art using custom widget
    if let Some(ref ratatui_img) = app.album_art_ratatui {
        // First render the block
        f.render_widget(album_art_block, left_chunks[0]);

        // Then render the image inside the block (accounting for borders)
        let inner_area = left_chunks[0];
        let image_area = ratatui::layout::Rect {
            x: inner_area.x + 1,
            y: inner_area.y + 1,
            width: inner_area.width.saturating_sub(2),
            height: inner_area.height.saturating_sub(2),
        };

        // Render each row of pixels
        for (y, row) in ratatui_img.pixels.iter().enumerate() {
            if y >= image_area.height as usize {
                break;
            }

            let mut spans = Vec::new();
            for (x, &(r, g, b)) in row.iter().enumerate() {
                if x * 2 >= image_area.width as usize {
                    break;
                }
                // Use two spaces to create a square pixel
                spans.push(Span::styled("  ", Style::default().bg(Color::Rgb(r, g, b))));
            }

            let line = Line::from(spans);
            let paragraph = Paragraph::new(vec![line]);
            let line_area = ratatui::layout::Rect {
                x: image_area.x,
                y: image_area.y + y as u16,
                width: image_area.width,
                height: 1,
            };
            f.render_widget(paragraph, line_area);
        }
    } else {
        let no_art_widget = Paragraph::new("No album art available")
            .block(album_art_block)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(no_art_widget, left_chunks[0]);
    }

    // Progress bar
    let progress_label = if let (Some(position), Some(length)) =
        (app.player_metadata.position, app.player_metadata.length)
    {
        let time_label = format!(
            "{} / {}",
            format_duration(position.as_millis() as i64),
            format_duration(length.as_millis() as i64)
        );

        // Add pause indicator if paused
        if app.player_metadata.status.as_deref() == Some("Paused") {
            format!("⏸  {time_label}")
        } else {
            time_label
        }
    } else {
        String::from("00:00 / 00:00")
    };

    let progress_percent = app.progress.clamp(0.0, 100.0) as u16;

    // Use interpolated color from extracted palette
    let progress_color = if let Some(ref palette) = app.color_palette {
        interpolate_color(&palette.progress_colors, app.progress)
    } else {
        Color::Cyan
    };

    let progress = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(Style::default().fg(progress_color).bg(Color::Black))
        .percent(progress_percent)
        .label(progress_label);

    // Create a smaller area for the progress bar with 1 char padding on each side
    let progress_area = ratatui::layout::Rect {
        x: left_chunks[1].x + 1,
        y: left_chunks[1].y,
        width: left_chunks[1].width.saturating_sub(4), // Remove 1 from each side
        height: left_chunks[1].height,
    };

    f.render_widget(progress, progress_area);

    // Middle column - Metadata
    let metadata_block = Block::default()
        // .title("Album Info")
        .borders(Borders::NONE)
        .style(Style::default().fg(Color::Yellow));

    let metadata_text = if let Some(ref album) = app.album_metadata {
        format_album_metadata(album, &app.player_metadata, app.color_palette.as_ref())
    } else {
        // Show playerctl metadata when Tidal API fails
        format_playerctl_metadata(&app.player_metadata)
    };

    let metadata_widget = Paragraph::new(metadata_text)
        .block(metadata_block)
        .wrap(Wrap { trim: true });

    f.render_widget(metadata_widget, main_chunks[1]);

    // Right column - Lyrics
    // let lyrics_title = if app.lyrics_loading {
    //     "Lyrics (Loading...)"
    // } else {
    //     "Lyrics"
    // };

    let lyrics_block = Block::default()
        // .title(lyrics_title)
        .borders(Borders::NONE)
        .style(Style::default().fg(Color::Green));

    // Prepare lyrics content
    let lyrics_content = if let Some(ref lyrics) = app.lyrics {
        // Add empty line at the top for spacing
        let mut lines: Vec<Line> = vec![Line::from("")];

        // Calculate current line based on position
        let current_line_idx = if lyrics.is_synced {
            if let Some(position) = app.player_metadata.position {
                parser::find_current_line(lyrics, position.as_millis() as u64)
            } else {
                None
            }
        } else {
            None
        };

        // Format lyrics with highlighting
        for (idx, line) in lyrics.lines.iter().enumerate() {
            let is_current = current_line_idx == Some(idx);
            let style = if is_current {
                // Use color from palette if available
                if let Some(ref palette) = app.color_palette {
                    if let Some(&(r, g, b)) = palette.info_colors.first() {
                        Style::default()
                            .fg(Color::Rgb(r, g, b))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    }
                } else {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                }
            } else {
                Style::default().fg(Color::White)
            };

            lines.push(Line::from(vec![
                Span::raw("  "), // Add left padding to lyrics
                Span::styled(line.text.clone(), style),
            ]));
        }
        lines
    } else if app.lyrics_loading {
        vec![
            Line::from(""), // Empty line for spacing
            Line::from(vec![
                Span::raw("  "), // Add left padding
                Span::styled("Fetching lyrics...", Style::default().fg(Color::DarkGray)),
            ]),
        ]
    } else {
        vec![
            Line::from(""), // Empty line for spacing
            Line::from(vec![
                Span::raw("  "), // Add left padding
                Span::styled("No lyrics available", Style::default().fg(Color::DarkGray)),
            ]),
        ]
    };

    // Calculate scroll offset to center current line
    let visible_height = main_chunks[2].height.saturating_sub(2) as usize; // subtract top padding
    let scroll_offset = if let Some(ref lyrics) = app.lyrics {
        if lyrics.is_synced {
            if let Some(current_idx) = parser::find_current_line(
                lyrics,
                app.player_metadata
                    .position
                    .map(|p| p.as_millis() as u64)
                    .unwrap_or(0),
            ) {
                // Account for the empty line at the top (current_idx + 1)
                let adjusted_idx = current_idx + 1;

                // Only scroll when current line is 3/4 down the visible area
                // This keeps lyrics more stable and only scrolls when necessary
                let three_quarters_down = (visible_height * 3) / 4;
                if adjusted_idx > three_quarters_down {
                    // Ensure we never scroll past 0 (which would show content above lyrics)
                    adjusted_idx
                        .saturating_sub(three_quarters_down)
                        .min(lyrics.lines.len().saturating_sub(visible_height))
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    };

    let lyrics_widget = Paragraph::new(lyrics_content)
        .block(lyrics_block)
        .wrap(Wrap { trim: true })
        .scroll((scroll_offset as u16, 0));

    f.render_widget(lyrics_widget, main_chunks[2]);
}

fn format_album_metadata(
    album: &AlbumMetadata,
    player: &PlayerMetadata,
    color_palette: Option<&ColorPalette>,
) -> Vec<Line<'static>> {
    let mut lines = vec![];

    // Add empty line for spacing since we removed the title
    lines.push(Line::from(""));

    // Get info colors from palette or use defaults
    let colors = if let Some(palette) = color_palette {
        palette
            .info_colors
            .iter()
            .map(|&(r, g, b)| Color::Rgb(r, g, b))
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    // Helper to get color by index or fall back to default
    let get_color =
        |index: usize, default: Color| -> Color { colors.get(index).copied().unwrap_or(default) };

    // Left padding for all lines
    let padding = "  ";

    // Fixed width for labels (14 chars should be enough for "Popularity")
    let label_width = 14;

    // Artist (color 0)
    let artist_name = if album.all_artists() == "Unknown Artist" && !player.artist.is_empty() {
        // Use playerctl artist as fallback
        player.artist.clone()
    } else {
        album.all_artists()
    };

    // Name (Album title) - use "Name" instead of "Album"
    lines.push(Line::from(vec![
        Span::raw(padding),
        Span::styled(
            format!("{:<width$}", "Name", width = label_width),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            album.title.clone(),
            Style::default().fg(get_color(1, Color::White)),
        ),
    ]));

    // Artist
    lines.push(Line::from(vec![
        Span::raw(padding),
        Span::styled(
            format!("{:<width$}", "Artist", width = label_width),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            artist_name,
            Style::default()
                .fg(get_color(0, Color::White))
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // Release Date
    if let Some(date) = &album.release_date {
        lines.push(Line::from(vec![
            Span::raw(padding),
            Span::styled(
                format!("{:<width$}", "Released", width = label_width),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                date.clone(),
                Style::default().fg(get_color(2, Color::White)),
            ),
        ]));
    }

    // Track count
    if let Some(tracks) = album.number_of_tracks {
        lines.push(Line::from(vec![
            Span::raw(padding),
            Span::styled(
                format!("{:<width$}", "Tracks", width = label_width),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                tracks.to_string(),
                Style::default().fg(get_color(3, Color::White)),
            ),
        ]));
    }

    // Duration
    if let Some(duration_seconds) = album.duration {
        let duration = format_duration(duration_seconds as i64 * 1000);
        lines.push(Line::from(vec![
            Span::raw(padding),
            Span::styled(
                format!("{:<width$}", "Duration", width = label_width),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(duration, Style::default().fg(get_color(1, Color::White))),
        ]));
    }

    // Audio Quality
    if let Some(quality) = &album.audio_quality {
        lines.push(Line::from(vec![
            Span::raw(padding),
            Span::styled(
                format!("{:<width$}", "Quality", width = label_width),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                quality.clone(),
                Style::default().fg(get_color(0, Color::Cyan)),
            ),
        ]));
    }

    // Popularity
    if let Some(popularity) = album.popularity {
        let pop_percent = (popularity * 100.0) as u32;
        lines.push(Line::from(vec![
            Span::raw(padding),
            Span::styled(
                format!("{:<width$}", "Popularity", width = label_width),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{pop_percent}%"),
                Style::default().fg(get_color(1, Color::Green)),
            ),
        ]));
    }

    // Copyright (use muted color)
    if let Some(copyright) = &album.copyright {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw(padding),
            Span::styled("Copyright: ", Style::default().fg(Color::DarkGray)),
            Span::styled(copyright.clone(), Style::default().fg(Color::DarkGray)),
        ]));
    }

    lines
}

fn format_duration(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}

fn interpolate_color(colors: &[(u8, u8, u8)], progress: f32) -> Color {
    if colors.is_empty() {
        return Color::Cyan;
    }

    if colors.len() == 1 {
        let (r, g, b) = colors[0];
        return Color::Rgb(r, g, b);
    }

    // Progress is 0-100, convert to 0-1
    let progress = progress / 100.0;

    // For 3 colors, we have 2 segments: 0-0.5 and 0.5-1.0
    let segment_size = 1.0 / (colors.len() - 1) as f32;
    let segment_index = (progress / segment_size).floor() as usize;
    let segment_index = segment_index.min(colors.len() - 2);

    let local_progress = (progress - (segment_index as f32 * segment_size)) / segment_size;

    let (r1, g1, b1) = colors[segment_index];
    let (r2, g2, b2) = colors[segment_index + 1];

    let r = (r1 as f32 * (1.0 - local_progress) + r2 as f32 * local_progress) as u8;
    let g = (g1 as f32 * (1.0 - local_progress) + g2 as f32 * local_progress) as u8;
    let b = (b1 as f32 * (1.0 - local_progress) + b2 as f32 * local_progress) as u8;

    Color::Rgb(r, g, b)
}

fn format_playerctl_metadata(player: &PlayerMetadata) -> Vec<Line<'static>> {
    let mut lines = vec![];

    // Add empty line for spacing since we removed the title
    lines.push(Line::from(""));

    let padding = "  "; // Left padding for all lines

    // Artist
    lines.push(Line::from(vec![
        Span::raw(padding),
        Span::styled("Artist: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            player.artist.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // Title
    lines.push(Line::from(vec![
        Span::raw(padding),
        Span::styled("Title: ", Style::default().fg(Color::DarkGray)),
        Span::styled(player.title.clone(), Style::default().fg(Color::White)),
    ]));

    // Album (if available)
    if let Some(ref album) = player.album {
        lines.push(Line::from(vec![
            Span::raw(padding),
            Span::styled("Album: ", Style::default().fg(Color::DarkGray)),
            Span::styled(album.clone(), Style::default().fg(Color::White)),
        ]));
    }

    // Duration
    if let Some(length) = player.length {
        let duration = format_duration(length.as_millis() as i64);
        lines.push(Line::from(vec![
            Span::raw(padding),
            Span::styled("Duration: ", Style::default().fg(Color::DarkGray)),
            Span::styled(duration, Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(""));

    // Show source if available
    if let Some(ref source) = player.streaming_source {
        lines.push(Line::from(vec![
            Span::raw(padding),
            Span::styled("Source: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Streaming from {source}"),
                Style::default().fg(Color::Cyan),
            ),
        ]));
    }

    lines
}

pub fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok(terminal)
}

pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;
    terminal.show_cursor()?;
    Ok(())
}
