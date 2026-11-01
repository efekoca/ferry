use crate::version::{constraint_allows, PhpVersion};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub name: String,
    pub version: String,
    pub major_change: bool,
}

pub fn url(package: &str) -> Option<String> {
    let valid = package.split_once('/').is_some_and(|(v, n)| {
        let ok = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.".contains(&b));
        ok(v) && ok(n)
    });
    valid.then(|| format!("https://repo.packagist.org/p2/{package}.json"))
}

fn normalized(v: &str) -> Option<Vec<u64>> {
    let core = v.trim_start_matches(['v', 'V']);
    if core.contains('-') || core.contains("dev") {
        return None;
    }
    let mut parts: Vec<u64> = core.split('.').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    parts.resize(4, 0);
    Some(parts)
}

pub fn versions(json: &str, package: &str) -> Vec<(String, Vec<u64>, Option<String>)> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else { return Vec::new() };
    let Some(list) = value.pointer(&format!("/packages/{}", package.replace('~', "~0").replace('/', "~1"))).and_then(|v| v.as_array())
    else {
        return Vec::new();
    };
    let mut current = serde_json::Map::new();
    let mut out = Vec::new();
    for entry in list {
        let Some(obj) = entry.as_object() else { continue };
        for (k, v) in obj {
            if v.as_str() == Some("__unset") {
                current.remove(k);
            } else {
                current.insert(k.clone(), v.clone());
            }
        }
        let version = current.get("version").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let Some(norm) = current.get("version_normalized").and_then(|v| v.as_str()).and_then(normalized) else { continue };
        let php = current.get("require").and_then(|r| r.get("php")).and_then(|v| v.as_str()).map(str::to_string);
        out.push((version, norm, php));
    }
    out
}

pub fn lowest_compatible(json: &str, package: &str, installed: &str, target: PhpVersion) -> Option<Suggestion> {
    let floor = normalized(installed.trim_start_matches(['v', 'V']))?;
    let candidates: Vec<_> = versions(json, package)
        .into_iter()
        .filter(|(version, norm, php)| {
            *norm > floor
                && php.as_deref().is_none_or(|c| constraint_allows(c, target) == Some(true))
                && crate::framework::package_limit(package, version).is_none_or(|l| target <= l.max)
        })
        .collect();
    let major = candidates.iter().map(|c| c.1[0]).min()?;
    let (version, norm, _) = candidates.into_iter().filter(|c| c.1[0] == major).max_by(|a, b| a.1.cmp(&b.1))?;
    Some(Suggestion { name: package.to_string(), major_change: norm.first() != floor.first(), version })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"minified":"composer/2.0","packages":{"acme/log":[
        {"version":"3.1.0","version_normalized":"3.1.0.0","require":{"php":">=8.1"}},
        {"version":"3.0.0","version_normalized":"3.0.0.0"},
        {"version":"2.9.0-RC1","version_normalized":"2.9.0.0-RC1","require":{"php":">=7.2"}},
        {"version":"2.8.0","version_normalized":"2.8.0.0","require":{"php":">=7.2 <8.4"}},
        {"version":"2.7.0","version_normalized":"2.7.0.0","require":"__unset"},
        {"version":"1.0.0","version_normalized":"1.0.0.0","require":{"php":">=5.3"}}
    ]}}"#;

    #[test]
    fn picks_the_lowest_stable_release_for_the_target() {
        assert_eq!(versions(SAMPLE, "acme/log").len(), 5);
        let s = lowest_compatible(SAMPLE, "acme/log", "1.0.0", "8.4".parse().unwrap()).unwrap();
        assert_eq!((s.version.as_str(), s.major_change), ("2.7.0", true));
        let s = lowest_compatible(SAMPLE, "acme/log", "2.8.0", "8.4".parse().unwrap()).unwrap();
        assert_eq!((s.version.as_str(), s.major_change), ("3.1.0", true));
        assert_eq!(lowest_compatible(SAMPLE, "acme/log", "2.7.0", "8.3".parse().unwrap()).unwrap().version, "2.8.0");
        assert!(lowest_compatible(SAMPLE, "acme/log", "3.2.0", "8.4".parse().unwrap()).is_none());
        assert_eq!(url("Bad/../x"), None);
        assert_eq!(url("monolog/monolog").unwrap(), "https://repo.packagist.org/p2/monolog/monolog.json");
    }
}
