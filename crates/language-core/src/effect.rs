#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Effect {
    DbRead,
    DbWrite,
    SecurityAudit,
    Network(String),
}

impl Effect {
    pub fn source_name(&self) -> String {
        match self {
            Self::DbRead => "db.read".into(),
            Self::DbWrite => "db.write".into(),
            Self::SecurityAudit => "security.audit".into(),
            Self::Network(target) => format!("net.{target}"),
        }
    }
}
