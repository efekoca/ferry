use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineKind {
    Context,
    Add,
    Del,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_no: Option<usize>,
    pub new_no: Option<usize>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    pub old_start: usize,
    pub new_start: usize,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub hunks: Vec<Hunk>,
    pub additions: usize,
    pub deletions: usize,
}

fn strip_eol(s: &str) -> &str {
    s.strip_suffix('\n')
        .map(|s| s.strip_suffix('\r').unwrap_or(s))
        .unwrap_or(s)
}

pub fn compute(old: &str, new: &str, context: usize) -> FileDiff {
    let diff = TextDiff::from_lines(old, new);
    let mut hunks = Vec::new();
    let (mut additions, mut deletions) = (0, 0);
    for group in diff.grouped_ops(context) {
        let mut lines = Vec::new();
        let (mut old_start, mut new_start) = (None, None);
        for op in &group {
            for change in diff.iter_changes(op) {
                let kind = match change.tag() {
                    ChangeTag::Equal => LineKind::Context,
                    ChangeTag::Insert => {
                        additions += 1;
                        LineKind::Add
                    }
                    ChangeTag::Delete => {
                        deletions += 1;
                        LineKind::Del
                    }
                };
                let old_no = change.old_index().map(|i| i + 1);
                let new_no = change.new_index().map(|i| i + 1);
                old_start = old_start.or(old_no);
                new_start = new_start.or(new_no);
                lines.push(DiffLine {
                    kind,
                    old_no,
                    new_no,
                    text: strip_eol(change.value()).to_string(),
                });
            }
        }
        hunks.push(Hunk {
            old_start: old_start.unwrap_or(0),
            new_start: new_start.unwrap_or(0),
            lines,
        });
    }
    FileDiff {
        hunks,
        additions,
        deletions,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchError {
    Malformed,
    Mismatch(usize),
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match *self {
            PatchError::Malformed => crate::t!("diff biçimi tanınmadı", "unrecognized diff format"),
            PatchError::Mismatch(n) => crate::t!(
                "diff, dosyanın şu anki hâliyle eşleşmiyor (parça {n})",
                "the diff doesn't match the file's current contents (hunk {n})"
            ),
        };
        f.write_str(&msg)
    }
}

impl std::error::Error for PatchError {}

struct ParsedHunk {
    old_start_hint: Option<usize>,
    old: Vec<String>,
    new: Vec<String>,
}

fn parse_unified(diff: &str) -> Result<Vec<ParsedHunk>, PatchError> {
    let mut hunks: Vec<ParsedHunk> = Vec::new();
    for line in diff.lines() {
        if line.starts_with("@@") {
            let hint = line
                .split_whitespace()
                .find_map(|p| p.strip_prefix('-'))
                .and_then(|p| p.split(',').next())
                .and_then(|n| n.parse::<usize>().ok());
            hunks.push(ParsedHunk {
                old_start_hint: hint,
                old: Vec::new(),
                new: Vec::new(),
            });
            continue;
        }
        if hunks.is_empty() || line.starts_with("--- ") || line.starts_with("+++ ") {
            continue;
        }
        let hunk = hunks.last_mut().unwrap();
        if line.starts_with('\\') {
            continue;
        }
        match line.split_at_checked(1) {
            Some((" ", rest)) => {
                hunk.old.push(rest.to_string());
                hunk.new.push(rest.to_string());
            }
            Some(("-", rest)) => hunk.old.push(rest.to_string()),
            Some(("+", rest)) => hunk.new.push(rest.to_string()),
            None if line.is_empty() => {
                hunk.old.push(String::new());
                hunk.new.push(String::new());
            }
            _ => return Err(PatchError::Malformed),
        }
    }
    if hunks.is_empty() {
        return Err(PatchError::Malformed);
    }
    Ok(hunks)
}

pub fn apply_unified(original: &str, diff: &str) -> Result<String, PatchError> {
    let hunks = parse_unified(diff)?;
    let eol = if original.contains("\r\n") { "\r\n" } else { "\n" };
    let had_final_newline = original.is_empty() || original.ends_with('\n');
    let lines: Vec<&str> = original.split_inclusive('\n').map(strip_eol).collect();

    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut cursor = 0;
    for (n, hunk) in hunks.iter().enumerate() {
        let matches_at = |start: usize| {
            start + hunk.old.len() <= lines.len()
                && hunk.old.iter().zip(&lines[start..]).all(|(a, b)| a == b)
        };
        let hinted = hunk.old_start_hint.map(|h| h.saturating_sub(1)).filter(|&h| h >= cursor);
        let start = match hinted {
            Some(h) if matches_at(h) => Some(h),
            _ => (cursor..=lines.len().saturating_sub(hunk.old.len())).find(|&s| matches_at(s)),
        };
        let start = start.ok_or(PatchError::Mismatch(n + 1))?;
        out.extend(lines[cursor..start].iter().map(|s| s.to_string()));
        out.extend(hunk.new.iter().cloned());
        cursor = start + hunk.old.len();
    }
    out.extend(lines[cursor..].iter().map(|s| s.to_string()));

    let mut result = out.join(eol);
    if had_final_newline && !out.is_empty() {
        result.push_str(eol);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL: &str = "<?php\n\nfunction f(Foo $a = null)\n{\n    return utf8_encode($a);\n}\n";

    #[test]
    fn computes_numbered_hunks() {
        let new = ORIGINAL.replace("Foo $a", "?Foo $a");
        let d = compute(ORIGINAL, &new, 1);
        assert_eq!((d.additions, d.deletions), (1, 1));
        assert_eq!(d.hunks.len(), 1);
        let h = &d.hunks[0];
        assert_eq!(h.old_start, 2);
        let kinds: Vec<_> = h.lines.iter().map(|l| l.kind).collect();
        assert_eq!(kinds, vec![LineKind::Context, LineKind::Del, LineKind::Add, LineKind::Context]);
        assert_eq!(h.lines[1].old_no, Some(3));
        assert_eq!(h.lines[2].new_no, Some(3));
        assert_eq!(h.lines[2].text, "function f(?Foo $a = null)");
    }

    #[test]
    fn applies_rector_style_diff() {
        let diff = "--- Original\n+++ New\n@@ -3,4 +3,4 @@\n-function f(Foo $a = null)\n+function f(?Foo $a = null)\n {\n-    return utf8_encode($a);\n+    return mb_convert_encoding($a, 'UTF-8', 'ISO-8859-1');\n }\n";
        let patched = apply_unified(ORIGINAL, diff).unwrap();
        assert!(patched.contains("?Foo $a"));
        assert!(patched.contains("mb_convert_encoding"));
        assert!(patched.ends_with("}\n"));
        assert_eq!(patched.lines().count(), ORIGINAL.lines().count());
    }

    #[test]
    fn applies_without_line_numbers_and_keeps_crlf() {
        let original = ORIGINAL.replace('\n', "\r\n");
        let diff = "@@ @@\n {\n-    return utf8_encode($a);\n+    return $a;\n }\n";
        let patched = apply_unified(&original, diff).unwrap();
        assert_eq!(patched, original.replace("utf8_encode($a)", "$a"));
    }

    #[test]
    fn rejects_stale_diffs() {
        let diff = "@@ -5,1 +5,1 @@\n-    return something_else($a);\n+    return $a;\n";
        assert_eq!(apply_unified(ORIGINAL, diff), Err(PatchError::Mismatch(1)));
        assert_eq!(apply_unified(ORIGINAL, "not a diff"), Err(PatchError::Malformed));
    }

    #[test]
    fn preserves_missing_final_newline() {
        let original = "<?php\necho 1;";
        let diff = "@@ -2,1 +2,1 @@\n-echo 1;\n+echo 2;\n\\ No newline at end of file\n";
        assert_eq!(apply_unified(original, diff).unwrap(), "<?php\necho 2;");
    }
}
