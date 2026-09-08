use language_core::{AppError, Route, Value};
use std::collections::HashMap;

pub(super) fn enforce_route_tenant(
    route: &Route,
    env: &HashMap<String, Value>,
) -> Result<(), AppError> {
    let Some(field) = route.tenant_field.as_deref() else {
        return Ok(());
    };
    let tenant = env.get(field).and_then(string_value).ok_or(AppError::Forbidden)?;
    let memberships = match env.get("__authMemberships") {
        Some(Value::StringList(values)) => values,
        _ => return Err(AppError::Forbidden),
    };
    if memberships.iter().any(|membership| membership == tenant) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn string_value(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) | Value::Email(value) | Value::Url(value) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use language_core::{HttpMethod, RouteAuth};

    fn route() -> Route {
        Route {
            name: "tenant".into(),
            method: HttpMethod::Get,
            path: "/t/:tenant".into(),
            segments: Vec::new(),
            query_fields: Vec::new(),
            form_fields: Vec::new(),
            form_schema: None,
            json_fields: Vec::new(),
            upload: None,
            validations: Vec::new(),
            tenant_field: Some("tenant".into()),
            auth: RouteAuth::User,
            rate_policy: None,
            budget_profile: None,
            idempotent: false,
            public_cache: None,
            invalidate_caches: Vec::new(),
            handler: "page".into(),
        }
    }

    #[test]
    fn active_tenant_must_be_a_membership() {
        let mut env = HashMap::new();
        env.insert("tenant".into(), Value::String("acme".into()));
        env.insert("__authMemberships".into(), Value::StringList(vec!["acme".into()]));
        assert_eq!(enforce_route_tenant(&route(), &env), Ok(()));
        env.insert("tenant".into(), Value::String("other".into()));
        assert_eq!(enforce_route_tenant(&route(), &env), Err(AppError::Forbidden));
    }
}
