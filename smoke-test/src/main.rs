use std::env::args;
use std::io;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread::spawn;

fn main() -> io::Result<()> {
    let address = args().nth(1).unwrap_or("0.0.0.0:8080".into());
    let listener = TcpListener::bind(address)?;
    for stream in listener.incoming() {
        spawn(|| {
            let _ = stream.and_then(process_stream);
        });
    }
    Ok(())
}

fn process_stream(mut stream: TcpStream) -> io::Result<()> {
    loop {
        let mut buf = [0; 1028];
        let n = stream.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        stream.write_all(&buf[0..n])?
    }
}
