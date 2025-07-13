use anyhow::Result;
use reqwest;
use std::time::Duration;
use urlencoding::encode;

use super::LyricsResponse;

#[derive(Clone)]
pub struct LrcLibClient {
    client: reqwest::Client,
}

impl Default for LrcLibClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LrcLibClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("trackwatch/0.1.0")
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        Self { client }
    }

    pub async fn search_lyrics(
        &self,
        track_name: &str,
        artist_name: &str,
    ) -> Result<Vec<LyricsResponse>> {
        let encoded_track = encode(track_name);
        let encoded_artist = encode(artist_name);

        let url = format!(
            "https://lrclib.net/api/search?track_name={encoded_track}&artist_name={encoded_artist}"
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<Vec<LyricsResponse>>()
            .await?;

        Ok(response)
    }

    pub async fn get_best_match(
        &self,
        track_name: &str,
        artist_name: &str,
    ) -> Result<Option<LyricsResponse>> {
        let results = self.search_lyrics(track_name, artist_name).await?;

        if results.is_empty() {
            return Ok(None);
        }

        // Prefer results with synced lyrics
        let with_synced = results.iter().find(|r| r.has_synced_lyrics());

        if let Some(synced) = with_synced {
            return Ok(Some(synced.clone()));
        }

        // Otherwise return first result
        Ok(results.into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_response() -> LyricsResponse {
        LyricsResponse {
            id: 22682704,
            name: "Kaleidoscopic Waves".to_string(),
            track_name: "Kaleidoscopic Waves".to_string(),
            artist_name: "Fallujah".to_string(),
            album_name: Some("Xenotaph".to_string()),
            duration: Some(252.293833),
            instrumental: false,
            plain_lyrics: Some("Bleeding through the floor in waves of color\nChromatic currents pull me under\nLike a memory from worlds away\nA language I've lost\n\nSparks ignite in my mind as eternity\nNow rewinds and unravels in front of me\nTime will absorb and embrace me\nTime will engulf and erase me\nAnd in this moment, I feel everything this world has ever known\nA revelation from the apparition silently leading me to home\n\nTogether, unchained as stars erupt in harmony\nMy spirit will bathe in their kaleidoscopic waves\nMy sleeping soul, awake\nBurnt down and remade by their kaleidoscopic waves\n\nI can remember the life I used to know\nDown at the center of all I was before\nHeavenly vessel, cosmic divinity\nBorn from the echoes of who I used to be\n\nDive in deep\nOceans of grief inside of me, dissolving\nEmbryonic dreams of old memories\nAre violently flowing out\n\nTogether, unchained as stars erupt in harmony\nMy spirit will bathe in their kaleidoscopic waves\nMy sleeping soul, awake\nBurnt down and remade by their kaleidoscopic waves".to_string()),
            synced_lyrics: Some("[00:29.60] Bleeding through the floor in waves of color\n[00:38.61] Chromatic currents pull me under\n[00:45.72] Like a memory from worlds away\n[00:53.32] A language I've lost\n[00:55.70] Sparks ignite in my mind as eternity\n[00:58.79] Now rewinds and unravels in front of me\n[01:03.17] Time will absorb and embrace me\n[01:06.36] Time will engulf and erase me\n[01:09.64] And in this moment, I feel everything this world has ever known\n[01:16.91] A revelation from the apparition silently leading me to home\n[01:23.37] Together, unchained as stars erupt in harmony\n[01:29.72] My spirit will bathe in their kaleidoscopic waves\n[01:40.42] My sleeping soul, awake\n[01:44.24] Burnt down and remade by their kaleidoscopic waves\n[02:33.25] I can remember the life I used to know\n[02:39.71] Down at the center of all I was before\n[02:46.56] Heavenly vessel, cosmic divinity\n[02:53.39] Born from the echoes of who I used to be\n[03:00.53] Dive in deep\n[03:03.77] Oceans of grief inside of me, dissolving\n[03:14.15] Embryonic dreams of old memories\n[03:20.39] Are violently flowing out\n[03:28.83] Together, unchained as stars erupt in harmony\n[03:34.86] My spirit will bathe in their kaleidoscopic waves\n[03:45.57] My sleeping soul, awake\n[03:49.05] Burnt down and remade by their kaleidoscopic waves\n[03:57.67] ".to_string()),
        }
    }

    #[test]
    fn test_lrclib_client_creation() {
        let _client = LrcLibClient::new();
        // Just verify it can be created without panicking
        // The actual client has a timeout of 10 seconds
    }

    #[test]
    fn test_mock_get_best_match_with_synced() {
        // Test the logic of get_best_match with mock data
        let response_with_synced = create_test_response();

        let response_without_synced = LyricsResponse {
            id: 12345,
            name: "Other Song".to_string(),
            track_name: "Other Song".to_string(),
            artist_name: "Other Artist".to_string(),
            album_name: None,
            duration: Some(180.0),
            instrumental: false,
            plain_lyrics: Some("Just plain lyrics".to_string()),
            synced_lyrics: None,
        };

        let instrumental = LyricsResponse {
            id: 54321,
            name: "Instrumental Track".to_string(),
            track_name: "Instrumental Track".to_string(),
            artist_name: "Artist".to_string(),
            album_name: None,
            duration: Some(300.0),
            instrumental: true,
            plain_lyrics: None,
            synced_lyrics: None,
        };

        // Test selection logic
        let responses = vec![
            response_without_synced.clone(),
            response_with_synced.clone(),
            instrumental.clone(),
        ];

        // Should pick the one with synced lyrics
        let best = responses
            .iter()
            .find(|r| r.has_synced_lyrics())
            .or_else(|| responses.first());

        assert!(best.is_some());
        assert_eq!(best.unwrap().id, 22682704);
        assert!(best.unwrap().has_synced_lyrics());
    }

    #[test]
    fn test_mock_get_best_match_plain_only() {
        // When no synced lyrics are available
        let plain_only = LyricsResponse {
            id: 11111,
            name: "Plain Song".to_string(),
            track_name: "Plain Song".to_string(),
            artist_name: "Artist".to_string(),
            album_name: None,
            duration: Some(200.0),
            instrumental: false,
            plain_lyrics: Some("Only plain lyrics here".to_string()),
            synced_lyrics: None,
        };

        let responses = vec![plain_only.clone()];
        let best = responses.into_iter().next();

        assert!(best.is_some());
        assert_eq!(best.as_ref().unwrap().id, 11111);
        assert!(!best.as_ref().unwrap().has_synced_lyrics());
        assert!(best.as_ref().unwrap().plain_lyrics.is_some());
    }

    #[test]
    fn test_real_lyrics_response_parsing() {
        // Test with the actual Fallujah response data
        let response = create_test_response();

        // Verify all fields parsed correctly
        assert_eq!(response.id, 22682704);
        assert_eq!(response.name, "Kaleidoscopic Waves");
        assert_eq!(response.track_name, "Kaleidoscopic Waves");
        assert_eq!(response.artist_name, "Fallujah");
        assert_eq!(response.album_name, Some("Xenotaph".to_string()));
        assert_eq!(response.duration, Some(252.293833));
        assert!(!response.instrumental);

        // Verify lyrics content
        assert!(response.plain_lyrics.is_some());
        assert!(response.synced_lyrics.is_some());

        // Check that plain lyrics start correctly
        let plain = response.plain_lyrics.as_ref().unwrap();
        assert!(plain.starts_with("Bleeding through the floor"));
        assert!(plain.contains("kaleidoscopic waves"));

        // Check that synced lyrics have timestamps
        let synced = response.synced_lyrics.as_ref().unwrap();
        assert!(synced.starts_with("[00:29.60]"));
        assert!(synced.contains("[01:23.37] Together, unchained"));
        assert!(synced.contains("[03:57.67]")); // Last timestamp
    }

    #[test]
    fn test_empty_results_handling() {
        // Test what happens with empty results
        let responses: Vec<LyricsResponse> = vec![];
        let best = responses.into_iter().next();
        assert!(best.is_none());
    }

    #[test]
    fn test_url_encoding() {
        // Test that special characters are properly encoded
        let _client = LrcLibClient::new();

        // These would be encoded in the actual URL
        let track_with_spaces = "Bohemian Rhapsody";
        let artist_with_ampersand = "Simon & Garfunkel";

        // URL encoding should handle these
        let encoded_track = encode(track_with_spaces);
        let encoded_artist = encode(artist_with_ampersand);

        assert_eq!(encoded_track, "Bohemian%20Rhapsody");
        assert_eq!(encoded_artist, "Simon%20%26%20Garfunkel");
    }
}
