use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{MessageEvent, RtcDataChannelState, WebSocket};

use crate::{
    ice_server::IceServer,
    messages::{ClientMessage, ServerAnswer, ServerMessage, ServerOffer},
    p2p_connection::{P2PConnection, SDP},
    utils::sleep,
};

struct P2PInner {
    id: Option<String>,
    socket: WebSocket,
    connections: HashMap<String, P2PConnection>,
    new_connections: Vec<P2PConnection>,
    ice_servers: Vec<IceServer>,
}

pub struct P2P {
    inner: Rc<RefCell<P2PInner>>,
}

impl P2P {
    pub async fn new(url: &str) -> P2P {
        let signaling = P2P {
            inner: Rc::new(RefCell::new(P2PInner {
                id: None,
                socket: P2P::create_socket(url).await,
                connections: HashMap::new(),
                ice_servers: vec![IceServer::from("stun:stun.l.google.com:19302")],
                new_connections: Vec::new(),
            })),
        };

        signaling.listen();

        return signaling;
    }

    pub async fn receive_connections(&self) -> Vec<P2PConnection> {
        std::mem::replace(&mut self.inner.borrow_mut().new_connections, Vec::new())
    }

    pub fn new_connection(&self, peer_id: &str) {
        let connection = self.inner.borrow_mut().connections.remove(peer_id).unwrap();
        self.inner.borrow_mut().new_connections.push(connection);
    }

    pub fn get_ice_servers(&self) -> Vec<IceServer> {
        self.inner.borrow().ice_servers.clone()
    }

    pub fn set_ice_servers(&self, ice_server: Vec<IceServer>) {
        self.inner.borrow_mut().ice_servers = ice_server;
    }

    pub async fn connect(&self, peer_id: &str) -> P2PConnection {
        let mut connection = P2PConnection::new(peer_id, self.clone());
        let offer = connection.create_offer().await;
        self.send(offer);

        loop {
            if connection.ready_state().await == RtcDataChannelState::Open {
                break;
            }

            sleep(0).await;
        }

        return connection;
    }

    pub async fn id(&mut self) -> String {
        if let Some(id) = &self.inner.borrow_mut().id {
            return id.to_string();
        }

        self.send(ClientMessage::GetMyID);

        loop {
            if let Some(id) = &self.inner.borrow_mut().id {
                return id.to_string();
            }

            sleep(0).await;
        }
    }

    async fn create_socket(url: &str) -> WebSocket {
        let socket = WebSocket::new(url).unwrap();

        loop {
            if socket.ready_state() == WebSocket::OPEN {
                return socket;
            }

            sleep(0).await;
        }
    }

    pub fn clone(&self) -> P2P {
        P2P {
            inner: self.inner.clone(),
        }
    }

    fn listen(&self) {
        let mut cloned = self.clone();

        let on_message_callback =
            Closure::<dyn FnMut(MessageEvent)>::new(move |message: MessageEvent| {
                let message = message.data().as_string().unwrap();
                if let Ok(message) = serde_json::from_str::<ServerMessage>(&message) {
                    cloned.handle_message(message);
                }
            });

        self.inner
            .borrow_mut()
            .socket
            .set_onmessage(Some(on_message_callback.as_ref().unchecked_ref()));

        on_message_callback.forget();
    }

    fn handle_message(&mut self, message: ServerMessage) {
        let mut cloned = self.clone();

        spawn_local(async move {
            match message {
                ServerMessage::ID(data) => cloned.inner.borrow_mut().id = Some(data.id),
                ServerMessage::Offer(offer) => cloned.handle_offer(offer).await,
                ServerMessage::Answer(answer) => cloned.handle_answer(answer).await,
            }
        });
    }

    fn send(&self, message: ClientMessage) {
        let json = serde_json::to_string(&message).unwrap();
        self.inner.borrow_mut().socket.send_with_str(&json).unwrap();
    }

    async fn handle_offer(&mut self, offer: ServerOffer) {
        let connection = P2PConnection::new(&offer.from, self.clone());
        connection.set_remote_sdp(&offer.sdp, SDP::Offer).await;
        let answer = connection.create_answer().await;
        self.send(answer);
        self.inner
            .borrow_mut()
            .connections
            .insert(offer.from, connection);
    }

    async fn handle_answer(&self, answer: ServerAnswer) {
        if let Some(connection) = self.inner.borrow_mut().connections.get(&answer.from) {
            connection.set_remote_sdp(&answer.sdp, SDP::Answer).await;
        }
    }
}
