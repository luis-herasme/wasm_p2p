use std::vec::IntoIter;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{MessageEvent, RtcDataChannelState};

use crate::signaling::Signaling;
use crate::{
    ice_server::IceServer,
    messages::{ClientMessage, ServerAnswer, ServerMessage, ServerOffer},
    p2p_connection::{P2PConnection, SDP},
    utils::sleep,
};

struct P2PInner {
    signaling: Signaling,
    connections: HashMap<String, P2PConnection>,
    new_connections: Vec<P2PConnection>,
    ice_servers: Vec<IceServer>,
}

#[derive(Clone)]
pub struct P2P {
    inner: Rc<RefCell<P2PInner>>,
}

impl P2P {
    pub async fn new(url: &str) -> P2P {
        let signaling = Signaling::new(url).await;

        let p2p = P2P {
            inner: Rc::new(RefCell::new(P2PInner {
                signaling,
                connections: HashMap::new(),
                ice_servers: vec![IceServer::from("stun:stun.l.google.com:19302")],
                new_connections: Vec::new(),
            })),
        };

        p2p.listen();

        return p2p;
    }

    pub fn receive_connections(&self) -> IntoIter<P2PConnection> {
        std::mem::replace(&mut self.inner.borrow_mut().new_connections, Vec::new()).into_iter()
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
        self.inner
            .borrow_mut()
            .connections
            .insert(peer_id.to_string(), connection.clone());

        loop {
            if connection.ready_state().await == RtcDataChannelState::Open {
                break;
            }

            sleep(0).await;
        }

        return connection;
    }

    pub fn id(&self) -> String {
        self.inner.borrow().signaling.id()
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
            .signaling
            .socket
            .set_onmessage(Some(on_message_callback.as_ref().unchecked_ref()));

        on_message_callback.forget();
    }

    fn handle_message(&mut self, message: ServerMessage) {
        let mut cloned = self.clone();

        spawn_local(async move {
            match message {
                ServerMessage::ID(_) => {}
                ServerMessage::Offer(offer) => cloned.handle_offer(offer).await,
                ServerMessage::Answer(answer) => cloned.handle_answer(answer).await,
            }
        });
    }

    fn send(&self, message: ClientMessage) {
        let json = serde_json::to_string(&message).unwrap();
        self.inner
            .borrow_mut()
            .signaling
            .socket
            .send_with_str(&json)
            .unwrap();
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
