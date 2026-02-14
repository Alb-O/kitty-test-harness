//! Module for utility functions and helpers for the kitty test harness.

/// Helpers for environment detection and test gating.
pub mod env;
/// Terminal key encoding helpers and common key constants.
pub mod keys;
/// Test logging utilities for debugging.
pub mod log;
/// Mouse event encoding and sending.
pub mod mouse;
/// Common testing patterns (mock executables, env wrappers, etc.).
pub mod patterns;
/// Recording replay for automated session testing.
pub mod replay;
/// Window resize utilities.
pub mod resize;
/// Screen content parsing (separators, ANSI colors, etc.).
pub mod screen;
/// Helpers for waiting for certain conditions in the kitty harness.
pub mod wait;
/// Helpers for managing kitty windows and panels.
pub mod window;
