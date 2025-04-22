use std::{fmt, io};

#[derive(Debug)]
pub enum FilamentError {
    Io(io::Error),
    Parse(String),
    Elaboration(String),
    Graph(String),
    Export(Box<dyn std::error::Error>),
}

impl std::error::Error for FilamentError {}

impl fmt::Display for FilamentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilamentError::Io(err) => write!(f, "I/O error: {}", err),
            FilamentError::Parse(msg) => write!(f, "Parse error: {}", msg),
            FilamentError::Elaboration(msg) => write!(f, "Elaboration error: {}", msg),
            FilamentError::Graph(msg) => write!(f, "Graph error: {}", msg),
            FilamentError::Export(err) => write!(f, "Export error: {}", err),
        }
    }
}

impl From<io::Error> for FilamentError {
    fn from(error: io::Error) -> Self {
        FilamentError::Io(error)
    }
}

impl From<String> for FilamentError {
    fn from(error: String) -> Self {
        FilamentError::Parse(error)
    }
}

impl From<crate::export::Error> for FilamentError {
    fn from(error: crate::export::Error) -> Self {
        FilamentError::Export(Box::new(error))
    }
}
