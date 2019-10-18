use ensicoin_serializer::Serialize;

#[derive(Serialize)]
pub struct Ping;

impl Ping {
    pub fn new() -> Ping {
        Ping
    }
}

#[derive(Serialize)]
pub struct Pong;

impl Pong {
    pub fn new() -> Pong {
        Pong {}
    }
}
