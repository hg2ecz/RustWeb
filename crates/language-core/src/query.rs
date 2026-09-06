use crate::FunctionParam;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryCapability {
    Db,
    Transaction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryReturn {
    Void,
    /// Mutating query contract: exactly one row must be affected.
    Changed,
    One(String),
    Optional(String),
    List(String),
}

impl QueryReturn {
    pub fn model_name(&self) -> Option<&str> {
        match self {
            Self::Void | Self::Changed => None,
            Self::One(v) | Self::Optional(v) | Self::List(v) => Some(v),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryFunction {
    pub name: String,
    pub capability: QueryCapability,
    pub params: Vec<FunctionParam>,
    pub return_type: QueryReturn,
    pub sql: String,
}
