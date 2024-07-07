use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientOffer {
    pub to: String,
    pub sdp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientAnswer {
    pub to: String,
    pub sdp: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Answer(ClientAnswer),
    Offer(ClientOffer),
    GetMyID,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerOffer {
    pub from: String,
    pub to: String,
    pub sdp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerAnswer {
    pub from: String,
    pub to: String,
    pub sdp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ID {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ServerMessage {
    Answer(ServerAnswer),
    Offer(ServerOffer),
    ID(ID),
}
