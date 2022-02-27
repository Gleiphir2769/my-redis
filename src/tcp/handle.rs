use tokio::net::TcpStream;

pub trait Handler {
    fn handle(&self, stream: TcpStream);
    fn close(&self);
}