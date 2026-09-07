#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CredentialPurpose {
    Password,
    PasswordHash,
    ApiToken,
    SessionToken,
    SessionTokenHash,
    PasswordResetToken,
    PasswordResetTokenHash,
    CsrfToken,
    CsrfTokenHash,
    CryptoKey,
}

impl CredentialPurpose {
    pub const fn source_name(self) -> &'static str {
        match self {
            Self::Password => "Password",
            Self::PasswordHash => "PasswordHash",
            Self::ApiToken => "ApiToken",
            Self::SessionToken => "SessionToken",
            Self::SessionTokenHash => "SessionTokenHash",
            Self::PasswordResetToken => "PasswordResetToken",
            Self::PasswordResetTokenHash => "PasswordResetTokenHash",
            Self::CsrfToken => "CsrfToken",
            Self::CsrfTokenHash => "CsrfTokenHash",
            Self::CryptoKey => "CryptoKey",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "Password" => Some(Self::Password),
            "PasswordHash" => Some(Self::PasswordHash),
            "ApiToken" => Some(Self::ApiToken),
            "SessionToken" => Some(Self::SessionToken),
            "SessionTokenHash" => Some(Self::SessionTokenHash),
            "PasswordResetToken" => Some(Self::PasswordResetToken),
            "PasswordResetTokenHash" => Some(Self::PasswordResetTokenHash),
            "CsrfToken" => Some(Self::CsrfToken),
            "CsrfTokenHash" => Some(Self::CsrfTokenHash),
            "CryptoKey" => Some(Self::CryptoKey),
            _ => None,
        }
    }
}
