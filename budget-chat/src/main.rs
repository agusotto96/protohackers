use std::collections::HashSet;
use std::sync::Arc;
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
use tokio::sync::Mutex;
use tokio::task::spawn;

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    let (msg_sender, _) = channel::<Message>(50);
    let logged_names = Arc::new(Mutex::new(HashSet::new()));
    loop {
        let Ok((stream, _)) = listener.accept().await else { continue };
        let (r_stream, w_stream) = stream.into_split();
        let r_stream = BufReader::new(r_stream);
        let msg_sender = msg_sender.clone();
        let logged_names = logged_names.clone();
        spawn(async move {
            let _ = handle_connection(r_stream, w_stream, msg_sender, logged_names).await;
        });
    }
}

async fn handle_connection(
    mut r_stream: BufReader<OwnedReadHalf>,
    mut w_stream: OwnedWriteHalf,
    msg_sender: Sender<Message>,
    logged_names: Arc<Mutex<HashSet<String>>>,
) -> io::Result<()> {
    let Some(name) = ask_name(&mut r_stream, &mut w_stream).await? else { return Ok(()) };
    if !log_in(&logged_names, &name, &mut w_stream).await? {
        return Ok(());
    };
    let msg_receiver = msg_sender.subscribe();
    notify_log_in(&name, &msg_sender);
    {
        let name = name.clone();
        let msg_sender = msg_sender.clone();
        let logged_names = logged_names.clone();
        spawn(async move {
            let _ = read_msgs(&mut r_stream, msg_sender, name, logged_names).await;
        });
    }
    {
        let name = name.clone();
        let msg_sender = msg_sender.clone();
        let logged_names = logged_names.clone();
        spawn(async move {
            let _ = write_msgs(w_stream, msg_receiver, msg_sender, name, logged_names).await;
        });
    }
    Ok(())
}

async fn read_msgs(
    r_stream: &mut BufReader<OwnedReadHalf>,
    msg_sender: Sender<Message>,
    name: String,
    logged_names: Arc<Mutex<HashSet<String>>>,
) -> io::Result<()> {
    loop {
        let Ok(Some(bytes)) = read_bytes(r_stream).await else {
            eprintln!("wtf i am doing here1");
            log_out(&name, &logged_names).await;
            notify_log_out(&name, &msg_sender);
            return Ok(());
        };
        if let Some(chat_msg) = parse_chat_msg(bytes, &name) {
            msg_sender.send(chat_msg).unwrap();
        }
    }
}

async fn write_msgs(
    mut w_stream: OwnedWriteHalf,
    mut msg_receiver: Receiver<Message>,
    msg_sender: Sender<Message>,
    name: String,
    logged_names: Arc<Mutex<HashSet<String>>>,
) -> io::Result<()> {
    loop {
        let msg = msg_receiver.recv().await.unwrap();
        if msg.name == name {
            continue;
        }
        let bytes = msg.value.as_bytes();
        if write_bytes(&mut w_stream, bytes).await.is_err() {
            eprintln!("wtf i am doing here2");
            log_out(&name, &logged_names).await;
            notify_log_out(&name, &msg_sender);
            return Ok(());
        }
    }
}

/// Shows the user a welcome message asking for their name, and then attempts to parse their response.
async fn ask_name(
    r_stream: &mut BufReader<OwnedReadHalf>,
    w_stream: &mut OwnedWriteHalf,
) -> io::Result<Option<String>> {
    let welcome_msg = build_welcome_msg();
    write_bytes(w_stream, welcome_msg.as_bytes()).await?;
    let name = read_bytes(r_stream).await?.and_then(parse_name);
    Ok(name)
}

/// Attempts to add a name to the set of logged names. If added successfully, displays to the user the names already logged.
async fn log_in(
    logged_names: &Arc<Mutex<HashSet<String>>>,
    name: &str,
    w_stream: &mut OwnedWriteHalf,
) -> io::Result<bool> {
    let mut logged_names = logged_names.lock().await;
    if logged_names.contains(name) {
        return Ok(false);
    }
    let log_in_msg = build_log_in_msg(&logged_names);
    write_bytes(w_stream, log_in_msg.as_bytes()).await?;
    logged_names.insert(name.to_owned());
    Ok(true)
}

/// Notifies users of a newly registered user.
fn notify_log_in(name: &str, msg_sender: &Sender<Message>) {
    let new_user_msg = build_new_user_msg(name);
    msg_sender.send(new_user_msg).unwrap();
}

/// Removes a name from the set of logged names.
async fn log_out(name: &str, logged_names: &Arc<Mutex<HashSet<String>>>) {
    let mut logged_names = logged_names.lock().await;
    logged_names.remove(name);
}

/// Notifies users of a newly logged out user.
fn notify_log_out(name: &str, msg_sender: &Sender<Message>) {
    let log_out_msg = build_log_out_msg(name);
    msg_sender.send(log_out_msg).unwrap();
}

fn build_welcome_msg() -> String {
    "Welcome to budgetchat! What shall I call you?".to_owned()
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

fn build_log_out_msg(name: &str) -> Message {
    Message {
        name: name.to_owned(),
        value: format!("* {name} has left the room"),
    }
}

fn parse_chat_msg(bytes: Vec<u8>, name: &str) -> Option<Message> {
    let value = String::from_utf8(bytes).ok()?;
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

fn parse_name(bytes: Vec<u8>) -> Option<String> {
    let name = String::from_utf8(bytes).ok()?;
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

async fn read_bytes(r_stream: &mut BufReader<OwnedReadHalf>) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    let n = r_stream.read_until(EOM, &mut bytes).await?;
    if n == 0 {
        return Ok(None);
    }
    bytes.pop();
    Ok(Some(bytes))
}

async fn write_bytes(w_stream: &mut OwnedWriteHalf, bytes: &[u8]) -> io::Result<()> {
    let mut bytes = bytes.to_vec();
    bytes.push(EOM);
    w_stream.write_all(&bytes).await
}

#[derive(Clone, Debug)]
struct Message {
    name: String,
    value: String,
}

const EOM: u8 = b'\n';
