use crate::cache_safety::{
    action_has_business_audit, action_has_object_auth, validate_public_cache_statements,
};
use crate::diagnostics::CompileError;
use crate::domain_symbols::internal_domain_symbol;
use crate::lexer::tokenize;
use crate::module_namespace::resolve;
use crate::schema_declarations;
use crate::source_syntax::is_identifier;
use crate::type_resolution::resolve_value_type;
use language_core::{
    ActionBody, FormField, HttpMethod, PageBody, Program, PublicCachePolicy, Route, RouteAuth,
    RouteSegment, Statement, UploadField, ValidationKind, ValueType,
};
use std::collections::HashMap;

mod route_scanner;

pub(super) fn parse_routes(
    source: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    for route_source in route_scanner::top_level_route_declarations(source)? {
        let t = tokenize(&route_source)?;
        let i = 0;
        if t.get(i).map(String::as_str) != Some("route") {
            return Err(CompileError::Syntax(
                "internal route scanner error: declaration does not start with `route`".into(),
            ));
        }
        if i + 5 >= t.len() {
            return Err(CompileError::Syntax("incomplete route".into()));
        }
        let name = t[i + 1].clone();
        let method = HttpMethod::parse(&t[i + 2])
            .ok_or_else(|| CompileError::Syntax("unsupported route method".into()))?;
        let path = t[i + 3].clone();
        let segments = parse_route_segments(&name, &path, namespace, p)?;
        let mut c = i + 4;
        let mut query_fields = Vec::new();
        let mut form_fields = Vec::new();
        let mut json_fields = Vec::new();
        let mut upload = None;
        let mut validations = Vec::new();
        if t.get(c).map(String::as_str) == Some("query") {
            c += 1;
            while !matches!(
                t.get(c).map(String::as_str),
                Some("form")
                    | Some("json")
                    | Some("upload")
                    | Some("validate")
                    | Some("auth")
                    | Some("rate")
                    | Some("cache")
                    | Some("invalidate")
                    | Some("=>")
                    | None
            ) {
                query_fields.push(parse_typed_binding(&name, t.get(c).unwrap(), namespace, p)?);
                c += 1;
            }
        }
        let mut form_schema = None;
        if t.get(c).map(String::as_str) == Some("form") {
            c += 1;
            if let Some(token) = t.get(c) {
                if !token.contains('<') {
                    let schema_name = resolve(namespace, token);
                    let schema = p.form(&schema_name).ok_or_else(|| {
                        CompileError::Syntax(format!(
                            "route `{name}` references unknown form `{token}`"
                        ))
                    })?;
                    form_fields = schema.fields.clone();
                    validations = schema.validations.clone();
                    form_schema = Some(schema.name.clone());
                    c += 1;
                } else {
                    while !matches!(
                        t.get(c).map(String::as_str),
                        Some("json")
                            | Some("upload")
                            | Some("validate")
                            | Some("auth")
                            | Some("rate")
                            | Some("cache")
                            | Some("invalidate")
                            | Some("=>")
                            | None
                    ) {
                        form_fields.push(parse_typed_binding(
                            &name,
                            t.get(c).unwrap(),
                            namespace,
                            p,
                        )?);
                        c += 1;
                    }
                }
            }
        }
        if t.get(c).map(String::as_str) == Some("json") {
            c += 1;
            while !matches!(
                t.get(c).map(String::as_str),
                Some("upload")
                    | Some("validate")
                    | Some("auth")
                    | Some("rate")
                    | Some("cache")
                    | Some("invalidate")
                    | Some("=>")
                    | None
            ) {
                json_fields.push(parse_typed_binding(&name, t.get(c).unwrap(), namespace, p)?);
                c += 1;
            }
            if json_fields.is_empty() {
                return Err(CompileError::Syntax(format!(
                    "route `{name}` json body requires at least one typed field"
                )));
            }
        }
        if t.get(c).map(String::as_str) == Some("upload") {
            c += 1;
            let binding = t
                .get(c)
                .ok_or_else(|| CompileError::Syntax("upload binding expected".into()))?;
            let lt = binding.find('<').ok_or_else(|| {
                CompileError::Syntax("upload binding must use name<Upload> or name<Image>".into())
            })?;
            if !binding.ends_with('>') {
                return Err(CompileError::Syntax(
                    "upload binding must use name<Upload> or name<Image>".into(),
                ));
            }
            let upload_ty = &binding[lt + 1..binding.len() - 1];
            if !matches!(upload_ty, "Upload" | "Image") {
                return Err(CompileError::Syntax(
                    "upload binding must use name<Upload> or name<Image>".into(),
                ));
            }
            let uname = &binding[..lt];
            if !is_identifier(uname) {
                return Err(CompileError::Syntax("invalid upload binding name".into()));
            }
            if t.get(c + 1).map(String::as_str) != Some("to") {
                return Err(CompileError::Syntax(
                    "upload binding requires `to <relative-path>`".into(),
                ));
            }
            let dest = t
                .get(c + 2)
                .ok_or_else(|| CompileError::Syntax("upload destination expected".into()))?
                .clone();
            validate_upload_destination(&dest)?;
            if upload_ty == "Image"
                && !dest
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_'))
            {
                return Err(CompileError::Syntax(
                    "Image upload destination must be URL-safe (letters, digits, /, -, _)".into(),
                ));
            }
            upload = Some(UploadField {
                name: uname.into(),
                destination: dest,
                image: upload_ty == "Image",
            });
            c += 3;
        }
        if t.get(c).map(String::as_str) == Some("validate") {
            c += 1;
            while !matches!(
                t.get(c).map(String::as_str),
                Some("auth")
                    | Some("rate")
                    | Some("cache")
                    | Some("invalidate")
                    | Some("=>")
                    | None
            ) {
                let (rule, next) = schema_declarations::parse_validation_rule(&t, c)?;
                validations.push(rule);
                c = next;
            }
        }
        let mut auth = RouteAuth::Public;
        if t.get(c).map(String::as_str) == Some("auth") {
            c += 1;
            let mode = t
                .get(c)
                .ok_or_else(|| CompileError::Syntax("route auth mode expected".into()))?;
            match mode.as_str() {
                "user" => {
                    auth = RouteAuth::User;
                    c += 1
                }
                "mfa" => {
                    auth = RouteAuth::Mfa;
                    c += 1
                }
                "role" => {
                    let role = t
                        .get(c + 1)
                        .ok_or_else(|| {
                            CompileError::Syntax("route auth role name expected".into())
                        })?
                        .clone();
                    if !is_identifier(&role) {
                        return Err(CompileError::Syntax("invalid route role name".into()));
                    }
                    auth = RouteAuth::Role(role);
                    c += 2
                }
                _ => {
                    return Err(CompileError::Syntax(format!(
                        "unknown route auth mode `{mode}`"
                    )));
                }
            }
        }
        let mut rate_policy = None;
        if t.get(c).map(String::as_str) == Some("rate") {
            let policy = t
                .get(c + 1)
                .ok_or_else(|| {
                    CompileError::Syntax(format!("route `{name}` rate policy expected"))
                })?
                .clone();
            if !is_identifier(&policy) {
                return Err(CompileError::Syntax(format!(
                    "route `{name}` invalid rate policy name"
                )));
            }
            rate_policy = Some(policy);
            c += 2;
        }
        let mut public_cache = None;
        if t.get(c).map(String::as_str) == Some("cache") {
            if t.get(c + 1).map(String::as_str) != Some("public")
                || t.get(c + 2).map(String::as_str) != Some("ttl")
            {
                return Err(CompileError::Syntax(format!(
                    "route `{name}` cache syntax is `cache public ttl <seconds>`"
                )));
            }
            let ttl_secs: u64 = t
                .get(c + 3)
                .ok_or_else(|| CompileError::Syntax(format!("route `{name}` cache ttl expected")))?
                .parse()
                .map_err(|_| {
                    CompileError::Syntax(format!("route `{name}` cache ttl must be integer"))
                })?;
            if ttl_secs == 0 {
                return Err(CompileError::Syntax(format!(
                    "route `{name}` cache ttl must be > 0"
                )));
            }
            public_cache = Some(PublicCachePolicy { ttl_secs });
            c += 4;
        }
        let mut invalidate_caches = Vec::new();
        if t.get(c).map(String::as_str) == Some("invalidate") {
            if t.get(c + 1).map(String::as_str) != Some("cache") {
                return Err(CompileError::Syntax(format!(
                    "route `{name}` invalidation syntax is `invalidate cache <route>...`"
                )));
            }
            c += 2;
            while !matches!(t.get(c).map(String::as_str), Some("=>") | None) {
                let target = t.get(c).unwrap();
                if !is_identifier(target) {
                    return Err(CompileError::Syntax(format!(
                        "route `{name}` invalid cache route name `{target}`"
                    )));
                }
                if invalidate_caches.contains(target) {
                    return Err(CompileError::Syntax(format!(
                        "route `{name}` duplicate cache invalidation `{target}`"
                    )));
                }
                invalidate_caches.push(target.clone());
                c += 1;
            }
            if invalidate_caches.is_empty() {
                return Err(CompileError::Syntax(format!(
                    "route `{name}` invalidate cache requires at least one route name"
                )));
            }
        }
        if t.get(c).map(String::as_str) != Some("=>") {
            return Err(CompileError::Syntax(format!("route `{name}` expected =>")));
        }
        let source_handler = t
            .get(c + 1)
            .ok_or_else(|| CompileError::Syntax("route missing handler".into()))?;
        if t.get(c + 2).map(String::as_str) != Some(";") || t.get(c + 3).is_some() {
            return Err(CompileError::Syntax(format!(
                "route `{name}` must end with `;` immediately after the handler"
            )));
        }
        let handler = internal_domain_symbol(source_handler)
            .map(|name| resolve(namespace, &name))
            .ok_or_else(|| {
                CompileError::Syntax(format!("route `{name}` invalid handler `{source_handler}`"))
            })?;
        if method == HttpMethod::Get
            && (!form_fields.is_empty() || !json_fields.is_empty() || upload.is_some())
        {
            return Err(CompileError::Syntax(
                "GET route cannot declare form/json/upload".into(),
            ));
        }
        if method == HttpMethod::Post && !query_fields.is_empty() {
            return Err(CompileError::Syntax(
                "POST route query schema is not supported".into(),
            ));
        }
        if [
            !form_fields.is_empty(),
            !json_fields.is_empty(),
            upload.is_some(),
        ]
        .into_iter()
        .filter(|v| *v)
        .count()
            > 1
        {
            return Err(CompileError::Syntax(
                "POST route may declare exactly one body mode: form, json, or upload".into(),
            ));
        }
        let mut field_types = HashMap::new();
        for f in query_fields
            .iter()
            .chain(form_fields.iter())
            .chain(json_fields.iter())
        {
            if field_types.insert(f.name.clone(), f.ty).is_some() {
                return Err(CompileError::Syntax(format!(
                    "route `{name}` duplicate input field `{}`",
                    f.name
                )));
            }
        }
        for v in &validations {
            let ty = *field_types.get(&v.field).ok_or_else(|| {
                CompileError::Syntax(format!(
                    "validation references unknown query/form/json field `{}`",
                    v.field
                ))
            })?;
            match &v.kind {
                ValidationKind::Length { .. } if ty == ValueType::String => {}
                ValidationKind::Range { .. } if ty == ValueType::Int => {}
                ValidationKind::Pattern { .. } if ty == ValueType::String => {}
                ValidationKind::SameAs { other } => {
                    let other_ty = *field_types.get(other).ok_or_else(|| {
                        CompileError::Syntax(format!(
                            "same validation references unknown field `{other}`"
                        ))
                    })?;
                    if ty != other_ty {
                        return Err(CompileError::Syntax(format!(
                            "same validation requires matching field types `{}` and `{other}`",
                            v.field
                        )));
                    }
                    if matches!(ty, ValueType::Upload | ValueType::Image) {
                        return Err(CompileError::Syntax(format!(
                            "same validation does not support Upload/Image field `{}`",
                            v.field
                        )));
                    }
                }
                _ => {
                    return Err(CompileError::Syntax(format!(
                        "validation kind does not match field `{}` type",
                        v.field
                    )));
                }
            }
        }
        if p.routes
            .iter()
            .any(|r| r.method == method && r.path == path)
        {
            return Err(CompileError::DuplicateRoute(format!(
                "{} {}",
                t[i + 2],
                path
            )));
        }
        p.routes.push(Route {
            name,
            method,
            path,
            segments,
            query_fields,
            form_fields,
            form_schema,
            json_fields,
            upload,
            validations,
            auth,
            rate_policy,
            public_cache,
            invalidate_caches,
            handler,
        });
    }
    Ok(())
}

fn validate_upload_destination(v: &str) -> Result<(), CompileError> {
    if v.is_empty()
        || v.len() > 4096
        || v.starts_with('/')
        || v.contains('\\')
        || v.as_bytes().contains(&0)
        || v.split('/').any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(CompileError::Syntax(
            "upload destination must be a safe relative AppFs path".into(),
        ));
    }
    Ok(())
}
pub(super) fn parse_typed_binding(
    route: &str,
    raw: &str,
    namespace: &str,
    p: &Program,
) -> Result<FormField, CompileError> {
    let lt = raw.find('<').ok_or_else(|| {
        CompileError::Syntax(format!(
            "route `{route}` binding `{raw}` must use name<Type>"
        ))
    })?;
    if !raw.ends_with('>') {
        return Err(CompileError::Syntax("malformed typed binding".into()));
    }
    let name = &raw[..lt];
    if !is_identifier(name) {
        return Err(CompileError::Syntax(format!(
            "route `{route}` invalid binding name `{name}`"
        )));
    }
    let ty = resolve_value_type(&raw[lt + 1..raw.len() - 1], namespace, p)
        .filter(|t| *t != ValueType::Upload)
        .ok_or_else(|| CompileError::Syntax("unsupported route scalar binding type".into()))?;
    Ok(FormField {
        name: name.into(),
        ty,
    })
}
fn parse_route_segments(
    route: &str,
    path: &str,
    namespace: &str,
    p: &Program,
) -> Result<Vec<RouteSegment>, CompileError> {
    if !path.starts_with('/') {
        return Err(CompileError::Syntax(format!(
            "route `{route}` path must start /"
        )));
    }
    if path == "/" {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for raw in path.trim_start_matches('/').split('/') {
        if raw.is_empty() {
            return Err(CompileError::Syntax(format!(
                "route `{route}` contains empty path segment"
            )));
        }
        if let Some(v) = raw.strip_prefix(':') {
            let f = parse_typed_binding(route, v, namespace, p)?;
            out.push(RouteSegment::Param {
                name: f.name,
                ty: f.ty,
            });
        } else {
            out.push(RouteSegment::Static(raw.into()));
        }
    }
    Ok(out)
}
pub(super) fn validate_routes(p: &Program) -> Result<(), CompileError> {
    for r in &p.routes {
        let mut expected: Vec<(String, ValueType)> = r
            .segments
            .iter()
            .filter_map(|s| match s {
                RouteSegment::Param { name, ty } => Some((name.clone(), *ty)),
                _ => None,
            })
            .collect();
        expected.extend(r.query_fields.iter().map(|f| (f.name.clone(), f.ty)));
        expected.extend(r.form_fields.iter().map(|f| (f.name.clone(), f.ty)));
        expected.extend(r.json_fields.iter().map(|f| (f.name.clone(), f.ty)));
        if let Some(u) = &r.upload {
            expected.push((
                u.name.clone(),
                if u.image {
                    ValueType::Image
                } else {
                    ValueType::Upload
                },
            ));
        }
        let params = match r.method {
            HttpMethod::Get => p.page(&r.handler).map(|h| &h.params),
            HttpMethod::Post => p.action(&r.handler).map(|h| &h.params),
        }
        .ok_or_else(|| CompileError::UnknownHandler(r.handler.clone()))?;
        if expected.len() != params.len() {
            return Err(CompileError::RouteParamMismatch(format!(
                "route `{}` provides {} values, handler expects {}",
                r.name,
                expected.len(),
                params.len()
            )));
        }
        for ((n, t), a) in expected.iter().zip(params) {
            if n != &a.name || *t != a.ty {
                return Err(CompileError::RouteParamMismatch(format!(
                    "route `{}` `{n}` does not match handler `{}: {:?}`",
                    r.name, a.name, a.ty
                )));
            }
        }
        if matches!(r.auth, RouteAuth::Public) {
            let has_object_auth = match r.method {
                HttpMethod::Get => p.page(&r.handler).is_some_and(|h| {
                    let PageBody::Statements(s) = &h.body;
                    page_has_object_auth(s)
                }),
                HttpMethod::Post => p.action(&r.handler).is_some_and(|h| {
                    let ActionBody::Statements(s) = &h.body;
                    action_has_object_auth(s)
                }),
            };
            if has_object_auth {
                return Err(CompileError::Syntax(format!(
                    "route `{}` uses object authorization but is public; add `auth user`, `auth mfa`, or `auth role ...`",
                    r.name
                )));
            }
            let has_business_audit = match r.method {
                HttpMethod::Get => false,
                HttpMethod::Post => p.action(&r.handler).is_some_and(|h| {
                    let ActionBody::Statements(s) = &h.body;
                    action_has_business_audit(s)
                }),
            };
            if has_business_audit {
                return Err(CompileError::Syntax(format!(
                    "route `{}` writes business audit records but is public; add `auth user`, `auth mfa`, or `auth role ...`",
                    r.name
                )));
            }
        }
        let canonical_params = page_canonical_slug_params(p, &r.handler);
        for param in canonical_params {
            let is_path_slug = r.segments.iter().any(|segment| {
                matches!(segment, RouteSegment::Param { name, ty } if name == param && *ty == ValueType::Slug)
            });
            if !is_path_slug {
                return Err(CompileError::Syntax(format!(
                    "route `{}` handler declares canonical slug `{param}`, but `{param}` is not a Slug path parameter",
                    r.name
                )));
            }
        }
        if r.public_cache.is_some() {
            if r.method != HttpMethod::Get {
                return Err(CompileError::Syntax(format!(
                    "route `{}` public cache is GET-only",
                    r.name
                )));
            }
            if !matches!(r.auth, RouteAuth::Public) {
                return Err(CompileError::Syntax(format!(
                    "route `{}` authenticated routes cannot use public cache",
                    r.name
                )));
            }
            let page = p
                .page(&r.handler)
                .ok_or_else(|| CompileError::UnknownHandler(r.handler.clone()))?;
            let PageBody::Statements(statements) = &page.body;
            validate_public_cache_statements(&r.name, statements, p)?;
        }
        if !r.invalidate_caches.is_empty() {
            if r.method != HttpMethod::Post {
                return Err(CompileError::Syntax(format!(
                    "route `{}` cache invalidation is POST-only",
                    r.name
                )));
            }
            for target in &r.invalidate_caches {
                let cached = p.routes.iter().find(|x| x.name == *target).ok_or_else(|| {
                    CompileError::Syntax(format!(
                        "route `{}` invalidates unknown cache route `{target}`",
                        r.name
                    ))
                })?;
                if cached.public_cache.is_none() {
                    return Err(CompileError::Syntax(format!(
                        "route `{}` invalidates non-cached route `{target}`",
                        r.name
                    )));
                }
            }
        }
    }
    Ok(())
}

fn page_canonical_slug_params<'a>(p: &'a Program, handler: &str) -> Vec<&'a str> {
    fn collect<'a>(statements: &'a [Statement], out: &mut Vec<&'a str>) {
        for statement in statements {
            match statement {
                Statement::CanonicalSlug { param, .. } => out.push(param.as_str()),
                Statement::Resource { statements, .. } => collect(statements, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    if let Some(page) = p.page(handler) {
        let PageBody::Statements(statements) = &page.body;
        collect(statements, &mut out);
    }
    out
}

fn page_has_object_auth(statements: &[Statement]) -> bool {
    statements.iter().any(|s| match s {
        Statement::Authorize(_) => true,
        Statement::Resource { statements, .. } => page_has_object_auth(statements),
        _ => false,
    })
}

#[cfg(test)]
mod lexical_hardening_tests {
    use crate::compile_source;
    use language_core::ValidationKind;

    #[test]
    fn preserves_signed_integer_range_bounds() {
        let src = r#"
page fn calculator(ctx: PageContext, a: Int, b: Int) -> Result<Html, PageError> {
    return Ok(html {<p>{{ a }} {{ b }}</p>});
}
route calculator GET "/calculator"
    query a<Int> b<Int>
    validate a range -1000000 1000000 b range -1000000 1000000
    => calculator;
"#;
        let p = compile_source(src).expect("negative range bounds must compile");
        let route = p.routes.iter().find(|r| r.name == "calculator").unwrap();
        assert!(route.validations.iter().any(|rule| {
            rule.field == "a"
                && matches!(
                    &rule.kind,
                    ValidationKind::Range { min, max }
                        if *min == -1_000_000 && *max == 1_000_000
                )
        }));
    }

    #[test]
    fn rejects_unknown_route_characters_instead_of_dropping_them() {
        let src = r#"
page fn calculator(ctx: PageContext, a: Int) -> Result<Html, PageError> {
    return Ok(html {<p>{{ a }}</p>});
}
route calculator GET "/calculator" query a<Int> validate a range @1 10 => calculator;
"#;
        let err = compile_source(src).expect_err("unknown route punctuation must fail closed");
        let msg = err.to_string();
        assert!(msg.contains("unexpected character `@`"), "{msg}");
    }

    #[test]
    fn route_like_html_text_is_not_a_route_declaration() {
        let src = r#"
page fn index(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {
        <p>route fake GET /admin =&gt; missing</p>
    });
}
route index GET "/" => index;
"#;
        let p = compile_source(src).expect("route-like HTML text must remain HTML text");
        assert_eq!(p.routes.len(), 1);
        assert_eq!(p.routes[0].name, "index");
    }

    #[test]
    fn declaration_like_html_text_is_not_parsed_as_top_level_code() {
        let src = r#"
page fn index(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {
        <p>model Ghost and page fn fake are documentation text.</p>
    });
}
route index GET "/" => index;
"#;
        let p = compile_source(src)
            .expect("declaration-like HTML text must be ignored by declaration scanners");
        assert!(p.models.is_empty());
        assert_eq!(p.pages.len(), 1);
        assert_eq!(p.pages[0].name, "index");
    }
    #[test]
    fn multiline_post_route_accepts_indented_form_continuation() {
        let src = r#"
page fn contact_form(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {<p>Contact</p>});
}

action fn contact_submit(ctx: ActionContext, name: String) -> Result<Json, PageError> {
    return Ok(json(name));
}

route contact_form GET "/contact" => contact_form;
route contact_submit POST "/contact"
form name<String>
=> contact_submit;
"#;
        let p = compile_source(src).expect("multiline form route must compile");
        assert_eq!(p.routes.len(), 2);
        let post = p
            .routes
            .iter()
            .find(|route| route.name == "contact_submit")
            .expect("POST route must be present");
        assert_eq!(post.form_fields.len(), 1);
        assert_eq!(post.form_fields[0].name, "name");
    }
}
