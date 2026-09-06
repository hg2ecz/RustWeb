pub(super) fn is_top_level_declaration_at(source: &str, pos: usize) -> bool {
    let line_start = source[..pos].rfind('\n').map(|v| v + 1).unwrap_or(0);
    if !source[line_start..pos].trim().is_empty() {
        return false;
    }

    let b = source.as_bytes();
    let mut i = 0usize;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    while i < pos {
        let ch = b[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == b'\\' {
                escaped = true;
            } else if ch == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if ch == b'/' && b.get(i + 1) == Some(&b'/') {
            i += 2;
            while i < pos && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        match ch {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => depth = (depth - 1).max(0),
            _ => {}
        }
        i += 1;
    }
    depth == 0 && !in_string
}
