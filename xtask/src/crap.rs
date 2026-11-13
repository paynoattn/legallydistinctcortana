//! `CRAP = complexity² × (1 − coverage)³ + complexity`, per function in `crates/*/src`.
//!
//! Coverage is the fraction of the function's instrumented lines that ran, from the LCOV
//! report. A function in a file the report does not cover (for example a file excluded
//! from coverage) counts as 0% covered, so exclusions cannot hide complex code.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::Outcome;
use crate::complexity::{self, Function};
use crate::lcov::{self, LineHits};

const SHOWN: usize = 10;

#[derive(Debug)]
struct Score {
    location: String,
    name: String,
    complexity: u32,
    coverage: f64,
    crap: f64,
}

pub(crate) fn run(root: &Path, lcov_path: &str, max: f64) -> Outcome {
    let scores = score_workspace(root, lcov_path)?;
    print_table(
        &format!("Worst {SHOWN} of {} functions by CRAP:", scores.len()),
        scores.iter().take(SHOWN),
    );
    let over: Vec<&Score> = scores.iter().filter(|s| s.crap > max).collect();
    let branchy_new: Vec<&Score> = scores.iter().filter(|s| is_branchy_new(s)).collect();
    if over.is_empty() && branchy_new.is_empty() {
        println!("crap: ok, every function is at or below CRAP_MAX={max}");
        return Ok(());
    }
    if !over.is_empty() {
        let title = format!("{} functions above CRAP_MAX={max}:", over.len());
        print_table(&title, over.into_iter());
        println!(
            "Add tests for the uncovered branches, or split the function to lower its complexity."
        );
    }
    if !branchy_new.is_empty() {
        print_table(
            "Functions named `new` with branches:",
            branchy_new.into_iter(),
        );
        println!(
            "cargo-mutants never mutates functions named `new`, so their logic goes untested by \
             `make mutate`. Keep `new` branch-free; move validation to a named constructor (e.g. `parse`)."
        );
    }
    Err("crap: FAILED".into())
}

fn score_workspace(root: &Path, lcov_path: &str) -> Result<Vec<Score>, String> {
    let report = fs::read_to_string(root.join(lcov_path))
        .map_err(|e| format!("cannot read {lcov_path}: {e}. Run `make coverage` first."))?;
    let hits = canonical_keys(lcov::parse(&report));
    let mut scores = Vec::new();
    for file in source_files(&root.join("crates"))? {
        scores.extend(score_file(root, &file, &hits)?);
    }
    scores.sort_by(|a, b| b.crap.total_cmp(&a.crap));
    Ok(scores)
}

fn is_branchy_new(score: &Score) -> bool {
    let is_new = score.name == "new" || score.name.ends_with("::new");
    is_new && score.complexity > 1
}

pub(crate) fn crap(complexity: u32, coverage: f64) -> f64 {
    let c = f64::from(complexity);
    c * c * (1.0 - coverage).powi(3) + c
}

fn score_file(root: &Path, file: &Path, hits: &LineHits) -> Result<Vec<Score>, String> {
    let source = fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let functions =
        complexity::functions(&source).map_err(|e| format!("{}: {e}", file.display()))?;
    let relative = file
        .strip_prefix(root)
        .unwrap_or(file)
        .display()
        .to_string();
    let empty = BTreeMap::new();
    let file_hits = hits.get(file).unwrap_or(&empty);
    Ok(functions
        .into_iter()
        .map(|f| score(&relative, f, file_hits))
        .collect())
}

fn score(file: &str, function: Function, hits: &BTreeMap<usize, u64>) -> Score {
    let (covered, instrumented) =
        lcov::coverage_between(hits, function.first_line, function.last_line);
    let coverage = if instrumented == 0 {
        0.0
    } else {
        f64::from(covered) / f64::from(instrumented)
    };
    Score {
        location: format!("{file}:{}", function.first_line),
        name: function.name,
        complexity: function.complexity,
        coverage,
        crap: crap(function.complexity, coverage),
    }
}

fn print_table<'a>(title: &str, rows: impl Iterator<Item = &'a Score>) {
    println!("{title}");
    println!("  {:>6}  {:>4}  {:>5}  function", "CRAP", "CC", "cov%");
    for row in rows {
        println!(
            "  {:>6.1}  {:>4}  {:>5.1}  {} ({})",
            row.crap,
            row.complexity,
            row.coverage * 100.0,
            row.name,
            row.location
        );
    }
}

fn canonical_keys(hits: LineHits) -> LineHits {
    hits.into_iter()
        .map(|(path, lines)| (fs::canonicalize(&path).unwrap_or(path), lines))
        .collect()
}

/// Every `.rs` file under `crates/*/src`, canonicalized to match the report's paths.
fn source_files(crates: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in read_dir(crates)? {
        let src = entry.join("src");
        if src.is_dir() {
            collect_rust_files(&src, &mut files)?;
        }
    }
    files.sort();
    Ok(files)
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for path in read_dir(dir)? {
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(fs::canonicalize(&path).map_err(|e| format!("{}: {e}", path.display()))?);
        }
    }
    Ok(())
}

fn read_dir(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    entries
        .map(|entry| entry.map(|e| e.path()).map_err(|e| e.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(complexity: u32) -> Function {
        Function {
            name: "f".into(),
            first_line: 2,
            last_line: 5,
            complexity,
        }
    }

    #[test]
    fn fully_covered_scores_its_complexity() {
        assert!((crap(4, 1.0) - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn uncovered_scores_complexity_squared_plus_complexity() {
        assert!((crap(4, 0.0) - 20.0).abs() < f64::EPSILON);
        assert!((crap(2, 0.5) - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn scores_from_lines_inside_the_function_only() {
        let hits = BTreeMap::from([(1, 0), (2, 1), (3, 0), (5, 1), (6, 0)]);
        let scored = score("a.rs", function(2), &hits);
        assert!(
            (scored.coverage - 2.0 / 3.0).abs() < 1e-9,
            "coverage {}",
            scored.coverage
        );
        assert_eq!(scored.location, "a.rs:2");
    }

    #[test]
    fn constructors_named_new_must_be_branch_free() {
        let named = |name: &str, complexity| Score {
            location: String::new(),
            name: name.into(),
            complexity,
            coverage: 1.0,
            crap: 1.0,
        };
        assert!(is_branchy_new(&named("Title::new", 2)), "branchy method");
        assert!(is_branchy_new(&named("new", 2)), "branchy free function");
        assert!(
            !is_branchy_new(&named("Title::new", 1)),
            "straight-line new is fine"
        );
        assert!(
            !is_branchy_new(&named("Title::renew", 3)),
            "only exact `new`"
        );
    }

    #[test]
    fn functions_without_coverage_data_count_as_uncovered() {
        let scored = score("a.rs", function(3), &BTreeMap::new());
        assert!((scored.crap - 12.0).abs() < f64::EPSILON);
    }
}
