use crate::execution_context::Budget;
use crate::vm::eval_expr;
use language_core::{AppError, Expr, Route, RouteSegment, Value, ValueType};
use std::collections::HashMap;

pub(crate) fn build_current_route_url(
    route: &Route,
    env: &HashMap<String, Value>,
    replacement: Option<(&str, &Value)>,
) -> Result<String, AppError> {
    let mut out = String::new();
    if route.segments.is_empty() {
        out.push('/');
    } else {
        for segment in &route.segments {
            out.push('/');
            match segment {
                RouteSegment::Static(value) => out.push_str(value),
                RouteSegment::Param { name, ty } => {
                    let value = match replacement {
                        Some((replace_name, replace_value)) if replace_name == name => {
                            replace_value
                        }
                        _ => env.get(name).ok_or(AppError::Internal)?,
                    };
                    append_url_value(&mut out, value, *ty, true)?;
                }
            }
        }
    }
    if !route.query_fields.is_empty() {
        out.push('?');
        for (index, field) in route.query_fields.iter().enumerate() {
            if index > 0 {
                out.push('&');
            }
            out.push_str(&field.name);
            out.push('=');
            let value = env.get(&field.name).ok_or(AppError::Internal)?;
            append_url_value(&mut out, value, field.ty, false)?;
        }
    }
    Ok(out)
}

pub(crate) fn build_route_url(
    route: &Route,
    args: &[Expr],
    env: &HashMap<String, Value>,
    budget: &mut Budget,
    include_query: bool,
) -> Result<String, AppError> {
    let mut values = Vec::with_capacity(args.len());
    for e in args {
        values.push(eval_expr(e, env, budget)?);
    }
    let mut idx = 0usize;
    let mut out = String::new();
    if route.segments.is_empty() {
        out.push('/');
    } else {
        for seg in &route.segments {
            out.push('/');
            match seg {
                RouteSegment::Static(v) => out.push_str(v),
                RouteSegment::Param { ty, .. } => {
                    let value = values.get(idx).ok_or(AppError::Internal)?;
                    idx += 1;
                    append_url_value(&mut out, value, *ty, true)?;
                }
            }
        }
    }
    if include_query && !route.query_fields.is_empty() {
        out.push('?');
        for (n, field) in route.query_fields.iter().enumerate() {
            if n > 0 {
                out.push('&');
            }
            out.push_str(&field.name);
            out.push('=');
            let value = values.get(idx).ok_or(AppError::Internal)?;
            idx += 1;
            append_url_value(&mut out, value, field.ty, false)?;
        }
    }
    if idx != values.len() {
        return Err(AppError::Internal);
    }
    Ok(out)
}
fn append_url_value(
    out: &mut String,
    value: &Value,
    ty: ValueType,
    path: bool,
) -> Result<(), AppError> {
    let raw = match (value, ty) {
        (Value::String(v), ValueType::String | ValueType::Slug) => v.clone(),
        (Value::Email(v), ValueType::Email) => v.clone(),
        (Value::Url(v), ValueType::Url) => v.clone(),
        (Value::Int(v), ValueType::Int) => v.to_string(),
        (Value::F32(v), ValueType::F32) => v.get().to_string(),
        (Value::Bool(v), ValueType::Bool) => v.to_string(),
        (Value::Date(v), ValueType::Date) => v.format("%Y-%m-%d").to_string(),
        (Value::DateTime(v), ValueType::DateTime) => {
            v.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
        }
        (Value::Uuid(v), ValueType::Uuid) => v.hyphenated().to_string(),
        (Value::Decimal(v), ValueType::Decimal) => v.normalize().to_string(),
        (Value::Image(v), ValueType::Image) => v.canonical(),
        (Value::Enum { enum_id, variant }, ValueType::Enum(expected)) if *enum_id == expected => {
            variant.clone()
        }
        (
            _,
            ValueType::Upload | ValueType::F32Array | ValueType::StringList | ValueType::StringDict,
        ) => return Err(AppError::Internal),
        _ => return Err(AppError::Internal),
    };
    percent_encode_into(&raw, out, path);
    Ok(())
}
fn percent_encode_into(raw: &str, out: &mut String, path: bool) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for b in raw.as_bytes() {
        let safe = b.is_ascii_alphanumeric() || matches!(*b, b'-' | b'.' | b'_' | b'~');
        if safe {
            out.push(*b as char);
        } else {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 15) as usize] as char);
        }
    }
    let _ = path;
}

const MAX_MARKDOWN_BYTES: usize = 512 * 1024;

fn markdown_href_allowed(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 4096
        || value.bytes().any(|b| b < 0x20 || b == 0x7f || b == b'\\')
    {
        return false;
    }
    (value.starts_with('/') && !value.starts_with("//"))
        || value.starts_with('#')
        || value.starts_with("https://")
        || value.starts_with("http://")
}

fn render_markdown_inline(
    input: &str,
    out: &mut String,
    budget: &mut Budget,
) -> Result<(), AppError> {
    render_markdown_inline_depth(input, out, budget, 0)
}
fn render_markdown_inline_depth(
    input: &str,
    out: &mut String,
    budget: &mut Budget,
    depth: u8,
) -> Result<(), AppError> {
    if depth > 32 {
        return Err(AppError::InstructionLimit);
    }
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < input.len() {
        budget.charge(1)?;
        if input[i..].starts_with("**") {
            if let Some(rel) = input[i + 2..].find("**") {
                let end = i + 2 + rel;
                out.push_str("<strong>");
                render_markdown_inline_depth(&input[i + 2..end], out, budget, depth + 1)?;
                out.push_str("</strong>");
                i = end + 2;
                continue;
            }
        }
        if bytes[i] == b'*' {
            if let Some(rel) = input[i + 1..].find('*') {
                let end = i + 1 + rel;
                out.push_str("<em>");
                render_markdown_inline_depth(&input[i + 1..end], out, budget, depth + 1)?;
                out.push_str("</em>");
                i = end + 1;
                continue;
            }
        }
        if bytes[i] == b'`' {
            if let Some(rel) = input[i + 1..].find('`') {
                let end = i + 1 + rel;
                out.push_str("<code>");
                escape_html_into(&input[i + 1..end], out);
                out.push_str("</code>");
                i = end + 1;
                continue;
            }
        }
        if bytes[i] == b'[' {
            if let Some(text_rel) = input[i + 1..].find("](") {
                let text_end = i + 1 + text_rel;
                let url_start = text_end + 2;
                if let Some(url_rel) = input[url_start..].find(')') {
                    let url_end = url_start + url_rel;
                    let href = &input[url_start..url_end];
                    if markdown_href_allowed(href) {
                        out.push_str("<a href=\"");
                        escape_html_into(href, out);
                        out.push_str("\" rel=\"noopener noreferrer\">");
                        render_markdown_inline_depth(
                            &input[i + 1..text_end],
                            out,
                            budget,
                            depth + 1,
                        )?;
                        out.push_str("</a>");
                        i = url_end + 1;
                        continue;
                    }
                }
            }
        }
        let ch = input[i..].chars().next().ok_or(AppError::Internal)?;
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
        i += ch.len_utf8();
    }
    Ok(())
}

pub(crate) fn render_safe_markdown_into(
    input: &str,
    out: &mut String,
    budget: &mut Budget,
) -> Result<(), AppError> {
    if input.len() > MAX_MARKDOWN_BYTES {
        return Err(AppError::MemoryLimit);
    }
    budget.charge_alloc((input.len() as u64).saturating_mul(12))?;
    let mut in_code = false;
    let mut in_list = false;
    let mut paragraph = false;
    for line in input.lines() {
        budget.charge(1)?;
        if line.trim_start().starts_with("```") {
            if paragraph {
                out.push_str("</p>");
                paragraph = false;
            }
            if in_list {
                out.push_str("</ul>");
                in_list = false;
            }
            if in_code {
                out.push_str("</code></pre>");
            } else {
                out.push_str("<pre><code>");
            }
            in_code = !in_code;
            continue;
        }
        if in_code {
            escape_html_into(line, out);
            out.push('\n');
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if paragraph {
                out.push_str("</p>");
                paragraph = false;
            }
            if in_list {
                out.push_str("</ul>");
                in_list = false;
            }
            continue;
        }
        let leading = line.len() - line.trim_start().len();
        let t = line.trim_start();
        let mut heading = 0usize;
        for b in t.bytes() {
            if b == b'#' && heading < 6 {
                heading += 1
            } else {
                break;
            }
        }
        if heading > 0 && t.as_bytes().get(heading) == Some(&b' ') {
            if paragraph {
                out.push_str("</p>");
                paragraph = false;
            }
            if in_list {
                out.push_str("</ul>");
                in_list = false;
            }
            out.push_str(&format!("<h{heading}>"));
            render_markdown_inline(&t[heading + 1..], out, budget)?;
            out.push_str(&format!("</h{heading}>"));
            continue;
        }
        if leading <= 3 && (t.starts_with("- ") || t.starts_with("* ")) {
            if paragraph {
                out.push_str("</p>");
                paragraph = false;
            }
            if !in_list {
                out.push_str("<ul>");
                in_list = true;
            }
            out.push_str("<li>");
            render_markdown_inline(&t[2..], out, budget)?;
            out.push_str("</li>");
            continue;
        }
        if in_list {
            out.push_str("</ul>");
            in_list = false;
        }
        if !paragraph {
            out.push_str("<p>");
            paragraph = true;
        } else {
            out.push(' ');
        }
        render_markdown_inline(trimmed, out, budget)?;
    }
    if in_code {
        out.push_str("</code></pre>");
    }
    if paragraph {
        out.push_str("</p>");
    }
    if in_list {
        out.push_str("</ul>");
    }
    Ok(())
}

pub(crate) fn escape_html_into(input: &str, out: &mut String) {
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
}
