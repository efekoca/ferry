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

    pub const TARGETS: [PhpVersion; 5] = [
        PhpVersion::new(8, 1),
        PhpVersion::new(8, 2),
        PhpVersion::new(8, 3),
        PhpVersion::new(8, 4),
        PhpVersion::new(8, 5),
    ];

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
        let major = parts
            .next()
            .and_then(|p| p.parse().ok())
            .ok_or_else(|| crate::t!("geçersiz PHP sürümü: {s}", "invalid PHP version: {s}"))?;
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
            let alt = alt.trim();
            if alt.is_empty() {
                continue;
            }
            alternatives.push(parse_and_group(alt)?);
        }
        if alternatives.is_empty() {
            return None;
        }
        Some(Self { alternatives })
    }

    pub fn allows(&self, v: Triple) -> bool {
        self.alternatives
            .iter()
            .any(|group| group.iter().all(|b| b.allows(v)))
    }

    pub fn allows_line(&self, line: PhpVersion) -> bool {
        (0..=40)
            .chain(std::iter::once(999))
            .any(|patch| self.allows((line.major, line.minor, patch)))
    }

    pub fn lowest_known_line(&self) -> Option<PhpVersion> {
        PhpVersion::KNOWN.into_iter().find(|v| self.allows_line(*v))
    }
}

pub fn constraint_allows(constraint: &str, target: PhpVersion) -> Option<bool> {
    Constraint::parse(constraint).map(|c| c.allows_line(target))
}

fn parse_and_group(input: &str) -> Option<Vec<Bound>> {
    if let Some((lo, hi)) = input.split_once(" - ") {
        let (lo, _) = parse_version(lo.trim())?;
        let (hi, hi_parts) = parse_version(hi.trim())?;
        let upper = if hi_parts < 3 {
            Bound::Lt(bump(hi, hi_parts))
        } else {
            Bound::Le(hi)
        };
        return Some(vec![Bound::Ge(lo), upper]);
    }

    let raw: Vec<&str> = input
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .collect();
    let mut tokens: Vec<String> = Vec::new();
    let mut pending_op: Option<&str> = None;
    for t in raw {
        if matches!(t, ">=" | "<=" | ">" | "<" | "!=" | "==" | "=" | "^" | "~") {
            pending_op = Some(t);
            continue;
        }
        match pending_op.take() {
            Some(op) => tokens.push(format!("{op}{t}")),
            None => tokens.push(t.to_string()),
        }
    }

    let mut bounds = Vec::new();
    for token in &tokens {
        bounds.extend(parse_atom(token)?);
    }
    Some(bounds)
}

fn parse_atom(token: &str) -> Option<Vec<Bound>> {
    let token = token.split('@').next().unwrap_or(token);
    let token = token.strip_suffix("-dev").unwrap_or(token);
    if token == "*" {
        return Some(vec![]);
    }
    let ops = [">=", "<=", "!=", "==", ">", "<", "=", "^", "~"];
    let (op, rest) = ops
        .iter()
        .find_map(|op| token.strip_prefix(op).map(|r| (*op, r)))
        .unwrap_or(("", token));
    let rest = rest.trim();

    if rest.contains('*') || rest.contains('x') || rest.contains('X') {
        if op == "!=" {
            return Some(vec![]);
        }
        let prefix: Vec<&str> = rest
            .split('.')
            .take_while(|p| !matches!(*p, "*" | "x" | "X"))
            .collect();
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
        _ if parts < 3 => vec![Bound::Ge(v), Bound::Lt(bump(v, parts))],
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
        nums[i] = digits.parse().ok()?;
        count = i + 1;
    }
    Some(((nums[0], nums[1], nums[2]), count))
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
mod tests {
    use super::*;

    fn v(s: &str) -> PhpVersion {
        s.parse().unwrap()
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
        let c = Constraint::parse("^7.4 || ^8.0").unwrap();
        assert_eq!(c.lowest_known_line(), Some(v("7.4")));
        let c = Constraint::parse(">=8.2").unwrap();
        assert_eq!(c.lowest_known_line(), Some(v("8.2")));
    }
}
