//! Module for utility functions and helpers for the kitty test harness.

/// Helpers for environment detection and test gating.
pub mod env;
/// Terminal key encoding helpers and common key constants.
pub mod keys;
/// Mouse event encoding and sending.
pub mod mouse;
/// Common testing patterns (mock executables, env wrappers, etc.).
pub mod patterns;
/// Screen content parsing (separators, ANSI colors, etc.).
pub mod screen;
/// Helpers for waiting for certain conditions in the kitty harness.
pub mod wait;
/// Helpers for managing kitty windows and panels.
pub mod window;
