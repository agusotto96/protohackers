use num_primes::BigUint;
use num_primes::Verification;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Number;
use std::env;
use std::str::FromStr;
use tokio::io;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

const EOR: u8 = b'\n';

const BAD_OUTPUT: &[u8] = &[b'n', b'o', b'p', b'e', EOR];

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
        let n = reader.read_until(EOR, &mut input).await?;
        if n == 0 {
            return Ok(());
        }
        let output = deserialize_request(&input)
            .map(|r| process_request(&r))
            .map_or(BAD_OUTPUT.into(), |r| serialize_response(&r));
        println!(
            "req: {:?}, res: {:?}",
            String::from_utf8_lossy(&input),
            String::from_utf8_lossy(&output)
        );
        w_socket.write_all(&output).await?;
        if output == BAD_OUTPUT {
            return Ok(());
        }
    }
}

fn deserialize_request(bytes: &[u8]) -> Option<Request> {
    serde_json::from_slice(bytes).ok()
}

fn process_request(request: &Request) -> Response {
    Response {
        method: Method::IsPrime,
        prime: is_prime(&request.number),
    }
}

fn serialize_response(response: &Response) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&response).unwrap();
    bytes.push(EOR);
    bytes
}

fn is_prime(number: &Number) -> bool {
    BigUint::from_str(&number.to_string()).map_or(false, |n| Verification::is_prime(&n))
}

#[derive(Serialize, Deserialize)]
struct Request {
    method: Method,
    number: Number,
}

#[derive(Serialize, Deserialize)]
struct Response {
    method: Method,
    prime: bool,
}

#[derive(Serialize, Deserialize)]
enum Method {
    #[serde(rename = "isPrime")]
    IsPrime,
}
