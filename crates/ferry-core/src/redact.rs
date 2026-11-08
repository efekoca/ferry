//! Masks what shouldn't leave a run: page text and logs can hold visitors' addresses and credentials.

pub(crate) const MASK: &str = "[redacted]";
const KEYS: [&str; 11] =
    ["password", "passwd", "pwd", "secret", "token", "api_key", "apikey", "access_key", "authorization", "client_secret", "cookie"];

fn word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || b"._%+-".contains(&c)
}

fn is_email(local: &[u8], domain: &[u8]) -> bool {
    !local.is_empty()
        && domain.contains(&b'.')
        && !domain.starts_with(b".")
        && !domain.ends_with(b".")
        && domain.iter().all(|c| c.is_ascii_alphanumeric() || b".-".contains(c))
}

fn value_start(b: &[u8], mut at: usize) -> Option<usize> {
    while b.get(at).is_some_and(|c| b"'\"".contains(c)) {
        at += 1;
    }
    while b.get(at) == Some(&b' ') {
        at += 1;
    }
    at = match (b.get(at), b.get(at + 1)) {
        (Some(b'='), Some(b'>')) => at + 2,
        (Some(b'=' | b':'), _) => at + 1,
        _ => return None,
    };
    while b.get(at).is_some_and(|c| b" '\"".contains(c)) {
        at += 1;
    }
    Some(at)
}

/// `Bearer` and the like only name the scheme: the credential comes after them.
fn after_scheme(b: &[u8], at: usize) -> usize {
    let scheme =
        ["bearer ", "basic ", "token "].iter().find(|s| b.len() > at + s.len() && b[at..at + s.len()].eq_ignore_ascii_case(s.as_bytes()));
    at + scheme.map_or(0, |s| s.len())
}

fn value_end(b: &[u8], from: usize) -> usize {
    (from..b.len()).find(|&k| b" \t\r\n'\"&,;<>)".contains(&b[k])).unwrap_or(b.len())
}

pub fn text(input: &str) -> String {
    let b = input.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut k = 0;
    while k < b.len() {
        if b[k..].starts_with(b"://") {
            let end = value_end(b, k + 3);
            if let Some(at) = b[k + 3..end].iter().rposition(|c| *c == b'@') {
                out.extend_from_slice(b"://");
                out.extend_from_slice(MASK.as_bytes());
                k += 3 + at;
                continue;
            }
        }
        if b[k] == b'@' {
            let from = out.iter().rposition(|c| !word(*c)).map_or(0, |p| p + 1);
            let end = (k + 1..b.len()).find(|&j| !word(b[j])).unwrap_or(b.len());
            let domain = b[k + 1..end].strip_suffix(b".").unwrap_or(&b[k + 1..end]);
            if is_email(&out[from..], domain) {
                out.truncate(from);
                out.extend_from_slice(MASK.as_bytes());
                k += 1 + domain.len();
                continue;
            }
        }
        if b[k..].len() > 7 && b[k..k + 7].eq_ignore_ascii_case(b"bearer ") {
            let end = value_end(b, k + 7);
            if end > k + 7 {
                out.extend_from_slice(&b[k..k + 7]);
                out.extend_from_slice(MASK.as_bytes());
                k = end;
                continue;
            }
        }
        let key = KEYS.iter().find(|key| b.len() >= k + key.len() && b[k..k + key.len()].eq_ignore_ascii_case(key.as_bytes()));
        if let Some(found) = key.and_then(|key| Some((k + key.len(), value_start(b, k + key.len())?))) {
            let (after, start) = found;
            let end = value_end(b, after_scheme(b, start));
            if end > start {
                out.extend_from_slice(&b[k..start.max(after)]);
                out.extend_from_slice(MASK.as_bytes());
                k = end;
                continue;
            }
        }
        out.push(b[k]);
        k += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::text;

    #[test]
    fn addresses_and_credentials_are_masked() {
        assert_eq!(text("from ada@example.com, ok"), "from [redacted], ok");
        assert_eq!(text("mysql://root:hunter2@db/app failed"), "mysql://[redacted]@db/app failed");
        assert_eq!(text("password=hunter2&next=1"), "password=[redacted]&next=1");
        assert_eq!(text("{\"api_key\": \"abc123\"}"), "{\"api_key\": \"[redacted]\"}");
        assert_eq!(text("'secret' => 'abc'"), "'secret' => '[redacted]'");
        assert_eq!(text("Authorization: Bearer abc.def"), "Authorization: [redacted]");
        assert_eq!(text("authorization: basic dXNlcjpwYXNz next"), "authorization: [redacted] next");
        assert_eq!(text("sent Bearer abc.def"), "sent Bearer [redacted]");
    }

    #[test]
    fn php_messages_stay_readable() {
        for message in [
            "syntax error, unexpected token \":\" in /app/a.php on line 3",
            "Undefined array key \"password\" in /app/a.php on line 3",
            "Undefined variable $token",
            "http://127.0.0.1:8080/users",
            "@deprecated use x",
            "Çağrı başarısız oldu",
        ] {
            assert_eq!(text(message), message);
        }
    }
}
