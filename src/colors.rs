use anyhow::Result;
use image::{DynamicImage, GenericImageView, Rgba};
use palette::{FromColor, Lab, Srgb};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    pub progress_colors: Vec<(u8, u8, u8)>, // 3 colors for gradient
    pub info_colors: Vec<(u8, u8, u8)>,     // 5 colors for text
}

pub struct ColorExtractor;

impl ColorExtractor {
    pub fn extract_palette(
        image: &DynamicImage,
        progress_count: usize,
        info_count: usize,
    ) -> Result<ColorPalette> {
        // Sample pixels from the image
        let pixels = Self::sample_pixels(image);

        // Extract dominant colors using k-means clustering
        let total_colors = progress_count + info_count + 2; // Extract extra colors for fallbacks
        let mut colors = Self::k_means_clustering(&pixels, total_colors)?;

        // Sort colors by brightness for contrast selection
        Self::sort_by_brightness(&mut colors);

        // For info colors: select lighter colors that contrast well with dark background
        let dark_bg = (20, 20, 20); // Typical dark terminal background
        let mut info_colors = Vec::new();

        // Start from the lightest colors and work down
        for &color in colors.iter().rev() {
            if Self::contrast_ratio(color, dark_bg) >= 7.0 {
                // Higher standard for text readability
                info_colors.push(color);
                if info_colors.len() >= info_count {
                    break;
                }
            }
        }

        // If we don't have enough high-contrast colors, brighten the lighter ones
        if info_colors.len() < info_count {
            // Take remaining lighter colors and brighten them
            for &color in colors.iter().rev().skip(info_colors.len()) {
                let brightened = Self::ensure_min_brightness(color, 180); // Minimum brightness
                info_colors.push(brightened);
                if info_colors.len() >= info_count {
                    break;
                }
            }
        }

        // Final fallback - use bright default colors
        while info_colors.len() < info_count {
            let defaults = [
                (255, 255, 255), // White
                (200, 200, 255), // Light blue
                (255, 200, 200), // Light red
                (200, 255, 200), // Light green
                (255, 255, 200), // Light yellow
            ];
            if let Some(&color) = defaults.get(info_colors.len()) {
                info_colors.push(color);
            }
        }

        // For progress colors: ensure good contrast between darkest and lightest
        let mut progress_colors = if colors.len() >= 3 {
            vec![
                colors[0],                // Darkest
                colors[colors.len() / 2], // Middle
                colors[colors.len() - 1], // Lightest
            ]
        } else {
            vec![(50, 50, 50), (128, 128, 128), (200, 200, 200)] // Fallback gradient
        };

        // Ensure progress colors are visible against black background
        for color in &mut progress_colors {
            if Self::contrast_ratio(*color, (0, 0, 0)) < 2.0 {
                // If too dark, lighten it
                let (r, g, b) = *color;
                *color = (
                    (r as u16 + 100).min(255) as u8,
                    (g as u16 + 100).min(255) as u8,
                    (b as u16 + 100).min(255) as u8,
                );
            }
        }

        Ok(ColorPalette {
            progress_colors,
            info_colors,
        })
    }

    fn sample_pixels(image: &DynamicImage) -> Vec<Lab> {
        let (width, height) = image.dimensions();
        let mut pixels = Vec::new();

        // Sample pixels in a grid pattern for better coverage
        let step = ((width * height) as f32).sqrt() as u32 / 20; // Sample ~400 pixels
        let step = step.max(1);

        for y in (0..height).step_by(step as usize) {
            for x in (0..width).step_by(step as usize) {
                let Rgba([r, g, b, a]) = image.get_pixel(x, y);

                // Skip transparent or very dark pixels
                if a < 128 || (r < 20 && g < 20 && b < 20) {
                    continue;
                }

                // Convert to Lab color space for better perceptual clustering
                let rgb = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
                let lab = Lab::from_color(rgb);
                pixels.push(lab);
            }
        }

        pixels
    }

    fn k_means_clustering(pixels: &[Lab], k: usize) -> Result<Vec<(u8, u8, u8)>> {
        if pixels.is_empty() {
            return Ok(vec![(128, 128, 128); k]); // Gray fallback
        }

        // Initialize centroids using k-means++ algorithm
        let mut centroids = Self::initialize_centroids(pixels, k);
        let mut assignments = vec![0; pixels.len()];

        // Run k-means iterations
        for _ in 0..50 {
            // Assign pixels to nearest centroid
            let mut changed = false;
            for (i, pixel) in pixels.iter().enumerate() {
                let nearest = Self::find_nearest_centroid(pixel, &centroids);
                if assignments[i] != nearest {
                    assignments[i] = nearest;
                    changed = true;
                }
            }

            if !changed {
                break;
            }

            // Update centroids
            Self::update_centroids(pixels, &assignments, &mut centroids);
        }

        // Convert centroids back to RGB
        Ok(centroids
            .iter()
            .map(|lab| {
                let rgb = Srgb::from_color(*lab);
                let r = (rgb.red * 255.0).round() as u8;
                let g = (rgb.green * 255.0).round() as u8;
                let b = (rgb.blue * 255.0).round() as u8;
                (r, g, b)
            })
            .collect())
    }

    fn initialize_centroids(pixels: &[Lab], k: usize) -> Vec<Lab> {
        let mut centroids = Vec::with_capacity(k);

        // First centroid is random
        centroids.push(pixels[0]);

        // Rest use k-means++ initialization
        for _ in 1..k {
            let mut distances = vec![f32::MAX; pixels.len()];

            // Calculate minimum distance to existing centroids for each pixel
            for (i, pixel) in pixels.iter().enumerate() {
                for centroid in &centroids {
                    let dist = Self::color_distance(pixel, centroid);
                    distances[i] = distances[i].min(dist);
                }
            }

            // Choose pixel with maximum minimum distance
            let max_idx = distances
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(idx, _)| idx)
                .unwrap_or(0);

            centroids.push(pixels[max_idx]);
        }

        centroids
    }

    fn find_nearest_centroid(pixel: &Lab, centroids: &[Lab]) -> usize {
        centroids
            .iter()
            .enumerate()
            .map(|(i, c)| (i, Self::color_distance(pixel, c)))
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn update_centroids(pixels: &[Lab], assignments: &[usize], centroids: &mut [Lab]) {
        let k = centroids.len();
        let mut sums = vec![(0.0, 0.0, 0.0); k];
        let mut counts = vec![0; k];

        // Sum assigned pixels
        for (pixel, &assignment) in pixels.iter().zip(assignments) {
            sums[assignment].0 += pixel.l;
            sums[assignment].1 += pixel.a;
            sums[assignment].2 += pixel.b;
            counts[assignment] += 1;
        }

        // Calculate new centroids
        for (i, centroid) in centroids.iter_mut().enumerate() {
            if counts[i] > 0 {
                *centroid = Lab::new(
                    sums[i].0 / counts[i] as f32,
                    sums[i].1 / counts[i] as f32,
                    sums[i].2 / counts[i] as f32,
                );
            }
        }
    }

    fn color_distance(a: &Lab, b: &Lab) -> f32 {
        let dl = a.l - b.l;
        let da = a.a - b.a;
        let db = a.b - b.b;
        (dl * dl + da * da + db * db).sqrt()
    }

    /// Calculate relative luminance of a color (WCAG formula)
    pub fn relative_luminance(color: (u8, u8, u8)) -> f32 {
        let (r, g, b) = color;

        // Convert to linear RGB
        let r_linear = Self::srgb_to_linear(r as f32 / 255.0);
        let g_linear = Self::srgb_to_linear(g as f32 / 255.0);
        let b_linear = Self::srgb_to_linear(b as f32 / 255.0);

        // Calculate luminance
        0.2126 * r_linear + 0.7152 * g_linear + 0.0722 * b_linear
    }

    fn srgb_to_linear(value: f32) -> f32 {
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Calculate contrast ratio between two colors (WCAG formula)
    pub fn contrast_ratio(color1: (u8, u8, u8), color2: (u8, u8, u8)) -> f32 {
        let lum1 = Self::relative_luminance(color1);
        let lum2 = Self::relative_luminance(color2);

        let lighter = lum1.max(lum2);
        let darker = lum1.min(lum2);

        (lighter + 0.05) / (darker + 0.05)
    }

    /// Sort colors by brightness (darkest to lightest)
    pub fn sort_by_brightness(colors: &mut [(u8, u8, u8)]) {
        colors.sort_by(|a, b| {
            let brightness_a = Self::calculate_brightness(*a);
            let brightness_b = Self::calculate_brightness(*b);
            brightness_a.partial_cmp(&brightness_b).unwrap()
        });
    }

    fn calculate_brightness(color: (u8, u8, u8)) -> f32 {
        let (r, g, b) = color;
        // Simple average brightness
        (r as f32 + g as f32 + b as f32) / (3.0 * 255.0)
    }

    /// Ensure a color has minimum brightness
    fn ensure_min_brightness(color: (u8, u8, u8), min_value: u8) -> (u8, u8, u8) {
        let (r, g, b) = color;

        // If any channel is already bright enough, just boost the dark ones
        if r >= min_value || g >= min_value || b >= min_value {
            return (
                r.max(min_value / 2),
                g.max(min_value / 2),
                b.max(min_value / 2),
            );
        }

        // Otherwise, boost all channels proportionally
        let max_channel = r.max(g).max(b);
        if max_channel == 0 {
            // Grayscale if all black
            return (min_value, min_value, min_value);
        }

        let scale = min_value as f32 / max_channel as f32;
        (
            (r as f32 * scale) as u8,
            (g as f32 * scale) as u8,
            (b as f32 * scale) as u8,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, Rgba, RgbaImage};

    fn create_test_image(width: u32, height: u32, color: Rgba<u8>) -> DynamicImage {
        let mut img = RgbaImage::new(width, height);
        for pixel in img.pixels_mut() {
            *pixel = color;
        }
        DynamicImage::ImageRgba8(img)
    }

    fn create_gradient_image(width: u32, height: u32) -> DynamicImage {
        let mut img = RgbaImage::new(width, height);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let r = (x * 255 / width) as u8;
            let g = (y * 255 / height) as u8;
            let b = 128;
            *pixel = Rgba([r, g, b, 255]);
        }
        DynamicImage::ImageRgba8(img)
    }

    #[test]
    fn test_relative_luminance() {
        // Test black
        assert_eq!(ColorExtractor::relative_luminance((0, 0, 0)), 0.0);

        // Test white
        let white_lum = ColorExtractor::relative_luminance((255, 255, 255));
        assert!((white_lum - 1.0).abs() < 0.01);

        // Test red
        let red_lum = ColorExtractor::relative_luminance((255, 0, 0));
        assert!((red_lum - 0.2126).abs() < 0.01);

        // Test green (should be brightest primary)
        let green_lum = ColorExtractor::relative_luminance((0, 255, 0));
        assert!((green_lum - 0.7152).abs() < 0.01);

        // Test blue (should be darkest primary)
        let blue_lum = ColorExtractor::relative_luminance((0, 0, 255));
        assert!((blue_lum - 0.0722).abs() < 0.01);
    }

    #[test]
    fn test_contrast_ratio() {
        // Maximum contrast: white on black
        let max_contrast = ColorExtractor::contrast_ratio((255, 255, 255), (0, 0, 0));
        assert!((max_contrast - 21.0).abs() < 0.1);

        // Minimum contrast: same color
        let min_contrast = ColorExtractor::contrast_ratio((128, 128, 128), (128, 128, 128));
        assert!((min_contrast - 1.0).abs() < 0.01);

        // WCAG AA threshold test (should be >= 4.5)
        let aa_contrast = ColorExtractor::contrast_ratio((255, 255, 255), (96, 96, 96));
        assert!(aa_contrast >= 4.5);

        // Test symmetry
        let contrast1 = ColorExtractor::contrast_ratio((255, 255, 255), (0, 0, 0));
        let contrast2 = ColorExtractor::contrast_ratio((0, 0, 0), (255, 255, 255));
        assert!((contrast1 - contrast2).abs() < 0.01);
    }

    #[test]
    fn test_sort_by_brightness() {
        let mut colors = vec![
            (255, 255, 255), // White (brightest)
            (0, 0, 0),       // Black (darkest)
            (128, 128, 128), // Gray (middle)
            (255, 0, 0),     // Red
            (0, 255, 0),     // Green
            (0, 0, 255),     // Blue
        ];

        ColorExtractor::sort_by_brightness(&mut colors);

        // Check that black is first (darkest)
        assert_eq!(colors[0], (0, 0, 0));

        // Check that white is last (brightest)
        assert_eq!(colors[colors.len() - 1], (255, 255, 255));

        // Check that colors are in ascending brightness order
        for i in 1..colors.len() {
            let brightness_prev = ColorExtractor::calculate_brightness(colors[i - 1]);
            let brightness_curr = ColorExtractor::calculate_brightness(colors[i]);
            assert!(brightness_prev <= brightness_curr);
        }
    }

    #[test]
    fn test_ensure_min_brightness() {
        // Test black to minimum brightness
        let brightened = ColorExtractor::ensure_min_brightness((0, 0, 0), 100);
        assert_eq!(brightened, (100, 100, 100));

        // Test already bright color (should keep brightest channel)
        let bright = ColorExtractor::ensure_min_brightness((200, 100, 50), 150);
        assert!(bright.0 >= 150 || bright.1 >= 150 || bright.2 >= 150);

        // Test proportional scaling
        let scaled = ColorExtractor::ensure_min_brightness((50, 100, 25), 200);
        assert_eq!(scaled.1, 200); // Green was max, should be 200
        assert_eq!(scaled.0, 100); // Red should be scaled proportionally
        assert_eq!(scaled.2, 50); // Blue should be scaled proportionally
    }

    #[test]
    fn test_extract_palette_single_color() {
        let img = create_test_image(10, 10, Rgba([255, 0, 0, 255]));
        let palette = ColorExtractor::extract_palette(&img, 3, 5).unwrap();

        assert_eq!(palette.progress_colors.len(), 3);
        assert_eq!(palette.info_colors.len(), 5);

        // All colors should be derived from red or fallbacks
        for color in &palette.info_colors {
            // Should be either red-ish or a fallback bright color
            assert!(color.0 > 128 || color.1 > 128 || color.2 > 128);
        }
    }

    #[test]
    fn test_extract_palette_gradient() {
        let img = create_gradient_image(50, 50);
        let palette = ColorExtractor::extract_palette(&img, 3, 5).unwrap();

        assert_eq!(palette.progress_colors.len(), 3);
        assert_eq!(palette.info_colors.len(), 5);

        // Progress colors should have increasing brightness
        let p1_brightness = ColorExtractor::calculate_brightness(palette.progress_colors[0]);
        let p3_brightness = ColorExtractor::calculate_brightness(palette.progress_colors[2]);
        assert!(p1_brightness < p3_brightness);

        // Info colors should all have good contrast against dark background
        for color in &palette.info_colors {
            let contrast = ColorExtractor::contrast_ratio(*color, (20, 20, 20));
            assert!(contrast >= 7.0 || ColorExtractor::calculate_brightness(*color) > 0.7);
        }
    }

    #[test]
    fn test_sample_pixels_skip_transparent() {
        let mut img = RgbaImage::new(10, 10);

        // Half transparent, half opaque
        for (x, _y, pixel) in img.enumerate_pixels_mut() {
            if x < 5 {
                *pixel = Rgba([255, 0, 0, 0]); // Transparent red
            } else {
                *pixel = Rgba([0, 255, 0, 255]); // Opaque green
            }
        }

        let dyn_img = DynamicImage::ImageRgba8(img);
        let pixels = ColorExtractor::sample_pixels(&dyn_img);

        // Should only have sampled the opaque pixels
        assert!(!pixels.is_empty());
        // Lab color for green should have negative 'a' value (red-green axis)
        for pixel in pixels {
            assert!(pixel.a < 0.0);
        }
    }

    #[test]
    fn test_calculate_brightness_edge_cases() {
        assert_eq!(ColorExtractor::calculate_brightness((0, 0, 0)), 0.0);
        assert_eq!(ColorExtractor::calculate_brightness((255, 255, 255)), 1.0);

        let gray = ColorExtractor::calculate_brightness((128, 128, 128));
        assert!((gray - 0.5019607).abs() < 0.01);
    }
}
