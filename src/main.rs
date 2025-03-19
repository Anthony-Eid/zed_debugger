use std::collections::HashMap;
use std::io::{self, Read};
use std::process;
use std::process::{Command, Stdio};
use serde_json::Value;

fn run_cargo_build_json() -> io::Result<Option<String>> {
    let mut cmd = Command::new("cargo")
        .args(["build", "--message-format=json"])
        .stdout(Stdio::piped())
        .spawn()?;

    let mut output = String::new();
    if let Some(mut stdout) = cmd.stdout.take() {
        stdout.read_to_string(&mut output)?;
    }

    let _status = cmd.wait()?;
    
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

fn main() {
    match run_cargo_build_json().unwrap() {
        Some(executable) => println!("Found executable at: {}", executable),
        None => println!("No executable found"),
    }
}
