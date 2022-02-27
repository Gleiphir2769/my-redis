use std::io::Cursor;

use my_redis::redis::frame::{self, Frame};


fn main() {
    let a = "*2\r\n$4\r\nfuck\r\n$2\r\nsh\r\n";
    let mut b = Cursor::new(a.as_bytes());
    let r = Frame::parse(&mut b);
    match r {
        Ok(frame) => {
            println!("{:?}", frame)
        }
        Err(error) => {
            println!("{}", error)
        }
    }
}
    