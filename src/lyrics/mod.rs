pub mod api;
pub mod cache;
pub mod parser;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsResponse {
    pub id: u64,
    pub name: String,
    #[serde(rename = "trackName")]
    pub track_name: String,
    #[serde(rename = "artistName")]
    pub artist_name: String,
    #[serde(rename = "albumName")]
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    pub instrumental: bool,
    #[serde(rename = "plainLyrics")]
    pub plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedLyrics {
    pub lines: Vec<LyricLine>,
    pub is_synced: bool,
}

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub timestamp_ms: Option<u64>, // milliseconds
    pub text: String,
}

impl LyricsResponse {
    pub fn has_synced_lyrics(&self) -> bool {
        self.synced_lyrics.is_some() && !self.instrumental
    }

    pub fn get_best_lyrics(&self) -> Option<&str> {
        if let Some(synced) = &self.synced_lyrics {
            Some(synced)
        } else {
            self.plain_lyrics.as_deref()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_synced_lyrics() {
        // Has synced lyrics and not instrumental
        let response = LyricsResponse {
            id: 1,
            name: "Test Song".to_string(),
            track_name: "Test Song".to_string(),
            artist_name: "Test Artist".to_string(),
            album_name: Some("Test Album".to_string()),
            duration: Some(180.0),
            instrumental: false,
            plain_lyrics: Some("Plain lyrics".to_string()),
            synced_lyrics: Some("[00:00.00] Synced lyrics".to_string()),
        };
        assert!(response.has_synced_lyrics());

        // No synced lyrics
        let response = LyricsResponse {
            id: 2,
            name: "Test Song 2".to_string(),
            track_name: "Test Song 2".to_string(),
            artist_name: "Test Artist".to_string(),
            album_name: None,
            duration: Some(200.0),
            instrumental: false,
            plain_lyrics: Some("Plain lyrics".to_string()),
            synced_lyrics: None,
        };
        assert!(!response.has_synced_lyrics());

        // Instrumental (even with synced lyrics)
        let response = LyricsResponse {
            id: 3,
            name: "Instrumental".to_string(),
            track_name: "Instrumental".to_string(),
            artist_name: "Test Artist".to_string(),
            album_name: None,
            duration: Some(240.0),
            instrumental: true,
            plain_lyrics: None,
            synced_lyrics: Some("[00:00.00] Should not show".to_string()),
        };
        assert!(!response.has_synced_lyrics());
    }

    #[test]
    fn test_get_best_lyrics() {
        // Prefers synced lyrics
        let response = LyricsResponse {
            id: 1,
            name: "Test Song".to_string(),
            track_name: "Test Song".to_string(),
            artist_name: "Test Artist".to_string(),
            album_name: None,
            duration: Some(180.0),
            instrumental: false,
            plain_lyrics: Some("Plain lyrics".to_string()),
            synced_lyrics: Some("[00:00.00] Synced lyrics".to_string()),
        };
        assert_eq!(response.get_best_lyrics(), Some("[00:00.00] Synced lyrics"));

        // Falls back to plain lyrics
        let response = LyricsResponse {
            id: 2,
            name: "Test Song 2".to_string(),
            track_name: "Test Song 2".to_string(),
            artist_name: "Test Artist".to_string(),
            album_name: None,
            duration: Some(200.0),
            instrumental: false,
            plain_lyrics: Some("Plain lyrics only".to_string()),
            synced_lyrics: None,
        };
        assert_eq!(response.get_best_lyrics(), Some("Plain lyrics only"));

        // No lyrics at all
        let response = LyricsResponse {
            id: 3,
            name: "No Lyrics".to_string(),
            track_name: "No Lyrics".to_string(),
            artist_name: "Test Artist".to_string(),
            album_name: None,
            duration: Some(150.0),
            instrumental: false,
            plain_lyrics: None,
            synced_lyrics: None,
        };
        assert_eq!(response.get_best_lyrics(), None);
    }

    #[test]
    fn test_lyrics_response_creation() {
        let response = LyricsResponse {
            id: 12345,
            name: "Amazing Song".to_string(),
            track_name: "Amazing Song".to_string(),
            artist_name: "Great Artist".to_string(),
            album_name: Some("Best Album".to_string()),
            duration: Some(215.5),
            instrumental: false,
            plain_lyrics: Some("Verse 1\nChorus\nVerse 2".to_string()),
            synced_lyrics: Some("[00:00.00] Verse 1\n[00:30.00] Chorus".to_string()),
        };

        assert_eq!(response.id, 12345);
        assert_eq!(response.name, "Amazing Song");
        assert_eq!(response.track_name, "Amazing Song");
        assert_eq!(response.artist_name, "Great Artist");
        assert_eq!(response.album_name, Some("Best Album".to_string()));
        assert_eq!(response.duration, Some(215.5));
        assert!(!response.instrumental);
        assert!(response.plain_lyrics.is_some());
        assert!(response.synced_lyrics.is_some());
    }

    #[test]
    fn test_parsed_lyrics_creation() {
        let lyrics = ParsedLyrics {
            lines: vec![
                LyricLine {
                    timestamp_ms: Some(0),
                    text: "First line".to_string(),
                },
                LyricLine {
                    timestamp_ms: Some(5000),
                    text: "Second line".to_string(),
                },
                LyricLine {
                    timestamp_ms: None,
                    text: "Unsynced line".to_string(),
                },
            ],
            is_synced: true,
        };

        assert_eq!(lyrics.lines.len(), 3);
        assert!(lyrics.is_synced);
        assert_eq!(lyrics.lines[0].text, "First line");
        assert_eq!(lyrics.lines[0].timestamp_ms, Some(0));
        assert_eq!(lyrics.lines[2].timestamp_ms, None);
    }
}
