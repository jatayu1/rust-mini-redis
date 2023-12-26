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
    // The `Connection` let us read/write redis **frames**.
    let mut connection = Connection::new(socket);

    if let Some(frame) = connection.read_frame().await.unwrap() {
        println!("GOT: {:?}", frame);

        //respond with error
        let response = Frame::Error("unimplemented".to_string());
        connection.write_frame(&response).await.unwrap();
    }
}
