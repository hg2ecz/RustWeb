use chrono::{DateTime, NaiveDate, Utc};
use language_core::{
    AppError, F32Value, FormFailure, FormField, FormFieldIssue, HttpMethod, ImageRef, Program,
    Route, RouteSegment, ValidationKind, Value, ValueType,
};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub fn route_meta_for_request<'a>(
    program: &'a Program,
    method: HttpMethod,
    path: &str,
) -> Result<&'a Route, AppError> {
    match_route(program, method, path).map(|(route, _)| route)
}

pub(crate) fn match_route<'a>(
    program: &'a Program,
    method: HttpMethod,
    path: &str,
) -> Result<(&'a Route, HashMap<String, Value>), AppError> {
    let req: Vec<&str> = if path == "/" {
        vec![]
    } else {
        path.strip_prefix('/')
            .ok_or(AppError::BadRequest)?
            .split('/')
            .collect()
    };
    let mut bad = false;
    let mut other = false;
    for route in &program.routes {
        if route.segments.len() != req.len() {
            continue;
        }
        let mut env = HashMap::new();
        let mut matched = true;
        let mut badparam = false;
        for (pattern, actual) in route.segments.iter().zip(&req) {
            match pattern {
                RouteSegment::Static(x) if x == actual => {}
                RouteSegment::Static(_) => {
                    matched = false;
                    break;
                }
                RouteSegment::Param { name, ty } => match decode_path_param(program, actual, *ty) {
                    Ok(v) => {
                        env.insert(name.clone(), v);
                    }
                    Err(_) => {
                        matched = false;
                        badparam = true;
                        break;
                    }
                },
            }
        }
        if !matched {
            if badparam && route.method == method {
                bad = true;
            }
            continue;
        }
        if route.method != method {
            other = true;
            continue;
        }
        return Ok((route, env));
    }
    if bad {
        Err(AppError::BadRequest)
    } else if other {
        Err(AppError::MethodNotAllowed)
    } else {
        Err(AppError::NotFound)
    }
}
pub(crate) fn decode_named_form_into(
    program: &Program,
    route: &Route,
    pairs: &[(String, String)],
    env: &mut HashMap<String, Value>,
) -> Result<(), AppError> {
    let schema_name = route.form_schema.as_deref().ok_or(AppError::Internal)?;
    let expected: HashSet<&str> = route.form_fields.iter().map(|f| f.name.as_str()).collect();
    let mut seen = HashSet::new();
    let mut raw = HashMap::new();
    for (n, v) in pairs {
        if !expected.contains(n.as_str()) || !seen.insert(n.as_str()) {
            return Err(AppError::BadRequest);
        }
        raw.insert(n.as_str(), v.as_str());
    }
    let mut issues = Vec::new();
    let mut values = Vec::new();
    for f in &route.form_fields {
        let supplied = raw.get(f.name.as_str()).copied();
        let text = match supplied {
            Some(v) => v,
            None if f.ty == ValueType::Bool => "false",
            None => {
                issues.push(FormFieldIssue {
                    field: f.name.clone(),
                    code: "required".into(),
                });
                ""
            }
        };
        values.push((f.name.clone(), text.to_string()));
        if supplied.is_none() && f.ty != ValueType::Bool {
            continue;
        }
        match decode_scalar(program, text, f.ty) {
            Ok(v) => {
                env.insert(f.name.clone(), v);
            }
            Err(_) => issues.push(FormFieldIssue {
                field: f.name.clone(),
                code: "invalid_type".into(),
            }),
        }
    }
    if issues.is_empty() {
        for rule in &route.validations {
            let Some(value) = env.get(&rule.field) else {
                continue;
            };
            let ok = match (&rule.kind, value) {
                (ValidationKind::Length { min, max }, Value::String(v)) => {
                    v.chars().count() >= *min && v.chars().count() <= *max
                }
                (ValidationKind::Range { min, max }, Value::Int(v)) => v >= min && v <= max,
                (ValidationKind::Pattern { regex }, Value::String(v)) => regex::Regex::new(regex)
                    .map(|re| re.is_match(v))
                    .unwrap_or(false),
                (ValidationKind::SameAs { other }, v) => env.get(other).is_some_and(|x| x == v),
                _ => false,
            };
            if !ok {
                let code = match &rule.kind {
                    ValidationKind::Length { .. } => "length",
                    ValidationKind::Range { .. } => "range",
                    ValidationKind::Pattern { .. } => "pattern",
                    ValidationKind::SameAs { .. } => "same",
                };
                issues.push(FormFieldIssue {
                    field: rule.field.clone(),
                    code: code.into(),
                });
            }
        }
    }
    if !issues.is_empty() {
        return Err(AppError::FormInvalid(FormFailure {
            schema: schema_name.into(),
            values,
            issues,
        }));
    }
    Ok(())
}

pub(crate) fn decode_fields_into(
    program: &Program,
    schema: &[FormField],
    pairs: &[(String, String)],
    env: &mut HashMap<String, Value>,
) -> Result<(), AppError> {
    let expected: HashSet<&str> = schema.iter().map(|f| f.name.as_str()).collect();
    let mut seen = HashSet::new();
    let mut vals = HashMap::new();
    for (n, v) in pairs {
        if !expected.contains(n.as_str()) || !seen.insert(n.as_str()) {
            return Err(AppError::BadRequest);
        }
        vals.insert(n.as_str(), v.as_str());
    }
    if vals.len() != schema.len() {
        return Err(AppError::BadRequest);
    }
    for f in schema {
        let raw = vals
            .get(f.name.as_str())
            .copied()
            .ok_or(AppError::BadRequest)?;
        env.insert(f.name.clone(), decode_scalar(program, raw, f.ty)?);
    }
    Ok(())
}
pub(crate) fn validate_route_inputs(
    route: &Route,
    env: &HashMap<String, Value>,
) -> Result<(), AppError> {
    for rule in &route.validations {
        let value = env.get(&rule.field).ok_or(AppError::BadRequest)?;
        match (&rule.kind, value) {
            (ValidationKind::Length { min, max }, Value::String(v))
                if v.chars().count() >= *min && v.chars().count() <= *max => {}
            (ValidationKind::Range { min, max }, Value::Int(v)) if v >= min && v <= max => {}
            (ValidationKind::Pattern { regex }, Value::String(v))
                if regex::Regex::new(regex)
                    .map(|re| re.is_match(v))
                    .unwrap_or(false) => {}
            (ValidationKind::SameAs { other }, v) if env.get(other).is_some_and(|x| x == v) => {}
            _ => return Err(AppError::BadRequest),
        }
    }
    Ok(())
}
fn decode_path_param(program: &Program, raw: &str, ty: ValueType) -> Result<Value, AppError> {
    decode_scalar(program, &percent_decode(raw, false)?, ty)
}
use crate::scalars::{is_canonical_slug, normalize_email, normalize_url};

pub(crate) fn decode_scalar(
    program: &Program,
    raw: &str,
    ty: ValueType,
) -> Result<Value, AppError> {
    match ty {
        ValueType::String => Ok(Value::String(raw.into())),
        ValueType::Email => normalize_email(raw)
            .map(Value::Email)
            .ok_or(AppError::BadRequest),
        ValueType::Url => normalize_url(raw)
            .map(Value::Url)
            .ok_or(AppError::BadRequest),
        ValueType::Slug => {
            if is_canonical_slug(raw) {
                Ok(Value::String(raw.into()))
            } else {
                Err(AppError::BadRequest)
            }
        }
        ValueType::Int => raw
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|_| AppError::BadRequest),
        ValueType::F32Array | ValueType::StringList | ValueType::StringDict => {
            Err(AppError::BadRequest)
        }
        ValueType::F32 => raw
            .parse::<f32>()
            .ok()
            .and_then(F32Value::new)
            .map(Value::F32)
            .ok_or(AppError::BadRequest),
        ValueType::Bool => match raw {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(AppError::BadRequest),
        },
        ValueType::Date => NaiveDate::parse_from_str(raw, "%Y-%m-%d")
            .map(Value::Date)
            .map_err(|_| AppError::BadRequest),
        ValueType::DateTime => DateTime::parse_from_rfc3339(raw)
            .map(|v| Value::DateTime(v.with_timezone(&Utc)))
            .map_err(|_| AppError::BadRequest),
        ValueType::Uuid => Uuid::parse_str(raw)
            .map(Value::Uuid)
            .map_err(|_| AppError::BadRequest),
        ValueType::Decimal => Decimal::from_str_exact(raw)
            .map(Value::Decimal)
            .map_err(|_| AppError::BadRequest),
        ValueType::Image => ImageRef::parse(raw)
            .map(Value::Image)
            .ok_or(AppError::BadRequest),
        ValueType::Enum(enum_id) => {
            let def = program.enum_by_id(enum_id).ok_or(AppError::Internal)?;
            if def.variants.iter().any(|v| v == raw) {
                Ok(Value::Enum {
                    enum_id,
                    variant: raw.into(),
                })
            } else {
                Err(AppError::BadRequest)
            }
        }
        ValueType::Upload => Err(AppError::BadRequest),
    }
}

pub fn decode_urlencoded_limited(
    body: &[u8],
    max_fields: usize,
    max_field_bytes: usize,
) -> Result<Vec<(String, String)>, AppError> {
    let text = std::str::from_utf8(body).map_err(|_| AppError::BadRequest)?;
    if text.is_empty() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for pair in text.split('&') {
        if out.len() >= max_fields {
            return Err(AppError::BadRequest);
        }
        if pair.len() > max_field_bytes.saturating_mul(3).saturating_add(8) {
            return Err(AppError::BadRequest);
        }
        let (n, v) = pair.split_once('=').unwrap_or((pair, ""));
        let n = percent_decode(n, true)?;
        let v = percent_decode(v, true)?;
        if n.is_empty()
            || n.len() > max_field_bytes
            || v.len() > max_field_bytes
            || n.bytes().any(|b| b < 0x20 || b == 0x7f)
            || v.bytes().any(|b| b == 0)
        {
            return Err(AppError::BadRequest);
        }
        out.push((n, v));
    }
    Ok(out)
}
pub fn decode_urlencoded(body: &[u8]) -> Result<Vec<(String, String)>, AppError> {
    decode_urlencoded_limited(body, 64, 8192)
}
fn percent_decode(raw: &str, plus: bool) -> Result<String, AppError> {
    let b = raw.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' => {
                let hi = *b.get(i + 1).ok_or(AppError::BadRequest)?;
                let lo = *b.get(i + 2).ok_or(AppError::BadRequest)?;
                let x = (hex(hi).ok_or(AppError::BadRequest)? << 4)
                    | hex(lo).ok_or(AppError::BadRequest)?;
                if x == 0 || (!plus && x == b'/') {
                    return Err(AppError::BadRequest);
                }
                out.push(x);
                i += 3
            }
            b'+' if plus => {
                out.push(b' ');
                i += 1
            }
            x if x == 0 || (!plus && x == b'/') => return Err(AppError::BadRequest),
            x => {
                out.push(x);
                i += 1
            }
        }
    }
    String::from_utf8(out).map_err(|_| AppError::BadRequest)
}
fn hex(v: u8) -> Option<u8> {
    match v {
        b'0'..=b'9' => Some(v - b'0'),
        b'a'..=b'f' => Some(v - b'a' + 10),
        b'A'..=b'F' => Some(v - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn validate_redirect_location(v: &str) -> Result<(), AppError> {
    if !v.starts_with('/')
        || v.starts_with("//")
        || v.contains('\\')
        || v.bytes().any(|b| b <= 0x20 || b == 0x7f)
    {
        Err(AppError::BadRequest)
    } else {
        Ok(())
    }
}
