use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumMetadata {
    pub id: String,
    pub title: String,
    pub artists: Vec<ArtistInfo>,
    pub album_type: Option<String>, // "album", "single", "compilation"
    pub release_date: Option<String>, // YYYY-MM-DD
    pub number_of_tracks: Option<u32>,
    pub duration: Option<u32>,         // total duration in seconds
    pub audio_quality: Option<String>, // "LOSSLESS", "HIRES_LOSSLESS", "MQA"
    pub popularity: Option<f64>,       // 0.0-1.0
    pub copyright: Option<String>,
    pub cover_url: Option<String>, // Direct URL to album art
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistInfo {
    pub id: String,
    pub name: String,
}

impl AlbumMetadata {
    /// Parse ISO 8601 duration string (PT3M45S) to seconds
    pub fn parse_iso8601_duration(iso_duration: &str) -> u32 {
        if !iso_duration.starts_with("PT") {
            return 0;
        }

        let duration_part = &iso_duration[2..]; // Remove "PT"
        let mut total_seconds = 0u32;
        let mut current_number = String::new();

        for ch in duration_part.chars() {
            if ch.is_ascii_digit() {
                current_number.push(ch);
            } else {
                if let Ok(num) = current_number.parse::<u32>() {
                    match ch {
                        'H' => total_seconds += num * 3600, // Hours
                        'M' => total_seconds += num * 60,   // Minutes
                        'S' => total_seconds += num,        // Seconds
                        _ => {}
                    }
                }
                current_number.clear();
            }
        }

        total_seconds
    }

    /// Format duration in seconds as MM:SS or H:MM:SS
    pub fn format_duration(seconds: u32) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        if hours > 0 {
            format!("{hours}:{minutes:02}:{secs:02}")
        } else {
            format!("{minutes}:{secs:02}")
        }
    }

    /// Get primary artist name
    pub fn primary_artist(&self) -> String {
        self.artists
            .first()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Unknown Artist".to_string())
    }

    /// Get all artist names joined
    pub fn all_artists(&self) -> String {
        if self.artists.is_empty() {
            "Unknown Artist".to_string()
        } else {
            self.artists
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso8601_duration() {
        // Basic durations
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT3M45S"), 225);
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT1H30M"), 5400);
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT45S"), 45);
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT1H"), 3600);
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT30M"), 1800);

        // Complex duration
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT2H15M30S"), 8130);

        // Edge cases
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT0S"), 0);
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT"), 0);
        assert_eq!(AlbumMetadata::parse_iso8601_duration(""), 0);
        assert_eq!(AlbumMetadata::parse_iso8601_duration("invalid"), 0);

        // Large values
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT99H59M59S"), 359999);
    }

    #[test]
    fn test_format_duration() {
        // MM:SS format for durations under an hour
        assert_eq!(AlbumMetadata::format_duration(0), "0:00");
        assert_eq!(AlbumMetadata::format_duration(5), "0:05");
        assert_eq!(AlbumMetadata::format_duration(59), "0:59");
        assert_eq!(AlbumMetadata::format_duration(60), "1:00");
        assert_eq!(AlbumMetadata::format_duration(225), "3:45");
        assert_eq!(AlbumMetadata::format_duration(599), "9:59");
        assert_eq!(AlbumMetadata::format_duration(3599), "59:59");

        // H:MM:SS format for durations of an hour or more
        assert_eq!(AlbumMetadata::format_duration(3600), "1:00:00");
        assert_eq!(AlbumMetadata::format_duration(3661), "1:01:01");
        assert_eq!(AlbumMetadata::format_duration(5400), "1:30:00");
        assert_eq!(AlbumMetadata::format_duration(8130), "2:15:30");
        assert_eq!(AlbumMetadata::format_duration(359999), "99:59:59");
    }

    #[test]
    fn test_primary_artist() {
        // With artists
        let album = AlbumMetadata {
            id: "1".to_string(),
            title: "Test Album".to_string(),
            artists: vec![
                ArtistInfo {
                    id: "1".to_string(),
                    name: "First Artist".to_string(),
                },
                ArtistInfo {
                    id: "2".to_string(),
                    name: "Second Artist".to_string(),
                },
            ],
            album_type: None,
            release_date: None,
            number_of_tracks: None,
            duration: None,
            audio_quality: None,
            popularity: None,
            copyright: None,
            cover_url: None,
        };
        assert_eq!(album.primary_artist(), "First Artist");

        // No artists
        let empty_album = AlbumMetadata {
            id: "2".to_string(),
            title: "Empty Album".to_string(),
            artists: vec![],
            album_type: None,
            release_date: None,
            number_of_tracks: None,
            duration: None,
            audio_quality: None,
            popularity: None,
            copyright: None,
            cover_url: None,
        };
        assert_eq!(empty_album.primary_artist(), "Unknown Artist");
    }

    #[test]
    fn test_all_artists() {
        // Single artist
        let single = AlbumMetadata {
            id: "1".to_string(),
            title: "Single Artist Album".to_string(),
            artists: vec![ArtistInfo {
                id: "1".to_string(),
                name: "Solo Artist".to_string(),
            }],
            album_type: None,
            release_date: None,
            number_of_tracks: None,
            duration: None,
            audio_quality: None,
            popularity: None,
            copyright: None,
            cover_url: None,
        };
        assert_eq!(single.all_artists(), "Solo Artist");

        // Multiple artists
        let collab = AlbumMetadata {
            id: "2".to_string(),
            title: "Collaboration Album".to_string(),
            artists: vec![
                ArtistInfo {
                    id: "1".to_string(),
                    name: "Artist One".to_string(),
                },
                ArtistInfo {
                    id: "2".to_string(),
                    name: "Artist Two".to_string(),
                },
                ArtistInfo {
                    id: "3".to_string(),
                    name: "Artist Three".to_string(),
                },
            ],
            album_type: None,
            release_date: None,
            number_of_tracks: None,
            duration: None,
            audio_quality: None,
            popularity: None,
            copyright: None,
            cover_url: None,
        };
        assert_eq!(collab.all_artists(), "Artist One, Artist Two, Artist Three");

        // No artists
        let empty = AlbumMetadata {
            id: "3".to_string(),
            title: "Empty Artists Album".to_string(),
            artists: vec![],
            album_type: None,
            release_date: None,
            number_of_tracks: None,
            duration: None,
            audio_quality: None,
            popularity: None,
            copyright: None,
            cover_url: None,
        };
        assert_eq!(empty.all_artists(), "Unknown Artist");
    }

    #[test]
    fn test_album_metadata_creation() {
        let album = AlbumMetadata {
            id: "123".to_string(),
            title: "Test Album".to_string(),
            artists: vec![ArtistInfo {
                id: "456".to_string(),
                name: "Test Artist".to_string(),
            }],
            album_type: Some("album".to_string()),
            release_date: Some("2024-01-01".to_string()),
            number_of_tracks: Some(12),
            duration: Some(2700),
            audio_quality: Some("LOSSLESS".to_string()),
            popularity: Some(0.85),
            copyright: Some("© 2024 Test Records".to_string()),
            cover_url: Some("https://example.com/cover.jpg".to_string()),
        };

        // Test that all fields are set correctly
        assert_eq!(album.id, "123");
        assert_eq!(album.title, "Test Album");
        assert_eq!(album.artists.len(), 1);
        assert_eq!(album.artists[0].name, "Test Artist");
        assert_eq!(album.album_type, Some("album".to_string()));
        assert_eq!(album.release_date, Some("2024-01-01".to_string()));
        assert_eq!(album.number_of_tracks, Some(12));
        assert_eq!(album.duration, Some(2700));
        assert_eq!(album.audio_quality, Some("LOSSLESS".to_string()));
        assert_eq!(album.popularity, Some(0.85));
        assert_eq!(album.copyright, Some("© 2024 Test Records".to_string()));
        assert_eq!(
            album.cover_url,
            Some("https://example.com/cover.jpg".to_string())
        );
    }

    #[test]
    fn test_parse_iso8601_duration_edge_cases() {
        // Invalid characters
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT3M45X"), 180); // Ignores invalid unit X, only processes 3M
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PTabc"), 0);

        // Missing PT prefix
        assert_eq!(AlbumMetadata::parse_iso8601_duration("3M45S"), 0);
        assert_eq!(AlbumMetadata::parse_iso8601_duration("T3M45S"), 0);

        // Only hours
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT24H"), 86400);

        // Multiple digits
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT100M"), 6000);
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT1000S"), 1000);
    }

    #[test]
    fn test_format_duration_edge_cases() {
        // Exactly one hour
        assert_eq!(AlbumMetadata::format_duration(3600), "1:00:00");

        // Just under one hour
        assert_eq!(AlbumMetadata::format_duration(3599), "59:59");

        // Just over one hour
        assert_eq!(AlbumMetadata::format_duration(3601), "1:00:01");

        // 24 hours
        assert_eq!(AlbumMetadata::format_duration(86400), "24:00:00");
    }

    #[test]
    fn test_artist_info_creation() {
        let artist = ArtistInfo {
            id: "789".to_string(),
            name: "Amazing Artist".to_string(),
        };

        assert_eq!(artist.id, "789");
        assert_eq!(artist.name, "Amazing Artist");
    }
}
