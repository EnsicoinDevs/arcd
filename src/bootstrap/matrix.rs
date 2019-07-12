use futures::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub homeserver: String,
    pub token: String,
    pub user_id: String,
}

#[derive(Deserialize, Debug)]
struct Event {
    pub content: HashMap<String, String>,
    #[serde(rename = "type")]
    pub event_type: String,
}

#[derive(Deserialize, Debug)]
struct UnsignedData {
    pub age: Option<i64>,
    pub redacted_because: Option<Event>,
    pub transaction_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct EventContent {
    pub avatar_url: Option<String>,
    pub displayname: Option<String>,
    pub membership: String,
    pub is_direct: Option<bool>,
    pub unsigned: Option<UnsignedData>,
}

#[derive(Deserialize, Debug)]
struct MemberEvent {
    pub content: EventContent,
    #[serde(rename = "type")]
    pub member_type: String,
    pub event_id: String,
    pub sender: String,
    pub origin_server_ts: i64,
    pub unsigned: Option<UnsignedData>,
    pub room_id: String,
    pub prev_content: Option<EventContent>,
    pub state_key: String,
}

#[derive(Deserialize, Debug)]
struct MembersReply {
    pub chunk: Vec<MemberEvent>,
}

pub struct MatrixClient {
    client: reqwest::Client,
    config: Config,
}

impl MatrixClient {
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn new(config: Config) -> MatrixClient {
        MatrixClient {
            client: reqwest::Client::new(),
            config,
        }
    }

    pub fn get_room_id(&self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let join_req: HashMap<String, String> = self
            .client
            .post(&format!(
                "{}/join/%23ensicoin%3Amatrix.org?access_token={}",
                self.config.homeserver, self.config.token
            ))
            .send()?
            .json()?;
        Ok(join_req.get("room_id").cloned())
    }
    pub fn get_bots(
        &self,
        room_id: &str,
        magic: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let members: MembersReply = self
            .client
            .get(&format!(
                "{}/rooms/{}/members?access_token={}",
                self.config.homeserver, room_id, self.config.token
            ))
            .send()?
            .json()?;
        let mut bots = Vec::new();
        for member in members.chunk {
            match member.content.displayname {
                Some(s) => {
                    if member.sender != self.config.user_id && s.starts_with(&magic) {
                        bots.push(s)
                    }
                }
                _ => (),
            }
        }
        Ok(bots)
    }
    pub fn set_name(
        &self,
        magic: &str,
        ip: &str,
        port: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let name = format!("{}_{}:{}", magic, ip, port);
        let mut payload = HashMap::new();
        payload.insert("displayname", name);
        self.client
            .put(&format!(
                "{}/profile/{}/displayname?access_token={}",
                self.config.homeserver, self.config.user_id, self.config.token
            ))
            .json(&payload)
            .send()?;
        Ok(())
    }
    pub fn set_status(&self, status: &Status) -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = HashMap::new();
        payload.insert("presence", status.to_string());
        self.client
            .put(&format!(
                "{}/presence/{}/status?access_token={}",
                self.config.homeserver, self.config.user_id, self.config.token
            ))
            .json(&payload)
            .send()?;
        Ok(())
    }
}

pub fn async_set_status(config: &Config, status: &Status) {
    let mut payload = HashMap::new();
    payload.insert("presence", status.to_string());
    let client = reqwest::r#async::Client::new();
    tokio::spawn(
        client
            .put(&format!(
                "{}/presence/{}/status?access_token={}",
                config.homeserver, config.user_id, config.token
            ))
            .json(&payload)
            .send()
            .map_err(|e| warn!("Error changing matrix status: {}", e))
            .map(|_| ()),
    );
}

pub enum Status {
    Online,
    Offline,
    Unavailable,
}

impl Status {
    fn to_string(&self) -> &'static str {
        match self {
            Status::Online => "online",
            Status::Offline => "offline",
            Status::Unavailable => "unavailable",
        }
    }
}
