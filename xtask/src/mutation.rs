//! Mutation score gate over cargo-mutants' `outcomes.json`.
//!
//! Score = caught / (caught + missed + timeout). Unviable mutants (ones that do not
//! compile) prove nothing either way and are left out. Timeouts count as survivors: a
//! mutant that only makes the tests hang was not caught by an assertion.

use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::Outcome;

#[derive(Debug, Deserialize)]
struct Summary {
    caught: u32,
    missed: u32,
    timeout: u32,
    unviable: u32,
}

pub(crate) fn run(outcomes: &Path, min: f64) -> Outcome {
    let text = fs::read_to_string(outcomes).map_err(|e| {
        format!(
            "cannot read {}: {e}. Did cargo mutants run?",
            outcomes.display()
        )
    })?;
    let summary: Summary =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", outcomes.display()))?;
    let verdict = judge(&summary, min);
    match &verdict {
        Ok(message) => println!("{message}"),
        Err(_) => print_survivors(outcomes),
    }
    verdict.map(drop)
}

fn judge(summary: &Summary, min: f64) -> Result<String, String> {
    let Summary {
        caught,
        missed,
        timeout,
        unviable,
    } = *summary;
    let viable = caught + missed + timeout;
    if viable == 0 {
        return Ok(format!(
            "mutate: ok, no viable mutants to test ({unviable} unviable)"
        ));
    }
    let score = 100.0 * f64::from(caught) / f64::from(viable);
    let line = format!(
        "mutation score {score:.1}% (caught {caught}, missed {missed}, timeout {timeout}; {unviable} unviable excluded), minimum {min}%"
    );
    if score >= min {
        Ok(format!("mutate: ok, {line}"))
    } else {
        Err(format!("mutate: FAILED, {line}"))
    }
}

fn print_survivors(outcomes: &Path) {
    let dir = outcomes.parent().unwrap_or_else(|| Path::new("."));
    for list in ["missed.txt", "timeout.txt"] {
        let survivors = fs::read_to_string(dir.join(list)).unwrap_or_default();
        for mutant in survivors.lines() {
            println!("  survived: {mutant}");
        }
    }
    println!(
        "Kill each survivor with a test that fails on the mutated code (see the hardener skill)."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary([caught, missed, timeout, unviable]: [u32; 4]) -> Summary {
        Summary {
            caught,
            missed,
            timeout,
            unviable,
        }
    }

    #[test]
    fn passes_at_exactly_the_minimum() {
        assert!(judge(&summary([9, 1, 0, 5]), 90.0).is_ok(), "90% meets 90%");
    }

    #[test]
    fn fails_just_below_the_minimum() {
        let verdict = judge(&summary([8, 1, 1, 0]), 90.0);
        assert_eq!(
            verdict,
            Err("mutate: FAILED, mutation score 80.0% (caught 8, missed 1, timeout 1; 0 unviable excluded), minimum 90%".into())
        );
    }

    #[test]
    fn unviable_mutants_do_not_count() {
        assert!(
            judge(&summary([9, 1, 0, 1000]), 90.0).is_ok(),
            "unviable excluded from score"
        );
        assert!(
            judge(&summary([0, 0, 0, 3]), 90.0).is_ok(),
            "nothing viable to test"
        );
    }

    #[test]
    fn reads_cargo_mutants_summary_fields() {
        let parsed: Summary =
            serde_json::from_str(r#"{"total_mutants":3,"caught":1,"missed":1,"timeout":0,"unviable":1,"success":0,"outcomes":[]}"#)
                .unwrap();
        assert_eq!(
            (
                parsed.caught,
                parsed.missed,
                parsed.timeout,
                parsed.unviable
            ),
            (1, 1, 0, 1)
        );
    }
}
