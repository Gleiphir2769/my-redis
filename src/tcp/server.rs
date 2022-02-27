use log::debug;
use tokio::net::{TcpListener, TcpStream};
use crate::tcp::handle::Handler;
use std::fmt;


pub async fn listen_and_serve(handler: &impl Handler) {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        debug!("accept connect");
        handler.handle(socket);
    }
}

