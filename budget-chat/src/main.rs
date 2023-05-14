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
    let (tx, _) = channel::<Message>(50);
    let active_users = Arc::new(Mutex::new(HashSet::new()));
    loop {
        let (socket, client) = listener.accept().await?;
        let (reader, writer) = socket.into_split();
        let client = client.to_string();
        let reader = BufReader::new(reader);
        let tx = tx.clone();
        let active_users = active_users.clone();
        spawn(async move {
            let _ = process_socket(reader, writer, client, tx, active_users).await;
        });
    }
}

async fn process_socket(
    mut reader: BufReader<OwnedReadHalf>,
    mut writer: OwnedWriteHalf,
    client: String,
    tx: Sender<Message>,
    active_users: Arc<Mutex<HashSet<String>>>,
) -> io::Result<()> {
    let Some(name) = ask_name(&mut reader, &mut writer).await? else { return Ok(()) };
    let welcome_message = {
        let mut active_users = active_users.lock().unwrap();
        let names = active_users
            .iter()
            .cloned()
            .collect::<Vec<String>>()
            .join(", ");
        if !active_users.insert(name.clone()) {
            return Ok(());
        }
        let mut welcome_message = format!("* The room contains: {names}").into_bytes();
        welcome_message.push(EOM);
        welcome_message
    };
    let message = Message {
        name: name.clone(),
        client: client.clone(),
        value: format!("* {} has entered the room", name.clone()),
        is_chat: false,
    };
    let rx = tx.subscribe();
    tx.send(message).unwrap();
    writer.write_all(&welcome_message).await?;
    let r_name = name.clone();
    let r_client = client.clone();
    spawn(async move {
        let _ = read_message(&mut reader, tx, r_name, r_client, active_users).await;
    });
    spawn(async move {
        let _ = write_message(&mut writer, rx, client).await;
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
    input.pop();
    let name = deserialize_name(input);
    Ok(name)
}

async fn read_message(
    reader: &mut BufReader<OwnedReadHalf>,
    tx: Sender<Message>,
    name: String,
    client: String,
    active_users: Arc<Mutex<HashSet<String>>>,
) -> io::Result<()> {
    loop {
        let mut input = Vec::new();
        let n = reader.read_until(EOM, &mut input).await?;
        if n == 0 {
            let message = Message {
                name: name.clone(),
                client: client.clone(),
                value: format!("* {} has left the room", name.clone()),
                is_chat: false,
            };
            let mut active_users = active_users.lock().unwrap();
            active_users.remove(&name);
            tx.send(message).unwrap();
            return Ok(());
        }
        input.pop();
        if let Some(value) = deserialize_message(input) {
            let message = Message {
                name: name.clone(),
                client: client.clone(),
                value,
                is_chat: true,
            };
            tx.send(message).unwrap();
        }
    }
}

async fn write_message(
    writer: &mut OwnedWriteHalf,
    mut rx: Receiver<Message>,
    client: String,
) -> io::Result<()> {
    loop {
        let message = rx.recv().await.unwrap();
        if message.client != client {
            let mut bytes = if message.is_chat {
                format!("[{}] {}", message.name, message.value).into_bytes()
            } else {
                message.value.into_bytes()
            };
            bytes.push(EOM);
            writer.write_all(&bytes).await?;
        }
    }
}

fn deserialize_name(bytes: Vec<u8>) -> Option<String> {
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

fn deserialize_message(bytes: Vec<u8>) -> Option<String> {
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

#[derive(Clone, Debug)]
struct Message {
    name: String,
    client: String,
    value: String,
    is_chat: bool,
}
