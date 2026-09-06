use crate::execution_context::Budget;
use crate::rendering::{build_route_url, escape_html_into, render_safe_markdown_into};
use crate::vm::eval_expr;
use language_core::{AppError, Html, HtmlAttrKind, HtmlPart, HtmlTemplate, Program, Value};
use std::collections::HashMap;

pub(super) fn render_html(
    program: &Program,
    t: &HtmlTemplate,
    env: &HashMap<String, Value>,
    budget: &mut Budget,
) -> Result<Html, AppError> {
    let mut out = String::new();
    render_template_into(program, t, env, budget, &mut out, None)?;
    Ok(Html::trusted_compiler_output(out))
}
fn render_template_into(
    program: &Program,
    t: &HtmlTemplate,
    env: &HashMap<String, Value>,
    budget: &mut Budget,
    out: &mut String,
    slot: Option<(&HtmlTemplate, &HashMap<String, Value>)>,
) -> Result<(), AppError> {
    for p in &t.parts {
        budget.charge(1)?;
        match p {
            HtmlPart::Text(v) => {
                budget.charge_alloc(v.len() as u64)?;
                out.push_str(v)
            }
            HtmlPart::EscapedExpr(e) => {
                let v = eval_expr(e, env, budget)?
                    .display_text()
                    .ok_or(AppError::Internal)?;
                budget.charge_alloc((v.len() as u64).saturating_mul(12))?;
                escape_html_into(&v, out);
            }
            HtmlPart::Markdown(e) => {
                let v = match eval_expr(e, env, budget)? {
                    Value::String(v) => v,
                    _ => return Err(AppError::Internal),
                };
                render_safe_markdown_into(&v, out, budget)?;
            }
            HtmlPart::Flash => {
                let kind = match env.get("__flashKind") {
                    Some(Value::String(v)) => v.as_str(),
                    None => "",
                    _ => return Err(AppError::Internal),
                };
                let message = match env.get("__flashMessage") {
                    Some(Value::String(v)) => v.as_str(),
                    None => "",
                    _ => return Err(AppError::Internal),
                };
                if !message.is_empty() {
                    if !matches!(kind, "success" | "info" | "warning" | "error") {
                        return Err(AppError::Internal);
                    }
                    budget.charge_alloc(
                        (message.len() as u64).saturating_mul(12).saturating_add(96),
                    )?;
                    out.push_str("<div class=\"rw-flash rw-flash-");
                    out.push_str(kind);
                    out.push_str("\" role=\"status\">");
                    escape_html_into(message, out);
                    out.push_str("</div>");
                }
            }
            HtmlPart::Image { image, alt } => {
                let image = match eval_expr(image, env, budget)? {
                    Value::Image(v) => v,
                    _ => return Err(AppError::Internal),
                };
                let alt = match eval_expr(alt, env, budget)? {
                    Value::String(v) => v,
                    _ => return Err(AppError::Internal),
                };
                budget.charge_alloc((image.path.len() + alt.len() + 128) as u64)?;
                out.push_str("<img src=\"/__rw/media/");
                escape_html_into(&image.path, out);
                out.push_str("\" alt=\"");
                escape_html_into(&alt, out);
                out.push_str("\" width=\"");
                out.push_str(&image.width.to_string());
                out.push_str("\" height=\"");
                out.push_str(&image.height.to_string());
                out.push_str("\" loading=\"lazy\" decoding=\"async\">");
            }
            HtmlPart::RouteAttr { kind, route, args } => {
                let target = program
                    .routes
                    .iter()
                    .find(|r| r.name == *route)
                    .ok_or(AppError::Internal)?;
                let url = build_route_url(
                    target,
                    args,
                    env,
                    budget,
                    matches!(kind, HtmlAttrKind::Href),
                )?;
                match kind {
                    HtmlAttrKind::Href => out.push_str("href=\""),
                    HtmlAttrKind::Action => out.push_str("action=\""),
                };
                escape_html_into(&url, out);
                out.push('"');
            }
            HtmlPart::For {
                item,
                collection,
                template,
            } => {
                let values = match env.get(collection) {
                    Some(Value::List(v)) => v.clone(),
                    _ => return Err(AppError::Internal),
                };
                for value in values {
                    budget.charge(1)?;
                    let mut nested = env.clone();
                    nested.insert(item.clone(), value);
                    render_template_into(program, template, &nested, budget, out, slot)?;
                }
            }
            HtmlPart::IfSome { value, template } => match env.get(value) {
                Some(Value::Null) => {}
                Some(Value::Record(_)) => {
                    render_template_into(program, template, env, budget, out, slot)?
                }
                _ => return Err(AppError::Internal),
            },
            HtmlPart::ComponentCall { component, args } => {
                let def = program.component(component).ok_or(AppError::Internal)?;
                if def.params.len() != args.len() {
                    return Err(AppError::Internal);
                }
                let mut nested = HashMap::new();
                for key in ["__flashKind", "__flashMessage"] {
                    if let Some(value) = env.get(key) {
                        nested.insert(key.to_string(), value.clone());
                    }
                }
                for (param, arg) in def.params.iter().zip(args) {
                    let value = eval_expr(arg, env, budget)?;
                    budget.charge_value(&value)?;
                    nested.insert(param.name.clone(), value);
                }
                render_template_into(program, &def.template, &nested, budget, out, None)?;
            }
            HtmlPart::LayoutCall {
                layout,
                args,
                content,
            } => {
                let def = program.layout(layout).ok_or(AppError::Internal)?;
                if def.params.len() != args.len() {
                    return Err(AppError::Internal);
                }
                let mut nested = HashMap::new();
                for key in ["__flashKind", "__flashMessage"] {
                    if let Some(value) = env.get(key) {
                        nested.insert(key.to_string(), value.clone());
                    }
                }
                for (param, arg) in def.params.iter().zip(args) {
                    let value = eval_expr(arg, env, budget)?;
                    budget.charge_value(&value)?;
                    nested.insert(param.name.clone(), value);
                }
                render_template_into(
                    program,
                    &def.template,
                    &nested,
                    budget,
                    out,
                    Some((content, env)),
                )?;
            }
            HtmlPart::ContentSlot => {
                let (content, content_env) = slot.ok_or(AppError::Internal)?;
                render_template_into(program, content, content_env, budget, out, None)?;
            }
        }
    }
    Ok(())
}
