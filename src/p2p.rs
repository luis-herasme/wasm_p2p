use crate::{
    messages::{ClientMessage, ServerAnswer, ServerMessage, ServerOffer},
    p2p_connection::{P2PConnection, SDP},
    utils::sleep,
};
use serde;
use serde::Serialize;
use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IceServer {
    pub urls: String,
    pub credential: Option<String>,
    pub credential_type: Option<String>,
    pub username: Option<String>,
}

pub struct P2P {
    id: Option<String>,
    socket: WebSocket,
    connections: HashMap<String, P2PConnection>,
    pub ice_servers: Vec<IceServer>,
}

impl P2P {
    pub async fn new(url: &str) -> Rc<RefCell<P2P>> {
        let p2p = P2P {
            socket: P2P::create_signaling_socket(url).await,
            id: None,
            connections: HashMap::new(),
            ice_servers: vec![IceServer {
                urls: String::from("stun:stun.l.google.com:19302"),
                credential: None,
                credential_type: None,
                username: None,
            }],
        };

        return p2p.init();
    }

    pub async fn id(&mut self) -> String {
        if let Some(id) = &self.id {
            return id.to_string();
        }

        self.send(ClientMessage::GetMyID);

        loop {
            if let Some(id) = &self.id {
                return id.to_string();
            }

            sleep(0).await;
        }
    }

    pub async fn connect(&self, peer_id: &str) -> P2PConnection {
        let mut connection = P2PConnection::new(peer_id);
        let offer = connection.create_offer().await;
        self.send(offer);
        return connection;
    }

    async fn handle_offer(&mut self, offer: ServerOffer) {
        let connection = P2PConnection::new(&offer.from);
        connection.set_remote_sdp(&offer.sdp, SDP::Offer).await;
        let answer = connection.create_answer().await;
        self.send(answer);
        self.connections.insert(offer.from, connection);
    }

    async fn handle_answer(&self, answer: ServerAnswer) {
        if let Some(connection) = self.connections.get(&answer.from) {
            connection.set_remote_sdp(&answer.sdp, SDP::Answer).await;
        }
    }

    async fn create_signaling_socket(url: &str) -> WebSocket {
        let socket = WebSocket::new(url).unwrap();

        loop {
            if socket.ready_state() == WebSocket::OPEN {
                return socket;
            }

            sleep(0).await;
        }
    }

    fn init(self) -> Rc<RefCell<P2P>> {
        let p2p = Rc::new(RefCell::new(self));

        let on_message_p2p = Rc::clone(&p2p);
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |message: MessageEvent| {
            let message = message.data().as_string().unwrap();
            if let Ok(message) = serde_json::from_str::<ServerMessage>(&message) {
                wasm_bindgen_futures::spawn_local(handler_message(on_message_p2p.clone(), message));
            }
        });

        p2p.borrow_mut()
            .socket
            .set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        on_message.forget();

        return p2p;
    }

    pub fn send(&self, message: ClientMessage) {
        let json = serde_json::to_string(&message).unwrap();
        self.socket.send_with_str(&json).unwrap();
    }
}

async fn handler_message(p2p: Rc<RefCell<P2P>>, message: ServerMessage) {
    let mut p2p = p2p.borrow_mut();

    match message {
        ServerMessage::ID(data) => p2p.id = Some(data.id),
        ServerMessage::Offer(offer) => p2p.handle_offer(offer).await,
        ServerMessage::Answer(answer) => p2p.handle_answer(answer).await,
    }
}
