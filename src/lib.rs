pub mod redis;
pub mod data_struct;
pub mod tcp;


pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;