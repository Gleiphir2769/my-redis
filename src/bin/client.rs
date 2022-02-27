use mini_redis::{client, blocking_client::Message};
use bytes::Bytes;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>
    }
}

type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(32);
    let tx2 = tx.clone();

    let t1 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Get {
            key: "hello".to_string(),
            resp: resp_tx,
        };

        let _ = tx.send(cmd).await;

        let res = resp_rx.await;
        println!("GOT = {:?}", res);
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Set {
            key: "hello".to_string(),
            val: "world".into(),
            resp: resp_tx,
        };

        let _ = tx2.send(cmd).await;

        let res = resp_rx.await;
        println!("GOT = {:?}", res);
    });

    let manager = tokio::spawn(async move {
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();
        
        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::Get{key, resp} => {
                    let res = client.get(&key).await;
                    let _ = resp.send(res);
                }
                Command::Set{key,val,resp} => {
                    let res = client.set(&key, val).await;
                    let _ = resp.send(res);
                } 
            }
        }
    }); 

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}