use std::fmt;
use std::io;

/// Unified error type for all pdfium-rs operations.
#[derive(Debug)]
pub enum Error {
    /// I/O error (file not found, read failure, etc.)
    Io(io::Error),
    /// Invalid or corrupted PDF structure.
    InvalidPdf(String),
    /// Unsupported PDF feature.
    Unsupported(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        todo!()
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        todo!()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_io() {
        let err = Error::Io(io::Error::new(io::ErrorKind::NotFound, "file not found"));
        let msg = format!("{err}");
        assert!(msg.contains("file not found"), "got: {msg}");
    }

    #[test]
    fn error_display_invalid_pdf() {
        let err = Error::InvalidPdf("missing xref".into());
        let msg = format!("{err}");
        assert!(msg.contains("missing xref"), "got: {msg}");
    }

    #[test]
    fn error_display_unsupported() {
        let err = Error::Unsupported("XFA forms".into());
        let msg = format!("{err}");
        assert!(msg.contains("XFA forms"), "got: {msg}");
    }

    #[test]
    fn error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn error_source_io() {
        let err = Error::Io(io::Error::new(io::ErrorKind::NotFound, "not found"));
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn error_source_invalid_pdf() {
        let err = Error::InvalidPdf("bad".into());
        assert!(std::error::Error::source(&err).is_none());
    }
}
