use std::collections::HashSet;
use std::env::args;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::io;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpListener;
use tokio::sync::broadcast::channel;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::Sender;
use tokio::task::spawn;

#[tokio::main]
async fn main() -> io::Result<()> {
    let address = args().nth(1).unwrap_or("0.0.0.0:8080".into());
    let listener = TcpListener::bind(address).await?;
    let (msg_sender, _) = channel::<Message>(50);
    let logged_names = Arc::new(Mutex::new(HashSet::new()));
    loop {
        let (stream, _) = listener.accept().await?;
        let (r_stream, w_stream) = stream.into_split();
        let r_stream = RStream::new(r_stream);
        let w_stream = WStream::new(w_stream);
        let msg_sender = msg_sender.clone();
        let logged_names = logged_names.clone();
        spawn(async move {
            let _ = process_connection(r_stream, w_stream, msg_sender, logged_names).await;
        });
    }
}

async fn process_connection(
    mut r_stream: RStream,
    mut w_stream: WStream,
    msg_sender: Sender<Message>,
    logged_names: Arc<Mutex<HashSet<String>>>,
) -> io::Result<()> {
    let Some(name) = ask_name(&mut r_stream, &mut w_stream).await? else { return Ok(()) };
    let Some(log_in_msg) = log_in(&logged_names, &name) else { return Ok(()) };
    w_stream.write(&log_in_msg).await?;
    let new_user_msg = build_new_user_msg(&name);
    let msg_receiver = msg_sender.subscribe();
    msg_sender.send(new_user_msg).unwrap();
    {
        let name = name.clone();
        spawn(async move {
            let _ = read_msgs(r_stream, msg_sender, name, logged_names).await;
        });
    }
    {
        let name = name.clone();
        spawn(async move {
            let _ = write_msgs(w_stream, msg_receiver, name).await;
        });
    }
    Ok(())
}

async fn ask_name(r_stream: &mut RStream, w_stream: &mut WStream) -> io::Result<Option<String>> {
    w_stream.write(&build_welcome_msg()).await?;
    let name = r_stream.read().await?.and_then(parse_name);
    Ok(name)
}

fn build_welcome_msg() -> String {
    "Welcome to budgetchat! What shall I call you?".to_owned()
}

fn parse_name(bytes: Vec<u8>) -> Option<String> {
    let name = String::from_utf8(bytes).ok()?;
    let name = name.replace('\r', "");
    if name.is_empty() {
        return None;
    }
    if !name.is_ascii() {
        return None;
    }
    if !name.chars().all(char::is_alphanumeric) {
        return None;
    }
    Some(name)
}

fn log_in(logged_names: &Arc<Mutex<HashSet<String>>>, name: &str) -> Option<String> {
    let mut logged_names = logged_names.lock().unwrap();
    let log_in_msg = build_log_in_msg(&logged_names);
    if !logged_names.insert(name.to_owned()) {
        return None;
    }
    Some(log_in_msg)
}

fn build_log_in_msg(logged_names: &HashSet<String>) -> String {
    let logged_names = logged_names
        .iter()
        .cloned()
        .collect::<Vec<String>>()
        .join(", ");
    format!("* The room contains: {logged_names}")
}

fn build_new_user_msg(name: &str) -> Message {
    Message {
        name: name.to_owned(),
        value: format!("* {name} has entered the room"),
    }
}

async fn read_msgs(
    mut r_stream: RStream,
    msg_sender: Sender<Message>,
    name: String,
    logged_names: Arc<Mutex<HashSet<String>>>,
) -> io::Result<()> {
    loop {
        let Some(bytes) = r_stream.read().await? else {
            let log_out_msg = log_out(&name, &logged_names);
            msg_sender.send(log_out_msg).unwrap();
            return Ok(());
        };
        if let Some(chat_msg) = parse_chat_msg(bytes, &name) {
            msg_sender.send(chat_msg).unwrap();
        }
    }
}

fn log_out(name: &str, logged_names: &Arc<Mutex<HashSet<String>>>) -> Message {
    let mut logged_names = logged_names.lock().unwrap();
    logged_names.remove(name);
    build_log_out_msg(name)
}

fn build_log_out_msg(name: &str) -> Message {
    Message {
        name: name.to_owned(),
        value: format!("* {name} has left the room"),
    }
}

fn parse_chat_msg(bytes: Vec<u8>, name: &str) -> Option<Message> {
    let value = String::from_utf8(bytes).ok()?;
    let value = value.replace('\r', "");
    if value.is_empty() {
        return None;
    }
    if !value.is_ascii() {
        return None;
    }
    let msg = Message {
        name: name.to_owned(),
        value: format!("[{name}] {value}"),
    };
    Some(msg)
}

async fn write_msgs(
    mut w_stream: WStream,
    mut msg_receiver: Receiver<Message>,
    name: String,
) -> io::Result<()> {
    loop {
        let msg = msg_receiver.recv().await.unwrap();
        if msg.name != name {
            w_stream.write(&msg.value).await?;
        }
    }
}

#[derive(Clone, Debug)]
struct Message {
    name: String,
    value: String,
}

struct RStream {
    r_stream: BufReader<OwnedReadHalf>,
}

impl RStream {
    fn new(r_stream: OwnedReadHalf) -> Self {
        Self {
            r_stream: BufReader::new(r_stream),
        }
    }
    async fn read(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut bytes = Vec::new();
        let n = self.r_stream.read_until(EOM, &mut bytes).await?;
        if n == 0 {
            return Ok(None);
        }
        bytes.pop();
        Ok(Some(bytes))
    }
}

struct WStream {
    w_stream: OwnedWriteHalf,
}

impl WStream {
    fn new(w_stream: OwnedWriteHalf) -> Self {
        Self { w_stream }
    }
    async fn write(&mut self, value: &str) -> io::Result<()> {
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(EOM);
        self.w_stream.write_all(&bytes).await
    }
}

const EOM: u8 = b'\n';
