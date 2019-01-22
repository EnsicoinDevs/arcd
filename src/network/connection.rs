enum State {
    Initiated,
    Replied,
    Idle,
    Confirm,
    Ack,
}

pub struct Connection {
    state: State,
    stream: std::net::TcpStream,
    server: std::sync::mpsc::Sender<Box<Message + Send>>,
}
