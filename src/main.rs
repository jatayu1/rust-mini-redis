use tokio::net::{TcpListener, TcpStream};
use mini_redis::{Connection, Frame};

#[tokio::main]
async fn main() {
    // Bind the listener to adderss
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        // The second item contains the IP and port of the new connections.
        let (socket, _) = listener.accept().await.unwrap();
        
        tokio::spawn(async move {
            process(socket).await;
        });  
    }
}

async fn process(socket: TcpStream) {
    use mini_redis::Command::{self, Get, Set};
    use std::collections::HashMap;

    // A hashmap is used to store data
    let mut db = HashMap::new();

    // Connection, provided by `mini-redis`, handles parsing frames the socket
    let mut connection = Connection::new(socket);

    // Use `read_frame` to recive a command from the connection.
    while let Some(frame) = connection.read_frame().await.unwrap(){
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                //The value is stored as `Vec<u8>`
                db.insert(cmd.key().to_string(), cmd.value().to_vec());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                if let Some(value) = db.get(cmd.key()){
                    // `Frame::Bulk` expects data to be of type `Bytes`.
                    // `&Vec<u8>` is converted to `Bytes` uising `into()`.
                    Frame::Bulk(value.clone().into())
                } else {
                    Frame::Null
                }
            }
            cmd => {
                panic!("unimplemented {:?}", cmd)
            }
        };

        connection.write_frame(&response).await.unwrap();
    }
}
