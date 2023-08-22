use std::{
    io::{self, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread::{sleep, spawn},
    time::Duration,
};

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    for stream in listener.incoming().flatten() {
        spawn(|| handle_connection(stream));
    }
    Ok(())
}

fn handle_connection(stream: TcpStream) -> Option<()> {
    let mut write_stream = stream.try_clone().ok()?;
    let mut bytes = BufReader::new(stream).bytes().flatten();
    let mut client = Client::default();
    loop {
        let Some(request) = deserialize_request(&mut bytes) else {
            send_error_msg(&mut write_stream, "")?;
            None?
        };
        match request {
            Request::IAmCamera(i_am_camera) => {
                if client.i_am_camera.is_none() && client.i_am_dispatcher.is_none() {
                    client.i_am_camera = Some(i_am_camera);
                } else {
                    send_error_msg(&mut write_stream, "")?;
                    None?;
                }
            }
            Request::IAmDispatcher(i_am_dispatcher) => {
                if client.i_am_camera.is_none() && client.i_am_dispatcher.is_none() {
                    client.i_am_dispatcher = Some(i_am_dispatcher);
                    todo!()
                } else {
                    send_error_msg(&mut write_stream, "")?;
                    None?;
                }
            }
            Request::WantHeartbeat(want_heartbeat) => {
                if client.want_heartbeat.is_none() {
                    client.want_heartbeat = Some(want_heartbeat);
                    let write_stream = write_stream.try_clone().ok()?;
                    spawn(move || handle_want_heartbeat(write_stream, want_heartbeat));
                } else {
                    send_error_msg(&mut write_stream, "")?;
                    None?;
                }
            }
            Request::Plate(plate) => {
                let Some(i_am_camera) = client.i_am_camera else {
                    send_error_msg(&mut write_stream, "")?;
                    None?
                };
                todo!()
            }
        }
    }
}

fn handle_want_heartbeat(mut stream: TcpStream, want_heartbeat: WantHeartbeat) -> io::Result<()> {
    let interval: u64 = want_heartbeat.interval.into();
    loop {
        stream.write_all(&[HEARTBEAT_FLAG])?;
        sleep(Duration::from_millis(interval * 100));
    }
}

fn send_error_msg(stream: &mut TcpStream, msg: &str) -> Option<()> {
    let bytes = serialize_error_msg(msg)?;
    stream.write_all(&bytes).ok()
}

#[derive(Default, Debug, Clone)]
struct Client {
    i_am_camera: Option<IAmCamera>,
    i_am_dispatcher: Option<IAmDispatcher>,
    want_heartbeat: Option<WantHeartbeat>,
}

#[derive(Debug, Clone)]
struct Plate {
    plate: String,
    timestamp: u32,
}

#[derive(Debug, Clone)]
struct Ticket {
    plate: String,
    road: u16,
    mile1: u16,
    timestamp1: u32,
    mile2: u16,
    timestamp2: u32,
    speed: u16,
}

#[derive(Debug, Clone, Copy)]
struct WantHeartbeat {
    interval: u32,
}

#[derive(Debug, Clone, Copy)]
struct IAmCamera {
    road: u16,
    mile: u16,
    limit: u16,
}

#[derive(Debug, Clone)]
struct IAmDispatcher {
    roads: Vec<u16>,
}

enum Request {
    IAmCamera(IAmCamera),
    IAmDispatcher(IAmDispatcher),
    WantHeartbeat(WantHeartbeat),
    Plate(Plate),
}

const ERROR_FLAG: u8 = 0x10;

const PLATE_FLAG: u8 = 0x20;

const TICKET_FLAG: u8 = 0x21;

const WANT_HEARTBEAT_FLAG: u8 = 0x40;

const HEARTBEAT_FLAG: u8 = 0x41;

const I_AM_CAMERA_FLAG: u8 = 0x80;

const I_AM_DISPATCHER_FLAG: u8 = 0x81;

fn deserialize_request(bytes: &mut impl Iterator<Item = u8>) -> Option<Request> {
    let flag = bytes.next()?;
    match flag {
        I_AM_CAMERA_FLAG => deserialize_i_am_camera(bytes).map(Request::IAmCamera),
        I_AM_DISPATCHER_FLAG => deserialize_i_am_dispatcher(bytes).map(Request::IAmDispatcher),
        WANT_HEARTBEAT_FLAG => deserialize_want_heartbeat(bytes).map(Request::WantHeartbeat),
        PLATE_FLAG => deserialize_plate(bytes).map(Request::Plate),
        _ => None,
    }
}

fn serialize_error_msg(msg: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes.push(ERROR_FLAG);
    bytes.extend(serialize_str(msg)?);
    Some(bytes)
}

fn deserialize_plate(bytes: &mut impl Iterator<Item = u8>) -> Option<Plate> {
    let plate = Plate {
        plate: deserialize_str(bytes)?,
        timestamp: deserialize_u32(bytes)?,
    };
    Some(plate)
}

fn serialize_ticket(ticket: &Ticket) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes.push(TICKET_FLAG);
    bytes.extend(serialize_str(&ticket.plate)?);
    bytes.extend(serialize_u16(ticket.road));
    bytes.extend(serialize_u16(ticket.mile1));
    bytes.extend(serialize_u32(ticket.timestamp1));
    bytes.extend(serialize_u16(ticket.mile2));
    bytes.extend(serialize_u32(ticket.timestamp2));
    bytes.extend(serialize_u16(ticket.speed));
    Some(bytes)
}

fn deserialize_want_heartbeat(bytes: &mut impl Iterator<Item = u8>) -> Option<WantHeartbeat> {
    let want_heartbeat = WantHeartbeat {
        interval: deserialize_u32(bytes)?,
    };
    Some(want_heartbeat)
}

fn deserialize_i_am_camera(bytes: &mut impl Iterator<Item = u8>) -> Option<IAmCamera> {
    let i_am_camera = IAmCamera {
        road: deserialize_u16(bytes)?,
        mile: deserialize_u16(bytes)?,
        limit: deserialize_u16(bytes)?,
    };
    Some(i_am_camera)
}

fn deserialize_i_am_dispatcher(bytes: &mut impl Iterator<Item = u8>) -> Option<IAmDispatcher> {
    let i_am_dispatcher = IAmDispatcher {
        roads: deserialize_vec(bytes)?,
    };
    Some(i_am_dispatcher)
}

fn serialize_u16(u16: u16) -> Vec<u8> {
    u16.to_be_bytes().to_vec()
}

fn deserialize_u16(bytes: &mut impl Iterator<Item = u8>) -> Option<u16> {
    let bytes = [bytes.next()?, bytes.next()?];
    let u16 = u16::from_be_bytes(bytes);
    Some(u16)
}

fn serialize_u32(u32: u32) -> Vec<u8> {
    u32.to_be_bytes().to_vec()
}

fn deserialize_u32(bytes: &mut impl Iterator<Item = u8>) -> Option<u32> {
    let bytes = [bytes.next()?, bytes.next()?, bytes.next()?, bytes.next()?];
    let u32 = u32::from_be_bytes(bytes);
    Some(u32)
}

fn serialize_str(str: &str) -> Option<Vec<u8>> {
    let len = str.len().try_into().ok()?;
    if !str.is_ascii() {
        None?;
    }
    let mut bytes = Vec::new();
    bytes.push(len);
    bytes.extend_from_slice(str.as_bytes());
    Some(bytes)
}

fn deserialize_str(bytes: &mut impl Iterator<Item = u8>) -> Option<String> {
    let len = bytes.next()?.into();
    let bytes = bytes.take(len).collect::<Vec<u8>>();
    if bytes.len() != len {
        None?;
    }
    if !bytes.is_ascii() {
        None?;
    }
    String::from_utf8(bytes).ok()
}

fn deserialize_vec(bytes: &mut impl Iterator<Item = u8>) -> Option<Vec<u16>> {
    let len = bytes.next()?;
    (0..len)
        .map(|_| deserialize_u16(bytes))
        .collect::<Option<Vec<u16>>>()
}

#[cfg(test)]
mod tests {
    use crate::deserialize_i_am_camera;
    use crate::deserialize_i_am_dispatcher;
    use crate::deserialize_plate;
    use crate::deserialize_want_heartbeat;
    use crate::serialize_error_msg;
    use crate::serialize_ticket;
    use crate::Ticket;
    #[test]
    fn serialize_error_msg_test() {
        let error_msg = "bad";
        let bytes = b"\x10\x03\x62\x61\x64";
        assert_eq!(serialize_error_msg(error_msg).unwrap(), bytes.to_vec());
    }
    #[test]
    fn deserialize_plate_test() {
        let mut bytes = b"\x07\x52\x45\x30\x35\x42\x4b\x47\x00\x01\xe2\x40"
            .to_vec()
            .into_iter();
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
        let mut bytes = b"\x00\x00\x04\xdb".to_vec().into_iter();
        let want_heartbeat = deserialize_want_heartbeat(&mut bytes).unwrap();
        assert_eq!(want_heartbeat.interval, 1243);
    }
    #[test]
    fn deserialize_i_am_camera_test() {
        let mut bytes = b"\x01\x70\x04\xd2\x00\x28".to_vec().into_iter();
        let i_am_camera = deserialize_i_am_camera(&mut bytes).unwrap();
        assert_eq!(i_am_camera.road, 368);
        assert_eq!(i_am_camera.mile, 1234);
        assert_eq!(i_am_camera.limit, 40);
    }
    #[test]
    fn deserialize_i_am_dispatcher_test() {
        let mut bytes = b"\x03\x00\x42\x01\x70\x13\x88".to_vec().into_iter();
        let i_am_dispatcher = deserialize_i_am_dispatcher(&mut bytes).unwrap();
        assert_eq!(i_am_dispatcher.roads, vec![66, 368, 5000]);
    }
}
