use anyhow::Result;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use trackwatch::{
    colors::ColorPalette,
    config::Config,
    display::{DisplayFormatter, PixelatedImage, RatatuiImage},
    lyrics::{api::LrcLibClient, cache::LyricsCache, parser},
    models::AlbumMetadata,
    player::{self, PlayerMetadata},
    providers::{tidal::TidalProvider, MusicProvider},
    ui::{self, App},
};

const IMAGE_SIZE: u32 = 30; // 30x30 pixels like the Go version

fn main() -> Result<()> {
    // Load .env file if it exists
    dotenv::dotenv().ok();

    // Check if playerctl is installed first
    if !player::is_playerctl_installed() {
        println!("playerctl is not installed. Please install playerctl to use this application.");
        return Ok(());
    }

    // Load configuration
    let config = Config::from_env();

    // Setup terminal
    let mut terminal = ui::setup_terminal()?;

    // Create app state
    let app = Arc::new(Mutex::new(App::new()));
    let app_clone = Arc::clone(&app);

    // Spawn background thread for fetching player data
    thread::spawn(move || {
        let mut provider = if config.has_tidal_credentials() {
            Some(TidalProvider::new(
                config.tidal_client_id.clone().unwrap(),
                config.tidal_client_secret.clone().unwrap(),
            ))
        } else {
            None
        };
        let formatter = DisplayFormatter::new(IMAGE_SIZE);

        // Initialize lyrics components
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let lyrics_client = LrcLibClient::new();
        let lyrics_cache = LyricsCache::new().unwrap();

        let mut last_track: Option<PlayerMetadata> = None;
        let mut cached_album_metadata: Option<AlbumMetadata> = None;
        let mut cached_album_art: Option<PixelatedImage> = None;
        let mut cached_album_art_ratatui: Option<RatatuiImage> = None;
        let mut cached_color_palette: Option<ColorPalette> = None;
        let mut last_position: Option<Duration> = None;

        loop {
            // Get current track metadata from playerctl
            match player::get_current_track() {
                Ok(player_metadata) => {
                    // Check if track changed
                    let track_changed = match &last_track {
                        None => true,
                        Some(last) => {
                            last.artist != player_metadata.artist
                                || last.title != player_metadata.title
                                || last.album != player_metadata.album
                        }
                    };

                    if track_changed {
                        // Try to get album metadata from Tidal if available
                        cached_album_metadata = if let (Some(album), Some(ref mut provider)) = 
                            (player_metadata.album.as_ref(), provider.as_mut()) {
                            match provider.get_album_metadata(&player_metadata.artist, album) {
                                Ok(metadata) => {
                                    // Also fetch album art
                                    if let Some(ref cover_url) = metadata.cover_url {
                                        match formatter.fetch_and_process_all_formats(cover_url) {
                                            Ok((pixelated, ratatui, colors)) => {
                                                cached_album_art = Some(pixelated);
                                                cached_album_art_ratatui = Some(ratatui);
                                                cached_color_palette = Some(colors);
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to fetch album art: {e}");
                                                cached_album_art = None;
                                                cached_album_art_ratatui = None;
                                                cached_color_palette = None;
                                            }
                                        }
                                    }
                                    Some(metadata)
                                }
                                Err(_) => {
                                    // Silently fail - we'll use playerctl data
                                    // Try to use playerctl album art as fallback
                                    if let Some(ref art_url) = player_metadata.art_url {
                                        match formatter.fetch_and_process_all_formats(art_url) {
                                            Ok((pixelated, ratatui, colors)) => {
                                                cached_album_art = Some(pixelated);
                                                cached_album_art_ratatui = Some(ratatui);
                                                cached_color_palette = Some(colors);
                                            }
                                            Err(e) => {
                                                eprintln!(
                                                    "Failed to fetch playerctl album art: {e}"
                                                );
                                                cached_album_art = None;
                                                cached_album_art_ratatui = None;
                                                cached_color_palette = None;
                                            }
                                        }
                                    } else {
                                        cached_album_art = None;
                                        cached_album_art_ratatui = None;
                                        cached_color_palette = None;
                                    }

                                    None
                                }
                            }
                        } else {
                            // No album info from playerctl, but try to get art anyway
                            if let Some(ref art_url) = player_metadata.art_url {
                                match formatter.fetch_and_process_all_formats(art_url) {
                                    Ok((pixelated, ratatui, colors)) => {
                                        cached_album_art = Some(pixelated);
                                        cached_album_art_ratatui = Some(ratatui);
                                        cached_color_palette = Some(colors);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to fetch playerctl album art: {e}");
                                        cached_album_art = None;
                                        cached_album_art_ratatui = None;
                                        cached_color_palette = None;
                                    }
                                }
                            } else {
                                cached_album_art = None;
                                cached_album_art_ratatui = None;
                                cached_color_palette = None;
                            }
                            None
                        };

                        // Fetch lyrics for the new track
                        let app_for_loading = Arc::clone(&app_clone);
                        let artist = player_metadata.artist.clone();
                        let title = player_metadata.title.clone();
                        let lyrics_client_clone = lyrics_client.clone();
                        let lyrics_cache_clone = lyrics_cache.clone();

                        // Set loading state
                        if let Ok(mut app) = app_for_loading.lock() {
                            app.set_lyrics_loading(true);
                        }

                        // Spawn async task for lyrics fetching
                        runtime.spawn(async move {
                            // Check cache first
                            if let Some(cached_response) = lyrics_cache_clone.get(&artist, &title) {
                                if let Some(response) = cached_response {
                                    if let Some(lyrics_text) = response.get_best_lyrics() {
                                        let parsed = parser::parse_lrc(lyrics_text);
                                        if let Ok(mut app) = app_for_loading.lock() {
                                            app.update_lyrics(Some(parsed));
                                        }
                                    }
                                }
                                return;
                            }

                            // Fetch from API
                            match lyrics_client_clone.get_best_match(&title, &artist).await {
                                Ok(Some(response)) => {
                                    // Cache the response
                                    let _ =
                                        lyrics_cache_clone.set(&artist, &title, Some(&response));

                                    if let Some(lyrics_text) = response.get_best_lyrics() {
                                        let parsed = parser::parse_lrc(lyrics_text);
                                        if let Ok(mut app) = app_for_loading.lock() {
                                            app.update_lyrics(Some(parsed));
                                        }
                                    }
                                }
                                Ok(None) => {
                                    // Cache "not found" result
                                    let _ = lyrics_cache_clone.set(&artist, &title, None);
                                    if let Ok(mut app) = app_for_loading.lock() {
                                        app.update_lyrics(None);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to fetch lyrics: {e}");
                                    if let Ok(mut app) = app_for_loading.lock() {
                                        app.update_lyrics(None);
                                    }
                                }
                            }
                        });
                    }

                    // Handle position and progress based on play state
                    let (position, progress) =
                        if player_metadata.status.as_deref() == Some("Playing") {
                            // Update position and calculate progress when playing
                            last_position = player_metadata.position;
                            (
                                player_metadata.position,
                                player_metadata.get_progress_percentage().unwrap_or(0.0),
                            )
                        } else {
                            // When paused, use last known position
                            let frozen_position = last_position.or(player_metadata.position);
                            let frozen_progress = if let (Some(pos), Some(len)) =
                                (frozen_position, player_metadata.length)
                            {
                                if len.as_secs() > 0 {
                                    (pos.as_secs_f32() / len.as_secs_f32()) * 100.0
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            };
                            (frozen_position, frozen_progress)
                        };

                    // Update app state with potentially frozen position
                    if let Ok(mut app) = app_clone.lock() {
                        let mut metadata_with_position = player_metadata.clone();
                        metadata_with_position.position = position;

                        app.update_metadata(
                            cached_album_art.clone(),
                            cached_album_art_ratatui.clone(),
                            cached_album_metadata.clone(),
                            metadata_with_position,
                            progress,
                            cached_color_palette.clone(),
                        );
                    }

                    last_track = Some(player_metadata);
                }
                Err(_) => {
                    if last_track.is_some() {
                        // Clear app state when player stops
                        if let Ok(mut app) = app_clone.lock() {
                            app.waiting_for_player = true;
                            app.album_art = None;
                            app.album_art_ratatui = None;
                            app.album_metadata = None;
                            app.player_metadata = PlayerMetadata {
                                artist: String::new(),
                                title: String::new(),
                                album: None,
                                position: None,
                                length: None,
                                streaming_source: None,
                                art_url: None,
                                status: None,
                            };
                            app.progress = 0.0;
                            app.color_palette = None;
                            app.lyrics = None;
                        }
                        last_track = None;
                        cached_album_metadata = None;
                        cached_album_art = None;
                        cached_album_art_ratatui = None;
                        cached_color_palette = None;
                        last_position = None;
                    }
                    // Don't print error message - it's normal when no player is running
                }
            }

            // Sleep for 500ms before next update
            thread::sleep(Duration::from_millis(500));
        }
    });

    // Run the UI
    let res = ui::run_app(&mut terminal, app);

    // Restore terminal
    ui::restore_terminal(&mut terminal)?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}
