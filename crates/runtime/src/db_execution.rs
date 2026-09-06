use super::scalars::{is_canonical_slug, normalize_email, normalize_url};
use crate::execution_context::Budget;
use crate::vm::eval_expr;
use chrono::{DateTime, NaiveDate, Utc};
use data::{BindSet, ColumnSpec, Database, DbScalarType, DbValue, PreparedSql, RowShape};
use language_core::{
    AppError, Expr, F32Value, ImageRef, Program, QueryCall, QueryReturn, Value, ValueType,
};
use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

pub(super) async fn execute_read_query(
    program: &Program,
    call: &QueryCall,
    env: &HashMap<String, Value>,
    budget: &mut Budget,
    db: &Database,
) -> Result<Value, AppError> {
    budget.charge(5)?;
    let q = program.query(&call.query).ok_or(AppError::Internal)?;
    let model_name = q.return_type.model_name().ok_or(AppError::Internal)?;
    let model = program.model(model_name).ok_or(AppError::Internal)?;
    let sql = PreparedSql::compile(q.sql.clone()).map_err(|_| AppError::Database)?;
    let binds = build_binds(q, &call.args, env, budget)?;
    let shape = row_shape(model);
    let rows = db
        .fetch_all(&sql, &binds, &shape)
        .await
        .map_err(|_| AppError::Database)?;
    decode_query_rows(program, &q.return_type, model, rows)
}

pub(crate) async fn execute_tx_query(
    program: &Program,
    call: &QueryCall,
    env: &HashMap<String, Value>,
    budget: &mut Budget,
    tx: &mut data::DbTransaction<'_>,
) -> Result<Value, AppError> {
    budget.charge(5)?;
    let q = program.query(&call.query).ok_or(AppError::Internal)?;
    let sql = PreparedSql::compile(q.sql.clone()).map_err(|_| AppError::Database)?;
    let binds = build_binds(q, &call.args, env, budget)?;
    if matches!(&q.return_type, &QueryReturn::Void | &QueryReturn::Changed) {
        let result = tx.execute(&sql, &binds).await.map_err(|e| {
            if e.is_unique_violation() {
                AppError::Conflict
            } else {
                AppError::Database
            }
        })?;
        if matches!(&q.return_type, &QueryReturn::Changed) {
            return match result.rows_affected {
                1 => Ok(Value::Bool(true)),
                0 => Err(AppError::Conflict),
                _ => Err(AppError::Database),
            };
        }
        return Ok(Value::Bool(true));
    }
    let model_name = q.return_type.model_name().ok_or(AppError::Internal)?;
    let model = program.model(model_name).ok_or(AppError::Internal)?;
    let shape = row_shape(model);
    let rows = tx.fetch_all(&sql, &binds, &shape).await.map_err(|e| {
        if e.is_unique_violation() {
            AppError::Conflict
        } else {
            AppError::Database
        }
    })?;
    decode_query_rows(program, &q.return_type, model, rows)
}

fn decode_query_rows(
    program: &Program,
    return_type: &QueryReturn,
    model: &language_core::Model,
    rows: Vec<data::DbRow>,
) -> Result<Value, AppError> {
    fn record(
        program: &Program,
        model: &language_core::Model,
        row: &data::DbRow,
    ) -> Result<Value, AppError> {
        let mut record = HashMap::new();
        for field in &model.fields {
            let value = row.get(&field.name).ok_or(AppError::Database)?;
            record.insert(field.name.clone(), db_to_value(program, value, field.ty)?);
        }
        Ok(Value::Record(record))
    }
    match return_type {
        QueryReturn::Void | QueryReturn::Changed => Err(AppError::Internal),
        QueryReturn::One(_) => {
            if rows.len() != 1 {
                return if rows.is_empty() {
                    Err(AppError::NotFound)
                } else {
                    Err(AppError::Database)
                };
            }
            record(program, model, &rows[0])
        }
        QueryReturn::Optional(_) => {
            if rows.len() > 1 {
                return Err(AppError::Database);
            }
            if rows.is_empty() {
                Ok(Value::Null)
            } else {
                record(program, model, &rows[0])
            }
        }
        QueryReturn::List(_) => {
            let mut out = Vec::with_capacity(rows.len());
            for row in &rows {
                out.push(record(program, model, row)?);
            }
            Ok(Value::List(out))
        }
    }
}

fn build_binds(
    q: &language_core::QueryFunction,
    args: &[Expr],
    env: &HashMap<String, Value>,
    budget: &mut Budget,
) -> Result<BindSet, AppError> {
    if q.params.len() != args.len() {
        return Err(AppError::Internal);
    }
    let mut binds = BindSet::new();
    for (param, expr) in q.params.iter().zip(args) {
        let value = eval_expr(expr, env, budget)?;
        let db_value = match (value, param.ty) {
            (Value::String(x), ValueType::String | ValueType::Slug) => DbValue::String(x),
            (Value::Email(x), ValueType::Email) => DbValue::String(x),
            (Value::Url(x), ValueType::Url) => DbValue::String(x),
            (Value::Int(x), ValueType::Int) => DbValue::Int(x),
            (Value::F32(x), ValueType::F32) => DbValue::String(x.get().to_string()),
            (Value::Bool(x), ValueType::Bool) => DbValue::Bool(x),
            (Value::Date(x), ValueType::Date) => DbValue::String(x.format("%Y-%m-%d").to_string()),
            (Value::DateTime(x), ValueType::DateTime) => {
                DbValue::String(x.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true))
            }
            (Value::Uuid(x), ValueType::Uuid) => DbValue::String(x.hyphenated().to_string()),
            (Value::Decimal(x), ValueType::Decimal) => DbValue::String(x.normalize().to_string()),
            (Value::Image(x), ValueType::Image) => DbValue::String(x.canonical()),
            (Value::Enum { enum_id, variant }, ValueType::Enum(expected))
                if enum_id == expected =>
            {
                DbValue::String(variant)
            }
            _ => return Err(AppError::Internal),
        };
        binds
            .insert(param.name.clone(), db_value)
            .map_err(|_| AppError::Database)?;
    }
    Ok(binds)
}

fn row_shape(model: &language_core::Model) -> RowShape {
    RowShape {
        columns: model
            .fields
            .iter()
            .map(|field| ColumnSpec {
                name: field.name.clone(),
                ty: match field.ty {
                    ValueType::String | ValueType::Email | ValueType::Url | ValueType::Slug => {
                        DbScalarType::String
                    }
                    ValueType::Int => DbScalarType::Int,
                    ValueType::F32 => DbScalarType::String,
                    ValueType::F32Array => unreachable!("Array<F32> cannot be a model field"),
                    ValueType::StringList => unreachable!("List<String> cannot be a model field"),
                    ValueType::StringDict => {
                        unreachable!("Dict<String,String> cannot be a model field")
                    }
                    ValueType::Bool => DbScalarType::Bool,
                    ValueType::Date
                    | ValueType::DateTime
                    | ValueType::Uuid
                    | ValueType::Decimal
                    | ValueType::Image
                    | ValueType::Enum(_) => DbScalarType::String,
                    ValueType::Upload => unreachable!("Upload cannot be a model field"),
                },
            })
            .collect(),
    }
}

pub(crate) fn db_to_value(
    program: &Program,
    value: &DbValue,
    ty: ValueType,
) -> Result<Value, AppError> {
    match (value, ty) {
        (DbValue::String(x), ValueType::String | ValueType::Slug) => {
            if ty == ValueType::Slug && !is_canonical_slug(x) {
                Err(AppError::Database)
            } else {
                Ok(Value::String(x.clone()))
            }
        }
        (DbValue::String(x), ValueType::Email) => normalize_email(x)
            .filter(|v| v == x)
            .map(Value::Email)
            .ok_or(AppError::Database),
        (DbValue::String(x), ValueType::Url) => normalize_url(x)
            .filter(|v| v == x)
            .map(Value::Url)
            .ok_or(AppError::Database),
        (DbValue::Int(x), ValueType::Int) => Ok(Value::Int(*x)),
        (DbValue::String(x), ValueType::F32) => x
            .parse::<f32>()
            .ok()
            .and_then(F32Value::new)
            .map(Value::F32)
            .ok_or(AppError::Database),
        (DbValue::Bool(x), ValueType::Bool) => Ok(Value::Bool(*x)),
        (DbValue::String(x), ValueType::Date) => NaiveDate::parse_from_str(x, "%Y-%m-%d")
            .map(Value::Date)
            .map_err(|_| AppError::Database),
        (DbValue::String(x), ValueType::DateTime) => DateTime::parse_from_rfc3339(x)
            .map(|v| Value::DateTime(v.with_timezone(&Utc)))
            .map_err(|_| AppError::Database),
        (DbValue::String(x), ValueType::Uuid) => Uuid::parse_str(x)
            .map(Value::Uuid)
            .map_err(|_| AppError::Database),
        (DbValue::String(x), ValueType::Decimal) => Decimal::from_str_exact(x)
            .map(Value::Decimal)
            .map_err(|_| AppError::Database),
        (DbValue::String(x), ValueType::Image) => ImageRef::parse(x)
            .map(Value::Image)
            .ok_or(AppError::Database),
        (DbValue::String(x), ValueType::Enum(enum_id)) => {
            let def = program.enum_by_id(enum_id).ok_or(AppError::Internal)?;
            if def.variants.iter().any(|v| v == x) {
                Ok(Value::Enum {
                    enum_id,
                    variant: x.clone(),
                })
            } else {
                Err(AppError::Database)
            }
        }
        _ => Err(AppError::Database),
    }
}
