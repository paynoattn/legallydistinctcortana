//! Minimal LCOV reader: per-file line hit counts (`SF:` and `DA:` records only).

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// Hit count per instrumented line, keyed by the source file path as written in the report.
pub(crate) type LineHits = HashMap<PathBuf, BTreeMap<usize, u64>>;

pub(crate) fn parse(report: &str) -> LineHits {
    let mut files = LineHits::new();
    let mut current: Option<PathBuf> = None;
    for record in report.lines() {
        if let Some(path) = record.strip_prefix("SF:") {
            current = Some(PathBuf::from(path));
        } else if let (Some(path), Some((line, hits))) =
            (&current, record.strip_prefix("DA:").and_then(line_hits))
        {
            *files
                .entry(path.clone())
                .or_default()
                .entry(line)
                .or_default() += hits;
        }
    }
    files
}

fn line_hits(data: &str) -> Option<(usize, u64)> {
    let mut fields = data.split(',');
    let line = fields.next()?.parse().ok()?;
    let hits = fields.next()?.parse().ok()?;
    Some((line, hits))
}

/// `(covered, instrumented)` line counts for lines `first..=last`.
pub(crate) fn coverage_between(
    hits: &BTreeMap<usize, u64>,
    first: usize,
    last: usize,
) -> (u32, u32) {
    let lines: Vec<u64> = hits.range(first..=last).map(|(_, count)| *count).collect();
    let covered = lines.iter().filter(|count| **count > 0).count();
    (to_u32(covered), to_u32(lines.len()))
}

fn to_u32(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT: &str = "TN:\nSF:/src/a.rs\nFN:1,f\nDA:1,3\nDA:2,0\nDA:4,1\nend_of_record\nSF:/src/b.rs\nDA:7,0\nend_of_record\n";

    #[test]
    fn reads_line_hits_per_file() {
        let files = parse(REPORT);
        let a = &files[&PathBuf::from("/src/a.rs")];
        assert_eq!(
            a.iter().map(|(l, h)| (*l, *h)).collect::<Vec<_>>(),
            vec![(1, 3), (2, 0), (4, 1)]
        );
        assert_eq!(files[&PathBuf::from("/src/b.rs")][&7], 0);
    }

    #[test]
    fn sums_duplicate_line_records() {
        let files = parse("SF:/x.rs\nDA:1,2\nDA:1,3\nDA:2,bad\n");
        assert_eq!(
            files[&PathBuf::from("/x.rs")]
                .clone()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![(1, 5)]
        );
    }

    #[test]
    fn ignores_data_before_any_file() {
        assert!(parse("DA:1,1\n").is_empty(), "no SF record yet");
    }

    #[test]
    fn counts_covered_lines_in_an_inclusive_range() {
        let files = parse(REPORT);
        let a = &files[&PathBuf::from("/src/a.rs")];
        assert_eq!(coverage_between(a, 1, 4), (2, 3));
        assert_eq!(coverage_between(a, 2, 2), (0, 1));
        assert_eq!(coverage_between(a, 5, 9), (0, 0));
    }
}
