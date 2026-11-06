use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhpVersion {
    pub major: u32,
    pub minor: u32,
}

impl PhpVersion {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub const TARGETS: [PhpVersion; 5] =
        [PhpVersion::new(8, 1), PhpVersion::new(8, 2), PhpVersion::new(8, 3), PhpVersion::new(8, 4), PhpVersion::new(8, 5)];

    pub const KNOWN: [PhpVersion; 14] = [
        PhpVersion::new(5, 6),
        PhpVersion::new(7, 0),
        PhpVersion::new(7, 1),
        PhpVersion::new(7, 2),
        PhpVersion::new(7, 3),
        PhpVersion::new(7, 4),
        PhpVersion::new(8, 0),
        PhpVersion::new(8, 1),
        PhpVersion::new(8, 2),
        PhpVersion::new(8, 3),
        PhpVersion::new(8, 4),
        PhpVersion::new(8, 5),
        PhpVersion::new(8, 6),
        PhpVersion::new(9, 0),
    ];

    pub fn compact(&self) -> String {
        format!("php{}{}", self.major, self.minor)
    }
}

impl fmt::Display for PhpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl FromStr for PhpVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let mut parts = s.strip_suffix("-dev").unwrap_or(s).split('.');
        let major =
            parts.next().and_then(|p| p.parse().ok()).ok_or_else(|| crate::t!("geçersiz PHP sürümü: {s}", "invalid PHP version: {s}"))?;
        let minor = parts.next().map(|p| p.parse().ok()).unwrap_or(Some(0));
        let minor = minor.ok_or_else(|| crate::t!("geçersiz PHP sürümü: {s}", "invalid PHP version: {s}"))?;
        Ok(Self { major, minor })
    }
}

impl Serialize for PhpVersion {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PhpVersion {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

type Triple = (u32, u32, u32);

#[derive(Debug, Clone, Copy)]
enum Bound {
    Ge(Triple),
    Gt(Triple),
    Le(Triple),
    Lt(Triple),
    Ne(Triple),
}

impl Bound {
    fn allows(&self, v: Triple) -> bool {
        match *self {
            Bound::Ge(b) => v >= b,
            Bound::Gt(b) => v > b,
            Bound::Le(b) => v <= b,
            Bound::Lt(b) => v < b,
            Bound::Ne(b) => v != b,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Constraint {
    alternatives: Vec<Vec<Bound>>,
}

impl Constraint {
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }
        let mut alternatives = Vec::new();
        for alt in input.split("||").flat_map(|a| a.split('|')) {
            alternatives.push(parse_and_group(alt.trim())?);
        }
        Some(Self { alternatives })
    }

    pub fn allows(&self, v: Triple) -> bool {
        self.alternatives.iter().any(|group| group.iter().all(|b| b.allows(v)))
    }

    pub fn allows_line(&self, line: PhpVersion) -> bool {
        !self.line_patches(line).is_empty()
    }

    fn line_patches(&self, line: PhpVersion) -> Vec<(u64, u64)> {
        self.alternatives.iter().flat_map(|group| group_patches(group, (line.major, line.minor))).collect()
    }

    pub fn lowest_known_line(&self) -> Option<PhpVersion> {
        PhpVersion::KNOWN.into_iter().find(|v| self.allows_line(*v))
    }
}

fn group_patches(group: &[Bound], line: (u32, u32)) -> Vec<(u64, u64)> {
    let (mut lo, mut hi) = (0u64, u64::MAX);
    let mut excluded: Vec<(u64, u64)> = Vec::new();
    let patch_floor = |v: Triple| match (v.0, v.1).cmp(&line) {
        std::cmp::Ordering::Less => Some(0),
        std::cmp::Ordering::Equal => Some(u64::from(v.2)),
        std::cmp::Ordering::Greater => None,
    };
    for bound in group {
        match *bound {
            Bound::Ge(v) => match patch_floor(v) {
                Some(p) => lo = lo.max(p),
                None => return Vec::new(),
            },
            Bound::Gt(v) => match patch_floor(v) {
                Some(p) if (v.0, v.1) == line => lo = lo.max(p + 1),
                Some(_) => {}
                None => return Vec::new(),
            },
            Bound::Le(v) => match (v.0, v.1).cmp(&line) {
                std::cmp::Ordering::Less => return Vec::new(),
                std::cmp::Ordering::Equal => hi = hi.min(u64::from(v.2)),
                std::cmp::Ordering::Greater => {}
            },
            Bound::Lt(v) => match (v.0, v.1).cmp(&line) {
                std::cmp::Ordering::Less => return Vec::new(),
                std::cmp::Ordering::Equal if v.2 == 0 => return Vec::new(),
                std::cmp::Ordering::Equal => hi = hi.min(u64::from(v.2) - 1),
                std::cmp::Ordering::Greater => {}
            },
            Bound::Ne(v) if (v.0, v.1) == line => excluded.push((u64::from(v.2), u64::from(v.2))),
            Bound::Ne(_) => {}
        }
    }
    subtract(&[(lo, hi)], &excluded)
}

fn subtract(allowed: &[(u64, u64)], excluded: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let mut excluded = excluded.to_vec();
    excluded.sort_unstable();
    let mut out = Vec::new();
    for &(lo, hi) in allowed {
        let mut next = lo;
        let mut open = true;
        for &(start, end) in &excluded {
            if !open || start > hi {
                break;
            }
            if end < next {
                continue;
            }
            if start > next {
                out.push((next, start - 1));
            }
            match end.checked_add(1) {
                Some(n) => next = n,
                None => open = false,
            }
        }
        if open && next <= hi {
            out.push((next, hi));
        }
    }
    out
}

pub fn php_allows(require: Option<&str>, conflict: Option<&str>, target: PhpVersion) -> Option<bool> {
    php_patches(require, conflict, target).map(|patches| !patches.is_empty())
}

/// The patch releases of the target's line that these fields allow, as inclusive ranges.
pub(crate) fn php_patches(require: Option<&str>, conflict: Option<&str>, target: PhpVersion) -> Option<Vec<(u64, u64)>> {
    let allowed = match require {
        Some(r) => Constraint::parse(r)?.line_patches(target),
        None => vec![(0, u64::MAX)],
    };
    let blocked = match conflict {
        Some(c) => Constraint::parse(c)?.line_patches(target),
        None => Vec::new(),
    };
    Some(subtract(&allowed, &blocked))
}

pub(crate) fn shared_patches(a: &[(u64, u64)], b: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let pairs = a.iter().flat_map(|x| b.iter().map(move |y| (x.0.max(y.0), x.1.min(y.1))));
    pairs.filter(|(lo, hi)| lo <= hi).collect()
}

pub fn constraint_allows(constraint: &str, target: PhpVersion) -> Option<bool> {
    Constraint::parse(constraint).map(|c| c.allows_line(target))
}

fn parse_and_group(input: &str) -> Option<Vec<Bound>> {
    if let Some((lo, hi)) = input.split_once(" - ") {
        let (lo, _) = parse_version(lo.trim())?;
        let (hi, hi_parts) = parse_version(hi.trim())?;
        let upper = if hi_parts < 3 { Bound::Lt(bump(hi, hi_parts)) } else { Bound::Le(hi) };
        return Some(vec![Bound::Ge(lo), upper]);
    }

    let trimmed = input.trim();
    if trimmed.starts_with(',') || trimmed.ends_with(',') || input.split(',').any(|part| part.trim().is_empty()) {
        return None;
    }
    let raw: Vec<&str> = input.split(|c: char| c == ',' || c.is_whitespace()).filter(|t| !t.is_empty()).collect();
    let mut tokens: Vec<String> = Vec::new();
    let mut pending_op: Option<&str> = None;
    for t in raw {
        if matches!(t, ">=" | "<=" | ">" | "<" | "!=" | "==" | "=" | "^" | "~") {
            if pending_op.replace(t).is_some() {
                return None;
            }
            continue;
        }
        match pending_op.take() {
            Some(op) => tokens.push(format!("{op}{t}")),
            None => tokens.push(t.to_string()),
        }
    }
    if pending_op.is_some() || tokens.is_empty() {
        return None;
    }

    let mut bounds = Vec::new();
    for token in &tokens {
        bounds.extend(parse_atom(token)?);
    }
    Some(bounds)
}

fn parse_atom(token: &str) -> Option<Vec<Bound>> {
    let (token, stability) = token.split_once('@').unwrap_or((token, "stable"));
    if !["dev", "alpha", "beta", "rc", "stable"].contains(&stability.to_ascii_lowercase().as_str()) {
        return None;
    }
    if let Some(branch) = token.strip_suffix("-dev") {
        if token.starts_with(['>', '<', '^', '~', '!']) {
            return parse_atom(branch.trim_end_matches(['x', 'X', '*']).trim_end_matches('.'));
        }
        let digits = branch.trim_start_matches(['v', 'V', '^', '~', '=', '>', '<', '!']).trim_end_matches(['.', 'x', 'X', '*']);
        return parse_version(digits).map(|_| vec![Bound::Lt((0, 0, 0))]);
    }
    if token == "*" {
        return Some(vec![]);
    }
    let ops = [">=", "<=", "!=", "==", ">", "<", "=", "^", "~"];
    let (op, rest) = ops.iter().find_map(|op| token.strip_prefix(op).map(|r| (*op, r))).unwrap_or(("", token));
    let rest = rest.trim();

    if rest.contains('*') || rest.contains('x') || rest.contains('X') {
        if !op.is_empty() {
            return None;
        }
        let parts: Vec<&str> = rest.split('.').collect();
        let prefix: Vec<&str> = parts.iter().take_while(|p| !matches!(**p, "*" | "x" | "X")).copied().collect();
        if parts[prefix.len()..].iter().any(|p| !matches!(*p, "*" | "x" | "X")) {
            return None;
        }
        let (base, parts) = parse_version(&prefix.join("."))?;
        return Some(vec![Bound::Ge(base), Bound::Lt(bump(base, parts))]);
    }

    let (v, parts) = parse_version(rest)?;
    let bounds = match op {
        ">=" => vec![Bound::Ge(v)],
        ">" => vec![Bound::Gt(v)],
        "<=" => vec![Bound::Le(v)],
        "<" => vec![Bound::Lt(v)],
        "!=" => vec![Bound::Ne(v)],
        "^" => {
            let upper = if v.0 > 0 {
                (v.0.saturating_add(1), 0, 0)
            } else if v.1 > 0 {
                (0, v.1.saturating_add(1), 0)
            } else {
                (0, 0, v.2.saturating_add(1))
            };
            vec![Bound::Ge(v), Bound::Lt(upper)]
        }
        "~" => {
            let upper = match parts {
                1 | 2 => (v.0.saturating_add(1), 0, 0),
                _ => (v.0, v.1.saturating_add(1), 0),
            };
            vec![Bound::Ge(v), Bound::Lt(upper)]
        }
        _ => vec![Bound::Ge(v), Bound::Le(v)],
    };
    Some(bounds)
}

fn parse_version(s: &str) -> Option<(Triple, usize)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    if s.is_empty() {
        return None;
    }
    let mut nums = [0u32; 3];
    let mut count = 0;
    for (i, part) in s.split('.').enumerate() {
        if i >= 4 {
            return None;
        }
        if i == 3 {
            part.parse::<u32>().ok()?;
            continue;
        }
        let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
        let suffix = &part[digits.len()..];
        if !suffix.is_empty() && (!s.ends_with(part) || !is_stability(suffix)) {
            return None;
        }
        nums[i] = digits.parse().ok()?;
        count = i + 1;
    }
    Some(((nums[0], nums[1], nums[2]), count))
}

fn is_stability(suffix: &str) -> bool {
    let s = suffix.trim_start_matches(['-', '.', '_']).to_ascii_lowercase();
    let word = s.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    ["alpha", "beta", "a", "b", "rc", "patch", "pl", "p", "dev", "stable"].contains(&word)
}

fn bump(v: Triple, parts: usize) -> Triple {
    match parts {
        0 => (u32::MAX, 0, 0),
        1 => (v.0.saturating_add(1), 0, 0),
        2 => (v.0, v.1.saturating_add(1), 0),
        _ => (v.0, v.1, v.2.saturating_add(1)),
    }
}

#[cfg(test)]
mod dev_range_tests {
    use super::*;

    #[test]
    fn a_dev_suffix_in_a_range_keeps_the_range() {
        let t = |c: &str, v: (u32, u32)| php_allows(Some(c), None, PhpVersion::new(v.0, v.1));
        assert_eq!(t(">=8.1 <8.5-dev", (8, 4)), Some(true));
        assert_eq!(t(">=8.1 <8.5-dev", (8, 5)), Some(false));
        assert_eq!(t("^8.1-dev", (8, 4)), Some(true));
        assert_eq!(t(">=8.2.x-dev", (8, 1)), Some(false));
        assert_eq!(t("8.4.x-dev", (8, 4)), Some(false), "a branch isn't a release");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> PhpVersion {
        s.parse().unwrap()
    }

    #[test]
    fn malformed_constraints_are_unknown_and_lines_are_checked_exactly() {
        let t = PhpVersion::new(8, 4);
        for bad in [">", "8.4bananas", ">=", ">= <8.5", "^~8.4", "8.4.x.y"] {
            assert_eq!(constraint_allows(bad, t), None, "{bad}");
        }
        for (c, want) in [
            (">=8.4.41 <8.4.100", true),
            (">=8.4.41, <8.4.42", true),
            (">8.4.99999", true),
            (">=8.4 <8.4.0", false),
            ("<8.4.0", false),
            ("!=8.4.0", true),
            ("8.4.*", true),
            ("~8.4.3", true),
            (">= 8.4", true),
            ("^8.4@dev", true),
            (">=8.4.0-beta1", true),
            ("8.4.0RC1", true),
            ("~8.3.0 || ^8.5", false),
            ("^8.3 || ^8.5", true),
        ] {
            assert_eq!(constraint_allows(c, t), Some(want), "{c}");
        }
    }

    #[test]
    fn composer_rejects_operators_on_wildcards() {
        assert_eq!(constraint_allows("!=8.2.*", PhpVersion::new(8, 2)), None);
        assert_eq!(constraint_allows(">=7.4 !=8.0.*", PhpVersion::new(8, 0)), None);
    }

    #[test]
    fn parses_and_displays_versions() {
        assert_eq!(v("8.3"), PhpVersion::new(8, 3));
        assert_eq!(v("8").to_string(), "8.0");
        assert_eq!(v("8.4").compact(), "php84");
        assert!(v("7.4") < v("8.0"));
        assert!("abc".parse::<PhpVersion>().is_err());
    }

    #[test]
    fn caret_and_or() {
        let c = "^7.2 || ^8.0";
        assert_eq!(constraint_allows(c, v("7.4")), Some(true));
        assert_eq!(constraint_allows(c, v("8.4")), Some(true));
        assert_eq!(constraint_allows(c, v("9.0")), Some(false));
        assert_eq!(constraint_allows("^7.4|^8.0", v("8.3")), Some(true));
    }

    #[test]
    fn ranges_and_comparisons() {
        assert_eq!(constraint_allows(">=7.1", v("8.5")), Some(true));
        assert_eq!(constraint_allows(">=7.4 <8.3", v("8.3")), Some(false));
        assert_eq!(constraint_allows(">=7.4 <8.3", v("8.2")), Some(true));
        assert_eq!(constraint_allows(">= 7.2, < 8.4", v("8.3")), Some(true));
        assert_eq!(constraint_allows("<8.0", v("8.0")), Some(false));
        assert_eq!(constraint_allows(">=8.3.5", v("8.3")), Some(true));
        assert_eq!(constraint_allows("7.2 - 8.1", v("8.1")), Some(true));
        assert_eq!(constraint_allows("7.2 - 8.1", v("8.2")), Some(false));
    }

    #[test]
    fn tilde_and_wildcards() {
        assert_eq!(constraint_allows("~7.4", v("7.4")), Some(true));
        assert_eq!(constraint_allows("~7.4", v("8.0")), Some(false));
        assert_eq!(constraint_allows("~8.2.0", v("8.3")), Some(false));
        assert_eq!(constraint_allows("~8.2.0", v("8.2")), Some(true));
        assert_eq!(constraint_allows("7.4.*", v("7.4")), Some(true));
        assert_eq!(constraint_allows("8.*", v("8.5")), Some(true));
        assert_eq!(constraint_allows("8.*", v("7.4")), Some(false));
        assert_eq!(constraint_allows("*", v("8.5")), Some(true));
        assert_eq!(constraint_allows(">=8.1@dev", v("8.2")), Some(true));
    }

    #[test]
    fn unparseable_returns_none() {
        assert_eq!(constraint_allows("", v("8.3")), None);
        assert_eq!(constraint_allows("dev-master", v("8.3")), None);
    }

    #[test]
    fn lowest_known_line() {
        assert!(Constraint::parse(">=8.0 ||").is_none());
        assert!(Constraint::parse("|| >=8.0").is_none());
        let c = Constraint::parse("^7.4 || ^8.0").unwrap();
        assert_eq!(c.lowest_known_line(), Some(v("7.4")));
        let c = Constraint::parse(">=8.2").unwrap();
        assert_eq!(c.lowest_known_line(), Some(v("8.2")));
    }

    /// Expected values come from Composer's own VersionParser, for a PHP 8.4 line.
    #[test]
    fn constraints_match_composer() {
        let cases: &[(&str, Option<bool>)] = &[
            ("8", Some(false)),
            ("=8", Some(false)),
            ("==8", Some(false)),
            ("8.4", Some(true)),
            ("8.4.0", Some(true)),
            ("8.4.1", Some(true)),
            ("=8.4", Some(true)),
            ("8.4.*", Some(true)),
            ("8.*", Some(true)),
            ("8.4.x", Some(true)),
            ("=8.4.*", None),
            ("!=8.4.*", None),
            (">=7.*", None),
            (">8.3.*", None),
            ("<8.*", None),
            ("^8.*", None),
            ("~8.*", None),
            ("~8.4.*", None),
            ("8.4,", None),
            ("8.4,,8.5", None),
            (",8.4", None),
            ("8.4@bananas", None),
            ("8.4@dev", Some(true)),
            ("^8.4@stable", Some(true)),
            ("8.4.x-dev", Some(false)),
            ("^8.4", Some(true)),
            ("^8.0", Some(true)),
            ("^7.4 || ^8.0", Some(true)),
            ("^7.4|^8.0", Some(true)),
            ("~8.3", Some(true)),
            ("~8.3.0", Some(false)),
            ("~8", Some(true)),
            (">=8.1 <8.5", Some(true)),
            (">=8.1,<8.5", Some(true)),
            (">=8.4.41 <8.4.100", Some(true)),
            (">8.4", Some(true)),
            (">=8.5", Some(false)),
            ("<8.4", Some(false)),
            ("<8.4.1", Some(true)),
            ("<=8.4", Some(true)),
            ("!=8.4.0", Some(true)),
            ("!=8.4", Some(true)),
            ("8.0 - 8.4", Some(true)),
            ("8.0 - 8.4.1", Some(true)),
            (">= 8.1", Some(true)),
            (">=8.1 || <7", Some(true)),
            ("*", Some(true)),
            (">", None),
            ("8.4bananas", None),
            ("", None),
            ("v8.4", Some(true)),
            ("^v8.4", Some(true)),
            (">=8.2.0-beta1", Some(true)),
            ("8.4.0.0", Some(true)),
            ("8.4.1.2.3", None),
            ("^0.3", Some(false)),
            ("~0.3", Some(false)),
            ("8.4 8.5", Some(false)),
            (">=7.4 <8.0 || >=8.1", Some(true)),
        ];
        let wrong: Vec<_> = cases
            .iter()
            .filter(|(c, want)| constraint_allows(c, PhpVersion::new(8, 4)) != *want)
            .map(|(c, want)| (*c, *want, constraint_allows(c, PhpVersion::new(8, 4))))
            .collect();
        assert!(wrong.is_empty(), "{wrong:#?}");
    }
}
