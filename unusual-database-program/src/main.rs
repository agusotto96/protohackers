use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, UdpSocket};

fn main() -> io::Result<()> {
    let mut store: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
    let version_key = b"version";
    let version_value = b"Otto 1.0";
    store.insert(version_key.to_vec(), version_value.to_vec());
    let mut socket = UdpSocket::bind("0.0.0.0:8080")?;
    loop {
        let Ok((bytes, address)) = read_bytes(&mut socket) else { continue };
        match deserialize_request(&bytes) {
            Request::Insert { key, value } => {
                if key != version_key {
                    store.insert(key, value);
                }
            }
            Request::Retrieve { key } => {
                if let Some(value) = store.get(&key) {
                    let mut bytes = key;
                    bytes.push(b'=');
                    bytes.extend(value);
                    let _ = write_bytes(&mut socket, &bytes, address);
                }
            }
        }
    }
}

fn read_bytes(socket: &mut UdpSocket) -> io::Result<(Vec<u8>, SocketAddr)> {
    let mut buffer = [0; 999];
    let (n, address) = socket.recv_from(&mut buffer)?;
    let bytes = buffer[..n].to_vec();
    Ok((bytes, address))
}

fn write_bytes(socket: &mut UdpSocket, bytes: &[u8], address: SocketAddr) -> io::Result<usize> {
    socket.send_to(bytes, address)
}

fn deserialize_request(bytes: &[u8]) -> Request {
    match bytes.iter().position(|b| *b == b'=') {
        Some(index) => Request::Insert {
            key: bytes[..index].to_vec(),
            value: bytes[index + 1..].to_vec(),
        },
        None => Request::Retrieve {
            key: bytes.to_owned(),
        },
    }
}

enum Request {
    Insert { key: Vec<u8>, value: Vec<u8> },
    Retrieve { key: Vec<u8> },
}
