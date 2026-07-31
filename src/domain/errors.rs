use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct RedirectSourceError {
    message: String,
}

impl RedirectSourceError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Error)]
#[error("{message}")]
pub struct RedirectEventSinkError {
    message: String,
}

impl RedirectEventSinkError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
