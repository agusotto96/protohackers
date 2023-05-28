use std::io;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread::spawn;

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    for stream in listener.incoming().flatten() {
        spawn(|| {
            let _ = handle_connection(stream);
        });
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let mut inserts: Vec<Insert> = Vec::new();
    loop {
        match read_message(&mut stream)? {
            Some(Message::Insert(insert)) => {
                inserts.push(insert);
            }
            Some(Message::Query(query)) => {
                let mean_price = calculate_mean_price(&mut inserts, query);
                write_mean_price(&mut stream, mean_price)?;
            }
            None => {
                return Ok(());
            }
        }
    }
}

fn read_message(stream: &mut TcpStream) -> io::Result<Option<Message>> {
    let mut buffer = [0; 9];
    stream.read_exact(&mut buffer)?;
    let message = parse_message(&buffer);
    Ok(message)
}

fn parse_message(bytes: &[u8; 9]) -> Option<Message> {
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

fn calculate_mean_price(inserts: &mut Vec<Insert>, query: Query) -> i32 {
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

fn write_mean_price(stream: &mut TcpStream, mean_price: i32) -> io::Result<()> {
    let bytes = mean_price.to_be_bytes();
    stream.write_all(&bytes)
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
