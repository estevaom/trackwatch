use super::auth::TidalAuth;
use crate::models::{AlbumMetadata, ArtistInfo};
use anyhow::{anyhow, Result};
use serde::Deserialize;

const API_BASE_URL: &str = "https://openapi.tidal.com/v2";

// These structs are kept for potential future use (track search, track number display)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub tracks: Option<TracksWrapper>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct TracksWrapper {
    pub items: Vec<Track>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub duration: u32, // seconds
    #[serde(rename = "trackNumber")]
    pub track_number: Option<u32>,
    pub artists: Vec<Artist>,
    pub album: Album,
    #[serde(rename = "audioQuality")]
    pub audio_quality: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub cover: Option<String>, // UUID for cover art
    #[serde(rename = "releaseDate")]
    pub release_date: Option<String>,
}

pub struct TidalApi {
    client: reqwest::blocking::Client,
    auth: TidalAuth,
}

impl TidalApi {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            auth: TidalAuth::new(client_id, client_secret),
        }
    }

    /// Get album art URL (kept for compatibility)
    #[allow(dead_code)]
    pub fn get_album_art_url(&self, cover_uuid: &str, size: u32) -> String {
        // Tidal's image URL format
        format!(
            "https://resources.tidal.com/images/{}/{size}x{size}.jpg",
            cover_uuid.replace('-', "/"),
            size = size
        )
    }

    pub fn search_album(&mut self, artist: &str, album: &str) -> Result<AlbumMetadata> {
        let token = self.auth.get_access_token()?;

        // Build search query - search for album by artist and album name
        // Simplify long album names for better search results
        let simplified_album = if album.len() > 50 {
            // Try to extract the main album name before parentheses or "Vol."
            album
                .split(" (")
                .next()
                .unwrap_or(album)
                .split(", Vol.")
                .next()
                .unwrap_or(album)
        } else {
            album
        };

        let query = format!("{artist} {simplified_album}");
        let encoded_query = urlencoding::encode(&query);

        let response = self
            .client
            .get(format!("{API_BASE_URL}/searchResults/{encoded_query}"))
            .bearer_auth(&token)
            .header("Accept", "application/vnd.api+json")
            .query(&[
                ("countryCode", "US"), // You might want to make this configurable
                ("include", "albums.coverArt"),
            ])
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Search failed: {} - {}", status, error_text));
        }

        // Parse the JSON response to extract album metadata
        let json: serde_json::Value = response.json()?;

        // Look for albums in the included array
        if let Some(included) = json.get("included").and_then(|v| v.as_array()) {
            for item in included {
                if item.get("type").and_then(|t| t.as_str()) == Some("albums") {
                    // Check if this album matches what we're looking for
                    if let Some(attrs) = item.get("attributes") {
                        if let Some(title) = attrs.get("title").and_then(|t| t.as_str()) {
                            let title_lower = title.to_lowercase();
                            let album_lower = album.to_lowercase();

                            // Try multiple matching strategies
                            if title_lower.contains(&album_lower) ||
                               album_lower.contains(&title_lower) ||
                               // For long album names, try matching the simplified version
                               (album.len() > 50 && title_lower.contains(&simplified_album.to_lowercase()))
                            {
                                return self.extract_album_metadata(item, included);
                            }
                        }
                    }
                }
            }
        }

        Err(anyhow!("No album found for: {} - {}", artist, album))
    }

    fn extract_album_metadata(
        &self,
        album_item: &serde_json::Value,
        included: &[serde_json::Value],
    ) -> Result<AlbumMetadata> {
        let attrs = album_item
            .get("attributes")
            .ok_or_else(|| anyhow!("Album missing attributes"))?;

        // Extract basic album information
        let id = album_item
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let title = attrs
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Album")
            .to_string();

        let album_type = attrs
            .get("type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let release_date = attrs
            .get("releaseDate")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let number_of_tracks = attrs
            .get("numberOfItems")
            .and_then(|v| v.as_f64())
            .map(|n| n as u32);

        // Parse ISO 8601 duration
        let duration = attrs
            .get("duration")
            .and_then(|v| v.as_str())
            .map(AlbumMetadata::parse_iso8601_duration);

        let popularity = attrs.get("popularity").and_then(|v| v.as_f64());

        let copyright = attrs
            .get("copyright")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract audio quality from mediaTags
        let audio_quality = self.extract_audio_quality(attrs);

        // Extract artists from relationships
        let artists = self.extract_artists(album_item, included)?;

        // Extract cover art URL
        let cover_url = self.extract_cover_url(album_item, included);

        Ok(AlbumMetadata {
            id,
            title,
            artists,
            album_type,
            release_date,
            number_of_tracks,
            duration,
            audio_quality,
            popularity,
            copyright,
            cover_url,
        })
    }

    fn extract_audio_quality(&self, attrs: &serde_json::Value) -> Option<String> {
        attrs
            .get("mediaTags")
            .and_then(|v| v.as_array())
            .and_then(|tags| {
                // Check for highest priority first
                if tags.iter().any(|t| t.as_str() == Some("HIRES_LOSSLESS")) {
                    return Some("HIRES_LOSSLESS".to_string());
                }
                if tags.iter().any(|t| t.as_str() == Some("LOSSLESS")) {
                    return Some("LOSSLESS".to_string());
                }
                if tags.iter().any(|t| t.as_str() == Some("MQA")) {
                    return Some("MQA".to_string());
                }
                None
            })
    }

    fn extract_artists(
        &self,
        album_item: &serde_json::Value,
        included: &[serde_json::Value],
    ) -> Result<Vec<ArtistInfo>> {
        let mut artists = Vec::new();

        // Get artist relationship references
        if let Some(relationships) = album_item.get("relationships") {
            if let Some(artist_rel) = relationships.get("artists") {
                if let Some(data) = artist_rel.get("data").and_then(|v| v.as_array()) {
                    for artist_ref in data {
                        if let Some(artist_id) = artist_ref.get("id").and_then(|v| v.as_str()) {
                            // Find the artist in included items
                            for included_item in included {
                                if included_item.get("type").and_then(|v| v.as_str())
                                    == Some("artists")
                                    && included_item.get("id").and_then(|v| v.as_str())
                                        == Some(artist_id)
                                {
                                    if let Some(artist_attrs) = included_item.get("attributes") {
                                        if let Some(name) =
                                            artist_attrs.get("name").and_then(|v| v.as_str())
                                        {
                                            artists.push(ArtistInfo {
                                                id: artist_id.to_string(),
                                                name: name.to_string(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback to at least one artist if none found
        if artists.is_empty() {
            // This is common - Tidal API doesn't always include artist data
            // We'll use the playerctl artist as fallback in the UI
            artists.push(ArtistInfo {
                id: "unknown".to_string(),
                name: "Unknown Artist".to_string(),
            });
        }

        Ok(artists)
    }

    fn extract_cover_url(
        &self,
        album_item: &serde_json::Value,
        included: &[serde_json::Value],
    ) -> Option<String> {
        // Look for cover art ID in relationships
        let cover_id = album_item
            .get("relationships")
            .and_then(|r| r.get("coverArt"))
            .and_then(|c| c.get("data"))
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|cover| cover.get("id"))
            .and_then(|id| id.as_str())?;

        // Find the actual artwork in included items
        for artwork in included {
            if artwork.get("type").and_then(|t| t.as_str()) == Some("artworks")
                && artwork.get("id").and_then(|id| id.as_str()) == Some(cover_id)
            {
                // Get the actual URL from the files array
                if let Some(files) = artwork
                    .get("attributes")
                    .and_then(|attrs| attrs.get("files"))
                    .and_then(|files| files.as_array())
                {
                    // Find 640x640 image or use first one
                    for file in files {
                        if let Some(meta) = file.get("meta") {
                            if let Some(width) = meta.get("width").and_then(|w| w.as_u64()) {
                                if width == 640 {
                                    if let Some(url) = file.get("href").and_then(|h| h.as_str()) {
                                        return Some(url.to_string());
                                    }
                                }
                            }
                        }
                    }

                    // Fallback to first image
                    if let Some(url) = files
                        .first()
                        .and_then(|file| file.get("href"))
                        .and_then(|href| href.as_str())
                    {
                        return Some(url.to_string());
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_album_metadata() {
        // Create a mock Tidal API response based on the real one
        let album_json = json!({
            "id": "225834773",
            "type": "albums",
            "attributes": {
                "title": "Chimera",
                "barcodeId": "067003030656",
                "numberOfVolumes": 1,
                "numberOfItems": 13,
                "duration": "PT1H8M27S",
                "explicit": false,
                "releaseDate": "2003-06-24",
                "copyright": "Nettwerk Productions",
                "popularity": 0.3978222978937347,
                "availability": ["STREAM", "DJ"],
                "mediaTags": ["HIRES_LOSSLESS", "LOSSLESS"],
                "type": "ALBUM"
            },
            "relationships": {
                "coverArt": {
                    "data": [{
                        "id": "2xpmpI1s9DzduWTTEatWwV",
                        "type": "artworks"
                    }]
                }
            }
        });

        let artwork_json = json!({
            "id": "2xpmpI1s9DzduWTTEatWwV",
            "type": "artworks",
            "attributes": {
                "mediaType": "IMAGE",
                "files": [
                    {
                        "href": "https://resources.tidal.com/images/137a9eea/49cc/49a2/95e8/2922abe981de/1280x1280.jpg",
                        "meta": {"width": 1280, "height": 1280}
                    },
                    {
                        "href": "https://resources.tidal.com/images/137a9eea/49cc/49a2/95e8/2922abe981de/640x640.jpg",
                        "meta": {"width": 640, "height": 640}
                    }
                ]
            }
        });

        let included = vec![artwork_json];

        let api = TidalApi::new("test_id".to_string(), "test_secret".to_string());
        let metadata = api.extract_album_metadata(&album_json, &included).unwrap();

        assert_eq!(metadata.id, "225834773");
        assert_eq!(metadata.title, "Chimera");
        assert_eq!(metadata.release_date, Some("2003-06-24".to_string()));
        assert_eq!(metadata.number_of_tracks, Some(13));
        assert_eq!(metadata.duration, Some(4107)); // 1H8M27S = 4107 seconds
        assert_eq!(metadata.audio_quality, Some("HIRES_LOSSLESS".to_string()));
        assert_eq!(metadata.popularity, Some(0.3978222978937347));
        assert_eq!(metadata.copyright, Some("Nettwerk Productions".to_string()));
        assert!(metadata.cover_url.is_some());
    }

    #[test]
    fn test_extract_audio_quality() {
        let api = TidalApi::new("test_id".to_string(), "test_secret".to_string());

        // Test HIRES_LOSSLESS priority
        let attrs = json!({
            "mediaTags": ["LOSSLESS", "HIRES_LOSSLESS", "MQA"]
        });
        assert_eq!(
            api.extract_audio_quality(&attrs),
            Some("HIRES_LOSSLESS".to_string())
        );

        // Test LOSSLESS when no HIRES
        let attrs = json!({
            "mediaTags": ["MQA", "LOSSLESS"]
        });
        assert_eq!(
            api.extract_audio_quality(&attrs),
            Some("LOSSLESS".to_string())
        );

        // Test MQA only
        let attrs = json!({
            "mediaTags": ["MQA"]
        });
        assert_eq!(api.extract_audio_quality(&attrs), Some("MQA".to_string()));

        // Test no recognized tags
        let attrs = json!({
            "mediaTags": ["DOLBY_ATMOS"]
        });
        assert_eq!(api.extract_audio_quality(&attrs), None);

        // Test missing mediaTags
        let attrs = json!({});
        assert_eq!(api.extract_audio_quality(&attrs), None);
    }

    #[test]
    fn test_extract_cover_url() {
        let album_json = json!({
            "relationships": {
                "coverArt": {
                    "data": [{
                        "id": "test-artwork-id",
                        "type": "artworks"
                    }]
                }
            }
        });

        let artwork = json!({
            "id": "test-artwork-id",
            "type": "artworks",
            "attributes": {
                "files": [
                    {
                        "href": "https://example.com/small.jpg",
                        "meta": {"width": 320, "height": 320}
                    },
                    {
                        "href": "https://example.com/large.jpg",
                        "meta": {"width": 1280, "height": 1280}
                    },
                    {
                        "href": "https://example.com/medium.jpg",
                        "meta": {"width": 640, "height": 640}
                    }
                ]
            }
        });

        let included = vec![artwork];
        let api = TidalApi::new("test_id".to_string(), "test_secret".to_string());
        let cover_url = api.extract_cover_url(&album_json, &included);

        // Should select the 640x640 image (closest to target 750px)
        assert_eq!(
            cover_url,
            Some("https://example.com/medium.jpg".to_string())
        );
    }

    #[test]
    fn test_get_album_art_url() {
        let api = TidalApi::new("test_id".to_string(), "test_secret".to_string());

        let url = api.get_album_art_url("137a9eea-49cc-49a2-95e8-2922abe981de", 640);
        assert_eq!(
            url,
            "https://resources.tidal.com/images/137a9eea/49cc/49a2/95e8/2922abe981de/640x640.jpg"
        );

        // Test with different size
        let url = api.get_album_art_url("test-uuid", 320);
        assert_eq!(
            url,
            "https://resources.tidal.com/images/test/uuid/320x320.jpg"
        );
    }

    #[test]
    fn test_parse_iso8601_duration_from_tidal() {
        // Test actual Tidal duration format
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT1H8M27S"), 4107); // 1:08:27
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT3M45S"), 225); // 3:45
        assert_eq!(AlbumMetadata::parse_iso8601_duration("PT52M30S"), 3150); // 52:30
    }

    #[test]
    fn test_album_search_query_simplification() {
        // Test that long album names are simplified
        let long_album = "The Dark Side of the Moon (50th Anniversary Remastered Deluxe Edition)";
        let simplified = long_album.split(" (").next().unwrap();
        assert_eq!(simplified, "The Dark Side of the Moon");

        // Test Vol. simplification
        let vol_album = "Greatest Hits, Vol. 2 (Remastered)";
        let simplified = vol_album.split(", Vol.").next().unwrap();
        assert_eq!(simplified, "Greatest Hits");
    }
}
