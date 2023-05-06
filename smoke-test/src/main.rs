use std::env;
use tokio::io;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> io::Result<()> {
    let address = env::args().nth(1).unwrap_or("0.0.0.0:8080".into());
    let listener = TcpListener::bind(address).await?;
    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let _ = process_socket(socket).await;
        });
    }
}

async fn process_socket(mut socket: TcpStream) -> io::Result<()> {
    let (r_socket, mut w_socket) = socket.split();
    let mut reader = BufReader::new(r_socket);
    loop {
        let mut input = Vec::new();
        let n = reader.read_buf(&mut input).await?;
        if n == 0 {
            return Ok(());
        }
        w_socket.write_all(&input).await?;
    }
}
