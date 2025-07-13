use super::{LyricLine, ParsedLyrics};
use regex::Regex;

pub fn parse_lrc(lrc_content: &str) -> ParsedLyrics {
    let mut lines = Vec::new();

    for line in lrc_content.lines() {
        if let Some(parsed) = parse_lrc_line(line) {
            lines.push(parsed);
        }
    }

    // Sort by timestamp if synced
    lines.sort_by_key(|line| line.timestamp_ms.unwrap_or(u64::MAX));

    ParsedLyrics {
        is_synced: lines.iter().any(|l| l.timestamp_ms.is_some()),
        lines,
    }
}

fn parse_lrc_line(line: &str) -> Option<LyricLine> {
    // Match format: [MM:SS.ms] Text
    let timestamp_regex = Regex::new(r"^\[(\d{2}):(\d{2})\.(\d{2})\]\s*(.*)$").unwrap();

    if let Some(captures) = timestamp_regex.captures(line) {
        let minutes: u64 = captures[1].parse().ok()?;
        let seconds: u64 = captures[2].parse().ok()?;
        let centiseconds: u64 = captures[3].parse().ok()?;

        let timestamp_ms = (minutes * 60 * 1000) + (seconds * 1000) + (centiseconds * 10);

        Some(LyricLine {
            timestamp_ms: Some(timestamp_ms),
            text: captures[4].to_string(),
        })
    } else if !line.trim().is_empty() && !line.starts_with('[') {
        // Plain text line without timestamp
        Some(LyricLine {
            timestamp_ms: None,
            text: line.to_string(),
        })
    } else {
        None
    }
}

pub fn find_current_line(lyrics: &ParsedLyrics, position_ms: u64) -> Option<usize> {
    if !lyrics.is_synced {
        return None;
    }

    let mut current_index = None;

    for (i, line) in lyrics.lines.iter().enumerate() {
        if let Some(timestamp) = line.timestamp_ms {
            if timestamp <= position_ms {
                current_index = Some(i);
            } else {
                break;
            }
        }
    }

    current_index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lrc_line() {
        let line = "[01:23.45] Test lyrics";
        let parsed = parse_lrc_line(line).unwrap();
        assert_eq!(parsed.timestamp_ms, Some(83450));
        assert_eq!(parsed.text, "Test lyrics");
    }

    #[test]
    fn test_parse_plain_line() {
        let line = "Plain lyrics without timestamp";
        let parsed = parse_lrc_line(line).unwrap();
        assert_eq!(parsed.timestamp_ms, None);
        assert_eq!(parsed.text, "Plain lyrics without timestamp");
    }

    #[test]
    fn test_parse_lrc_line_various_timestamps() {
        // Test zero timestamp
        let parsed = parse_lrc_line("[00:00.00] Start").unwrap();
        assert_eq!(parsed.timestamp_ms, Some(0));
        assert_eq!(parsed.text, "Start");

        // Test single digit seconds
        let parsed = parse_lrc_line("[00:05.50] Five seconds").unwrap();
        assert_eq!(parsed.timestamp_ms, Some(5500));

        // Test maximum valid values
        let parsed = parse_lrc_line("[99:59.99] Max time").unwrap();
        assert_eq!(parsed.timestamp_ms, Some(5999990));

        // Test with extra spaces
        let parsed = parse_lrc_line("[01:30.00]     Spaced text").unwrap();
        assert_eq!(parsed.timestamp_ms, Some(90000));
        assert_eq!(parsed.text, "Spaced text");

        // Test empty text after timestamp
        let parsed = parse_lrc_line("[01:00.00]").unwrap();
        assert_eq!(parsed.timestamp_ms, Some(60000));
        assert_eq!(parsed.text, "");
    }

    #[test]
    fn test_parse_lrc_line_invalid_formats() {
        // Missing closing bracket
        assert!(parse_lrc_line("[01:23.45 Test").is_none());

        // Invalid timestamp format
        assert!(parse_lrc_line("[1:23.45] Test").is_none());
        assert!(parse_lrc_line("[01:2.45] Test").is_none());
        assert!(parse_lrc_line("[01:23.4] Test").is_none());

        // Non-numeric values
        assert!(parse_lrc_line("[aa:bb.cc] Test").is_none());

        // Empty line
        assert!(parse_lrc_line("").is_none());

        // Just whitespace
        assert!(parse_lrc_line("   ").is_none());

        // Metadata tags (should be ignored)
        assert!(parse_lrc_line("[ar:Artist Name]").is_none());
        assert!(parse_lrc_line("[ti:Song Title]").is_none());
        assert!(parse_lrc_line("[al:Album Name]").is_none());
    }

    #[test]
    fn test_parse_lrc_full_song() {
        let lrc_content = r#"[ar:Test Artist]
[ti:Test Song]
[00:00.00] Intro line
[00:05.50] First verse
[00:10.00] Second verse
Plain text without timestamp
[00:15.75] Chorus begins
[00:20.00]
[00:25.00] Bridge"#;

        let parsed = parse_lrc(lrc_content);

        // Should be synced since it has timestamps
        assert!(parsed.is_synced);

        // Should have 7 lines (metadata excluded, but plain text and empty text included)
        assert_eq!(parsed.lines.len(), 7);

        // Check first line
        assert_eq!(parsed.lines[0].timestamp_ms, Some(0));
        assert_eq!(parsed.lines[0].text, "Intro line");

        // Check plain text line (should be sorted to end)
        let plain_line = parsed
            .lines
            .iter()
            .find(|l| l.timestamp_ms.is_none())
            .unwrap();
        assert_eq!(plain_line.text, "Plain text without timestamp");

        // Check empty text line
        let empty_line = parsed.lines.iter().find(|l| l.text.is_empty()).unwrap();
        assert_eq!(empty_line.timestamp_ms, Some(20000));

        // Verify sorting by checking timestamps are in order
        let mut last_timestamp = 0;
        for line in &parsed.lines {
            if let Some(ts) = line.timestamp_ms {
                assert!(ts >= last_timestamp);
                last_timestamp = ts;
            }
        }
    }

    #[test]
    fn test_parse_lrc_unsynced() {
        let lrc_content = r#"Line 1
Line 2
Line 3"#;

        let parsed = parse_lrc(lrc_content);

        // Should not be synced
        assert!(!parsed.is_synced);

        // Should have 3 lines
        assert_eq!(parsed.lines.len(), 3);

        // All should have no timestamps
        for line in &parsed.lines {
            assert!(line.timestamp_ms.is_none());
        }
    }

    #[test]
    fn test_parse_lrc_mixed_content() {
        let lrc_content = r#"[00:00.00] First synced line
Unsynced line
[00:10.00] Second synced line"#;

        let parsed = parse_lrc(lrc_content);

        // Should be synced (has at least one timestamp)
        assert!(parsed.is_synced);

        // Should have 3 lines
        assert_eq!(parsed.lines.len(), 3);

        // Check ordering after sort
        assert_eq!(parsed.lines[0].timestamp_ms, Some(0));
        assert_eq!(parsed.lines[1].timestamp_ms, Some(10000));
        assert_eq!(parsed.lines[2].timestamp_ms, None); // Unsynced sorted to end
    }

    #[test]
    fn test_find_current_line() {
        let lyrics = ParsedLyrics {
            is_synced: true,
            lines: vec![
                LyricLine {
                    timestamp_ms: Some(0),
                    text: "Line 1".to_string(),
                },
                LyricLine {
                    timestamp_ms: Some(5000),
                    text: "Line 2".to_string(),
                },
                LyricLine {
                    timestamp_ms: Some(10000),
                    text: "Line 3".to_string(),
                },
                LyricLine {
                    timestamp_ms: Some(15000),
                    text: "Line 4".to_string(),
                },
            ],
        };

        // Before first line
        assert_eq!(find_current_line(&lyrics, 0), Some(0));

        // During first line
        assert_eq!(find_current_line(&lyrics, 2500), Some(0));

        // Exactly at second line
        assert_eq!(find_current_line(&lyrics, 5000), Some(1));

        // During third line
        assert_eq!(find_current_line(&lyrics, 12000), Some(2));

        // After last line
        assert_eq!(find_current_line(&lyrics, 20000), Some(3));
    }

    #[test]
    fn test_find_current_line_unsynced() {
        let lyrics = ParsedLyrics {
            is_synced: false,
            lines: vec![
                LyricLine {
                    timestamp_ms: None,
                    text: "Line 1".to_string(),
                },
                LyricLine {
                    timestamp_ms: None,
                    text: "Line 2".to_string(),
                },
            ],
        };

        // Should always return None for unsynced lyrics
        assert_eq!(find_current_line(&lyrics, 0), None);
        assert_eq!(find_current_line(&lyrics, 5000), None);
    }

    #[test]
    fn test_find_current_line_empty() {
        let lyrics = ParsedLyrics {
            is_synced: true,
            lines: vec![],
        };

        // Should return None for empty lyrics
        assert_eq!(find_current_line(&lyrics, 5000), None);
    }

    #[test]
    fn test_timestamp_calculation_precision() {
        // Test precise millisecond calculations
        let test_cases = vec![
            ("[00:00.01]", 10),     // 1 centisecond = 10ms
            ("[00:00.10]", 100),    // 10 centiseconds = 100ms
            ("[00:01.00]", 1000),   // 1 second
            ("[01:00.00]", 60000),  // 1 minute
            ("[10:30.55]", 630550), // Complex time
        ];

        for (input, expected_ms) in test_cases {
            let line = format!("{input} Text");
            let parsed = parse_lrc_line(&line).unwrap();
            assert_eq!(
                parsed.timestamp_ms,
                Some(expected_ms),
                "Failed for input: {input}"
            );
        }
    }
}
