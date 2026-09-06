use crate::diagnostics::CompileError;
use crate::domain_symbols::{display_domain_symbol, internal_domain_symbol};
use crate::expression::{infer_expr_type, parse_expr_in_namespace, validate_expr};
use crate::handler_types::{StaticType, query_static_type};
use crate::module_namespace::{is_symbol_path, last_segment, resolve};
use crate::source_syntax::{is_identifier, read_ident, split_top_level};
use language_core::{
    BusinessAudit, ObjectAuthorization, Program, QueryCall, QueryCapability, ValueType,
};
use std::collections::HashMap;

pub(super) fn parse_object_authorize(
    text: &str,
    known: &HashMap<String, StaticType>,
    p: &Program,
    handler: &str,
) -> Result<ObjectAuthorization, CompileError> {
    // Grammar: authorize <record> owner <String-field> [or role <Role>]...
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 4 || words[0] != "authorize" || words[2] != "owner" {
        return Err(CompileError::Syntax(format!(
            "{handler} authorization syntax is `authorize <record> owner <field> [or role <Role>]...`"
        )));
    }
    let object = words[1];
    let owner_field = words[3];
    if !is_identifier(object) || !is_identifier(owner_field) {
        return Err(CompileError::Syntax(format!(
            "{handler} authorization identifiers are invalid"
        )));
    }
    let model_name = match known.get(object) {
        Some(StaticType::Model(m)) => m,
        Some(StaticType::OptionalModel(_)) => {
            return Err(CompileError::Syntax(format!(
                "{handler} authorization requires a non-optional loaded object; handle absence before authorization"
            )));
        }
        _ => {
            return Err(CompileError::Syntax(format!(
                "{handler} authorization object `{object}` must be a loaded model"
            )));
        }
    };
    let model = p
        .model(model_name)
        .ok_or_else(|| CompileError::UnknownModel(model_name.clone()))?;
    let field=model.fields.iter().find(|f|f.name==owner_field).ok_or_else(||CompileError::Syntax(format!("{handler} authorization owner field `{owner_field}` does not exist on model `{model_name}`")))?;
    if field.ty != ValueType::String {
        return Err(CompileError::Syntax(format!(
            "{handler} authorization owner field `{owner_field}` must be String"
        )));
    }
    let mut roles = Vec::new();
    let mut i = 4;
    while i < words.len() {
        if i + 2 >= words.len()
            || words[i] != "or"
            || words[i + 1] != "role"
            || !is_identifier(words[i + 2])
        {
            return Err(CompileError::Syntax(format!(
                "{handler} authorization expected `or role <Role>`"
            )));
        }
        let role = words[i + 2].to_string();
        if roles.contains(&role) {
            return Err(CompileError::Syntax(format!(
                "{handler} duplicate authorization role `{role}`"
            )));
        }
        roles.push(role);
        i += 3;
    }
    Ok(ObjectAuthorization {
        object: object.into(),
        owner_field: owner_field.into(),
        allow_roles: roles,
    })
}

fn audit_object_id_type_allowed(ty: ValueType) -> bool {
    !matches!(
        ty,
        ValueType::Image
            | ValueType::Upload
            | ValueType::F32Array
            | ValueType::StringList
            | ValueType::StringDict
    )
}
fn audit_change_type_allowed(ty: ValueType) -> bool {
    !matches!(
        ty,
        ValueType::String
            | ValueType::Email
            | ValueType::Url
            | ValueType::Image
            | ValueType::Upload
            | ValueType::F32Array
            | ValueType::StringList
            | ValueType::StringDict
    )
}

pub(super) fn parse_business_audit(
    handler: &str,
    namespace: &str,
    line: &str,
    known: &HashMap<String, StaticType>,
    p: &Program,
) -> Result<BusinessAudit, CompileError> {
    // Grammar:
    // audit <ObjectType> <object-id-expr> action <actionName>
    // audit <ObjectType> <object-id-expr> action <actionName> from <expr> to <expr>
    let rest = line
        .strip_prefix("audit ")
        .ok_or_else(|| CompileError::Syntax("internal audit parser error".into()))?
        .trim();
    let object_type_raw = rest.split_whitespace().next().ok_or_else(|| {
        CompileError::Syntax(format!("action `{handler}` audit object type expected"))
    })?;
    if !is_symbol_path(object_type_raw) {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` audit object type `{object_type_raw}` is invalid"
        )));
    }
    let object_type = resolve(namespace, object_type_raw);
    if !last_segment(&object_type)
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
    {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` audit object type `{object_type}` must start with an uppercase ASCII letter"
        )));
    }
    if p.model(&object_type).is_none() {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` audit references unknown model/object `{object_type}`"
        )));
    }
    let after_object = rest[object_type_raw.len()..].trim_start();
    let action_pos = after_object.find(" action ").ok_or_else(|| CompileError::Syntax(format!("action `{handler}` audit syntax is `audit <ObjectType> <id> action <name> [from <old> to <new>]`")))?;
    let object_id_raw = after_object[..action_pos].trim();
    if object_id_raw.is_empty() {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` audit object id expression expected"
        )));
    }
    let object_id = parse_expr_in_namespace(object_id_raw, namespace, p)?;
    validate_expr(&object_id, known, p)?;
    let id_ty = infer_expr_type(&object_id, known, p)?;
    if !audit_object_id_type_allowed(id_ty) {
        return Err(CompileError::Syntax(format!(
            "action `{handler}` audit object id must be a scalar auditable type"
        )));
    }

    let action_and_change = after_object[action_pos + " action ".len()..].trim();
    let action = read_ident(action_and_change, 0).ok_or_else(|| {
        CompileError::Syntax(format!("action `{handler}` audit action name expected"))
    })?;
    let tail = action_and_change[action.len()..].trim();
    let (previous, new_value) = if tail.is_empty() {
        (None, None)
    } else {
        let change = tail.strip_prefix("from ").ok_or_else(|| {
            CompileError::Syntax(format!(
                "action `{handler}` audit change must use `from <old> to <new>`"
            ))
        })?;
        let to_pos = change.find(" to ").ok_or_else(|| {
            CompileError::Syntax(format!("action `{handler}` audit `from` requires `to`"))
        })?;
        let old_raw = change[..to_pos].trim();
        let new_raw = change[to_pos + 4..].trim();
        if old_raw.is_empty() || new_raw.is_empty() {
            return Err(CompileError::Syntax(format!(
                "action `{handler}` audit `from`/`to` expressions cannot be empty"
            )));
        }
        let old = parse_expr_in_namespace(old_raw, namespace, p)?;
        let new = parse_expr_in_namespace(new_raw, namespace, p)?;
        validate_expr(&old, known, p)?;
        validate_expr(&new, known, p)?;
        let old_ty = infer_expr_type(&old, known, p)?;
        let new_ty = infer_expr_type(&new, known, p)?;
        if old_ty != new_ty {
            return Err(CompileError::Syntax(format!(
                "action `{handler}` audit `from` and `to` types must match"
            )));
        }
        if !audit_change_type_allowed(old_ty) {
            return Err(CompileError::Syntax(format!(
                "action `{handler}` audit values must be scalar auditable types"
            )));
        }
        (Some(old), Some(new))
    };
    Ok(BusinessAudit {
        object_type,
        object_id,
        action,
        previous,
        new_value,
        source_action: display_domain_symbol(handler),
    })
}

pub(super) fn parse_query_call(
    rhs: &str,
    namespace: &str,
    p: &Program,
    known: &HashMap<String, StaticType>,
    cap: QueryCapability,
) -> Result<Option<(QueryCall, StaticType)>, CompileError> {
    let rhs = rhs.trim().strip_suffix('?').unwrap_or(rhs.trim()).trim();
    let open = match rhs.find('(') {
        Some(v) => v,
        None => return Ok(None),
    };
    if !rhs.ends_with(')') {
        return Ok(None);
    }
    let source_qname = rhs[..open].trim();
    let Some(qname) = internal_domain_symbol(source_qname).map(|name| resolve(namespace, &name))
    else {
        return Ok(None);
    };
    let q = match p.query(&qname) {
        Some(v) => v,
        None => return Ok(None),
    };
    if q.capability != cap {
        return Err(CompileError::Syntax(format!(
            "query `{qname}` requires {:?}, not {:?}",
            q.capability, cap
        )));
    }
    let raw_args = split_top_level(&rhs[open + 1..rhs.len() - 1], ',');
    if raw_args.is_empty() {
        return Err(CompileError::Syntax(format!(
            "query call `{qname}` missing capability argument"
        )));
    }
    let expected_cap = match cap {
        QueryCapability::Db => "db",
        QueryCapability::Transaction => "tx",
    };
    if raw_args[0].trim() != expected_cap {
        return Err(CompileError::Syntax(format!(
            "query `{qname}` first argument must be `{expected_cap}`"
        )));
    }
    if raw_args.len() - 1 != q.params.len() {
        return Err(CompileError::Syntax(format!(
            "query `{qname}` expects {} value arguments",
            q.params.len()
        )));
    }
    let mut args = Vec::new();
    for (raw, param) in raw_args.iter().skip(1).zip(&q.params) {
        let e = parse_expr_in_namespace(raw.trim(), namespace, p)?;
        validate_expr(&e, known, p)?;
        if infer_expr_type(&e, known, p)? != param.ty {
            return Err(CompileError::Syntax(format!(
                "query `{qname}` argument `{}` type mismatch",
                param.name
            )));
        }
        args.push(e);
    }
    Ok(Some((
        QueryCall {
            query: qname.into(),
            args,
        },
        query_static_type(q),
    )))
}
