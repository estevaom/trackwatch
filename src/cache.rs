use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::colors::ColorPalette;
use crate::display::{PixelatedImage, RatatuiImage};

const CACHE_DIR: &str = ".cache/trackwatch";
const CACHE_EXPIRY_DAYS: u64 = 30;

#[derive(Serialize, Deserialize)]
pub struct CachedImage {
    pub pixelated: PixelatedImage,
    pub ratatui: RatatuiImage,
    pub color_palette: ColorPalette,
    pub cached_at: u64, // Unix timestamp
}

pub struct ImageCache {
    pub cache_dir: PathBuf,
}

impl ImageCache {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")?;
        let cache_dir = Path::new(&home).join(CACHE_DIR);

        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir)?;

        Ok(Self { cache_dir })
    }

    pub fn get(&self, url: &str) -> Option<CachedImage> {
        let cache_key = self.generate_cache_key(url);
        let cache_path = self.cache_dir.join(format!("{cache_key}.json"));

        // Check if cache file exists
        if !cache_path.exists() {
            return None;
        }

        // Read and deserialize
        match fs::read_to_string(&cache_path) {
            Ok(contents) => {
                match serde_json::from_str::<CachedImage>(&contents) {
                    Ok(cached) => {
                        // Check if cache is expired
                        if self.is_expired(cached.cached_at) {
                            // Delete expired cache
                            let _ = fs::remove_file(&cache_path);
                            None
                        } else {
                            Some(cached)
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to deserialize cache: {e}");
                        // Delete invalid cache file
                        let _ = fs::remove_file(&cache_path);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read cache file: {e}");
                None
            }
        }
    }

    pub fn set(
        &self,
        url: &str,
        pixelated: PixelatedImage,
        ratatui: RatatuiImage,
        color_palette: ColorPalette,
    ) -> Result<()> {
        let cache_key = self.generate_cache_key(url);
        let cache_path = self.cache_dir.join(format!("{cache_key}.json"));

        let cached = CachedImage {
            pixelated,
            ratatui,
            color_palette,
            cached_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };

        let json = serde_json::to_string(&cached)?;
        fs::write(cache_path, json)?;

        Ok(())
    }

    fn generate_cache_key(&self, url: &str) -> String {
        // Use SHA256 hash of URL as cache key
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn is_expired(&self, cached_at: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Handle future timestamps (should not be expired)
        if cached_at > now {
            return false;
        }

        let age_days = (now - cached_at) / (60 * 60 * 24);
        age_days > CACHE_EXPIRY_DAYS
    }

    pub fn clear(&self) -> Result<()> {
        // Remove all cache files
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }

    pub fn size(&self) -> Result<u64> {
        let mut total_size = 0;
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                total_size += entry.metadata()?.len();
            }
        }
        Ok(total_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_cache_key() {
        let temp_dir = std::env::temp_dir();
        let cache = ImageCache {
            cache_dir: temp_dir,
        };

        // Same URL should produce same key
        let key1 = cache.generate_cache_key("https://example.com/image.jpg");
        let key2 = cache.generate_cache_key("https://example.com/image.jpg");
        assert_eq!(key1, key2);

        // Different URLs should produce different keys
        let key3 = cache.generate_cache_key("https://example.com/other.jpg");
        assert_ne!(key1, key3);

        // Key should be a valid hex string (SHA256 produces 64 chars)
        assert_eq!(key1.len(), 64);
        assert!(key1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_is_expired() {
        let temp_dir = std::env::temp_dir();
        let cache = ImageCache {
            cache_dir: temp_dir,
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Fresh cache (just created)
        assert!(!cache.is_expired(now));

        // Cache from 1 day ago
        assert!(!cache.is_expired(now - 60 * 60 * 24));

        // Cache from 29 days ago (still valid)
        assert!(!cache.is_expired(now - 60 * 60 * 24 * 29));

        // Cache from 30 days ago (exactly at expiry)
        assert!(!cache.is_expired(now - 60 * 60 * 24 * 30));

        // Cache from 31 days ago (expired)
        assert!(cache.is_expired(now - 60 * 60 * 24 * 31));

        // Very old cache
        assert!(cache.is_expired(now - 60 * 60 * 24 * 365));
    }

    #[test]
    fn test_cache_key_consistency() {
        let temp_dir = std::env::temp_dir();
        let cache = ImageCache {
            cache_dir: temp_dir,
        };

        // Test various URL formats
        let urls = vec![
            "https://example.com/image.jpg",
            "http://example.com/image.jpg",
            "https://example.com/image.jpg?param=value",
            "file:///home/user/image.jpg",
            "https://tidal.com/album/12345/cover.jpg",
        ];

        for url in &urls {
            let key = cache.generate_cache_key(url);
            // Verify key is deterministic
            assert_eq!(key, cache.generate_cache_key(url));
            // Verify key format
            assert_eq!(key.len(), 64);
            assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn test_cached_image_struct() {
        let cached = CachedImage {
            pixelated: PixelatedImage {
                lines: vec!["test".to_string()],
            },
            ratatui: RatatuiImage {
                pixels: vec![vec![(255, 0, 0)]],
            },
            color_palette: ColorPalette {
                progress_colors: vec![(255, 0, 0), (0, 255, 0), (0, 0, 255)],
                info_colors: vec![(255, 255, 255); 5],
            },
            cached_at: 1234567890,
        };

        assert_eq!(cached.pixelated.lines.len(), 1);
        assert_eq!(cached.ratatui.pixels.len(), 1);
        assert_eq!(cached.color_palette.progress_colors.len(), 3);
        assert_eq!(cached.color_palette.info_colors.len(), 5);
        assert_eq!(cached.cached_at, 1234567890);
    }

    #[test]
    fn test_cache_expiry_edge_cases() {
        let temp_dir = std::env::temp_dir();
        let cache = ImageCache {
            cache_dir: temp_dir,
        };

        // Test with future timestamp (should not be expired)
        let future = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600; // 1 hour in future
        assert!(!cache.is_expired(future));

        // Test with very old timestamp (epoch)
        assert!(cache.is_expired(0));

        // Test with timestamp from 1 year ago
        let one_year_ago = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (365 * 24 * 60 * 60);
        assert!(cache.is_expired(one_year_ago));
    }
}
