use crate::module_namespace::resolve;
use language_core::{Program, ValueType};

pub(crate) fn resolve_value_type(
    raw: &str,
    namespace: &str,
    program: &Program,
) -> Option<ValueType> {
    ValueType::parse(raw).or_else(|| {
        let symbol = resolve(namespace, raw);
        program
            .enum_by_name(&symbol)
            .map(|(id, _)| ValueType::Enum(id))
    })
}
