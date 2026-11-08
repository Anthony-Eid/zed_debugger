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

/// Ignore diagnostics and library artifacts; select this package's binary.
fn find_executable(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let message: Value = serde_json::from_str(line).ok()?;
        if message["reason"] != "compiler-artifact"
            || message["target"]["name"] != "zed_debugger"
            || !message["target"]["kind"]
                .as_array()?
                .iter()
                .any(|kind| kind == "bin")
        {
            return None;
        }
        message["executable"].as_str().map(String::from)
    })
}

/// Runs `cargo build --message-format=json` as an asynchronous subprocess.
/// Parses the stdout from Cargo, which is a stream of JSON objects,
/// to find and return the path to the compiled executable.
///
/// Returns:
/// - `Ok(String)` containing the executable path if compilation succeeded and an executable was built.
/// - `Err` if the subprocess failed to start, there was an IO error reading from it, or no executable was found in the output.
async fn run_cargo_build_json() -> io::Result<String> {
    let mut child = Command::new("cargo")
        .args(["build", "--bin", "zed_debugger", "--message-format=json"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdout(Stdio::piped())
        .spawn()?;

    let mut output = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        stdout.read_to_string(&mut output).await?;
    }

    let status = child.status().await?;
    if !status.success() {
        return Err(io::Error::other(format!("cargo build failed: {status}")));
    }

    find_executable(&output).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "cargo emitted no binary executable",
        )
    })
}

/// Entry point for the application.
/// It triggers the async cargo build function and prints the resulting executable path.
#[apply(main!)]
async fn main() -> io::Result<()> {
    let executable = run_cargo_build_json().await?;
    println!("Found executable at: {executable}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::find_executable;
    use serde_json::json;

    #[test]
    fn selects_binary_after_unrelated_output() {
        let artifact = json!({
            "reason": "compiler-artifact",
            "target": {"name": "zed_debugger", "kind": ["bin"]},
            "executable": "/tmp/zed_debugger"
        });
        let output = format!("not json\n{{\"reason\":\"build-finished\"}}\n{artifact}\n");
        assert_eq!(
            find_executable(&output).as_deref(),
            Some("/tmp/zed_debugger")
        );
    }

    #[test]
    fn ignores_libraries_and_other_binaries() {
        for (name, kind) in [("zed_debugger", "lib"), ("other_example", "bin")] {
            let artifact = json!({
                "reason": "compiler-artifact",
                "target": {"name": name, "kind": [kind]},
                "executable": "/tmp/unrelated"
            });
            assert_eq!(find_executable(&artifact.to_string()), None);
        }
    }

    #[test]
    fn handles_empty_output_and_null_executables() {
        assert_eq!(find_executable(""), None);
        let artifact = json!({
            "reason": "compiler-artifact",
            "target": {"name": "zed_debugger", "kind": ["bin"]},
            "executable": null
        });
        assert_eq!(find_executable(&artifact.to_string()), None);
    }
}
