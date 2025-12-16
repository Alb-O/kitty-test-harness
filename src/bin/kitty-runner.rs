//! A simple wrapper to run a command under `kitty --dump-commands=yes` and filter its output to only visible text.

#![allow(unused_crate_dependencies)]

use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Escape shell arguments for safe use in bash -c
fn shell_escape(args: &[String]) -> String {
	args.iter()
		.map(|arg| {
			if arg
				.chars()
				.all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/' || c == '.')
			{
				arg.clone()
			} else {
				format!("'{}'", arg.replace('\'', r"'\''"))
			}
		})
		.collect::<Vec<_>>()
		.join(" ")
}

/// Filter out kitty and graphics library noise from stderr
fn should_filter_stderr(line: &str) -> bool {
	// Filter out common kitty/graphics library warnings and errors
	let filters = [
		"libEGL warning:",
		"MESA:",
		"libEGL error:",
		"[glfw error",
		"glfw error",
		"process_desktop_settings:",
		"org.freedesktop.DBus.Error",
		"org.freedesktop.portal.Desktop",
		"org.freedesktop.Notifications",
		"MESA-LOADER:",
		"ZINK:",
		"egl:",
		"dri2 screen",
	];

	filters.iter().any(|filter| line.contains(filter))
}

/// Trim blank lines from the beginning and end of a vector of lines
fn trim_blank_lines(lines: &[String]) -> &[String] {
	let start = lines
		.iter()
		.position(|line| !line.trim().is_empty())
		.unwrap_or(0);
	let end = lines
		.iter()
		.rposition(|line| !line.trim().is_empty())
		.map(|pos| pos + 1)
		.unwrap_or(0);

	if start < end { &lines[start..end] } else { &[] }
}

fn main() -> io::Result<()> {
	let args: Vec<String> = env::args().skip(1).collect();

	if args.is_empty() {
		eprintln!("Usage: kitty-runner <command> [args...]");
		eprintln!("Example: kitty-runner cargo test");
		std::process::exit(1);
	}

	// Create a wrapper script that runs the command and reports the exit code
	// We use a special marker to find the exit code in kitty's output
	let wrapper_script = format!(
		r#"{}; EXIT_CODE=$?; echo "KITTY_RUNNER_EXIT_CODE:$EXIT_CODE"; exit $EXIT_CODE"#,
		shell_escape(&args)
	);

	// Spawn the command with kitty --dump-commands=yes wrapping
	let mut kitty_cmd = Command::new("kitty");
	kitty_cmd
		.arg("--dump-commands=yes")
		.arg("bash")
		.arg("-c")
		.arg(&wrapper_script)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped());

	let mut child = kitty_cmd.spawn()?;

	// Process stdout
	let stdout_handle = if let Some(stdout) = child.stdout.take() {
		let reader = BufReader::new(stdout);
		Some(std::thread::spawn(move || {
			let mut output = String::new();
			let mut exit_code: Option<i32> = None;

			for line in reader.lines().map_while(Result::ok) {
				if line.starts_with("draw ") {
					// Extract the text after "draw " and add it to output
					if let Some(text) = line.strip_prefix("draw ") {
						// Check for our exit code marker
						if let Some(code_str) = text.strip_prefix("KITTY_RUNNER_EXIT_CODE:") {
							exit_code = code_str.parse().ok();
						} else {
							output.push_str(text);
						}
					}
				} else if line == "screen_linefeed" {
					// Add a newline when we see a linefeed command
					output.push('\n');
				}
				// Ignore screen_carriage_return and other commands
			}
			(output, exit_code)
		}))
	} else {
		None
	};

	// Process stderr (filter out kitty/graphics library noise)
	let stderr_handle = if let Some(stderr) = child.stderr.take() {
		let reader = BufReader::new(stderr);
		Some(std::thread::spawn(move || {
			let mut lines = Vec::new();
			for line in reader.lines().map_while(Result::ok) {
				// Filter out kitty and graphics library warnings/errors
				if should_filter_stderr(&line) {
					continue;
				}
				lines.push(line);
			}
			lines
		}))
	} else {
		None
	};

	// Wait for the process to complete
	child.wait()?;

	// Wait for stderr thread and print filtered output
	if let Some(handle) = stderr_handle
		&& let Ok(lines) = handle.join()
	{
		// Trim blank lines from beginning and end
		let trimmed_lines = trim_blank_lines(&lines);
		for line in trimmed_lines {
			eprintln!("{}", line);
		}
	}

	// Get the filtered output and the actual exit code
	let actual_exit_code = if let Some(handle) = stdout_handle
		&& let Ok((output, exit_code)) = handle.join()
	{
		// Trim leading and trailing blank lines from output
		let trimmed_output = output.trim_matches(|c| c == '\n' || c == '\r');

		if !trimmed_output.is_empty() {
			let stdout = io::stdout();
			let mut handle = stdout.lock();
			writeln!(handle, "{}", trimmed_output)?;
			handle.flush()?;
		}
		exit_code
	} else {
		None
	};

	// Exit with the captured exit code from the actual command
	std::process::exit(actual_exit_code.unwrap_or(1))
}
