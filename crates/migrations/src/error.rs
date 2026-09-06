use thiserror::Error;

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("unsupported database URL")]
    UnsupportedUrl,
    #[error(
        "remote migration database requires TLS; use --allow-insecure-db only for an explicit trusted exception"
    )]
    InsecureRemoteDb,
    #[error("invalid migration filename `{0}`; expected NNNN_name.sql")]
    InvalidFilename(String),
    #[error("migration directory must be a real directory, not a symlink")]
    UnsafeMigrationDirectory,
    #[error("migration `{0}` must be a regular non-symlink file")]
    UnsafeMigrationFile(String),
    #[error("migration `{0}` exceeds the 4 MiB file limit")]
    MigrationTooLarge(String),
    #[error("too many migration files")]
    TooManyMigrations,
    #[error("duplicate migration version {0}")]
    DuplicateVersion(i64),
    #[error("migration versions must be strictly increasing positive integers")]
    InvalidVersion,
    #[error("migration {version} checksum changed after it was applied")]
    ChecksumMismatch { version: i64 },
    #[error("migration {version} name changed after it was applied")]
    NameMismatch { version: i64 },
    #[error("database contains applied migration {0} that is missing locally")]
    MissingLocalMigration(i64),
    #[error(
        "pending migration {version} is older than already applied migration {max_applied}; renumber it instead of inserting history"
    )]
    OutOfOrderPending { version: i64, max_applied: i64 },
    #[error("migration lock is already held")]
    LockBusy,
    #[error("migration {version} contains no executable SQL")]
    EmptyMigration { version: i64 },
    #[error("migration {version} failed at statement {statement}: {source}")]
    Statement {
        version: i64,
        statement: usize,
        source: sqlx::Error,
    },
}
