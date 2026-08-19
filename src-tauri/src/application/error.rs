use std::fmt::{Display, Formatter};

use crate::{core::action_execution::ActionExecutionError, storage::StorageError};

#[derive(Debug)]
pub enum AppError {
    Validation(String),
    NotFound(String),
    Conflict(String),
    Domain(ActionExecutionError),
    Storage(StorageError),
    Serialization(String),
}

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(message)
            | Self::NotFound(message)
            | Self::Conflict(message)
            | Self::Serialization(message) => formatter.write_str(message),
            Self::Domain(error) => error.fmt(formatter),
            Self::Storage(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for AppError {}

impl From<ActionExecutionError> for AppError {
    fn from(error: ActionExecutionError) -> Self {
        Self::Domain(error)
    }
}

impl From<StorageError> for AppError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::NotFound(message) => Self::NotFound(message),
            StorageError::Conflict(message) => Self::Conflict(message),
            other => Self::Storage(other),
        }
    }
}
