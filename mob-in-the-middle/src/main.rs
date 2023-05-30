use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread::spawn;

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    for stream in listener.incoming().flatten() {
        spawn(|| {
            let Ok(reader) = stream.try_clone().map(BufReader::new) else { return };
            let Ok(chat_stream) = TcpStream::connect("chat.protohackers.com:16963") else { return };
            let Ok(chat_reader) = chat_stream.try_clone().map(BufReader::new) else { return };
            spawn(|| {
                let _ = handle_connection(reader, chat_stream);
            });
            spawn(|| {
                let _ = handle_connection(chat_reader, stream);
            });
        });
    }
    Ok(())
}

fn handle_connection(mut reader: BufReader<TcpStream>, mut stream: TcpStream) -> io::Result<()> {
    loop {
        let Some(request) = read_request(&mut reader)? else { return Ok(()) };
        let request = tamper(&request);
        stream.write_all(&request)?;
    }
}

fn read_request(reader: &mut BufReader<TcpStream>) -> io::Result<Option<Vec<u8>>> {
    let mut buffer = Vec::new();
    let n = reader.read_until(EOR, &mut buffer)?;
    if n == 0 {
        return Ok(None);
    }
    Ok(Some(buffer))
}

fn tamper(request: &[u8]) -> Vec<u8> {
    let mut request = request.to_vec();
    request.pop();
    let mut request = request
        .split(|b| *b == b' ')
        .map(|w| {
            if is_boguscoin_address(w) {
                TONY_ADRRESS.to_vec()
            } else {
                w.to_owned()
            }
        })
        .collect::<Vec<Vec<u8>>>()
        .join(&b' ');
    request.push(b'\n');
    request
}

fn is_boguscoin_address(bytes: &[u8]) -> bool {
    bytes.starts_with(b"7")
        && (bytes.len() >= 26 && bytes.len() <= 35)
        && bytes.iter().all(u8::is_ascii_alphanumeric)
}

const TONY_ADRRESS: &[u8; 27] = b"7YWHMfk9JZe0LM0g1ZauHuiSxhI";

const EOR: u8 = b'\n';
