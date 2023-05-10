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
    let mut inserts: Vec<Insert> = Vec::new();
    loop {
        let mut input = [0; 9];
        let n = reader.read_exact(&mut input).await?;
        if n == 0 {
            return Ok(());
        }
        let output = deserialize_message(&input)
            .and_then(|m| process_message(&mut inserts, m))
            .map(serialize_response);
        if let Some(output) = output {
            w_socket.write_all(&output).await?;
        }
    }
}

fn deserialize_message(bytes: &[u8; 9]) -> Option<Message> {
    match bytes[0] {
        b'I' => {
            let timestamp = i32::from_be_bytes(bytes[1..5].try_into().unwrap());
            let price = i32::from_be_bytes(bytes[5..9].try_into().unwrap());
            let insert = Insert { timestamp, price };
            Some(Message::Insert(insert))
        }
        b'Q' => {
            let min_time = i32::from_be_bytes(bytes[1..5].try_into().unwrap());
            let max_time = i32::from_be_bytes(bytes[5..9].try_into().unwrap());
            let query = Query { min_time, max_time };
            Some(Message::Query(query))
        }
        _ => None,
    }
}

fn process_message(inserts: &mut Vec<Insert>, message: Message) -> Option<i32> {
    match message {
        Message::Insert(insert) => {
            process_insert(inserts, insert);
            None
        }
        Message::Query(query) => {
            let response = process_query(inserts, query);
            Some(response)
        }
    }
}

fn process_insert(inserts: &mut Vec<Insert>, insert: Insert) {
    inserts.push(insert);
}

fn process_query(inserts: &mut Vec<Insert>, query: Query) -> i32 {
    let mut sum: i128 = 0;
    let mut count: i128 = 0;
    for insert in inserts {
        if query.min_time <= insert.timestamp && insert.timestamp <= query.max_time {
            sum += i128::from(insert.price);
            count += 1;
        }
    }
    let mean = if count == 0 || query.min_time > query.max_time {
        0
    } else {
        sum / count
    };
    i32::try_from(mean).unwrap()
}

fn serialize_response(response: i32) -> [u8; 4] {
    response.to_be_bytes()
}

#[derive(Clone, Copy)]
enum Message {
    Insert(Insert),
    Query(Query),
}

#[derive(Clone, Copy)]
struct Insert {
    timestamp: i32,
    price: i32,
}

#[derive(Clone, Copy)]
struct Query {
    min_time: i32,
    max_time: i32,
}
