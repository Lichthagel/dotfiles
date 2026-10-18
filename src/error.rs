use std::io;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, InstallError>;

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("usage error: {0}")]
    Usage(String),
    #[error("operation cancelled")]
    Cancelled,
    #[error("package operation failed: {0}")]
    Package(String),
    #[error("secret operation failed: {0}")]
    Secret(String),
    #[error("setup failed: {0}")]
    Setup(String),
    #[error("mapping failed: {0}")]
    Mapping(String),
    #[error("asset error: {0}")]
    Asset(String),
    #[error("module state error: {0}")]
    State(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitCategory {
    Success,
    Usage,
    Operational,
}

impl ExitCategory {
    pub const fn status_code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::Usage => 2,
            Self::Operational => 1,
        }
    }
}

impl InstallError {
    pub fn secret(message: impl Into<String>) -> Self {
        Self::Secret(message.into())
    }

    pub const fn exit_category(&self) -> ExitCategory {
        match self {
            Self::Usage(_) => ExitCategory::Usage,
            Self::Cancelled => ExitCategory::Success,
            Self::Package(_)
            | Self::Secret(_)
            | Self::Setup(_)
            | Self::Mapping(_)
            | Self::Asset(_)
            | Self::State(_)
            | Self::Io(_) => ExitCategory::Operational,
        }
    }
}
