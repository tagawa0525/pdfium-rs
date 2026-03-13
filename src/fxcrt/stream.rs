use std::io::{self, Read, Seek, Write};

/// Readable and seekable stream for PDF input.
///
/// Combines `Read` + `Seek` semantics. Corresponds to C++
/// `IFX_SeekableReadStream`.
pub trait PdfRead: Read + Seek {
    fn stream_len(&mut self) -> io::Result<u64>;
}

/// Writable stream for PDF output.
///
/// Corresponds to C++ `IFX_WriteStream`.
pub trait PdfWrite: Write {
    fn flush_output(&mut self) -> io::Result<()>;
}

/// In-memory read stream backed by a byte slice.
pub struct MemoryStream {
    data: Vec<u8>,
    pos: u64,
}

impl MemoryStream {
    pub fn new(data: Vec<u8>) -> Self {
        todo!()
    }

    pub fn from_slice(data: &[u8]) -> Self {
        todo!()
    }

    pub fn as_bytes(&self) -> &[u8] {
        todo!()
    }
}

impl Read for MemoryStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        todo!()
    }
}

impl Seek for MemoryStream {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        todo!()
    }
}

impl PdfRead for MemoryStream {
    fn stream_len(&mut self) -> io::Result<u64> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::SeekFrom;

    #[test]
    fn memory_stream_new() {
        let stream = MemoryStream::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(stream.as_bytes(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn memory_stream_from_slice() {
        let stream = MemoryStream::from_slice(b"hello");
        assert_eq!(stream.as_bytes(), b"hello");
    }

    #[test]
    fn memory_stream_read() {
        let mut stream = MemoryStream::new(vec![1, 2, 3, 4, 5]);
        let mut buf = [0u8; 3];
        let n = stream.read(&mut buf).unwrap();
        assert_eq!(n, 3);
        assert_eq!(buf, [1, 2, 3]);

        let n = stream.read(&mut buf).unwrap();
        assert_eq!(n, 2);
        assert_eq!(buf[..2], [4, 5]);
    }

    #[test]
    fn memory_stream_read_empty() {
        let mut stream = MemoryStream::new(vec![]);
        let mut buf = [0u8; 3];
        let n = stream.read(&mut buf).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn memory_stream_seek_start() {
        let mut stream = MemoryStream::new(vec![10, 20, 30, 40, 50]);
        stream.seek(SeekFrom::Start(2)).unwrap();
        let mut buf = [0u8; 1];
        stream.read(&mut buf).unwrap();
        assert_eq!(buf[0], 30);
    }

    #[test]
    fn memory_stream_seek_end() {
        let mut stream = MemoryStream::new(vec![10, 20, 30, 40, 50]);
        stream.seek(SeekFrom::End(-2)).unwrap();
        let mut buf = [0u8; 1];
        stream.read(&mut buf).unwrap();
        assert_eq!(buf[0], 40);
    }

    #[test]
    fn memory_stream_seek_current() {
        let mut stream = MemoryStream::new(vec![10, 20, 30, 40, 50]);
        stream.seek(SeekFrom::Start(1)).unwrap();
        stream.seek(SeekFrom::Current(2)).unwrap();
        let mut buf = [0u8; 1];
        stream.read(&mut buf).unwrap();
        assert_eq!(buf[0], 40);
    }

    #[test]
    fn memory_stream_seek_returns_position() {
        let mut stream = MemoryStream::new(vec![0; 100]);
        let pos = stream.seek(SeekFrom::Start(42)).unwrap();
        assert_eq!(pos, 42);
    }

    #[test]
    fn memory_stream_len() {
        let mut stream = MemoryStream::new(vec![0; 42]);
        assert_eq!(stream.stream_len().unwrap(), 42);
    }

    #[test]
    fn memory_stream_seek_beyond_end() {
        let mut stream = MemoryStream::new(vec![1, 2, 3]);
        stream.seek(SeekFrom::Start(10)).unwrap();
        let mut buf = [0u8; 1];
        let n = stream.read(&mut buf).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn memory_stream_seek_before_start() {
        let mut stream = MemoryStream::new(vec![1, 2, 3]);
        let result = stream.seek(SeekFrom::Current(-1));
        assert!(result.is_err());
    }

    #[test]
    fn pdf_read_trait_object() {
        let mut stream: Box<dyn PdfRead> = Box::new(MemoryStream::new(b"pdf data".to_vec()));
        let mut buf = [0u8; 3];
        stream.read(&mut buf).unwrap();
        assert_eq!(&buf, b"pdf");
    }
}
