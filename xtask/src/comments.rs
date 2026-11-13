//! Bans comments that are not doc comments: `// ...` and `/* ... */`.
//!
//! Agents copy comments and use them to justify workarounds instead of fixing the
//! code. Doc comments (`///`, `//!`) stay because `missing_docs` requires them on every
//! public item.
//!
//! A comment is whatever text the tokenizer skips. Each file is tokenized with
//! `proc_macro2`, which turns doc comments into `#[doc = "..."]` tokens that keep the
//! comment's source span and drops every other comment. Any non-whitespace character
//! outside all token spans is therefore a comment. Strings, raw strings, chars, and
//! lifetimes are tokens, so `"// not a comment"` is never reported.

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{LineColumn, TokenStream, TokenTree};

use crate::Outcome;

const SOURCE_DIRS: [&str; 2] = ["crates", "xtask"];

/// Checks every `.rs` file under `crates/` and `xtask/` and fails listing each comment.
pub(crate) fn run(root: &Path) -> Outcome {
    let mut files = Vec::new();
    for dir in SOURCE_DIRS {
        collect_rust_files(&root.join(dir), &mut files)?;
    }
    files.sort();
    let mut findings = Vec::new();
    for file in &files {
        let relative = file
            .strip_prefix(root)
            .unwrap_or(file)
            .display()
            .to_string();
        let source = fs::read_to_string(file).map_err(|error| format!("{relative}: {error}"))?;
        let lines = comment_lines(&source).map_err(|error| format!("{relative}: {error}"))?;
        findings.extend(lines.into_iter().map(|line| format!("  {relative}:{line}")));
    }
    if findings.is_empty() {
        println!("comments: {} files, none outside doc comments", files.len());
        return Ok(());
    }
    Err(format!(
        "comments: {} line(s) with comments. Comments are banned: agents copy them and use \
         them to justify workarounds. Rename, extract, or type the code until it explains \
         itself, and encode constraints as types, tests, or lints.\n{}",
        findings.len(),
        findings.join("\n")
    ))
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) -> Outcome {
    let entries = fs::read_dir(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("{}: {error}", dir.display()))?
            .path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

/// One-based line numbers that hold comment text, in order.
pub(crate) fn comment_lines(source: &str) -> Result<Vec<usize>, String> {
    let tokens = TokenStream::from_str(source).map_err(|error| error.to_string())?;
    let mut covered: Vec<Vec<bool>> = source
        .lines()
        .map(|line| vec![false; line.chars().count()])
        .collect();
    cover_stream(tokens, &mut covered);
    Ok(source
        .lines()
        .zip(&covered)
        .enumerate()
        .filter(|(_, (text, mask))| has_uncovered_text(text, mask))
        .map(|(index, _)| index + 1)
        .collect())
}

fn has_uncovered_text(text: &str, mask: &[bool]) -> bool {
    text.chars()
        .zip(mask)
        .any(|(character, covered)| !covered && !character.is_whitespace())
}

fn cover_stream(stream: TokenStream, covered: &mut [Vec<bool>]) {
    for tree in stream {
        if let TokenTree::Group(group) = &tree {
            cover(group.span_open().start(), group.span_open().end(), covered);
            cover(
                group.span_close().start(),
                group.span_close().end(),
                covered,
            );
            cover_stream(group.stream(), covered);
        } else {
            cover(tree.span().start(), tree.span().end(), covered);
        }
    }
}

fn cover(start: LineColumn, end: LineColumn, covered: &mut [Vec<bool>]) {
    let first = start.line.saturating_sub(1);
    for (index, line) in covered
        .iter_mut()
        .enumerate()
        .skip(first)
        .take(end.line - first)
    {
        let from = if index == first { start.column } else { 0 };
        let to = if index + 1 == end.line {
            end.column
        } else {
            line.len()
        };
        line.iter_mut()
            .take(to)
            .skip(from)
            .for_each(|cell| *cell = true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(source: &str) -> Vec<usize> {
        comment_lines(source).expect("source tokenizes")
    }

    #[test]
    fn reports_line_comments() {
        assert_eq!(
            lines("// leading\nfn a() {} // trailing\nfn b() {}\n"),
            vec![1, 2],
            "both line comments are reported"
        );
    }

    #[test]
    fn reports_every_line_of_a_block_comment() {
        assert_eq!(
            lines("fn a() {}\n/* one\n   two */\nfn b() { /* inline */ }\n"),
            vec![2, 3, 4],
            "block comments are reported on each line they span"
        );
    }

    #[test]
    fn allows_doc_comments() {
        let source = "//! Crate docs.\n//! More.\n\n/// Item docs.\n/// More.\nfn a() {}\n\n\
                      /** Block docs. */\nfn b() {}\n";
        assert_eq!(
            lines(source),
            Vec::<usize>::new(),
            "doc comments are allowed"
        );
    }

    #[test]
    fn ignores_comment_markers_inside_literals() {
        let source = "const A: &str = \"// no\";\nconst B: &str = r#\"/* no */\"#;\n\
                      const C: &str = \"multi\n// line\n\";\nfn f<'a>(x: &'a str) -> char { '/' }\n";
        assert_eq!(lines(source), Vec::<usize>::new(), "literals are tokens");
    }

    #[test]
    fn handles_multibyte_text_before_a_comment() {
        assert_eq!(
            lines("const S: &str = \"héllo\"; // note\n"),
            vec![1],
            "columns count characters"
        );
    }

    #[test]
    fn rejects_source_that_does_not_tokenize() {
        assert!(comment_lines("fn a() {").is_err(), "unbalanced braces fail");
    }

    #[test]
    fn the_workspace_has_no_comments() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask lives in the workspace");
        assert_eq!(run(root), Ok(()), "the starter itself passes the gate");
    }
}
