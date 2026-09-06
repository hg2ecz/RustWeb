pub(super) fn slugify_ascii(input: &str) -> String {
    let mut out = String::new();
    let mut pending_sep = false;
    for ch in input.chars() {
        let mapped: Option<char> = match ch {
            'á' | 'Á' => Some('a'),
            'é' | 'É' => Some('e'),
            'í' | 'Í' => Some('i'),
            'ó' | 'Ó' | 'ö' | 'Ö' | 'ő' | 'Ő' => Some('o'),
            'ú' | 'Ú' | 'ü' | 'Ü' | 'ű' | 'Ű' => Some('u'),
            c if c.is_ascii_alphanumeric() => Some(c.to_ascii_lowercase()),
            _ => None,
        };
        if let Some(c) = mapped {
            let separator = pending_sep && !out.is_empty();
            let needed = 1usize + usize::from(separator);
            if out.len().saturating_add(needed) > 160 {
                break;
            }
            if separator {
                out.push('-');
            }
            pending_sep = false;
            out.push(c);
        } else if !out.is_empty() {
            pending_sep = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

pub(super) fn is_canonical_slug(raw: &str) -> bool {
    if raw.is_empty() || raw.len() > 160 {
        return false;
    }
    let mut prev_hyphen = false;
    for (i, b) in raw.bytes().enumerate() {
        if b == b'-' {
            if i == 0 || i + 1 == raw.len() || prev_hyphen {
                return false;
            }
            prev_hyphen = true;
        } else if b.is_ascii_lowercase() || b.is_ascii_digit() {
            prev_hyphen = false;
        } else {
            return false;
        }
    }
    true
}

pub(super) fn normalize_email(raw: &str) -> Option<String> {
    if raw.is_empty()
        || raw.len() > 254
        || !raw.is_ascii()
        || raw.bytes().any(|b| b <= 0x20 || b == 0x7f)
    {
        return None;
    }
    let (local, domain) = raw.rsplit_once('@')?;
    if local.is_empty()
        || local.len() > 64
        || domain.is_empty()
        || domain.len() > 253
        || local.contains('@')
    {
        return None;
    }
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return None;
    }
    if !local.bytes().all(|b| {
        b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'.' | b'!'
                    | b'#'
                    | b'$'
                    | b'%'
                    | b'&'
                    | b'\''
                    | b'*'
                    | b'+'
                    | b'-'
                    | b'/'
                    | b'='
                    | b'?'
                    | b'^'
                    | b'_'
                    | b'`'
                    | b'{'
                    | b'|'
                    | b'}'
                    | b'~'
            )
    }) {
        return None;
    }
    for label in domain.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return None;
        }
    }
    Some(format!("{}@{}", local, domain.to_ascii_lowercase()))
}

pub(super) fn normalize_url(raw: &str) -> Option<String> {
    if raw.is_empty() || raw.len() > 2048 || raw.bytes().any(|b| b <= 0x20 || b == 0x7f) {
        return None;
    }
    let parsed = url::Url::parse(raw).ok()?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return None;
    }
    Some(parsed.to_string())
}
