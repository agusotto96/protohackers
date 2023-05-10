use std::env;
use std::net::SocketAddr;
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
        let (socket, client) = listener.accept().await?;
        tokio::spawn(async move {
            let _ = process_socket(socket, client).await;
        });
    }
}

async fn process_socket(mut socket: TcpStream, client: SocketAddr) -> io::Result<()> {
    let (r_socket, mut w_socket) = socket.split();
    let mut reader = BufReader::new(r_socket);
    let mut inserts: Vec<(i32, i32)> = Vec::new();
    loop {
        let mut input = [0; 9];
        let n = reader.read_exact(&mut input).await?;
        if n == 0 {
            return Ok(());
        }
        if let Some(message) = deserialize_message(&input) {
            match message {
                Message::Insert { timestamp, price } => {
                    println!("(c:{:?}, im: {:?})", &client, &message);
                    inserts.push((timestamp, price));
                }
                Message::Query { min_time, max_time } => {
                    let inserts: Vec<(i32, i32)> = inserts
                        .iter()
                        .filter(|(t, _)| min_time <= *t && *t <= max_time)
                        .copied()
                        .collect();
                    let sum: i128 = inserts.iter().map(|(_, p)| i128::from(*p)).sum();
                    let mean: i32 = if inserts.is_empty() || min_time > max_time {
                        0
                    } else {
                        i32::try_from(sum / i128::try_from(inserts.len()).unwrap()).unwrap()
                    };
                    let output = mean.to_be_bytes();
                    println!(
                        "(c:{:?}, i: {:?}, qm: {:?}, m: {:?})",
                        &client, &inserts, &message, &mean
                    );
                    w_socket.write_all(&output).await?;
                }
            }
        } else {
            return Ok(());
        }
    }
}

fn deserialize_message(bytes: &[u8; 9]) -> Option<Message> {
    match bytes[0] {
        b'I' => {
            let timestamp = i32::from_be_bytes(bytes[1..5].try_into().unwrap());
            let price = i32::from_be_bytes(bytes[5..9].try_into().unwrap());
            let insert = Message::Insert { timestamp, price };
            Some(insert)
        }
        b'Q' => {
            let min_time = i32::from_be_bytes(bytes[1..5].try_into().unwrap());
            let max_time = i32::from_be_bytes(bytes[5..9].try_into().unwrap());
            let query = Message::Query { min_time, max_time };
            Some(query)
        }
        _ => None,
    }
}

#[derive(Debug)]
enum Message {
    Insert { timestamp: i32, price: i32 },
    Query { min_time: i32, max_time: i32 },
}
