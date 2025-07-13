use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerMetadata {
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub position: Option<Duration>,
    pub length: Option<Duration>,
    pub streaming_source: Option<String>,
    pub art_url: Option<String>,
    pub status: Option<String>, // "Playing", "Paused", "Stopped"
}

impl PlayerMetadata {
    pub fn get_progress_percentage(&self) -> Option<f32> {
        match (self.position, self.length) {
            (Some(pos), Some(len)) if len.as_secs() > 0 => {
                Some((pos.as_secs_f32() / len.as_secs_f32()) * 100.0)
            }
            _ => None,
        }
    }
}

pub fn get_current_track() -> Result<PlayerMetadata> {
    // Get metadata from playerctl
    let artist = get_playerctl_property("artist")?;
    let title = get_playerctl_property("title")?;
    let album = get_playerctl_property("album").ok();

    // Get position and length
    let position = get_playerctl_position().ok();
    let length = get_playerctl_length().ok();

    // Get streaming source from URL
    let streaming_source = get_playerctl_property("url")
        .ok()
        .and_then(|url| detect_streaming_source(&url));

    // Get album art URL
    let art_url = get_playerctl_mpris_property("artUrl").ok();

    // Get player status
    let status = get_player_status().ok();

    Ok(PlayerMetadata {
        artist,
        title,
        album,
        position,
        length,
        streaming_source,
        art_url,
        status,
    })
}

fn detect_streaming_source(url: &str) -> Option<String> {
    if url.contains("tidal.com") {
        Some("Tidal".to_string())
    } else if url.contains("youtube.com") || url.contains("youtu.be") {
        Some("YouTube".to_string())
    } else if url.contains("spotify.com") {
        Some("Spotify".to_string())
    } else if url.contains("soundcloud.com") {
        Some("SoundCloud".to_string())
    } else if url.contains("deezer.com") {
        Some("Deezer".to_string())
    } else if url.contains("music.apple.com") {
        Some("Apple Music".to_string())
    } else if url.contains("bandcamp.com") {
        Some("Bandcamp".to_string())
    } else if url.starts_with("file://") {
        Some("Local File".to_string())
    } else if !url.is_empty() {
        Some("Web".to_string())
    } else {
        None
    }
}

fn get_playerctl_property(property: &str) -> Result<String> {
    let output = Command::new("playerctl")
        .args(["metadata", &format!("xesam:{property}")])
        .output()?;

    if !output.status.success() {
        return Err(anyhow!("playerctl failed to get {}", property));
    }

    let value = String::from_utf8(output.stdout)?.trim().to_string();

    if value.is_empty() {
        return Err(anyhow!("No {} found", property));
    }

    Ok(value)
}

fn get_playerctl_position() -> Result<Duration> {
    let output = Command::new("playerctl").arg("position").output()?;

    if !output.status.success() {
        return Err(anyhow!("playerctl failed to get position"));
    }

    let seconds: f64 = String::from_utf8(output.stdout)?.trim().parse()?;

    Ok(Duration::from_secs_f64(seconds))
}

fn get_playerctl_length() -> Result<Duration> {
    let output = Command::new("playerctl")
        .args(["metadata", "mpris:length"])
        .output()?;

    if !output.status.success() {
        return Err(anyhow!("playerctl failed to get length"));
    }

    // mpris:length returns microseconds
    let microseconds: u64 = String::from_utf8(output.stdout)?.trim().parse()?;

    Ok(Duration::from_micros(microseconds))
}

fn get_playerctl_mpris_property(property: &str) -> Result<String> {
    let output = Command::new("playerctl")
        .args(["metadata", &format!("mpris:{property}")])
        .output()?;

    if !output.status.success() {
        return Err(anyhow!("playerctl failed to get mpris property"));
    }

    let value = String::from_utf8(output.stdout)?.trim().to_string();

    if value.is_empty() {
        return Err(anyhow!("No {} found", property));
    }

    Ok(value)
}

pub fn is_player_available() -> bool {
    Command::new("playerctl")
        .arg("status")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn is_playerctl_installed() -> bool {
    Command::new("playerctl")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn get_player_status() -> Result<String> {
    let output = Command::new("playerctl").arg("status").output()?;

    if !output.status.success() {
        return Err(anyhow!("playerctl failed to get status"));
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_progress_percentage() {
        // Normal case - 30 seconds of 120 seconds
        let metadata = PlayerMetadata {
            artist: "Test".to_string(),
            title: "Test".to_string(),
            album: None,
            position: Some(Duration::from_secs(30)),
            length: Some(Duration::from_secs(120)),
            streaming_source: None,
            art_url: None,
            status: None,
        };
        assert_eq!(metadata.get_progress_percentage(), Some(25.0));

        // At the end
        let metadata = PlayerMetadata {
            artist: "Test".to_string(),
            title: "Test".to_string(),
            album: None,
            position: Some(Duration::from_secs(120)),
            length: Some(Duration::from_secs(120)),
            streaming_source: None,
            art_url: None,
            status: None,
        };
        assert_eq!(metadata.get_progress_percentage(), Some(100.0));

        // No position
        let metadata = PlayerMetadata {
            artist: "Test".to_string(),
            title: "Test".to_string(),
            album: None,
            position: None,
            length: Some(Duration::from_secs(120)),
            streaming_source: None,
            art_url: None,
            status: None,
        };
        assert_eq!(metadata.get_progress_percentage(), None);

        // No length
        let metadata = PlayerMetadata {
            artist: "Test".to_string(),
            title: "Test".to_string(),
            album: None,
            position: Some(Duration::from_secs(30)),
            length: None,
            streaming_source: None,
            art_url: None,
            status: None,
        };
        assert_eq!(metadata.get_progress_percentage(), None);

        // Zero length (should return None to avoid division by zero)
        let metadata = PlayerMetadata {
            artist: "Test".to_string(),
            title: "Test".to_string(),
            album: None,
            position: Some(Duration::from_secs(30)),
            length: Some(Duration::from_secs(0)),
            streaming_source: None,
            art_url: None,
            status: None,
        };
        assert_eq!(metadata.get_progress_percentage(), None);

        // Fractional seconds
        let metadata = PlayerMetadata {
            artist: "Test".to_string(),
            title: "Test".to_string(),
            album: None,
            position: Some(Duration::from_millis(1500)),
            length: Some(Duration::from_millis(3000)),
            streaming_source: None,
            art_url: None,
            status: None,
        };
        assert_eq!(metadata.get_progress_percentage(), Some(50.0));
    }

    #[test]
    fn test_detect_streaming_source() {
        // Tidal
        assert_eq!(
            detect_streaming_source("https://tidal.com/track/12345"),
            Some("Tidal".to_string())
        );
        assert_eq!(
            detect_streaming_source("https://listen.tidal.com/album/98765"),
            Some("Tidal".to_string())
        );

        // YouTube
        assert_eq!(
            detect_streaming_source("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("YouTube".to_string())
        );
        assert_eq!(
            detect_streaming_source("https://youtu.be/dQw4w9WgXcQ"),
            Some("YouTube".to_string())
        );
        assert_eq!(
            detect_streaming_source("https://music.youtube.com/watch?v=abc123"),
            Some("YouTube".to_string())
        );

        // Spotify
        assert_eq!(
            detect_streaming_source("https://open.spotify.com/track/abc123"),
            Some("Spotify".to_string())
        );

        // SoundCloud
        assert_eq!(
            detect_streaming_source("https://soundcloud.com/artist/track"),
            Some("SoundCloud".to_string())
        );

        // Deezer
        assert_eq!(
            detect_streaming_source("https://www.deezer.com/track/12345"),
            Some("Deezer".to_string())
        );

        // Apple Music
        assert_eq!(
            detect_streaming_source("https://music.apple.com/us/album/song/123456"),
            Some("Apple Music".to_string())
        );

        // Bandcamp
        assert_eq!(
            detect_streaming_source("https://artist.bandcamp.com/track/song-name"),
            Some("Bandcamp".to_string())
        );

        // Local file
        assert_eq!(
            detect_streaming_source("file:///home/user/Music/song.mp3"),
            Some("Local File".to_string())
        );

        // Generic web URL
        assert_eq!(
            detect_streaming_source("https://random-music-site.com/play"),
            Some("Web".to_string())
        );

        // Empty URL
        assert_eq!(detect_streaming_source(""), None);
    }

    #[test]
    fn test_player_metadata_creation() {
        let metadata = PlayerMetadata {
            artist: "Test Artist".to_string(),
            title: "Test Song".to_string(),
            album: Some("Test Album".to_string()),
            position: Some(Duration::from_secs(60)),
            length: Some(Duration::from_secs(180)),
            streaming_source: Some("Tidal".to_string()),
            art_url: Some("https://example.com/art.jpg".to_string()),
            status: Some("Playing".to_string()),
        };

        assert_eq!(metadata.artist, "Test Artist");
        assert_eq!(metadata.title, "Test Song");
        assert_eq!(metadata.album, Some("Test Album".to_string()));
        assert_eq!(metadata.position, Some(Duration::from_secs(60)));
        assert_eq!(metadata.length, Some(Duration::from_secs(180)));
        assert_eq!(metadata.streaming_source, Some("Tidal".to_string()));
        assert_eq!(
            metadata.art_url,
            Some("https://example.com/art.jpg".to_string())
        );
        assert_eq!(metadata.status, Some("Playing".to_string()));
    }

    #[test]
    fn test_progress_percentage_precision() {
        // Test various precision cases
        let test_cases = vec![
            (1.0, 3.0, 33.333332),      // 1/3
            (2.0, 3.0, 66.66667),       // 2/3
            (1.0, 7.0, 14.285714),      // 1/7
            (0.001, 1.0, 0.1),          // Very small progress
            (999.999, 1000.0, 99.9999), // Almost complete
        ];

        for (pos, len, expected) in test_cases {
            let metadata = PlayerMetadata {
                artist: "Test".to_string(),
                title: "Test".to_string(),
                album: None,
                position: Some(Duration::from_secs_f64(pos)),
                length: Some(Duration::from_secs_f64(len)),
                streaming_source: None,
                art_url: None,
                status: None,
            };

            let progress = metadata.get_progress_percentage().unwrap();
            assert!(
                (progress - expected).abs() < 0.001,
                "Expected {expected} but got {progress} for {pos}/{len}"
            );
        }
    }
}
