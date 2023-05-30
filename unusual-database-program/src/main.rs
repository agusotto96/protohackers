use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, UdpSocket};

fn main() -> io::Result<()> {
    let mut store: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
    let mut socket = UdpSocket::bind("0.0.0.0:8080")?;
    loop {
        let Ok((bytes, address)) = read_bytes(&mut socket) else { continue };
        match deserialize_request(&bytes) {
            Request::Insert { key, value } => {
                store.insert(key, value);
            }
            Request::Retrieve { key } => {
                if let Some(value) = store.get(&key) {
                    let bytes = serialize_key_value(&key, value);
                    let _ = write_bytes(&mut socket, &bytes, address);
                }
            }
            Request::Version => {
                let bytes = serialize_key_value(VERSION_KEY, VERSION_VALUE);
                let _ = write_bytes(&mut socket, &bytes, address);
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
    if bytes == VERSION_KEY {
        return Request::Version;
    }
    match bytes.iter().position(|b| *b == KEY_VALUE_DELIMITER) {
        Some(index) => Request::Insert {
            key: bytes[..index].to_vec(),
            value: bytes[index + 1..].to_vec(),
        },
        None => Request::Retrieve {
            key: bytes.to_owned(),
        },
    }
}

fn serialize_key_value(key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut bytes = key.to_vec();
    bytes.push(KEY_VALUE_DELIMITER);
    bytes.extend(value);
    bytes
}

enum Request {
    Insert { key: Vec<u8>, value: Vec<u8> },
    Retrieve { key: Vec<u8> },
    Version,
}

const VERSION_KEY: &[u8; 7] = b"version";

const VERSION_VALUE: &[u8; 3] = b"1.0";

const KEY_VALUE_DELIMITER: u8 = b'=';
