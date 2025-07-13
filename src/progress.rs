use std::fmt;

pub struct ProgressBar {
    width: usize,
    filled_char: char,
    empty_char: char,
}

impl ProgressBar {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            filled_char: '█',
            empty_char: '░',
        }
    }

    pub fn render(&self, percentage: f32) -> String {
        let percentage = percentage.clamp(0.0, 100.0);
        let filled_count = ((percentage / 100.0) * self.width as f32).round() as usize;
        let empty_count = self.width.saturating_sub(filled_count);

        format!(
            "{}{}",
            self.filled_char.to_string().repeat(filled_count),
            self.empty_char.to_string().repeat(empty_count)
        )
    }
}

impl fmt::Display for ProgressBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_render() {
        let bar = ProgressBar::new(10);

        assert_eq!(bar.render(0.0), "░░░░░░░░░░");
        assert_eq!(bar.render(50.0), "█████░░░░░");
        assert_eq!(bar.render(100.0), "██████████");

        // Test edge cases
        assert_eq!(bar.render(-10.0), "░░░░░░░░░░");
        assert_eq!(bar.render(150.0), "██████████");
    }

    #[test]
    fn test_progress_bar_various_widths() {
        // Test width 1
        let bar1 = ProgressBar::new(1);
        assert_eq!(bar1.render(0.0), "░");
        assert_eq!(bar1.render(49.0), "░");
        assert_eq!(bar1.render(50.0), "█");
        assert_eq!(bar1.render(100.0), "█");

        // Test width 5
        let bar5 = ProgressBar::new(5);
        assert_eq!(bar5.render(0.0), "░░░░░");
        assert_eq!(bar5.render(20.0), "█░░░░");
        assert_eq!(bar5.render(40.0), "██░░░");
        assert_eq!(bar5.render(60.0), "███░░");
        assert_eq!(bar5.render(80.0), "████░");
        assert_eq!(bar5.render(100.0), "█████");

        // Test width 0 (edge case)
        let bar0 = ProgressBar::new(0);
        assert_eq!(bar0.render(50.0), "");
    }

    #[test]
    fn test_progress_bar_precise_percentages() {
        let bar = ProgressBar::new(10);

        // Test precise percentage boundaries
        assert_eq!(bar.render(10.0), "█░░░░░░░░░");
        assert_eq!(bar.render(25.0), "███░░░░░░░");
        assert_eq!(bar.render(33.33), "███░░░░░░░");
        assert_eq!(bar.render(75.0), "████████░░");
        assert_eq!(bar.render(90.0), "█████████░");

        // Test rounding behavior
        assert_eq!(bar.render(14.9), "█░░░░░░░░░"); // Should round to 1
        assert_eq!(bar.render(15.0), "██░░░░░░░░"); // Should round to 2
        assert_eq!(bar.render(94.9), "█████████░"); // Should round to 9
        assert_eq!(bar.render(95.0), "██████████"); // Should round to 10
    }

    #[test]
    fn test_progress_bar_display_trait() {
        let bar = ProgressBar::new(5);
        let display_output = format!("{bar}");
        assert_eq!(display_output, "░░░░░"); // Should show empty bar (0%)
    }

    #[test]
    fn test_progress_bar_float_precision() {
        let bar = ProgressBar::new(100);

        // Test very small percentages
        let result_001 = bar.render(0.01);
        assert_eq!(result_001.chars().next().unwrap(), '░');

        let result_049 = bar.render(0.49);
        assert_eq!(result_049.chars().next().unwrap(), '░');

        let result_050 = bar.render(0.50);
        assert_eq!(result_050.chars().next().unwrap(), '█');

        let result_1 = bar.render(1.0);
        assert_eq!(result_1.chars().next().unwrap(), '█');

        // Test that we get exactly 50 filled chars at 50%
        let half = bar.render(50.0);
        let filled_count = half.chars().filter(|&c| c == '█').count();
        assert_eq!(filled_count, 50);
    }

    #[test]
    fn test_progress_bar_negative_and_overflow() {
        let bar = ProgressBar::new(10);

        // Negative values should clamp to 0
        assert_eq!(bar.render(-1.0), "░░░░░░░░░░");
        assert_eq!(bar.render(-100.0), "░░░░░░░░░░");
        assert_eq!(bar.render(f32::NEG_INFINITY), "░░░░░░░░░░");

        // Values over 100 should clamp to 100
        assert_eq!(bar.render(101.0), "██████████");
        assert_eq!(bar.render(1000.0), "██████████");
        assert_eq!(bar.render(f32::INFINITY), "██████████");

        // NaN should probably clamp to 0 (safest option)
        let nan_result = bar.render(f32::NAN);
        assert!(nan_result == "░░░░░░░░░░" || nan_result == "██████████");
    }
}
