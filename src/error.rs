use std::io;

/// Unified error type for all pdfium-rs operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O error (file not found, read failure, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    /// Invalid or corrupted PDF structure.
    #[error("invalid PDF: {0}")]
    InvalidPdf(String),
    /// Unsupported PDF feature.
    #[error("unsupported: {0}")]
    Unsupported(String),
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
