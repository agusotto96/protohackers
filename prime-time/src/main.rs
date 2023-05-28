use num_bigint::BigUint;
use num_prime::nt_funcs;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Number;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::str::FromStr;
use std::thread::spawn;

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    for stream in listener.incoming().flatten() {
        let reader = BufReader::new(stream.try_clone()?);
        spawn(|| {
            let _ = handle_connection(stream, reader);
        });
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream, mut reader: BufReader<TcpStream>) -> io::Result<()> {
    loop {
        if let Some(request) = read_request(&mut reader)? {
            let response = Response {
                method: Method::IsPrime,
                prime: is_prime(&request.number),
            };
            write_is_prime_response(&mut stream, &response)?;
        } else {
            write_bad_request(&mut stream)?;
            return Ok(());
        }
    }
}

fn read_request(reader: &mut BufReader<TcpStream>) -> io::Result<Option<Request>> {
    let mut buffer = Vec::new();
    reader.read_until(EOR, &mut buffer)?;
    let request = serde_json::from_slice(&buffer).ok();
    Ok(request)
}

fn is_prime(number: &Number) -> bool {
    let number = number.to_string();
    let (integer, fractional) = number.split_once(|c| c == '.').unwrap_or((&number, ""));
    if integer.starts_with('-') {
        return false;
    }
    if fractional.contains(|d| d != '0') {
        return false;
    }
    let integer = BigUint::from_str(integer).unwrap();
    nt_funcs::is_prime(&integer, None).probably()
}

fn write_is_prime_response(stream: &mut TcpStream, response: &Response) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(response).unwrap();
    bytes.push(EOR);
    stream.write_all(&bytes)
}

fn write_bad_request(stream: &mut TcpStream) -> io::Result<()> {
    let mut bytes = b"nope".to_vec();
    bytes.push(EOR);
    stream.write_all(&bytes)
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

const EOR: u8 = b'\n';
