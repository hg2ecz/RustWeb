use std::fmt;

#[derive(Debug)]
pub enum CompileError {
    Io(std::io::Error),
    Syntax(String),
    DuplicateHandler(String),
    DuplicateRoute(String),
    UnknownHandler(String),
    RouteParamMismatch(String),
    UnknownVariable(String),
    UnsafeSql(String),
    UnsafeHtml(String),
    Security {
        code: &'static str,
        message: String,
        help: Option<String>,
    },
    UnknownQuery(String),
    UnknownModel(String),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Syntax(v) => write!(f, "syntax error: {v}"),
            Self::DuplicateHandler(v) => write!(f, "duplicate handler `{v}`"),
            Self::DuplicateRoute(v) => write!(f, "duplicate route `{v}`"),
            Self::UnknownHandler(v) => write!(f, "route references unknown handler `{v}`"),
            Self::RouteParamMismatch(v) => write!(f, "route parameter mismatch: {v}"),
            Self::UnknownVariable(v) => write!(f, "unknown variable `{v}`"),
            Self::UnsafeSql(v) => write!(f, "security error[SEC-A05-001]: unsafe SQL: {v}"),
            Self::UnsafeHtml(v) => write!(f, "security error[SEC-A05-002]: unsafe HTML: {v}"),
            Self::Security {
                code,
                message,
                help,
            } => {
                write!(f, "security error[{code}]: {message}")?;
                if let Some(help) = help {
                    write!(f, "\nhelp: {help}")?;
                }
                Ok(())
            }
            Self::UnknownQuery(v) => write!(f, "unknown query `{v}`"),
            Self::UnknownModel(v) => write!(f, "unknown model `{v}`"),
        }
    }
}

impl CompileError {
    pub(crate) fn security(
        code: &'static str,
        message: impl Into<String>,
        help: impl Into<Option<String>>,
    ) -> Self {
        Self::Security {
            code,
            message: message.into(),
            help: help.into(),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<std::io::Error> for CompileError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
