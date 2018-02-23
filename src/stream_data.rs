use std::io::Read;
use std::io::Write;
use std::net::SocketAddr;
use std::str::FromStr;
use error::MioResult;
use std::cmp;
use std::mem;
use std::io::Result as IoResult;

pub struct StreamData {
    pub reader: Vec<u8>,
    pub writer: Vec<u8>,
    pub remote_addr: SocketAddr,
}

impl StreamData {
    pub fn new(reader: Vec<u8>, writer: Vec<u8>) -> StreamData {
        StreamData {
            reader: reader,
            writer: writer,
            remote_addr: SocketAddr::from_str("0.0.0.0:0").unwrap(),
        }
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
}

impl Read for StreamData {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let amt = cmp::min(buf.len(), self.reader.len());

        let reader = mem::replace(&mut self.reader, Vec::new());
        let (a, b) = reader.split_at(amt);
        buf[..amt].copy_from_slice(a);
        self.reader = b.to_vec();

        Ok(amt)
    }
}

impl Write for StreamData {
    #[inline]
    fn write(&mut self, data: &[u8]) -> IoResult<usize> {
        self.writer.write(data)
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}