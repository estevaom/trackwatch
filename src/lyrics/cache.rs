use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::LyricsResponse;

const CACHE_DIR: &str = ".cache/trackwatch/lyrics";
const CACHE_EXPIRY_DAYS: u64 = 7; // Lyrics update more frequently

#[derive(Serialize, Deserialize)]
struct CachedLyrics {
    pub response: Option<LyricsResponse>, // None means "not found"
    pub cached_at: u64,
}

#[derive(Clone)]
pub struct LyricsCache {
    cache_dir: PathBuf,
}

impl LyricsCache {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")?;
        let cache_dir = Path::new(&home).join(CACHE_DIR);
        fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    pub fn get(&self, artist: &str, title: &str) -> Option<Option<LyricsResponse>> {
        let key = self.generate_key(artist, title);
        let cache_path = self.cache_dir.join(format!("{key}.json"));

        if !cache_path.exists() {
            return None;
        }

        let contents = fs::read_to_string(&cache_path).ok()?;
        let cached: CachedLyrics = serde_json::from_str(&contents).ok()?;

        // Check if cache is expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let expiry_time = cached.cached_at + (CACHE_EXPIRY_DAYS * 24 * 60 * 60);

        if now > expiry_time {
            // Remove expired cache
            let _ = fs::remove_file(&cache_path);
            return None;
        }

        Some(cached.response)
    }

    pub fn set(&self, artist: &str, title: &str, lyrics: Option<&LyricsResponse>) -> Result<()> {
        let key = self.generate_key(artist, title);
        let cache_path = self.cache_dir.join(format!("{key}.json"));

        let cached = CachedLyrics {
            response: lyrics.cloned(),
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let json = serde_json::to_string_pretty(&cached)?;
        fs::write(cache_path, json)?;

        Ok(())
    }

    fn generate_key(&self, artist: &str, title: &str) -> String {
        let mut hasher = Sha256::new();
        let normalized = format!("{}:{}", artist.to_lowercase(), title.to_lowercase());
        hasher.update(normalized.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        let temp_dir = std::env::temp_dir();
        let cache = LyricsCache {
            cache_dir: temp_dir,
        };

        // Same artist/title should produce same key
        let key1 = cache.generate_key("Queen", "Bohemian Rhapsody");
        let key2 = cache.generate_key("Queen", "Bohemian Rhapsody");
        assert_eq!(key1, key2);

        // Case insensitive
        let key3 = cache.generate_key("QUEEN", "BOHEMIAN RHAPSODY");
        let key4 = cache.generate_key("queen", "bohemian rhapsody");
        assert_eq!(key3, key4);
        assert_eq!(key1, key3);

        // Different songs should have different keys
        let key5 = cache.generate_key("Queen", "We Will Rock You");
        assert_ne!(key1, key5);

        // Different artists should have different keys
        let key6 = cache.generate_key("David Bowie", "Bohemian Rhapsody");
        assert_ne!(key1, key6);

        // Key should be valid hex string (SHA256)
        assert_eq!(key1.len(), 64);
        assert!(key1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_key_normalization() {
        let temp_dir = std::env::temp_dir();
        let cache = LyricsCache {
            cache_dir: temp_dir,
        };

        // Test various normalizations
        let test_cases = vec![
            (("The Beatles", "Let It Be"), ("the beatles", "let it be")),
            (("AC/DC", "Highway to Hell"), ("ac/dc", "highway to hell")),
            (
                ("P!nk", "Get the Party Started"),
                ("p!nk", "get the party started"),
            ),
        ];

        for ((artist1, title1), (artist2, title2)) in test_cases {
            let key1 = cache.generate_key(artist1, title1);
            let key2 = cache.generate_key(artist2, title2);
            assert_eq!(
                key1, key2,
                "Keys should match for {artist1}/{title1} vs {artist2}/{title2}"
            );
        }
    }
}
