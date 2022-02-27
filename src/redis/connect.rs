use std::{io::{Cursor, self}};

use bytes::{BytesMut, Buf};
use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}};
use crate::Result;
use super::frame::Frame;

struct Connection {
    stream: TcpStream,
    buf: BytesMut
}

impl Connection {
    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            if 0 == self.stream.read_buf(&mut self.buf).await? {
                if self.buf.is_empty() {
                    return Ok(None);
                } else {
                    return Err("connection has been canceled by peer".into());
                }
            }
        }
    }
    pub async fn write_frame(&mut self, frame: &Frame) -> Result<()> {
        Ok(())
    }
    pub fn new(stream: TcpStream) -> Connection {
        Connection{
            stream,
            buf: BytesMut::with_capacity(4096)
        }
    }
    pub fn parse_frame(&mut self) -> crate::Result<Option<Frame>> {
        use super::frame::Error::Incomplete;
        let mut buf = Cursor::new(&self.buf[..]);

        match Frame::check(&mut buf) {
            Ok(_) => {
                let len = buf.position() as usize;
                buf.set_position(0);
                let frame = Frame::parse(&mut buf)?;
                self.buf.advance(len);
                Ok(Some(frame))
            }
            Err(Incomplete) => {
                Ok(None)
            }
            Err(e) => {
                Err(e.into())
            }
        }

    }

    pub async fn write_value(&mut self, frame: &Frame) -> io::Result<()> {
        match frame {
            Frame::Simple(val) => {
                self.stream.write_u8(b'+').await?;
                self.stream.write_all(&mut val.as_bytes()).await?;
                self.stream.write_all("\r\n".as_bytes()).await?;
            }
            Frame::Error(val) => {
                self.stream.write_u8(b'-').await?;
                self.stream.write_all(&mut val.as_bytes()).await?;
                self.stream.write_all("\r\n".as_bytes()).await?;
            }
            Frame::Bulk(val) => {
                let len = val.len();

                self.stream.write_u8(b'$').await?;
                self.write_decimal(len as u64).await?;
                self.stream.write_all(val).await?;
                self.stream.write_all("\r\n".as_bytes()).await?;
            }
            Frame::Integer(val) => {
                self.stream.write_u8(b':').await?;
                self.write_decimal(*val).await?;
            }
            Frame::Null => {
                self.stream.write_all("\r\n".as_bytes()).await?;
            }
            Frame::Array(val) => {
                unreachable!()
            }
        }

        Ok(())
    }

    pub async fn write_decimal(&mut self, val: u64) -> io::Result<()> {
        use std::io::Write;

        let mut buf = [0u8, 20];
        let mut buf = Cursor::new(&mut buf[..]);

        write!(&mut buf, "{}", val)?;
        self.stream.write_all(buf.get_ref()).await?;
        self.stream.write_all("\r\n".as_bytes()).await?;
        Ok(())
    }
}