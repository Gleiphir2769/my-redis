
use tokio::net::{TcpListener, TcpStream};
use mini_redis::{Connection, Frame};
use mini_redis::Command::{self, Get, Set};
use tokio::sync::Mutex;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, MutexGuard};
use std::collections::HashMap;
use std::sync::Arc;


fn new_sharded_db(num_shards: usize) -> ShardedDb {
    let mut db = Vec::with_capacity(num_shards);
    for _ in 0..num_shards {
        db.push(Mutex::new(HashMap::new()));
    };
    let sharded_dict = ShardedDict{db: db};

    Arc::new(sharded_dict)
}

type ShardedDb = Arc<ShardedDict>;
struct ShardedDict {
    db: Vec<Mutex<HashMap<String, Vec<u8>>>>
}

impl <'a>ShardedDict {
    fn insert(&self, key: String, val: Vec<u8>) {
        let mut shard = self.db[calculate_hash(&key) % self.db.len()].lock().unwrap();
        shard.insert(key, val);
    }

    fn get(&'a self, key: &str) -> Option<Vec<u8>> {
        let shard = self.db[calculate_hash(&key) % self.db.len()].lock().unwrap();
        if let Some(vec) = shard.get(key) {
            Some(vec.to_owned())
        } else {
            None
        }
    }
}


fn calculate_hash<T: Hash>(t: &T) -> usize {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish() as usize
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    let db = new_sharded_db(5);

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let dbc = db.clone();
        tokio::spawn(
            async move {
                process(socket, dbc).await;
            }
        );
    }
}


async fn process(socket: TcpStream, db: ShardedDb) {

    // 使用 hashmap 来存储 redis 的数据

    // `mini-redis` 提供的便利函数，使用返回的 `connection` 可以用于从 socket 中读取数据并解析为数据帧
    let mut connection = Connection::new(socket);

    // 使用 `read_frame` 方法从连接获取一个数据帧：一条redis命令 + 相应的数据
    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                // 值被存储为 `Vec<u8>` 的形式
                db.insert(cmd.key().to_string(), cmd.value().to_vec());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Bulk(value.into())
                } else {
                    Frame::Null
                }
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };

        // 将请求响应返回给客户端
        connection.write_frame(&response).await.unwrap();
    }
}


