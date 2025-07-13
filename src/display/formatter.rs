use super::PixelatedImage;
use crate::cache::ImageCache;
use crate::colors::{ColorExtractor, ColorPalette};
use crate::models::AlbumMetadata;
use crate::player::PlayerMetadata;
use anyhow::Result;
use image::{imageops::FilterType, GenericImageView, Rgba};

// Terminal color constants
pub const COLOR_RESET: &str = "\x1B[0m";
pub const COLOR_BOLD: &str = "\x1B[1m";
pub const COLOR_CYAN: &str = "\x1B[36m";
pub const COLOR_YELLOW: &str = "\x1B[33m";
pub const COLOR_GREEN: &str = "\x1B[32m";
pub const COLOR_BLUE: &str = "\x1B[34m";

// Layout constants
const MAX_LABEL_WIDTH: usize = 12;
const MIN_PADDING: usize = 2;
const DEFAULT_SPACING: &str = "   "; // 3 spaces between image and info

pub struct DisplayFormatter {
    image_size: u32,
    cache: ImageCache,
}

impl DisplayFormatter {
    pub fn new(image_size: u32) -> Self {
        // Initialize cache - if it fails, we'll just continue without caching
        let cache = ImageCache::new().unwrap_or_else(|e| {
            eprintln!("Warning: Cache disabled - {e}");
            // Create a stub cache that uses the temp directory
            let temp_dir = std::env::temp_dir();
            ImageCache {
                cache_dir: temp_dir,
            }
        });

        Self { image_size, cache }
    }

    /// Display album art and track info side by side
    pub fn display_side_by_side(
        &self,
        album_metadata: Option<&AlbumMetadata>,
        player_metadata: &PlayerMetadata,
        progress_percentage: f32,
    ) -> Result<()> {
        // Get image lines
        let image_lines = match album_metadata.and_then(|a| a.cover_url.as_ref()) {
            Some(url) => self.fetch_and_render_image(url)?,
            None => self.get_placeholder_lines(),
        };

        // Get info lines
        let info_lines = self.format_album_info(album_metadata, player_metadata);

        // Display side by side
        self.display_side_by_side_content(&image_lines, &info_lines);

        // Display progress bar with time information
        self.display_progress_bar_with_time(progress_percentage, player_metadata);

        Ok(())
    }

    /// Update only the progress bar (for non-blinking updates)
    pub fn update_progress_bar(&self, progress_percentage: f32, metadata: &PlayerMetadata) {
        // Move up and clear the progress bar line
        print!("\x1B[1A"); // Move up 1 line
        print!("\x1B[2K"); // Clear line
        print!("\r"); // Move to beginning

        self.display_progress_bar_with_time(progress_percentage, metadata);
    }

    pub fn fetch_and_process_all_formats(
        &self,
        url: &str,
    ) -> Result<(PixelatedImage, super::RatatuiImage, ColorPalette)> {
        // Check cache first
        if let Some(cached) = self.cache.get(url) {
            return Ok((cached.pixelated, cached.ratatui, cached.color_palette));
        }

        // Load image from URL or local file
        let img = if url.starts_with("file://") {
            // Handle local file URLs
            let file_path = url.strip_prefix("file://").unwrap_or(url);
            image::open(file_path)?
        } else {
            // Download from HTTP/HTTPS
            let response = reqwest::blocking::get(url)?;
            let bytes = response.bytes()?;
            image::load_from_memory(&bytes)?
        };

        // Resize to target size
        let resized = img.resize_exact(self.image_size, self.image_size, FilterType::Lanczos3);

        // Create both formats
        let pixelated = PixelatedImage {
            lines: self.image_to_block_lines(&resized),
        };

        let mut pixels = Vec::new();
        let (width, height) = resized.dimensions();

        for y in 0..height {
            let mut row = Vec::new();
            for x in 0..width {
                let pixel = resized.get_pixel(x, y);
                let Rgba([r, g, b, _]) = pixel;
                row.push((r, g, b));
            }
            pixels.push(row);
        }

        let ratatui = super::RatatuiImage { pixels };

        // Extract color palette (3 for progress, 5 for info)
        let color_palette = ColorExtractor::extract_palette(&resized, 3, 5)?;

        // Cache all formats
        if let Err(e) = self.cache.set(
            url,
            pixelated.clone(),
            ratatui.clone(),
            color_palette.clone(),
        ) {
            eprintln!("Failed to cache image: {e}");
        }

        Ok((pixelated, ratatui, color_palette))
    }

    pub fn fetch_and_process_both_formats(
        &self,
        url: &str,
    ) -> Result<(PixelatedImage, super::RatatuiImage)> {
        let (pixelated, ratatui, _) = self.fetch_and_process_all_formats(url)?;
        Ok((pixelated, ratatui))
    }

    pub fn fetch_and_process_image(&self, url: &str) -> Result<PixelatedImage> {
        Ok(self.fetch_and_process_both_formats(url)?.0)
    }

    pub fn fetch_and_process_ratatui_image(&self, url: &str) -> Result<super::RatatuiImage> {
        Ok(self.fetch_and_process_both_formats(url)?.1)
    }

    fn fetch_and_render_image(&self, url: &str) -> Result<Vec<String>> {
        // Download the image
        let response = reqwest::blocking::get(url)?;
        let bytes = response.bytes()?;

        // Load image from bytes
        let img = image::load_from_memory(&bytes)?;

        // Resize to target size
        let resized = img.resize_exact(self.image_size, self.image_size, FilterType::Lanczos3);

        // Convert to block art lines
        Ok(self.image_to_block_lines(&resized))
    }

    fn image_to_block_lines(&self, img: &image::DynamicImage) -> Vec<String> {
        let (width, height) = img.dimensions();
        let mut lines = Vec::new();

        for y in 0..height {
            let mut line = String::new();

            for x in 0..width {
                let pixel = img.get_pixel(x, y);
                let Rgba([r, g, b, _]) = pixel;

                // ANSI escape code for 24-bit color background
                // Two spaces create a square "pixel" block
                line.push_str(&format!("\x1b[48;2;{r};{g};{b}m  \x1b[0m"));
            }

            lines.push(line);
        }

        lines
    }

    fn get_placeholder_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        for _ in 0..self.image_size {
            let mut line = String::new();
            for _ in 0..self.image_size {
                line.push_str("░░");
            }
            lines.push(line);
        }

        lines
    }

    fn format_album_info(
        &self,
        album_metadata: Option<&AlbumMetadata>,
        player_metadata: &PlayerMetadata,
    ) -> Vec<String> {
        let mut info_lines = Vec::new();

        if let Some(album) = album_metadata {
            // Album Name
            info_lines.push(self.format_info_line("Name", &album.title, COLOR_CYAN));

            // Artist(s)
            info_lines.push(self.format_info_line("Artist", &album.all_artists(), COLOR_YELLOW));

            // Skip Type row as it's always ALBUM

            // Release Date
            if let Some(ref release_date) = album.release_date {
                info_lines.push(self.format_info_line("Released", release_date, COLOR_GREEN));
            }

            // Number of Tracks
            if let Some(tracks) = album.number_of_tracks {
                info_lines.push(self.format_info_line("Tracks", &tracks.to_string(), COLOR_BLUE));
            }

            // Album Duration
            if let Some(duration) = album.duration {
                info_lines.push(self.format_info_line(
                    "Duration",
                    &AlbumMetadata::format_duration(duration),
                    COLOR_BLUE,
                ));
            }

            // Audio Quality
            if let Some(ref quality) = album.audio_quality {
                info_lines.push(self.format_info_line("Quality", quality, COLOR_YELLOW));
            }

            // Popularity
            if let Some(popularity) = album.popularity {
                info_lines.push(self.format_info_line(
                    "Popularity",
                    &format!("{:.1}%", popularity * 100.0),
                    COLOR_GREEN,
                ));
            }

            // Copyright
            if let Some(ref copyright) = album.copyright {
                // Truncate if too long
                let display_copyright = if copyright.len() > 40 {
                    format!("{}...", &copyright[..37])
                } else {
                    copyright.clone()
                };
                info_lines.push(self.format_info_line("Copyright", &display_copyright, COLOR_BLUE));
            }
        } else {
            // Fallback to basic track info if no album metadata
            info_lines.push(self.format_info_line("Track", &player_metadata.title, COLOR_CYAN));

            info_lines.push(self.format_info_line("Artist", &player_metadata.artist, COLOR_YELLOW));

            if let Some(ref album) = player_metadata.album {
                info_lines.push(self.format_info_line("Album", album, COLOR_GREEN));
            }
        }

        info_lines
    }

    fn format_info_line(&self, label: &str, value: &str, color: &str) -> String {
        let padding = if label.len() < MAX_LABEL_WIDTH {
            MAX_LABEL_WIDTH - label.len() + MIN_PADDING
        } else {
            MIN_PADDING
        };

        format!(
            "{}{}{}{}{}{}{}",
            COLOR_BOLD,
            label,
            COLOR_RESET,
            " ".repeat(padding),
            color,
            value,
            COLOR_RESET
        )
    }

    fn display_side_by_side_content(&self, image_lines: &[String], info_lines: &[String]) {
        let max_lines = image_lines.len().max(info_lines.len());

        for i in 0..max_lines {
            let image_line = image_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            let info_line = info_lines.get(i).map(|s| s.as_str()).unwrap_or("");

            println!("{image_line}{DEFAULT_SPACING}{info_line}");
        }
    }

    fn display_progress_bar_with_time(&self, percentage: f32, metadata: &PlayerMetadata) {
        let percentage = percentage.clamp(0.0, 100.0);
        let bar_width = (self.image_size * 2) as usize; // Match image width (2 chars per pixel)
        let filled_count = ((percentage / 100.0) * bar_width as f32).round() as usize;

        let filled = "█".repeat(filled_count);
        let empty = "░".repeat(bar_width - filled_count);

        // Format time display
        let time_display =
            if let (Some(position), Some(length)) = (metadata.position, metadata.length) {
                let pos_secs = position.as_secs();
                let pos_min = pos_secs / 60;
                let pos_sec = pos_secs % 60;

                let len_secs = length.as_secs();
                let dur_min = len_secs / 60;
                let dur_sec = len_secs % 60;

                format!(" {pos_min}:{pos_sec:02} / {dur_min}:{dur_sec:02}")
            } else {
                String::new()
            };

        println!("{filled}{empty}{time_display}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_placeholder_lines() {
        // Create formatter with temp cache dir to avoid HOME issues
        let formatter = DisplayFormatter {
            image_size: 5,
            cache: ImageCache {
                cache_dir: std::env::temp_dir(),
            },
        };
        let lines = formatter.get_placeholder_lines();

        // Should have 5 lines
        assert_eq!(lines.len(), 5);

        // Each line should have 5 blocks of "░░"
        for line in &lines {
            assert_eq!(line, "░░░░░░░░░░");
            // Note: "░" is a multi-byte UTF-8 character, so byte length is 30
            assert_eq!(line.len(), 30);
            // But character count is 10
            assert_eq!(line.chars().count(), 10);
        }

        // Test with different size
        let formatter2 = DisplayFormatter {
            image_size: 3,
            cache: ImageCache {
                cache_dir: std::env::temp_dir(),
            },
        };
        let lines2 = formatter2.get_placeholder_lines();
        assert_eq!(lines2.len(), 3);
        assert_eq!(lines2[0], "░░░░░░");
        assert_eq!(lines2[0].len(), 18); // 3 * 2 * 3 bytes per "░"
    }

    #[test]
    fn test_format_info_line() {
        let formatter = DisplayFormatter {
            image_size: 30,
            cache: ImageCache {
                cache_dir: std::env::temp_dir(),
            },
        };

        // Normal case
        let line = formatter.format_info_line("Artist", "Queen", COLOR_YELLOW);
        assert!(line.contains("Artist"));
        assert!(line.contains("Queen"));
        assert!(line.contains(COLOR_YELLOW));
        assert!(line.contains(COLOR_RESET));
        assert!(line.contains(COLOR_BOLD));

        // Long label
        let line = formatter.format_info_line("VeryLongLabelName", "Value", COLOR_CYAN);
        assert!(line.contains("VeryLongLabelName"));
        assert!(line.contains("Value"));

        // Empty values
        let line = formatter.format_info_line("", "Empty Label", COLOR_GREEN);
        assert!(line.contains("Empty Label"));
    }

    #[test]
    fn test_image_to_block_lines() {
        use image::{DynamicImage, RgbaImage};

        let formatter = DisplayFormatter {
            image_size: 2,
            cache: ImageCache {
                cache_dir: std::env::temp_dir(),
            },
        };

        // Create a 2x2 test image
        let mut img = RgbaImage::new(2, 2);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255])); // Red
        img.put_pixel(1, 0, image::Rgba([0, 255, 0, 255])); // Green
        img.put_pixel(0, 1, image::Rgba([0, 0, 255, 255])); // Blue
        img.put_pixel(1, 1, image::Rgba([255, 255, 255, 255])); // White

        let dynamic_img = DynamicImage::ImageRgba8(img);
        let lines = formatter.image_to_block_lines(&dynamic_img);

        assert_eq!(lines.len(), 2);

        // First line should have red and green blocks
        assert!(lines[0].contains("48;2;255;0;0")); // Red background
        assert!(lines[0].contains("48;2;0;255;0")); // Green background

        // Second line should have blue and white blocks
        assert!(lines[1].contains("48;2;0;0;255")); // Blue background
        assert!(lines[1].contains("48;2;255;255;255")); // White background

        // Each pixel should be two spaces
        assert!(lines[0].contains("  "));
    }

    #[test]
    fn test_format_info_line_spacing() {
        let formatter = DisplayFormatter {
            image_size: 30,
            cache: ImageCache {
                cache_dir: std::env::temp_dir(),
            },
        };

        // Test label padding calculation
        let short_label = "ID";
        let line = formatter.format_info_line(short_label, "12345", COLOR_CYAN);

        // Should have proper spacing between label and value
        // MAX_LABEL_WIDTH(12) - len("ID")(2) + MIN_PADDING(2) = 12 spaces
        let expected_spaces = 12;

        // Count spaces between RESET and color code
        let parts: Vec<&str> = line.split(COLOR_RESET).collect();
        if parts.len() >= 2 {
            let space_part = parts[1];
            let space_count = space_part.chars().take_while(|&c| c == ' ').count();
            assert_eq!(space_count, expected_spaces);
        }
    }
}
