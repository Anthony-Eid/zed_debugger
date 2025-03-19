use macro_rules_attribute::apply;
use serde_json::Value;
use smol::{
    io::AsyncReadExt,
    process::{Command, Stdio},
};
use smol_macros::main;
use std::io;

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

#[apply(main!)]
async fn main() {
    match run_cargo_build_json().await.unwrap() {
        Some(executable) => println!("Found executable at: {}", executable),
        None => println!("No executable found"),
    }
}
