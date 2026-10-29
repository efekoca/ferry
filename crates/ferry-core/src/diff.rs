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
    ops: Vec<(char, String)>,
}

fn hunk_range(part: Option<&str>) -> Option<(usize, usize)> {
    let mut it = part?.splitn(2, ',');
    let start = it.next()?.parse().ok()?;
    let len = match it.next() {
        Some(n) => n.parse().ok()?,
        None => 1,
    };
    Some((start, len))
}

fn parse_unified(diff: &str) -> Result<Vec<ParsedHunk>, PatchError> {
    let mut hunks: Vec<ParsedHunk> = Vec::new();
    let mut left: Option<(usize, usize)> = Some((0, 0));
    for line in diff.lines() {
        let between = left == Some((0, 0));
        if line.starts_with("@@") && (between || left.is_none()) {
            let mut parts = line.split_whitespace();
            let old = hunk_range(parts.find_map(|p| p.strip_prefix('-')));
            let new = hunk_range(parts.find_map(|p| p.strip_prefix('+')));
            let old_start_hint = old.map(|(s, len)| if len == 0 { s } else { s.saturating_sub(1) });
            hunks.push(ParsedHunk { old_start_hint, old: Vec::new(), ops: Vec::new() });
            left = old.zip(new).map(|((_, o), (_, n))| (o, n));
            continue;
        }
        if between || line.starts_with('\\') {
            continue;
        }
        let hunk = hunks.last_mut().unwrap();
        let (op, rest) = match line.split_at_checked(1) {
            Some((op @ (" " | "-" | "+"), rest)) => (op.chars().next().unwrap(), rest),
            None if line.is_empty() => (' ', ""),
            _ => return Err(PatchError::Malformed),
        };
        if let Some((o, n)) = left.as_mut() {
            if op != '+' {
                *o = o.checked_sub(1).ok_or(PatchError::Malformed)?;
            }
            if op != '-' {
                *n = n.checked_sub(1).ok_or(PatchError::Malformed)?;
            }
        }
        if op != '+' {
            hunk.old.push(rest.to_string());
        }
        hunk.ops.push((op, rest.to_string()));
    }
    if hunks.is_empty() || left.is_some_and(|l| l != (0, 0)) {
        return Err(PatchError::Malformed);
    }
    Ok(hunks)
}

fn split_eol(s: &str) -> (&str, &str) {
    let text = strip_eol(s);
    (text, &s[text.len()..])
}

pub fn apply_unified(original: &str, diff: &str) -> Result<String, PatchError> {
    let hunks = parse_unified(diff)?;
    let default_eol = if original.contains("\r\n") && !original.replace("\r\n", "").contains('\n') { "\r\n" } else { "\n" };
    let lines: Vec<(&str, &str)> = original.split_inclusive('\n').map(split_eol).collect();

    let mut out: Vec<(String, String)> = Vec::with_capacity(lines.len());
    let push = |out: &mut Vec<(String, String)>, text: String, eol: String, fallback: &str| {
        if let Some(last) = out.last_mut().filter(|l| l.1.is_empty()) {
            last.1 = fallback.to_string();
        }
        out.push((text, eol));
    };
    let mut cursor = 0;
    let mut last_added = false;
    for (n, hunk) in hunks.iter().enumerate() {
        let matches_at = |start: usize| {
            start + hunk.old.len() <= lines.len()
                && hunk.old.iter().zip(&lines[start..]).all(|(a, b)| a == b.0)
        };
        let hinted = hunk.old_start_hint.filter(|&h| h >= cursor);
        let start = match hinted {
            Some(h) if matches_at(h) => Some(h),
            _ => (cursor..=lines.len().saturating_sub(hunk.old.len())).find(|&s| matches_at(s)),
        };
        let start = start.ok_or(PatchError::Mismatch(n + 1))?;
        for (text, eol) in &lines[cursor..start] {
            push(&mut out, text.to_string(), eol.to_string(), default_eol);
        }
        let mut pos = start;
        let mut deleted_eol: Option<&str> = None;
        for (op, text) in &hunk.ops {
            let near = lines
                .get(pos)
                .or_else(|| pos.checked_sub(1).and_then(|p| lines.get(p)))
                .map(|l| l.1)
                .filter(|e| !e.is_empty())
                .unwrap_or(default_eol);
            match op {
                '+' => {
                    let eol = deleted_eol.filter(|e| !e.is_empty()).unwrap_or(near);
                    push(&mut out, text.clone(), eol.to_string(), eol);
                    last_added = true;
                    continue;
                }
                '-' => {
                    deleted_eol = Some(lines[pos].1);
                    pos += 1;
                }
                _ => {
                    deleted_eol = None;
                    push(&mut out, lines[pos].0.to_string(), lines[pos].1.to_string(), near);
                    pos += 1;
                }
            }
            last_added = false;
        }
        cursor = pos;
    }
    for (text, eol) in &lines[cursor..] {
        push(&mut out, text.to_string(), eol.to_string(), default_eol);
        last_added = false;
    }
    let had_final_newline = original.is_empty() || original.ends_with('\n');
    if let Some(last) = out.last_mut() {
        if !had_final_newline && last_added {
            last.1.clear();
        } else if had_final_newline && last.1.is_empty() {
            last.1 = default_eol.to_string();
        }
    }
    Ok(out.into_iter().flat_map(|(t, e)| [t, e]).collect())
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

    #[test]
    fn hunk_lines_that_look_like_headers() {
        assert_eq!(apply_unified("a\n-- x\nb\n", "--- a\n+++ b\n@@ -1,2 +1,1 @@\n a\n--- x\n").unwrap(), "a\nb\n");
        assert_eq!(apply_unified("a\nb\n", "@@ -1,2 +1,3 @@\n a\n+++ added\n b\n").unwrap(), "a\n++ added\nb\n");
    }

    #[test]
    fn keeps_each_line_ending() {
        assert_eq!(apply_unified("a\nb\r\nc\n", "@@ -1,1 +1,1 @@\n-a\n+A\n").unwrap(), "A\nb\r\nc\n");
        let src = "<?php\n$s = \"x\r\ny\";\nf();\n";
        assert_eq!(apply_unified(src, "@@ -3,1 +3,1 @@\n-f();\n+g();\n").unwrap(), src.replace("f()", "g()"));
    }

    #[test]
    fn pure_insertions_go_after_the_line() {
        assert_eq!(apply_unified("a\nb\nc\n", "@@ -2,0 +3 @@\n+X\n").unwrap(), "a\nb\nX\nc\n");
        assert_eq!(apply_unified("a", "@@ -1,0 +2 @@\n+b\n").unwrap(), "a\nb");
    }
}
