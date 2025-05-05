pub mod svg;

use crate::layout::{component, sequence};

// A single Exporter trait that works with any layout type
pub trait Exporter {
    fn export_component_layout(&self, _layout: &component::Layout) -> Result<(), Error> {
        Err(Error::Render(
            "Component layout export not implemented".to_string(),
        ))
    }

    fn export_sequence_layout(&self, _layout: &sequence::Layout) -> Result<(), Error> {
        Err(Error::Render(
            "Sequence layout export not implemented".to_string(),
        ))
    }
}

#[derive(Debug)]
pub enum Error {
    Render(String),
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Render(msg) => write!(f, "Render error: {msg}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Render(_) => None,
            Self::Io(err) => Some(err),
        }
    }
}
