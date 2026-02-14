//! Screen content parsing utilities for analyzing captured terminal output.
//!
//! This module provides helpers for extracting structured information from
//! raw ANSI terminal output, including:
//!
//! - Finding separator characters (│, ─) used in split layouts
//! - Extracting ANSI color codes for verifying styling changes
//!
//! # Example
//!
//! ```ignore
//! use kitty_test_harness::utils::screen::{find_vertical_separator_col, extract_row_colors};
//!
//! // After capturing screen content
//! let (raw, clean) = kitty.screen_text_clean();
//!
//! // Find a vertical separator in the clean output
//! if let Some(col) = find_vertical_separator_col(&clean) {
//!     // Extract colors from that row in the raw output
//!     let colors = extract_row_colors(&raw, 10);
//!     println!("Found {} distinct colors on row 10", colors.len());
//! }
//! ```

use std::collections::HashMap;

/// Vertical box-drawing character used as a separator in split layouts.
pub const VERTICAL_SEPARATOR: char = '│'; // U+2502

/// Horizontal box-drawing character used as a separator in split layouts.
pub const HORIZONTAL_SEPARATOR: char = '─'; // U+2500

/// Find the column position of vertical separators (│) in the screen.
///
/// Scans all lines of the clean (ANSI-stripped) screen output and returns
/// the column where vertical separator characters appear most frequently.
/// This is useful for locating the separator in a side-by-side split layout.
///
/// # Arguments
///
/// * `clean` - The clean (ANSI-stripped) screen text
///
/// # Returns
///
/// The column index where vertical separators appear, or `None` if no
/// consistent vertical separator line is found (requires > 5 occurrences).
///
/// # Example
///
/// ```
/// use kitty_test_harness::utils::screen::find_vertical_separator_col;
///
/// let screen = "left  │right\ntext  │more\nhere  │data\naaaa  │bbbb\ncccc  │dddd\neeee  │ffff";
/// assert_eq!(find_vertical_separator_col(screen), Some(6));
/// ```
pub fn find_vertical_separator_col(clean: &str) -> Option<usize> {
	let lines: Vec<&str> = clean.lines().collect();
	if lines.is_empty() {
		return None;
	}

	// Count occurrences of │ at each column position
	let mut col_counts: HashMap<usize, usize> = HashMap::new();

	for line in &lines {
		for (col, ch) in line.chars().enumerate() {
			if ch == VERTICAL_SEPARATOR {
				*col_counts.entry(col).or_insert(0) += 1;
			}
		}
	}

	// Find the column with the most separator characters (should be a consistent vertical line)
	col_counts
		.into_iter()
		.max_by_key(|(_, count)| *count)
		.filter(|(_, count)| *count > 5) // Must appear on multiple rows to be a real separator
		.map(|(col, _)| col)
}

/// Find the row position of horizontal separators (─) in the screen.
///
/// Scans all lines of the clean (ANSI-stripped) screen output and returns
/// the row where horizontal separator characters appear most frequently.
/// This is useful for locating the separator in a top/bottom split layout.
///
/// # Arguments
///
/// * `clean` - The clean (ANSI-stripped) screen text
///
/// # Returns
///
/// The row index where horizontal separators appear most densely, or `None`
/// if no row has enough separator characters (requires > 5 occurrences).
///
/// # Example
///
/// ```
/// use kitty_test_harness::utils::screen::find_horizontal_separator_row;
///
/// let screen = "top content\n───────────\nbottom text";
/// assert_eq!(find_horizontal_separator_row(screen), Some(1));
/// ```
pub fn find_horizontal_separator_row(clean: &str) -> Option<usize> {
	clean
		.lines()
		.enumerate()
		.map(|(row, line)| {
			let count = line.chars().filter(|&c| c == HORIZONTAL_SEPARATOR).count();
			(row, count)
		})
		.filter(|(_, count)| *count > 5) // Must have multiple separator chars to be a real separator
		.max_by_key(|(_, count)| *count)
		.map(|(row, _)| row)
}

/// Find all rows that contain a vertical separator at the given column.
///
/// # Arguments
///
/// * `clean` - The clean (ANSI-stripped) screen text
/// * `col` - The column index to check for separators
///
/// # Returns
///
/// A vector of row indices where the separator character appears at the specified column.
///
/// # Example
///
/// ```
/// use kitty_test_harness::utils::screen::find_separator_rows_at_col;
///
/// let screen = "a│b\nc│d\ne f";
/// let rows = find_separator_rows_at_col(screen, 1);
/// assert_eq!(rows, vec![0, 1]);
/// ```
pub fn find_separator_rows_at_col(clean: &str, col: usize) -> Vec<usize> {
	clean
		.lines()
		.enumerate()
		.filter(|(_, line)| line.chars().nth(col).is_some_and(|c| c == VERTICAL_SEPARATOR))
		.map(|(row, _)| row)
		.collect()
}

/// Find all columns that contain a horizontal separator at the given row.
///
/// # Arguments
///
/// * `clean` - The clean (ANSI-stripped) screen text
/// * `row` - The row index to check for separators
///
/// # Returns
///
/// A vector of column indices where the separator character appears at the specified row.
pub fn find_separator_cols_at_row(clean: &str, row: usize) -> Vec<usize> {
	clean
		.lines()
		.nth(row)
		.map(|line| {
			line.chars()
				.enumerate()
				.filter(|(_, c)| *c == HORIZONTAL_SEPARATOR)
				.map(|(col, _)| col)
				.collect()
		})
		.unwrap_or_default()
}

/// Represents an extracted ANSI color from terminal output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiColor {
	/// The raw ANSI escape sequence (e.g., "\x1b[38:2:255:128:0m")
	pub raw: String,
	/// Whether this is a foreground (true) or background (false) color
	pub is_foreground: bool,
	/// RGB values if this is a true-color (24-bit) specification
	pub rgb: Option<(u8, u8, u8)>,
	/// 256-color palette index if this is an indexed color
	pub palette_index: Option<u8>,
}

impl AnsiColor {
	/// Parse an ANSI SGR color sequence into an `AnsiColor` struct.
	///
	/// Supports both semicolon-separated (standard) and colon-separated (kitty)
	/// color specifications.
	/// Parse an ANSI SGR color sequence into an `AnsiColor`.
	pub fn parse_seq(seq: &str) -> Option<Self> {
		// Check if it's a foreground or background color
		let is_foreground = seq.contains("38;") || seq.contains("38:");
		let is_background = seq.contains("48;") || seq.contains("48:");

		if !is_foreground && !is_background {
			return None;
		}

		// Try to extract RGB or palette index
		let mut rgb = None;
		let mut palette_index = None;

		// Handle RGB colors (38;2;R;G;B or 38:2:R:G:B)
		if seq.contains(";2;") || seq.contains(":2:") {
			let parts: Vec<&str> = seq.trim_start_matches("\x1b[").trim_end_matches('m').split([';', ':']).collect();

			// Find the "2" marker and extract R, G, B
			if let Some(pos) = parts.iter().position(|&p| p == "2")
				&& parts.len() > pos + 3
				&& let (Ok(r), Ok(g), Ok(b)) = (parts[pos + 1].parse::<u8>(), parts[pos + 2].parse::<u8>(), parts[pos + 3].parse::<u8>())
			{
				rgb = Some((r, g, b));
			}
		}
		// Handle 256-color palette (38;5;N or 38:5:N)
		else if seq.contains(";5;") || seq.contains(":5:") {
			let parts: Vec<&str> = seq.trim_start_matches("\x1b[").trim_end_matches('m').split([';', ':']).collect();

			if let Some(pos) = parts.iter().position(|&p| p == "5")
				&& parts.len() > pos + 1
				&& let Ok(idx) = parts[pos + 1].parse::<u8>()
			{
				palette_index = Some(idx);
			}
		}

		Some(AnsiColor {
			raw: seq.to_string(),
			is_foreground,
			rgb,
			palette_index,
		})
	}
}

/// Extract all ANSI color codes from a specific row in the raw terminal output.
///
/// Returns a list of distinct color escape sequences found on that row.
/// This is useful for verifying that hover effects or other styling changes
/// are being applied correctly.
///
/// # Arguments
///
/// * `raw` - The raw terminal output (with ANSI escape sequences)
/// * `row` - The row index to extract colors from
///
/// # Returns
///
/// A vector of raw ANSI color escape sequences found on the specified row.
/// Duplicates are filtered out.
///
/// # Supported Formats
///
/// Both standard semicolon-separated and kitty's colon-separated formats:
/// - `\x1b[38;2;R;G;Bm` - Standard RGB foreground
/// - `\x1b[38:2:R:G:Bm` - Kitty RGB foreground
/// - `\x1b[38;5;Nm` - Standard 256-color foreground
/// - `\x1b[38:5:Nm` - Kitty 256-color foreground
/// - `\x1b[48;...]` variants for background colors
///
/// # Example
///
/// ```
/// use kitty_test_harness::utils::screen::extract_row_colors;
///
/// let raw = "normal\x1b[38;2;255;0;0mred\x1b[mtext";
/// let colors = extract_row_colors(raw, 0);
/// assert!(colors.iter().any(|c| c.contains("255")));
/// ```
pub fn extract_row_colors(raw: &str, row: usize) -> Vec<String> {
	let lines: Vec<&str> = raw.lines().collect();
	if row >= lines.len() {
		return vec![];
	}

	let line = lines[row];
	let mut colors = vec![];

	// Look for ANSI SGR sequences
	let mut i = 0;
	let chars: Vec<char> = line.chars().collect();
	while i < chars.len() {
		if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
			// Find the 'm' that ends the sequence
			let start = i;
			while i < chars.len() && chars[i] != 'm' {
				i += 1;
			}
			if i < chars.len() {
				let seq: String = chars[start..=i].iter().collect();
				// Check if it's a foreground or background color
				if (seq.contains("38;2;")
					|| seq.contains("38;5;")
					|| seq.contains("38:2:")
					|| seq.contains("38:5:")
					|| seq.contains("48;2;")
					|| seq.contains("48;5;")
					|| seq.contains("48:2:")
					|| seq.contains("48:5:"))
					&& !colors.contains(&seq)
				{
					colors.push(seq);
				}
			}
		}
		i += 1;
	}

	colors
}

/// Extract structured ANSI color information from a specific row.
///
/// Like [`extract_row_colors`], but returns parsed [`AnsiColor`] structs
/// with RGB values and other metadata extracted.
///
/// # Arguments
///
/// * `raw` - The raw terminal output (with ANSI escape sequences)
/// * `row` - The row index to extract colors from
///
/// # Returns
///
/// A vector of parsed `AnsiColor` structs.
pub fn extract_row_colors_parsed(raw: &str, row: usize) -> Vec<AnsiColor> {
	extract_row_colors(raw, row).into_iter().filter_map(|seq| AnsiColor::parse_seq(&seq)).collect()
}

/// Returns the active foreground RGB color when `needle` first appears in
/// the visible text of a raw ANSI line.
///
/// Walks the line character by character, tracking SGR foreground color
/// changes, and returns the color in effect at the position where `needle`
/// is found. Returns `None` if `needle` is not found or no foreground color
/// is active at that position.
///
/// # Example
///
/// ```
/// use kitty_test_harness::utils::screen::fg_color_at_text;
///
/// let line = "\x1b[38;2;100;100;100mhello world\x1b[m";
/// assert_eq!(fg_color_at_text(line, "hello"), Some((100, 100, 100)));
/// ```
pub fn fg_color_at_text(raw_line: &str, needle: &str) -> Option<(u8, u8, u8)> {
	let mut current_fg: Option<(u8, u8, u8)> = None;
	let mut visible = String::new();
	let chars: Vec<char> = raw_line.chars().collect();
	let mut i = 0;

	while i < chars.len() {
		if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
			let start = i;
			while i < chars.len() && chars[i] != 'm' {
				i += 1;
			}
			if i < chars.len() {
				let seq: String = chars[start..=i].iter().collect();
				if let Some(parsed) = AnsiColor::parse_seq(&seq) {
					if parsed.is_foreground {
						current_fg = parsed.rgb;
					}
				}
				if seq == "\x1b[m" || seq == "\x1b[0m" {
					current_fg = None;
				}
			}
			i += 1;
		} else {
			visible.push(chars[i]);
			if visible.ends_with(needle) {
				return current_fg;
			}
			i += 1;
		}
	}

	None
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_find_vertical_separator() {
		let screen = "left  │right\n\
		              text  │more\n\
		              here  │data\n\
		              foo   │bar\n\
		              a     │b\n\
		              c     │d";
		assert_eq!(find_vertical_separator_col(screen), Some(6));
	}

	#[test]
	fn test_find_horizontal_separator() {
		let screen = "top content here\n\
		              ────────────────\n\
		              bottom text here";
		assert_eq!(find_horizontal_separator_row(screen), Some(1));
	}

	#[test]
	fn test_separator_rows_at_col() {
		let screen = "a│b\nc│d\ne f";
		let rows = find_separator_rows_at_col(screen, 1);
		assert_eq!(rows, vec![0, 1]);
	}

	#[test]
	fn test_extract_colors_semicolon() {
		let raw = "text\x1b[38;2;255;128;64mcolored\x1b[m";
		let colors = extract_row_colors(raw, 0);
		assert_eq!(colors.len(), 1);
		assert!(colors[0].contains("38;2;255;128;64"));
	}

	#[test]
	fn test_extract_colors_colon() {
		let raw = "text\x1b[38:2:255:128:64mcolored\x1b[m";
		let colors = extract_row_colors(raw, 0);
		assert_eq!(colors.len(), 1);
		assert!(colors[0].contains("38:2:255:128:64"));
	}

	#[test]
	fn test_parse_rgb_color() {
		let seq = "\x1b[38;2;255;128;64m";
		let color = AnsiColor::parse_seq(seq).unwrap();
		assert!(color.is_foreground);
		assert_eq!(color.rgb, Some((255, 128, 64)));
		assert_eq!(color.palette_index, None);
	}

	#[test]
	fn test_parse_palette_color() {
		let seq = "\x1b[38;5;196m";
		let color = AnsiColor::parse_seq(seq).unwrap();
		assert!(color.is_foreground);
		assert_eq!(color.rgb, None);
		assert_eq!(color.palette_index, Some(196));
	}

	#[test]
	fn test_parse_kitty_format() {
		let seq = "\x1b[38:2:100:150:200m";
		let color = AnsiColor::parse_seq(seq).unwrap();
		assert!(color.is_foreground);
		assert_eq!(color.rgb, Some((100, 150, 200)));
	}
}
