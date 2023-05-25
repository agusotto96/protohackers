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

const EOM: u8 = b'\n';

#[tokio::main]
async fn main() -> io::Result<()> {
    let address = args().nth(1).unwrap_or("0.0.0.0:8080".into());
    let listener = TcpListener::bind(address).await?;
    let (sender, _) = channel::<Message>(50);
    let active_users = Arc::new(Mutex::new(HashSet::new()));
    loop {
        let (socket, _) = listener.accept().await?;
        let (reader, writer) = socket.into_split();
        let reader = BufReader::new(reader);
        let sender = sender.clone();
        let active_users = active_users.clone();
        spawn(async move {
            let _ = process_socket(reader, writer, sender, active_users).await;
        });
    }
}

async fn process_socket(
    mut reader: BufReader<OwnedReadHalf>,
    mut writer: OwnedWriteHalf,
    sender: Sender<Message>,
    active_users: Arc<Mutex<HashSet<String>>>,
) -> io::Result<()> {
    let Some(name) = ask_name(&mut reader, &mut writer).await? else { return Ok(()) };
    let Some(welcome_message) = build_welcome_message(&active_users, &name) else { return Ok(()) };
    writer.write_all(&welcome_message).await?;
    let new_user_message = build_new_user_message(&name);
    let receiver = sender.subscribe();
    sender.send(new_user_message).unwrap();
    spawn({
        let name = name.clone();
        async move {
            let _ = read_message(reader, sender, name, active_users).await;
        }
    });
    spawn({
        let name = name.clone();
        async move {
            let _ = write_message(writer, receiver, name).await;
        }
    });
    Ok(())
}

async fn ask_name(
    reader: &mut BufReader<OwnedReadHalf>,
    writer: &mut OwnedWriteHalf,
) -> io::Result<Option<String>> {
    writer
        .write_all(b"Welcome to budgetchat! What shall I call you?\n")
        .await?;
    let mut input = Vec::new();
    let n = reader.read_until(EOM, &mut input).await?;
    if n == 0 {
        return Ok(None);
    }
    let name = deserialize_name(input);
    Ok(name)
}

fn deserialize_name(mut bytes: Vec<u8>) -> Option<String> {
    bytes.pop();
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

fn build_welcome_message(
    active_users: &Arc<Mutex<HashSet<String>>>,
    name: &str,
) -> Option<Vec<u8>> {
    let mut active_users = active_users.lock().unwrap();
    let names = active_users
        .iter()
        .cloned()
        .collect::<Vec<String>>()
        .join(", ");
    if !active_users.insert(name.to_owned()) {
        return None;
    }
    let mut welcome_message = format!("* The room contains: {names}").into_bytes();
    welcome_message.push(EOM);
    Some(welcome_message)
}

fn build_new_user_message(name: &str) -> Message {
    Message {
        name: name.to_owned(),
        value: format!("* {name} has entered the room"),
    }
}

async fn read_message(
    mut reader: BufReader<OwnedReadHalf>,
    sender: Sender<Message>,
    name: String,
    active_users: Arc<Mutex<HashSet<String>>>,
) -> io::Result<()> {
    loop {
        let mut input = Vec::new();
        let n = reader.read_until(EOM, &mut input).await?;
        if n == 0 {
            log_out(&name, &active_users, &sender);
            return Ok(());
        }
        if let Some(value) = deserialize_message(input) {
            let message = Message {
                name: name.clone(),
                value: format!("[{name}] {value}"),
            };
            sender.send(message).unwrap();
        }
    }
}

fn log_out(name: &str, active_users: &Arc<Mutex<HashSet<String>>>, sender: &Sender<Message>) {
    let message = build_log_out_message(name);
    let mut active_users = active_users.lock().unwrap();
    active_users.remove(name);
    sender.send(message).unwrap();
}

fn build_log_out_message(name: &str) -> Message {
    Message {
        name: name.to_owned(),
        value: format!("* {name} has left the room"),
    }
}

fn deserialize_message(mut bytes: Vec<u8>) -> Option<String> {
    bytes.pop();
    let message = String::from_utf8(bytes).ok()?;
    let message = message.replace('\r', "");
    if message.is_empty() {
        return None;
    }
    if !message.is_ascii() {
        return None;
    }
    Some(message)
}

async fn write_message(
    mut writer: OwnedWriteHalf,
    mut receiver: Receiver<Message>,
    name: String,
) -> io::Result<()> {
    loop {
        let message = receiver.recv().await.unwrap();
        if message.name != name {
            let mut bytes = message.value.into_bytes();
            bytes.push(EOM);
            writer.write_all(&bytes).await?;
        }
    }
}

#[derive(Clone, Debug)]
struct Message {
    name: String,
    value: String,
}
