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
    loop {
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer)?;
        if n == 0 {
            return Ok(());
        }
        stream.write_all(&buffer[0..n])?;
    }
}
