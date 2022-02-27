// use tokio::sync::{mpsc, oneshot};
use tokio::net::{TcpStream, TcpListener};
use tokio::io;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            let (mut rd, mut wr) = socket.split();
            if io::copy(&mut rd, &mut wr).await.is_err() {
                eprintln!("Err occur!");
            }
        });
    }
}