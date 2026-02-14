//! Minimal kitty integration smoke test.

#![allow(unused_crate_dependencies)]

use std::path::PathBuf;
use std::time::Duration;

use kitty_test_harness::{
    require_kitty, wait_for_ready_marker, wait_for_screen_text, with_kitty_capture,
};

#[test]
fn kitty_smoke_capture_when_available() {
    if !require_kitty() {
        return;
    }

    let working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let marker = "__KITTY_SMOKE_OK__";

    let output = with_kitty_capture(&working_dir, "bash", |kitty| {
        wait_for_ready_marker(kitty);
        kitty.send_text(&format!("echo '{marker}'\n"));
        wait_for_screen_text(kitty, Duration::from_secs(3), |text| text.contains(marker))
    });

    assert!(
        output.contains(marker),
        "expected smoke marker in kitty screen output, got:\n{output}"
    );
}
