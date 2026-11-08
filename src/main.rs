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

/// Ignora los diagnósticos y artefactos de biblioteca; selecciona el binario de este paquete.
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

/// Ejecuta `cargo build --message-format=json` como un subproceso asíncrono.
/// Analiza la salida estándar de Cargo, que es un flujo de objetos JSON,
/// para encontrar y devolver la ruta del ejecutable compilado.
///
/// Devuelve:
/// - `Ok(String)` con la ruta del ejecutable si la compilación tuvo éxito y se creó un ejecutable.
/// - `Err` si el subproceso no se inició, hubo un error de E/S al leerlo o no se encontró un ejecutable en la salida.
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

/// Punto de entrada de la aplicación.
/// Ejecuta la función asíncrona de compilación de Cargo e imprime la ruta del ejecutable resultante.
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
