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

    ProtocalErr,

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
        let state = ReadState::new();
        
        loop {
            let mut msg = Cursor::new(get_line(src, &state)?);

            if !state.reading_multiline {
                match get_u8(&mut msg)? {
                    b'+' => {
                        let data = get_line(&mut msg, &state)?.to_vec();
                        return Ok(Frame::Simple(String::from_utf8(data)?))
                    }
                    b'-' => {
                        return Ok(Frame::Error(String::from_utf8(get_line(&mut msg, &state)?.to_vec())?))
                    }
                    b':' => {
                        return Ok(Frame::Integer(get_decimal(&mut msg)?))
                    }
                    b'$' => {
                        if b'-' == peek_u8(src)? {
                            skip(src, 4)?;
                            return Ok(Frame::Null)
                        } else {
                            let len: usize = get_decimal(src)?.try_into()?;
                            state.bulk_len = len;
                            
                            if src.remaining() < len + 2 {
                                return Err(Error::Incomplete);
                            } else {
                                let data = get_line(&mut msg, &state)?.to_vec();
                                return Ok(Frame::Bulk(Bytes::from(data)))
                            }
                        }
                    }
                    b'*' => {
                        let len = get_decimal(src)?.try_into()?;
                        let mut out = Vec::with_capacity(len);
        
                        for _ in 0..len {
                            out.push(Frame::parse(src)?); 
                        }
        
                        return Ok(Frame::Array(out));
                    }
                    _ => unimplemented!(),
                }
            }    
        }
    }
}

pub fn get_u8(src: &mut Cursor<&[u8]>) -> Result<u8, Error> {
    if !src.has_remaining() {
        return Err(Error::Incomplete);
    }

    Ok(src.get_u8())
}

pub fn get_line<'a>(src: &'a mut Cursor<&[u8]>, state: &ReadState) -> Result<&'a [u8], Error> {
    let start = src.position() as usize;
    let end = src.get_ref().len() - 1;

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            src.set_position((i + 2) as u64);
            if state.bulk_len != 0 && i-start != state.bulk_len {
                return Err(Error::ProtocalErr)
            }    
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

struct  ReadState {
    reading_multiline: bool,
    expected_args_count: u32,
    msg_type: i32,
    args: Vec<String>,
    bulk_len: usize
}

impl ReadState {
    fn finished(&self) -> bool {
        self.expected_args_count > 0 && self.args.len() == self.expected_args_count as usize
    }

    fn new() -> ReadState {
        ReadState { reading_multiline: false, expected_args_count: 0, msg_type: 0, args: vec![], bulk_len: 0 }
    }
}