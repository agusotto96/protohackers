use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::Shutdown::Both;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread::spawn;

const CHAT_ADDRESS: &str = "chat.protohackers.com:16963";

const TONY_BOGUSCOIN_ADRRESS: &[u8; 27] = b"7YWHMfk9JZe0LM0g1ZauHuiSxhI";

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    for stream in listener.incoming().flatten() {
        spawn(|| {
            let _ = handle_connection(stream);
        });
    }
    Ok(())
}

fn handle_connection(stream: TcpStream) -> io::Result<()> {
    let chat_stream = TcpStream::connect(CHAT_ADDRESS)?;
    let chat_reader = chat_stream.try_clone().map(BufReader::new)?;
    let reader = stream.try_clone().map(BufReader::new)?;
    spawn(|| {
        let _ = intercept_msg(reader, chat_stream);
    });
    spawn(|| {
        let _ = intercept_msg(chat_reader, stream);
    });
    Ok(())
}

fn intercept_msg(mut reader: BufReader<TcpStream>, mut stream: TcpStream) -> io::Result<()> {
    loop {
        let Ok(Some(msg)) = read_msg(&mut reader) else { break };
        let msg = tamper_msg(&msg);
        let Ok(()) = write_msg(&mut stream, &msg) else { break };
    }
    stream.shutdown(Both)?;
    Ok(())
}

fn read_msg(reader: &mut BufReader<TcpStream>) -> io::Result<Option<Vec<u8>>> {
    let mut msg = Vec::new();
    let n = reader.read_until(b'\n', &mut msg)?;
    if n == 0 {
        return Ok(None);
    }
    Ok(Some(msg))
}

fn write_msg(stream: &mut TcpStream, msg: &[u8]) -> io::Result<()> {
    stream.write_all(msg)
}

fn tamper_msg(msg: &[u8]) -> Vec<u8> {
    let msg = &msg[..msg.len() - 1];
    let mut msg = msg
        .split(|b| *b == b' ')
        .map(|w| {
            if is_boguscoin_address(w) {
                TONY_BOGUSCOIN_ADRRESS
            } else {
                w
            }
        })
        .collect::<Vec<&[u8]>>()
        .join(&b' ');
    msg.push(b'\n');
    msg
}

fn is_boguscoin_address(word: &[u8]) -> bool {
    word.starts_with(b"7")
        && (word.len() >= 26 && word.len() <= 35)
        && word.iter().all(u8::is_ascii_alphanumeric)
}
