use crate::{
    messages::{
        ClientAnswer, ClientMessage, ClientOffer, ServerAnswer, ServerMessage, ServerOffer,
    },
    utils::sleep,
};
use serde;
use serde::Serialize;
use std::{cell::RefCell, collections::HashMap, rc::Rc, vec::IntoIter};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    js_sys::Reflect, MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelEvent,
    RtcIceGatheringState, RtcPeerConnection, RtcSdpType, RtcSessionDescriptionInit, WebSocket,
};

pub enum ConnectionUpdate {
    Connected(String),
    Disconnected(String),
}

struct P2PInner {
    socket: WebSocket,
    id: Option<String>,
    connection_states: Vec<ConnectionUpdate>,
    peer_messages: Vec<(String, String)>,
    signaling_messages: Vec<String>,
    connections: HashMap<String, RtcPeerConnection>,
    channels: HashMap<String, RtcDataChannel>,
    ice_servers: Vec<IceServer>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IceServer {
    pub urls: String,
    pub credential: Option<String>,
    pub credential_type: Option<String>,
    pub username: Option<String>,
}

#[derive(Clone)]
pub struct P2P {
    inner: Rc<RefCell<P2PInner>>,
}

impl P2P {
    pub fn set_ice_servers(&self, ice_servers: Vec<IceServer>) {
        let mut inner = self.inner.borrow_mut();
        inner.ice_servers = ice_servers;
    }

    pub fn send(&self, other_peer_id: &str, data: &str) {
        let inner = self.inner.borrow();
        let other_peer_channel = inner.channels.get(other_peer_id);

        if let Some(channel) = other_peer_channel {
            channel.send_with_str(data).unwrap();
        }
    }

    pub fn send_u8_array(&self, other_peer_id: &str, data: &[u8]) {
        let inner = self.inner.borrow();
        let other_peer_channel = inner.channels.get(other_peer_id);

        if let Some(channel) = other_peer_channel {
            channel.send_with_u8_array(data).unwrap();
        }
    }

    pub fn new(url: &str) -> Self {
        let ice_servers: Vec<IceServer> = vec![IceServer {
            urls: String::from("stun:stun.l.google.com:19302"),
            credential: None,
            credential_type: None,
            username: None,
        }];

        let inner = P2PInner {
            socket: WebSocket::new(url).unwrap(),
            id: None,
            connection_states: Vec::new(),
            peer_messages: Vec::new(),
            connections: HashMap::new(),
            signaling_messages: Vec::new(),
            channels: HashMap::new(),
            ice_servers,
        };

        let mut p2p = P2P {
            inner: Rc::new(RefCell::new(inner)),
        };

        p2p.listen_to_signaling_messages();

        return p2p;
    }

    fn listen_to_signaling_messages(&mut self) {
        let p2p_inner = Rc::clone(&self.inner);

        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |message: MessageEvent| {
            let message = message.data().as_string().unwrap();
            p2p_inner.borrow_mut().signaling_messages.push(message);
        });

        self.inner
            .borrow_mut()
            .socket
            .set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        on_message.forget();
    }

    pub async fn update(&mut self) -> (IntoIter<(String, String)>, IntoIter<ConnectionUpdate>) {
        sleep(0).await;
        self.update_signaling().await;
        let messages = self.messages();
        let connection_updates = self.connection_updates();
        return (messages, connection_updates);
    }

    async fn update_signaling(&self) {
        let signaling_messages =
            std::mem::replace(&mut self.inner.borrow_mut().signaling_messages, Vec::new());

        for message in signaling_messages.into_iter() {
            if let Ok(value) = serde_json::from_str::<ServerMessage>(&message) {
                match value {
                    ServerMessage::ID(data) => {
                        self.inner.borrow_mut().id = Some(data.id);
                    }
                    ServerMessage::Offer(offer) => {
                        self.handle_offer(offer).await;
                    }
                    ServerMessage::Answer(answer) => {
                        self.handle_answer(answer).await;
                    }
                }
            }
        }
    }

    fn create_connection(&self) -> RtcPeerConnection {
        let mut config = RtcConfiguration::new();
        let config = config
            .ice_servers(&serde_wasm_bindgen::to_value(&self.inner.borrow().ice_servers).unwrap());
        let connection = RtcPeerConnection::new_with_configuration(&config).unwrap();
        return connection;
    }

    async fn handle_offer(&self, offer: ServerOffer) {
        let connection = self.create_connection();

        // Remote description
        let mut session_description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        session_description.sdp(&offer.sdp);
        let set_remote_description = connection.set_remote_description(&session_description);
        JsFuture::from(set_remote_description).await.unwrap();

        // Local description. Create answer
        let create_answer_promise = connection.create_answer();
        let answer = JsFuture::from(create_answer_promise).await.unwrap();

        let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))
            .unwrap()
            .as_string()
            .unwrap();

        let mut session_description = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        session_description.sdp(&answer_sdp);

        let set_local_description = connection.set_local_description(&session_description);
        JsFuture::from(set_local_description).await.unwrap();

        loop {
            if connection.ice_gathering_state() == RtcIceGatheringState::Complete {
                break;
            }

            sleep(1).await;
        }

        // Send answer
        let message = serde_json::to_string(&ClientMessage::Answer(ClientAnswer {
            to: offer.from,
            sdp: connection.local_description().unwrap().sdp(),
        }))
        .unwrap();

        {
            self.inner.borrow().socket.send_with_str(&message).unwrap();
        }

        let inner = Rc::clone(&self.inner);

        let on_data_channel =
            Closure::<dyn FnMut(RtcDataChannelEvent)>::new(move |event: RtcDataChannelEvent| {
                let channel = event.channel();
                let peer_id = { inner.borrow().id.clone().unwrap() };
                setup_channel(inner.clone(), channel, peer_id);
            });

        connection.set_ondatachannel(Some(on_data_channel.as_ref().unchecked_ref()));
        on_data_channel.forget();
    }

    async fn handle_answer(&self, answer: ServerAnswer) {
        if let Some(connection) = self.inner.borrow().connections.get(&answer.from) {
            let mut session_description = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
            session_description.sdp(&answer.sdp);
            let set_remote_description = connection.set_remote_description(&session_description);
            JsFuture::from(set_remote_description).await.unwrap();
        }
    }

    fn messages(&mut self) -> IntoIter<(String, String)> {
        std::mem::replace(&mut (*self.inner.borrow_mut()).peer_messages, Vec::new()).into_iter()
    }

    fn connection_updates(&mut self) -> IntoIter<ConnectionUpdate> {
        std::mem::replace(
            &mut (*self.inner.borrow_mut()).connection_states,
            Vec::new(),
        )
        .into_iter()
    }

    pub async fn id(&self) -> String {
        if let Some(id) = &self.inner.borrow().id {
            return id.to_string();
        }

        let socket = self.inner.borrow_mut().socket.clone();

        loop {
            if socket.ready_state() == WebSocket::OPEN {
                break;
            }

            sleep(1).await;
        }

        let message = serde_json::to_string(&ClientMessage::GetMyID).unwrap();
        socket.send_with_str(&message).unwrap();

        loop {
            self.update_signaling().await;

            if let Some(id) = &self.inner.borrow().id {
                return id.to_string();
            }

            sleep(1).await;
        }
    }

    async fn send_offer(&self, connection: RtcPeerConnection, peer_id: String) {
        let create_offer_promise = connection.create_offer();
        let offer = JsFuture::from(create_offer_promise).await.unwrap();

        let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))
            .unwrap()
            .as_string()
            .unwrap();

        let mut session_description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        session_description.sdp(&offer_sdp);

        let set_local_description = connection.set_local_description(&session_description);
        JsFuture::from(set_local_description).await.unwrap();

        loop {
            if connection.ice_gathering_state() == RtcIceGatheringState::Complete {
                break;
            }

            sleep(1).await;
        }

        let message = serde_json::to_string(&ClientMessage::Offer(ClientOffer {
            to: peer_id,
            sdp: connection.local_description().unwrap().sdp(),
        }))
        .unwrap();

        self.inner
            .borrow_mut()
            .socket
            .send_with_str(&message)
            .unwrap();
    }

    pub async fn connect(&mut self, peer_id: &str) {
        let connection = self.create_connection();
        let channel = connection.create_data_channel("channel");
        setup_channel(Rc::clone(&self.inner), channel, peer_id.to_string());

        self.send_offer(connection.clone(), peer_id.to_string().clone())
            .await;

        self.inner
            .borrow_mut()
            .connections
            .insert(peer_id.to_string(), connection);
    }
}

fn setup_channel(p2p_inner: Rc<RefCell<P2PInner>>, channel: RtcDataChannel, peer_id: String) {
    // Channel on open
    let inner_on_open = Rc::clone(&p2p_inner);
    let on_open_peer_id = peer_id.to_string();

    let on_open = Closure::<dyn FnMut()>::new(move || {
        inner_on_open
            .borrow_mut()
            .connection_states
            .push(ConnectionUpdate::Connected(on_open_peer_id.clone()));
    });

    // Channel on close
    let inner_on_close = Rc::clone(&p2p_inner);
    let on_close_peer_id = peer_id.to_string();

    let on_close = Closure::<dyn FnMut()>::new(move || {
        inner_on_close
            .borrow_mut()
            .connection_states
            .push(ConnectionUpdate::Disconnected(on_close_peer_id.clone()));
    });

    // Channel on message
    let inner_on_message = Rc::clone(&p2p_inner);
    let on_message_peer_id = peer_id.to_string();

    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |message: MessageEvent| {
        inner_on_message.borrow_mut().peer_messages.push((
            on_message_peer_id.clone(),
            message.data().as_string().unwrap(),
        ));
    });

    // Adds event listeners to channel
    channel.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    channel.set_onclose(Some(on_close.as_ref().unchecked_ref()));
    channel.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

    on_message.forget();
    on_open.forget();
    on_close.forget();

    p2p_inner
        .borrow_mut()
        .channels
        .insert(peer_id.clone(), channel.clone());
}
