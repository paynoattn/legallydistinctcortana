//! Repo tooling run through `cargo xtask <command>` (alias in `.cargo/config.toml`).
//!
//! Commands: `crap`, `arch`, `comments`, `mutation-score`, `free-port`. See README "Gates".
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "xtask is a CLI; printing reports is its interface"
)]

mod arch;
mod comments;
mod complexity;
mod crap;
mod lcov;
mod mutation;

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// A gate result: `Ok` passes, `Err` carries the message explaining what to fix.
type Outcome = Result<(), String>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Outcome {
    let root = workspace_root();
    match args.first().map(String::as_str) {
        Some("crap") => crap::run(&root, &option(args, "--lcov")?, number(args, "--max", 6.0)?),
        Some("arch") => arch::run(&root, &root.join(option(args, "--rules")?)),
        Some("comments") => comments::run(&root),
        Some("mutation-score") => mutation::run(
            &root.join(option(args, "--outcomes")?),
            number(args, "--min", 90.0)?,
        ),
        Some("free-port") => free_port(),
        _ => Err(USAGE.to_owned()),
    }
}

const USAGE: &str = "usage: cargo xtask <command>
  crap --lcov <path> [--max 6]              CRAP score per function, fail above max
  arch --rules <path>                       check crate dependencies against the rules file
  comments                                  fail on any comment that is not a doc comment
  mutation-score --outcomes <path> [--min 90]  fail if the mutation score is below min
  free-port                                 print an unused local TCP port";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(PathBuf::new, Path::to_path_buf)
}

fn option(args: &[String], flag: &str) -> Result<String, String> {
    args.iter()
        .skip_while(|arg| *arg != flag)
        .nth(1)
        .cloned()
        .ok_or_else(|| format!("missing {flag} <value>\n{USAGE}"))
}

fn number(args: &[String], flag: &str, default: f64) -> Result<f64, String> {
    if !args.iter().any(|arg| arg == flag) {
        return Ok(default);
    }
    let raw = option(args, flag)?;
    raw.parse()
        .map_err(|_| format!("{flag} must be a number, got {raw:?}"))
}

fn free_port() -> Outcome {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("bind failed: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("no local address: {e}"))?
        .port();
    println!("{port}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(raw: &[&str]) -> Vec<String> {
        raw.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn option_reads_the_value_after_the_flag() {
        assert_eq!(
            option(&args(&["crap", "--lcov", "x.info"]), "--lcov"),
            Ok("x.info".into())
        );
        assert!(
            option(&args(&["crap"]), "--lcov").is_err(),
            "missing flag is an error"
        );
    }

    #[test]
    fn number_defaults_and_parses() {
        assert!((number(&args(&["crap"]), "--max", 6.0).unwrap() - 6.0).abs() < f64::EPSILON);
        assert!(
            (number(&args(&["--max", "4.5"]), "--max", 6.0).unwrap() - 4.5).abs() < f64::EPSILON
        );
        assert!(
            number(&args(&["--max", "six"]), "--max", 6.0).is_err(),
            "non-number rejected"
        );
    }

    #[test]
    fn unknown_command_prints_usage() {
        assert_eq!(run(&args(&["nope"])), Err(USAGE.to_owned()));
    }
}
