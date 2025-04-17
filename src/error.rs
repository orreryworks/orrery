use std::{fmt, io};

#[derive(Debug)]
pub enum FilamentError {
    IoError(io::Error),
    ParseError(String),
    ElaborationError(String),
    GraphError(String),
    ExportError(Box<dyn std::error::Error>),
}

impl std::error::Error for FilamentError {}

impl fmt::Display for FilamentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilamentError::IoError(err) => write!(f, "I/O error: {}", err),
            FilamentError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            FilamentError::ElaborationError(msg) => write!(f, "Elaboration error: {}", msg),
            FilamentError::GraphError(msg) => write!(f, "Graph error: {}", msg),
            FilamentError::ExportError(err) => write!(f, "Export error: {}", err),
        }
    }
}

impl From<io::Error> for FilamentError {
    fn from(error: io::Error) -> Self {
        FilamentError::IoError(error)
    }
}

impl From<String> for FilamentError {
    fn from(error: String) -> Self {
        FilamentError::ParseError(error)
    }
}

impl From<crate::export::Error> for FilamentError {
    fn from(error: crate::export::Error) -> Self {
        FilamentError::ExportError(Box::new(error))
    }
}
