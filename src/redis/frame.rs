use core::fmt;
use std::num::TryFromIntError;
use std::{io::Cursor, string::FromUtf8Error};
use std::result::Result;

use bytes::{Buf, Bytes};

#[derive(Debug, Clone)]
pub enum Frame {
    Simple(String),
    Error(String),
    Integer(u64),
    Bulk(Bytes),
    Null,
    Array(Vec<Frame>),
}

#[derive(Debug)]
pub enum Error {
    /// Not enough data is available to parse a message
    Incomplete,

    /// Invalid message encoding
    Other(crate::Error),
}

impl Frame {
    pub(crate) fn array() -> Frame {
        Frame::Array(vec![])
    }

    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), Error> {
        match get_u8(src)? {
            b'+' => {
                get_line(src)?;
                Ok(())
            }
            b'-' => {
                get_line(src)?;
                Ok(())
            }
            b':' => {
                get_decimal(src)?;
                Ok(())
            }
            b'$' => {
                if b'-' == peek_u8(src)? {
                    skip(src, 4)?;
                    Ok(())
                } else {
                    let len: usize = get_decimal(src)?.try_into()?;
                    skip(src, len + 2)?;
                    Ok(())
                }
            }
            b'*' => {
                let len = get_decimal(src)?.try_into()?;

                for _ in 0..len {
                    Frame::check(src)?
                }

                Ok(())
            }
            _ => unimplemented!(),
        }
    }

    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        match get_u8(src)? {
            b'+' => {
                let data = get_line(src)?.to_vec();
                Ok(Frame::Simple(String::from_utf8(data)?))
            }
            b'-' => {
                Ok(Frame::Error(String::from_utf8(get_line(src)?.to_vec())?))
            }
            b':' => {
                Ok(Frame::Integer(get_decimal(src)?))
            }
            b'$' => {
                if b'-' == peek_u8(src)? {
                    skip(src, 4)?;
                    Ok(Frame::Null)
                } else {
                    let len: usize = get_decimal(src)?.try_into()?;
                    let n = len + 2;
                    
                    if src.remaining() < n {
                        return Err(Error::Incomplete);
                    } else {
                        let data = Bytes::copy_from_slice(&src.chunk()[..len]);
                        skip(src, n)?;
                        Ok(Frame::Bulk(data))
                    }
                }
            }
            b'*' => {
                let len = get_decimal(src)?.try_into()?;
                let mut out = Vec::with_capacity(len);

                for _ in 0..len {
                    out.push(Frame::parse(src)?); 
                }

                Ok(Frame::Array(out))
            }
            _ => unimplemented!(),
        }
    }
}

pub fn get_u8(src: &mut Cursor<&[u8]>) -> Result<u8, Error> {
    if !src.has_remaining() {
        return Err(Error::Incomplete);
    }

    Ok(src.get_u8())
}

pub fn get_line<'a>(src: &'a mut Cursor<&[u8]>) -> Result<&'a [u8], Error> {
    let start = src.position() as usize;
    let end = src.get_ref().len() - 1;

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            src.set_position((i + 2) as u64);

            return Ok(&src.get_ref()[start..i]);
        }
    }

    Err(Error::Incomplete)
}

pub fn get_decimal(src: &mut Cursor<&[u8]>) -> Result<u64, Error> {
    use atoi::atoi;

    let line = get_line(src)?;

    atoi::<u64>(line).ok_or_else(|| "protocol error; invalid frame format".into())
}

// 只取一个u8数据但cursor不动
pub fn peek_u8(src: &mut Cursor<&[u8]>) -> Result<u8, Error> {
    if !src.has_remaining() {
        return Err(Error::Incomplete);
    }

    Ok(src.chunk()[0])
}

pub fn skip(src: &mut Cursor<&[u8]>, n: usize) -> Result<(), Error> {
    if src.remaining() < n {
        return Err(Error::Incomplete);
    }

    src.advance(n);

    Ok(())
}

// 实现From方法以使用into将其他类型的对象转为Error
impl From<String> for Error {
    fn from(src: String) -> Error {
        Error::Other(src.into())
    }
}

impl From<&str> for Error {
    fn from(src: &str) -> Error {
        src.to_string().into()
    }
}

impl From<FromUtf8Error> for Error {
    fn from(_src: FromUtf8Error) -> Error {
        "protocol error; invalid frame format".into()
    }
}

impl From<TryFromIntError> for Error {
    fn from(_src: TryFromIntError) -> Error {
        "protocol error; invalid frame format".into()
    }
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Incomplete => "stream ended early".fmt(fmt),
            Error::Other(err) => err.fmt(fmt),
        }
    }
}
