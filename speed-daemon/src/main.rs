use std::io;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread::sleep;
use std::thread::spawn;
use std::time::Duration;

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    for stream in listener.incoming().flatten() {
        spawn(|| {
            handle_connection(stream);
        });
    }
    Ok(())
}

fn handle_connection(stream: TcpStream) -> Result<(), HandleConnErr> {
    let write_stream = stream.try_clone()?;
    let mut bytes = BufReader::new(stream).bytes();
    let flag = take_1(&mut bytes)?;
    match flag {
        I_AM_CAMERA_FLAG => {
            let i_am_camera = deserialize_i_am_camera(&mut bytes)?;
            let mut has_heartbeat = false;
            loop {
                let flag = take_1(&mut bytes)?;
                match (flag, has_heartbeat) {
                    (PLATE_FLAG, _) => {
                        let plate = deserialize_plate(&mut bytes)?;
                    }
                    (WANT_HEARTBEAT_FLAG, false) => {
                        let want_heartbeat = deserialize_want_heartbeat(&mut bytes)?;
                        let write_stream = write_stream.try_clone()?;
                        spawn(|| handle_want_heartbeat(write_stream, want_heartbeat));
                        has_heartbeat = true;
                    }
                    _ => Err(HandleConnErr::WrongFlag)?,
                }
            }
        }
        I_AM_DISPATCHER_FLAG => {
            let i_am_dispatcher = deserialize_i_am_dispatcher(&mut bytes)?;
            let flag = take_1(&mut bytes)?;
            match flag {
                WANT_HEARTBEAT_FLAG => {
                    let want_heartbeat = deserialize_want_heartbeat(&mut bytes)?;
                    let write_stream = write_stream.try_clone()?;
                    spawn(|| handle_want_heartbeat(write_stream, want_heartbeat));
                }
                _ => Err(HandleConnErr::WrongFlag)?,
            }
        }
        _ => Err(HandleConnErr::WrongFlag)?,
    }
    Ok(())
}

fn handle_want_heartbeat(mut stream: TcpStream, want_heartbeat: WantHeartbeat) -> io::Result<()> {
    let heartbeat = Heartbeat {};
    let bytes = serialize_heartbeat(&heartbeat);
    let interval: u64 = want_heartbeat.interval.into();
    loop {
        stream.write_all(&bytes)?;
        sleep(Duration::from_millis(interval * 100));
    }
}

enum HandleConnErr {
    IO(io::Error),
    ByteStreamErr(ByteStreamErr),
    DeserializePlateErr(DeserializePlateErr),
    WrongFlag,
}

impl From<io::Error> for HandleConnErr {
    fn from(value: io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<DeserializePlateErr> for HandleConnErr {
    fn from(value: DeserializePlateErr) -> Self {
        Self::DeserializePlateErr(value)
    }
}

impl From<ByteStreamErr> for HandleConnErr {
    fn from(value: ByteStreamErr) -> Self {
        Self::ByteStreamErr(value)
    }
}

const ERROR_FLAG: u8 = 0x10;

const PLATE_FLAG: u8 = 0x20;

const TICKET_FLAG: u8 = 0x21;

const WANT_HEARTBEAT_FLAG: u8 = 0x40;

const HEARTBEAT_FLAG: u8 = 0x41;

const I_AM_CAMERA_FLAG: u8 = 0x80;

const I_AM_DISPATCHER_FLAG: u8 = 0x81;

#[derive(Debug)]
struct ErrorMsg {
    msg: String,
}

#[derive(Debug)]
struct Plate {
    plate: String,
    timestamp: u32,
}

#[derive(Debug)]
struct Ticket {
    plate: String,
    road: u16,
    mile1: u16,
    timestamp1: u32,
    mile2: u16,
    timestamp2: u32,
    speed: u16,
}

#[derive(Debug)]
struct WantHeartbeat {
    interval: u32,
}

#[derive(Debug)]
struct Heartbeat {}

#[derive(Debug)]
struct IAmCamera {
    road: u16,
    mile: u16,
    limit: u16,
}

#[derive(Debug)]
struct IAmDispatcher {
    roads: Vec<u16>,
}

type ByteStream = dyn Iterator<Item = io::Result<u8>>;

#[derive(Debug)]
enum ByteStreamErr {
    IO(io::Error),
    Missing,
}

fn serialize_error_msg(error_msg: &ErrorMsg) -> Result<Vec<u8>, SerializeStrErr> {
    let mut bytes = Vec::new();
    bytes.push(ERROR_FLAG);
    bytes.extend(serialize_str(&error_msg.msg)?);
    Ok(bytes)
}

fn deserialize_plate(bytes: &mut ByteStream) -> Result<Plate, DeserializePlateErr> {
    let plate = Plate {
        plate: deserialize_str(bytes)?,
        timestamp: deserialize_u32(bytes)?,
    };
    Ok(plate)
}

#[derive(Debug)]
enum DeserializePlateErr {
    ByteStreamErr(ByteStreamErr),
    DeserializeStrErr(DeserializeStrErr),
}

impl From<ByteStreamErr> for DeserializePlateErr {
    fn from(value: ByteStreamErr) -> Self {
        Self::ByteStreamErr(value)
    }
}

impl From<DeserializeStrErr> for DeserializePlateErr {
    fn from(value: DeserializeStrErr) -> Self {
        Self::DeserializeStrErr(value)
    }
}

fn serialize_ticket(ticket: &Ticket) -> Result<Vec<u8>, SerializeStrErr> {
    let mut bytes = Vec::new();
    bytes.push(TICKET_FLAG);
    bytes.extend(serialize_str(&ticket.plate)?);
    bytes.extend(serialize_u16(ticket.road));
    bytes.extend(serialize_u16(ticket.mile1));
    bytes.extend(serialize_u32(ticket.timestamp1));
    bytes.extend(serialize_u16(ticket.mile2));
    bytes.extend(serialize_u32(ticket.timestamp2));
    bytes.extend(serialize_u16(ticket.speed));
    Ok(bytes)
}

fn deserialize_want_heartbeat(bytes: &mut ByteStream) -> Result<WantHeartbeat, ByteStreamErr> {
    let want_heartbeat = WantHeartbeat {
        interval: deserialize_u32(bytes)?,
    };
    Ok(want_heartbeat)
}

fn serialize_heartbeat(heartbeat: &Heartbeat) -> Vec<u8> {
    vec![HEARTBEAT_FLAG]
}

fn deserialize_i_am_camera(bytes: &mut ByteStream) -> Result<IAmCamera, ByteStreamErr> {
    let i_am_camera = IAmCamera {
        road: deserialize_u16(bytes)?,
        mile: deserialize_u16(bytes)?,
        limit: deserialize_u16(bytes)?,
    };
    Ok(i_am_camera)
}

fn deserialize_i_am_dispatcher(bytes: &mut ByteStream) -> Result<IAmDispatcher, ByteStreamErr> {
    let i_am_dispatcher = IAmDispatcher {
        roads: deserialize_vec(bytes)?,
    };
    Ok(i_am_dispatcher)
}

fn take_1(bytes: &mut ByteStream) -> Result<u8, ByteStreamErr> {
    match bytes.next() {
        Some(Ok(byte)) => Ok(byte),
        Some(Err(err)) => Err(ByteStreamErr::IO(err)),
        None => Err(ByteStreamErr::Missing),
    }
}

fn take_n(bytes: &mut ByteStream, n: usize) -> Result<Vec<u8>, ByteStreamErr> {
    let mut buf = Vec::new();
    for _ in 0..n {
        let byte = take_1(bytes)?;
        buf.push(byte);
    }
    Ok(buf)
}

fn serialize_u16(u16: u16) -> Vec<u8> {
    u16.to_be_bytes().to_vec()
}

fn deserialize_u16(bytes: &mut ByteStream) -> Result<u16, ByteStreamErr> {
    let bytes = [take_1(bytes)?, take_1(bytes)?];
    let u16 = u16::from_be_bytes(bytes);
    Ok(u16)
}

fn serialize_u32(u32: u32) -> Vec<u8> {
    u32.to_be_bytes().to_vec()
}

fn deserialize_u32(bytes: &mut ByteStream) -> Result<u32, ByteStreamErr> {
    let bytes = [
        take_1(bytes)?,
        take_1(bytes)?,
        take_1(bytes)?,
        take_1(bytes)?,
    ];
    let u32 = u32::from_be_bytes(bytes);
    Ok(u32)
}

fn serialize_str(str: &str) -> Result<Vec<u8>, SerializeStrErr> {
    let Ok(len) = str.len().try_into() else {
         Err(SerializeStrErr::BadLength)?
    };
    if !str.is_ascii() {
        Err(SerializeStrErr::NotAscii)?;
    }
    let mut bytes = Vec::new();
    bytes.push(len);
    bytes.extend_from_slice(str.as_bytes());
    Ok(bytes)
}

#[derive(Debug)]
enum SerializeStrErr {
    BadLength,
    NotAscii,
}

fn deserialize_str(bytes: &mut ByteStream) -> Result<String, DeserializeStrErr> {
    let len = take_1(bytes)?;
    let Ok(len) = len.try_into() else {
         Err(DeserializeStrErr::BadLength)?
    };
    let bytes = take_n(bytes, len)?;
    if !bytes.is_ascii() {
        Err(DeserializeStrErr::NotAscii)?;
    }
    let str = String::from_utf8_lossy(&bytes).into_owned();
    Ok(str)
}

#[derive(Debug)]
enum DeserializeStrErr {
    BadLength,
    NotAscii,
    ByteStreamErr(ByteStreamErr),
}

impl From<ByteStreamErr> for DeserializeStrErr {
    fn from(value: ByteStreamErr) -> Self {
        Self::ByteStreamErr(value)
    }
}

fn deserialize_vec(bytes: &mut ByteStream) -> Result<Vec<u16>, ByteStreamErr> {
    let len = take_1(bytes)?;
    (0..len)
        .map(|_| deserialize_u16(bytes))
        .collect::<Result<Vec<u16>, ByteStreamErr>>()
}

#[cfg(test)]
mod tests {
    use crate::deserialize_i_am_camera;
    use crate::deserialize_i_am_dispatcher;
    use crate::deserialize_plate;
    use crate::deserialize_want_heartbeat;
    use crate::serialize_error_msg;
    use crate::serialize_heartbeat;
    use crate::serialize_ticket;
    use crate::ErrorMsg;
    use crate::Heartbeat;
    use crate::Ticket;
    #[test]
    fn serialize_error_msg_test() {
        let error_msg = ErrorMsg {
            msg: "bad".to_owned(),
        };
        let bytes = b"\x10\x03\x62\x61\x64";
        assert_eq!(serialize_error_msg(&error_msg).unwrap(), bytes.to_vec());
    }
    #[test]
    fn deserialize_plate_test() {
        let mut bytes = b"\x07\x52\x45\x30\x35\x42\x4b\x47\x00\x01\xe2\x40"
            .to_vec()
            .into_iter()
            .map(Result::Ok);
        let plate = deserialize_plate(&mut bytes).unwrap();
        assert_eq!(plate.plate, "RE05BKG".to_owned());
        assert_eq!(plate.timestamp, 123456);
    }
    #[test]
    fn serialize_ticket_test() {
        let ticket = Ticket {
            plate: "RE05BKG".to_owned(),
            road: 368,
            mile1: 1234,
            timestamp1: 1000000,
            mile2: 1235,
            timestamp2: 1000060,
            speed: 6000,
        };
        let bytes = b"\x21\x07\x52\x45\x30\x35\x42\x4b\x47\x01\x70\x04\xd2\x00\x0f\x42\x40\x04\xd3\x00\x0f\x42\x7c\x17\x70";
        assert_eq!(serialize_ticket(&ticket).unwrap(), bytes.to_vec());
    }
    #[test]
    fn deserialize_want_heartbeat_test() {
        let mut bytes = b"\x00\x00\x04\xdb".to_vec().into_iter().map(Result::Ok);
        let want_heartbeat = deserialize_want_heartbeat(&mut bytes).unwrap();
        assert_eq!(want_heartbeat.interval, 1243);
    }
    #[test]
    fn serialize_heartbeat_test() {
        let heartbeat = Heartbeat {};
        let bytes = b"\x41";
        assert_eq!(serialize_heartbeat(&heartbeat), bytes.to_vec());
    }
    #[test]
    fn deserialize_i_am_camera_test() {
        let mut bytes = b"\x01\x70\x04\xd2\x00\x28"
            .to_vec()
            .into_iter()
            .map(Result::Ok);
        let i_am_camera = deserialize_i_am_camera(&mut bytes).unwrap();
        assert_eq!(i_am_camera.road, 368);
        assert_eq!(i_am_camera.mile, 1234);
        assert_eq!(i_am_camera.limit, 40);
    }
    #[test]
    fn deserialize_i_am_dispatcher_test() {
        let mut bytes = b"\x03\x00\x42\x01\x70\x13\x88"
            .to_vec()
            .into_iter()
            .map(Result::Ok);
        let i_am_dispatcher = deserialize_i_am_dispatcher(&mut bytes).unwrap();
        assert_eq!(i_am_dispatcher.roads, vec![66, 368, 5000]);
    }
}
