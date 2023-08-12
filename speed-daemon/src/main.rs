use std::io;
use std::io::Read;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread::spawn;

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    for stream in listener.incoming().flatten() {
        spawn(|| {
            handle_connection(stream);
        });
    }
    Ok(())
}

fn handle_connection(stream: TcpStream) {
    let mut bytes = stream.bytes().peekable();
    let Some(Ok(flag)) = bytes.peek() else { return };
    if *flag == PLATE_FLAG {
        let plate = Plate::deserialize(&mut bytes);
    }
}

const ERROR_FLAG: u8 = 0x10;

const PLATE_FLAG: u8 = 0x20;

const TICKET_FLAG: u8 = 0x21;

const WANT_HEARTBEAT_FLAG: u8 = 0x40;

const HEARTBEAT_FLAG: u8 = 0x41;

const I_AM_CAMERA_FLAG: u8 = 0x80;

const I_AM_DISPATCHER_FLAG: u8 = 0x81;

struct Error {
    msg: String,
}

struct Plate {
    plate: String,
    timestamp: u32,
}

struct Ticket {
    plate: String,
    road: u16,
    mile1: u16,
    timestamp1: u32,
    mile2: u16,
    timestamp2: u32,
    speed: u16,
}

struct WantHeartbeat {
    interval: u32,
}

struct Heartbeat {}

struct IAmCamera {
    road: u16,
    mile: u16,
    limit: u16,
}

struct IAmDispatcher {
    roads: Vec<u16>,
}

trait Serializable {
    fn serialize(&self) -> Option<Vec<u8>>;
}

trait Deserializable {
    fn deserialize<I>(bytes: &mut I) -> Option<Self>
    where
        I: Iterator<Item = io::Result<u8>>,
        Self: Sized;
}

impl Serializable for Error {
    fn serialize(&self) -> Option<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.push(ERROR_FLAG);
        bytes.extend(self.msg.serialize()?);
        Some(bytes)
    }
}

impl Deserializable for Plate {
    fn deserialize<I>(bytes: &mut I) -> Option<Self>
    where
        I: Iterator<Item = io::Result<u8>>,
        Self: Sized,
    {
        let flag = bytes.next()?.ok()?;
        if flag != PLATE_FLAG {
            return None;
        }
        let plate = Plate {
            plate: String::deserialize(bytes)?,
            timestamp: u32::deserialize(bytes)?,
        };
        Some(plate)
    }
}

impl Serializable for Ticket {
    fn serialize(&self) -> Option<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.push(TICKET_FLAG);
        bytes.extend(self.plate.serialize()?);
        bytes.extend(self.road.serialize()?);
        bytes.extend(self.mile1.serialize()?);
        bytes.extend(self.timestamp1.serialize()?);
        bytes.extend(self.mile2.serialize()?);
        bytes.extend(self.timestamp2.serialize()?);
        bytes.extend(self.speed.serialize()?);
        Some(bytes)
    }
}

impl Deserializable for WantHeartbeat {
    fn deserialize<I>(bytes: &mut I) -> Option<Self>
    where
        I: Iterator<Item = io::Result<u8>>,
        Self: Sized,
    {
        let flag = bytes.next()?.ok()?;
        if flag != WANT_HEARTBEAT_FLAG {
            return None;
        }
        let want_heartbeat = WantHeartbeat {
            interval: u32::deserialize(bytes)?,
        };
        Some(want_heartbeat)
    }
}

impl Serializable for Heartbeat {
    fn serialize(&self) -> Option<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.push(HEARTBEAT_FLAG);
        Some(bytes)
    }
}

impl Deserializable for IAmCamera {
    fn deserialize<I>(bytes: &mut I) -> Option<Self>
    where
        I: Iterator<Item = io::Result<u8>>,
        Self: Sized,
    {
        let flag = bytes.next()?.ok()?;
        if flag != I_AM_CAMERA_FLAG {
            return None;
        }
        let i_am_camera = IAmCamera {
            road: u16::deserialize(bytes)?,
            mile: u16::deserialize(bytes)?,
            limit: u16::deserialize(bytes)?,
        };
        Some(i_am_camera)
    }
}

impl Deserializable for IAmDispatcher {
    fn deserialize<I>(bytes: &mut I) -> Option<Self>
    where
        I: Iterator<Item = io::Result<u8>>,
        Self: Sized,
    {
        let flag = bytes.next()?.ok()?;
        if flag != I_AM_DISPATCHER_FLAG {
            return None;
        }
        let i_am_dispatcher = IAmDispatcher {
            roads: Vec::deserialize(bytes)?,
        };
        Some(i_am_dispatcher)
    }
}

impl Deserializable for u8 {
    fn deserialize<I>(bytes: &mut I) -> Option<Self>
    where
        I: Iterator<Item = io::Result<u8>>,
        Self: Sized,
    {
        let bytes: io::Result<Vec<u8>> = bytes.take(1).collect();
        let bytes: Vec<u8> = bytes.ok()?;
        let bytes: [u8; 1] = [bytes[0]];
        let u8 = u8::from_be_bytes(bytes);
        Some(u8)
    }
}

impl Serializable for u16 {
    fn serialize(&self) -> Option<Vec<u8>> {
        Some(self.to_be_bytes().to_vec())
    }
}

impl Deserializable for u16 {
    fn deserialize<I>(bytes: &mut I) -> Option<Self>
    where
        I: Iterator<Item = io::Result<u8>>,
        Self: Sized,
    {
        let bytes: io::Result<Vec<u8>> = bytes.take(2).collect();
        let bytes: Vec<u8> = bytes.ok()?;
        let bytes: [u8; 2] = [bytes[0], bytes[1]];
        let u16 = u16::from_be_bytes(bytes);
        Some(u16)
    }
}

impl Serializable for u32 {
    fn serialize(&self) -> Option<Vec<u8>> {
        Some(self.to_be_bytes().to_vec())
    }
}

impl Deserializable for u32 {
    fn deserialize<I>(bytes: &mut I) -> Option<Self>
    where
        I: Iterator<Item = io::Result<u8>>,
        Self: Sized,
    {
        let bytes: io::Result<Vec<u8>> = bytes.take(4).collect();
        let bytes: Vec<u8> = bytes.ok()?;
        let bytes: [u8; 4] = [bytes[0], bytes[1], bytes[2], bytes[3]];
        let u32 = u32::from_be_bytes(bytes);
        Some(u32)
    }
}

impl Serializable for String {
    fn serialize(&self) -> Option<Vec<u8>> {
        let len: u8 = self.len().try_into().ok()?;
        if !self.is_ascii() {
            return None;
        }
        let mut bytes = Vec::new();
        bytes.push(len);
        bytes.extend_from_slice(self.as_bytes());
        Some(bytes)
    }
}

impl Deserializable for String {
    fn deserialize<I>(bytes: &mut I) -> Option<Self>
    where
        I: Iterator<Item = io::Result<u8>>,
        Self: Sized,
    {
        let len: u8 = bytes.next()?.ok()?;
        let len: usize = len.try_into().ok()?;
        let bytes = bytes.take(len).collect::<Result<Vec<u8>, _>>().ok()?;
        if !bytes.is_ascii() {
            return None;
        }
        let string = String::from_utf8(bytes).ok()?;
        Some(string)
    }
}

impl Deserializable for Vec<u16> {
    fn deserialize<I>(bytes: &mut I) -> Option<Self>
    where
        I: Iterator<Item = io::Result<u8>>,
        Self: Sized,
    {
        let len = u8::deserialize(bytes)?;
        (0..len).map(|_| u16::deserialize(bytes)).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::Deserializable;
    use crate::Error;
    use crate::Heartbeat;
    use crate::IAmCamera;
    use crate::IAmDispatcher;
    use crate::Plate;
    use crate::Serializable;
    use crate::Ticket;
    use crate::WantHeartbeat;
    #[test]
    fn serialize_error() {
        let error = Error {
            msg: "bad".to_owned(),
        };
        let bytes = b"\x10\x03\x62\x61\x64";
        assert_eq!(error.serialize(), Some(bytes.to_vec()));
    }
    #[test]
    fn deserialize_plate() {
        let mut bytes = b"\x20\x07\x52\x45\x30\x35\x42\x4b\x47\x00\x01\xe2\x40"
            .to_vec()
            .into_iter()
            .map(Result::Ok);
        let plate = Plate::deserialize(&mut bytes).unwrap();
        assert_eq!(plate.plate, "RE05BKG".to_owned());
        assert_eq!(plate.timestamp, 123456);
    }
    #[test]
    fn serialize_ticket() {
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
        assert_eq!(ticket.serialize(), Some(bytes.to_vec()));
    }
    #[test]
    fn deserialize_want_heartbeat() {
        let mut bytes = b"\x40\x00\x00\x04\xdb".to_vec().into_iter().map(Result::Ok);
        let want_heartbeat = WantHeartbeat::deserialize(&mut bytes).unwrap();
        assert_eq!(want_heartbeat.interval, 1243);
    }
    #[test]
    fn serialize_heartbeat() {
        let heartbeat = Heartbeat {};
        let bytes = b"\x41";
        assert_eq!(heartbeat.serialize(), Some(bytes.to_vec()));
    }
    #[test]
    fn deserialize_i_am_camera() {
        let mut bytes = b"\x80\x01\x70\x04\xd2\x00\x28"
            .to_vec()
            .into_iter()
            .map(Result::Ok);
        let i_am_camera = IAmCamera::deserialize(&mut bytes).unwrap();
        assert_eq!(i_am_camera.road, 368);
        assert_eq!(i_am_camera.mile, 1234);
        assert_eq!(i_am_camera.limit, 40);
    }
    #[test]
    fn deserialize_i_am_dispatcher() {
        let mut bytes = b"\x81\x03\x00\x42\x01\x70\x13\x88"
            .to_vec()
            .into_iter()
            .map(Result::Ok);
        let i_am_dispatcher = IAmDispatcher::deserialize(&mut bytes).unwrap();
        assert_eq!(i_am_dispatcher.roads, vec![66, 368, 5000]);
    }
}
