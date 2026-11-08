//! A utility program for building Rust projects and discovering the resulting executable path.
//! 
//! This module uses async IO and process management to execute `cargo build --message-format=json`,
//! stream its output, and parse the JSON artifacts to find the absolute path of the built binary.
//! This is typically useful for debuggers or IDE extensions (like a Zed debugger integration) 
//! that need to build code and immediately launch a debugger on the resulting binary.

use macro_rules_attribute::apply;
use serde_json::Value;
use smol::{
    io::AsyncReadExt,
    process::{Command, Stdio},
};
use smol_macros::main;
use std::io;

/// Runs `cargo build --message-format=json` as an asynchronous subprocess.
/// Parses the stdout from Cargo, which is a stream of JSON objects,
/// to find and return the path to the compiled executable.
///
/// Returns:
/// - `Ok(Some(String))` containing the executable path if compilation succeeded and an executable was built.
/// - `Ok(None)` if compilation failed or no executable was found in the output.
/// - `Err` if the subprocess failed to start or there was an IO error reading from it.
async fn run_cargo_build_json() -> io::Result<Option<String>> {
    let mut child = Command::new("cargo")
        .args(["build", "--message-format=json"])
        .stdout(Stdio::piped())
        .spawn()?;

    let mut output = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        stdout.read_to_string(&mut output).await?;
    }

    let status = child.status().await?;
    if !status.success() {
        return Ok(None);
    }

    let executable = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .find_map(|json: Value| {
            json.get("executable")
                .and_then(Value::as_str)
                .map(String::from)
        });

    Ok(executable)
}

/// Entry point for the application.
/// It triggers the async cargo build function and prints the resulting executable path.
#[apply(main!)]
async fn main() {
    match run_cargo_build_json().await.unwrap() {
        Some(executable) => println!("Found executable at: {}", executable),
        None => println!("No executable found"),
    }
}
