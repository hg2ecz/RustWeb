use language_core::{FunctionParam, QueryFunction, QueryReturn, ValueType};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HandlerReturnKind {
    Html,
    Redirect,
    Json,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum StaticType {
    Scalar(ValueType),
    Model(String),
    OptionalModel(String),
    ListModel(String),
    Upload,
}

pub(super) fn scalar_known(params: &[FunctionParam]) -> HashMap<String, StaticType> {
    params
        .iter()
        .map(|param| {
            let ty = if param.ty == ValueType::Upload {
                StaticType::Upload
            } else {
                StaticType::Scalar(param.ty)
            };
            (param.name.clone(), ty)
        })
        .collect()
}

pub(super) fn query_static_type(query: &QueryFunction) -> StaticType {
    match &query.return_type {
        QueryReturn::Void | QueryReturn::Changed => StaticType::Scalar(ValueType::Bool),
        QueryReturn::One(model) => StaticType::Model(model.clone()),
        QueryReturn::Optional(model) => StaticType::OptionalModel(model.clone()),
        QueryReturn::List(model) => StaticType::ListModel(model.clone()),
    }
}
