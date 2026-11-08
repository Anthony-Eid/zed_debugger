# Debugger examples

A small workspace for stepping through code and testing cross-file editing in Delta.
Examples are intentionally deterministic: repeated runs produce the same sample data.

## Run the examples

| Language | Command | What to inspect |
| --- | --- | --- |
| Python 3.10+ | `python3 main.py` | Dataclasses, nested payloads, and helper tests |
| PHP 8.0+ | `php main.php` | Typed score records and gradebook summaries |
| C | `mkdir -p build && cc -std=c11 -Wall -Wextra -Werror main.c -o build/scores && ./build/scores` | Structs, pointers, and loop locals |
| Rust | `cargo run` | Async child process output and Cargo artifact selection |

The Rust example builds this package once as a subprocess, then prints the binary
path. It does not recursively run the resulting executable.

## Check changes

```sh
python3 -m unittest discover -s tests -v
cargo test
cargo fmt --check
php -l main.php
```

Python also runs its ten introductory tests when invoked directly. The separate
unittest suite covers empty inputs, invalid values, and deterministic sample data.
The C and PHP demos both print an average of `68.33` and a pass rate of `0.67`.

## Debugger setup

Zed launch configurations live in `.zed/debug.json`; build the Rust executable
with the **Rust: build** task before launching it. VS Code configurations live in
`.vscode/launch.json`, with build and test tasks in `.vscode/tasks.json`.
Install the appropriate language debugger extension or adapter for your editor.
PHP debugging additionally needs Xdebug enabled in your local PHP installation.
Attach configurations select a running process rather than storing a machine-specific PID.

## Sample documents

- `src/markdown_tester.md`: a rendering and debugger walkthrough.
- `src/project_scores.csv`: project-review data; an empty cell means missing data.
- `src/project_scores.tsv`: a tab-separated companion fixture.
- `src/test.csv`: fictional account data with ISO dates and boolean activity flags.
- `snippet_test.txt`: a short snippet-expansion checklist.

## Global provenance exercise

Review the working changes across language implementations, editor settings, tests,
and fixtures. Inspect attribution on both changed and unchanged lines, then make a
small manual edit to compare it with the agent-authored changes.

Generated logs, Python bytecode, and build outputs are not part of this exercise.
Ignore rules prevent new artifacts from being added; existing tracked artifacts
are left untouched.
