use crate::values::{F32Value, FunctionParam, ValueType};
use crate::web_types::FlashMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    ShiftLeft,
    ShiftRight,
    BitAnd,
    BitXor,
    BitOr,
    LogicalAnd,
    LogicalOr,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinExecutionKind {
    Simple,
    Regex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinMetadata {
    pub source_name: &'static str,
    pub min_args: usize,
    pub max_args: usize,
    pub instruction_cost: u64,
    pub uses_request_state: bool,
    pub execution_kind: BuiltinExecutionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinFunction {
    Sin,
    Cos,
    Sqrt,
    Abs,
    Ln,
    Log10,
    Log,
    Exp,
    Pow,
    Round,
    Floor,
    Ceil,
    MonotonicNanos,
    ToF32,
    StringLen,
    Trim,
    TrimStart,
    TrimEnd,
    Lower,
    Upper,
    Contains,
    StartsWith,
    EndsWith,
    Replace,
    Split,
    Substring,
    IndexOf,
    LastIndexOf,
    CharAt,
    Repeat,
    DictNew,
    ContainsKey,
    RemoveKey,
    RegexMatch,
    RegexReplace,
    RegexCaptures,
}

impl BuiltinFunction {
    pub const ALL: [Self; 36] = [
        Self::Sin,
        Self::Cos,
        Self::Sqrt,
        Self::Abs,
        Self::Ln,
        Self::Log10,
        Self::Log,
        Self::Exp,
        Self::Pow,
        Self::Round,
        Self::Floor,
        Self::Ceil,
        Self::MonotonicNanos,
        Self::ToF32,
        Self::StringLen,
        Self::Trim,
        Self::TrimStart,
        Self::TrimEnd,
        Self::Lower,
        Self::Upper,
        Self::Contains,
        Self::StartsWith,
        Self::EndsWith,
        Self::Replace,
        Self::Split,
        Self::Substring,
        Self::IndexOf,
        Self::LastIndexOf,
        Self::CharAt,
        Self::Repeat,
        Self::DictNew,
        Self::ContainsKey,
        Self::RemoveKey,
        Self::RegexMatch,
        Self::RegexReplace,
        Self::RegexCaptures,
    ];

    pub fn from_source_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|function| function.metadata().source_name == name)
    }

    pub const fn metadata(self) -> BuiltinMetadata {
        let (source_name, min_args, max_args, instruction_cost, uses_request_state) = match self {
            Self::Sin => ("sin", 1, 1, 15, false),
            Self::Cos => ("cos", 1, 1, 15, false),
            Self::Sqrt => ("sqrt", 1, 1, 15, false),
            Self::Abs => ("abs", 1, 1, 1, false),
            Self::Ln => ("ln", 1, 1, 15, false),
            Self::Log10 => ("log10", 1, 1, 15, false),
            Self::Log => ("log", 2, 2, 20, false),
            Self::Exp => ("exp", 1, 1, 15, false),
            Self::Pow => ("pow", 2, 2, 20, false),
            Self::Round => ("round", 1, 1, 2, false),
            Self::Floor => ("floor", 1, 1, 2, false),
            Self::Ceil => ("ceil", 1, 1, 2, false),
            Self::MonotonicNanos => ("monotonicNanos", 0, 0, 3, true),
            Self::ToF32 => ("toF32", 1, 1, 1, false),
            Self::StringLen => ("stringLen", 1, 1, 1, false),
            Self::Trim => ("trim", 1, 1, 2, false),
            Self::TrimStart => ("trimStart", 1, 1, 2, false),
            Self::TrimEnd => ("trimEnd", 1, 1, 2, false),
            Self::Lower => ("lower", 1, 1, 2, false),
            Self::Upper => ("upper", 1, 1, 2, false),
            Self::Contains => ("contains", 2, 2, 2, false),
            Self::StartsWith => ("startsWith", 2, 2, 2, false),
            Self::EndsWith => ("endsWith", 2, 2, 2, false),
            Self::Replace => ("replace", 3, 3, 4, false),
            Self::Split => ("split", 2, 2, 4, false),
            Self::Substring => ("substring", 2, 3, 4, false),
            Self::IndexOf => ("indexOf", 2, 2, 3, false),
            Self::LastIndexOf => ("lastIndexOf", 2, 2, 3, false),
            Self::CharAt => ("charAt", 2, 2, 2, false),
            Self::Repeat => ("repeat", 2, 2, 4, false),
            Self::DictNew => ("dict", 0, 0, 1, false),
            Self::ContainsKey => ("containsKey", 2, 2, 2, false),
            Self::RemoveKey => ("removeKey", 2, 2, 3, false),
            Self::RegexMatch => ("regexMatch", 2, 2, 20, false),
            Self::RegexReplace => ("regexReplace", 3, 3, 30, false),
            Self::RegexCaptures => ("regexCaptures", 2, 2, 25, false),
        };
        let execution_kind = match self {
            Self::RegexMatch | Self::RegexReplace | Self::RegexCaptures => {
                BuiltinExecutionKind::Regex
            }
            _ => BuiltinExecutionKind::Simple,
        };
        BuiltinMetadata {
            source_name,
            min_args,
            max_args,
            instruction_cost,
            uses_request_state,
            execution_kind,
        }
    }

    pub const fn source_name(self) -> &'static str {
        self.metadata().source_name
    }

    pub const fn instruction_cost(self) -> u64 {
        self.metadata().instruction_cost
    }

    pub const fn uses_request_state(self) -> bool {
        self.metadata().uses_request_state
    }

    pub const fn accepts_arity(self, count: usize) -> bool {
        let metadata = self.metadata();
        count >= metadata.min_args && count <= metadata.max_args
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    String(String),
    Int(i64),
    F32(F32Value),
    F32ArrayNew {
        len: Box<Expr>,
        fill: Box<Expr>,
    },
    CollectionIndex {
        collection: String,
        index: Box<Expr>,
    },
    CollectionLen {
        collection: String,
    },
    Bool(bool),
    EnumLiteral {
        enum_id: u16,
        variant: String,
    },
    Slugify(Box<Expr>),
    Builtin {
        function: BuiltinFunction,
        args: Vec<Expr>,
    },
    Variable(String),
    Field {
        base: String,
        field: String,
    },
    Not(Box<Expr>),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlAttrKind {
    Href,
    Action,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlPart {
    Text(String),
    EscapedExpr(Expr),
    Markdown(Expr),
    Image {
        image: Expr,
        alt: Expr,
    },
    Flash,
    RouteAttr {
        kind: HtmlAttrKind,
        route: String,
        args: Vec<Expr>,
    },
    For {
        item: String,
        collection: String,
        template: HtmlTemplate,
    },
    IfSome {
        value: String,
        template: HtmlTemplate,
    },
    ComponentCall {
        component: String,
        args: Vec<Expr>,
    },
    LayoutCall {
        layout: String,
        args: Vec<Expr>,
        content: HtmlTemplate,
    },
    ContentSlot,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlTemplate {
    pub parts: Vec<HtmlPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateParamType {
    Scalar(ValueType),
    Model(String),
    OptionalModel(String),
    ListModel(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateParam {
    pub name: String,
    pub ty: TemplateParamType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentFunction {
    pub name: String,
    pub params: Vec<TemplateParam>,
    pub template: HtmlTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutFunction {
    pub name: String,
    pub params: Vec<TemplateParam>,
    pub template: HtmlTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryCall {
    pub query: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectAuthorization {
    pub object: String,
    pub owner_field: String,
    pub allow_roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeStatement {
    Let {
        name: String,
        expr: Expr,
    },
    Set {
        name: String,
        expr: Expr,
    },
    F32ArraySet {
        array: String,
        index: Expr,
        value: Expr,
    },
    StringDictSet {
        dict: String,
        key: Expr,
        value: Expr,
    },
    While {
        condition: Expr,
        statements: Vec<ComputeStatement>,
    },
    If {
        condition: Expr,
        statements: Vec<ComputeStatement>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Let {
        name: String,
        expr: Expr,
    },
    Set {
        name: String,
        expr: Expr,
    },
    While {
        condition: Expr,
        statements: Vec<ComputeStatement>,
    },
    If {
        condition: Expr,
        statements: Vec<ComputeStatement>,
    },
    F32ArraySet {
        array: String,
        index: Expr,
        value: Expr,
    },
    StringDictSet {
        dict: String,
        key: Expr,
        value: Expr,
    },
    LetQuery {
        name: String,
        call: QueryCall,
    },
    Authorize(ObjectAuthorization),
    Resource {
        profile: String,
        source: SourceLocation,
        statements: Vec<Statement>,
    },
    CanonicalSlug {
        param: String,
        canonical: Expr,
    },
    ReturnHtml(HtmlTemplate),
    ReturnJson(Expr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusinessAudit {
    pub object_type: String,
    pub object_id: Expr,
    pub action: String,
    pub previous: Option<Expr>,
    pub new_value: Option<Expr>,
    pub source_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxStatement {
    LetQuery { name: String, call: QueryCall },
    Query(QueryCall),
    BusinessAudit(BusinessAudit),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionStatement {
    Let {
        name: String,
        expr: Expr,
    },
    Set {
        name: String,
        expr: Expr,
    },
    While {
        condition: Expr,
        statements: Vec<ComputeStatement>,
    },
    If {
        condition: Expr,
        statements: Vec<ComputeStatement>,
    },
    F32ArraySet {
        array: String,
        index: Expr,
        value: Expr,
    },
    StringDictSet {
        dict: String,
        key: Expr,
        value: Expr,
    },
    LetQuery {
        name: String,
        call: QueryCall,
    },
    Authorize(ObjectAuthorization),
    Transaction {
        statements: Vec<TxStatement>,
    },
    Resource {
        profile: String,
        source: SourceLocation,
        statements: Vec<ActionStatement>,
    },
    Flash(FlashMessage),
    ReturnRedirect(Expr),
    ReturnJson(Expr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub function: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUse {
    pub profile: String,
    pub source: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageBody {
    Statements(Vec<Statement>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionBody {
    Statements(Vec<ActionStatement>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageFunction {
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub needs_db: bool,
    pub body: PageBody,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionFunction {
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub needs_db: bool,
    pub body: ActionBody,
}
