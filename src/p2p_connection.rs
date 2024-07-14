use crate::{
    ice_server::IceServer,
    messages::{ClientAnswer, ClientMessage, ClientOffer},
    utils::sleep,
    P2P,
};
use std::vec::IntoIter;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    js_sys::Reflect, MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelEvent,
    RtcDataChannelState, RtcIceGatheringState, RtcPeerConnection, RtcSdpType,
    RtcSessionDescriptionInit,
};

#[derive(Clone)]
pub struct P2PConnection {
    pub id: String,
    connection: RtcPeerConnection,
    channel: Rc<RefCell<Option<RtcDataChannel>>>,
    messages: Rc<RefCell<Vec<String>>>,
    p2p: P2P,
}

pub enum SDP {
    Answer,
    Offer,
}

impl P2PConnection {
    pub fn new(peer_id: &str, p2p: P2P) -> Self {
        Self {
            id: peer_id.to_string(),
            connection: P2PConnection::create_connection(p2p.get_ice_servers()),
            channel: Rc::new(RefCell::new(None)),
            messages: Rc::new(RefCell::new(Vec::new())),
            p2p: p2p,
        }
    }

    pub async fn ready_state(&self) -> RtcDataChannelState {
        let channel_option = self.channel.borrow();

        if let Some(channel) = channel_option.as_ref() {
            channel.ready_state()
        } else {
            RtcDataChannelState::Closed
        }
    }

    pub async fn set_remote_sdp(&self, sdp: &str, sdp_type: SDP) {
        let sdp_type = match sdp_type {
            SDP::Answer => RtcSdpType::Answer,
            SDP::Offer => RtcSdpType::Offer,
        };

        let mut session_description = RtcSessionDescriptionInit::new(sdp_type);
        session_description.sdp(sdp);
        let set_remote_description = self.connection.set_remote_description(&session_description);
        JsFuture::from(set_remote_description).await.unwrap();
    }

    async fn create_local_sdp(&self, sdp_type: SDP) -> String {
        let create_sdp_promise = match sdp_type {
            SDP::Answer => self.connection.create_answer(),
            SDP::Offer => self.connection.create_offer(),
        };

        let sdp_js_value = JsFuture::from(create_sdp_promise).await.unwrap();
        let sdp = &Reflect::get(&sdp_js_value, &JsValue::from_str("sdp"))
            .unwrap()
            .as_string()
            .unwrap();

        let sdp_type = match sdp_type {
            SDP::Answer => RtcSdpType::Answer,
            SDP::Offer => RtcSdpType::Offer,
        };

        let mut session_description = RtcSessionDescriptionInit::new(sdp_type);
        session_description.sdp(sdp);

        JsFuture::from(self.connection.set_local_description(&session_description))
            .await
            .unwrap();

        loop {
            if self.connection.ice_gathering_state() == RtcIceGatheringState::Complete {
                break;
            }

            sleep(0).await;
        }

        return self.connection.local_description().unwrap().sdp();
    }

    fn create_connection(ice_servers: Vec<IceServer>) -> RtcPeerConnection {
        let mut config = RtcConfiguration::new();
        let config = config.ice_servers(&serde_wasm_bindgen::to_value(&ice_servers).unwrap());
        let connection = RtcPeerConnection::new_with_configuration(&config).unwrap();
        return connection;
    }

    pub async fn create_offer(&mut self) -> ClientMessage {
        let channel = self.connection.create_data_channel("channel");
        *self.channel.borrow_mut() = Some(channel.clone());
        let messages = Rc::clone(&self.messages);

        P2PConnection::listen_to_channel_messages(channel.clone(), messages.clone());

        let sdp = self.create_local_sdp(SDP::Offer).await;

        return ClientMessage::Offer(ClientOffer {
            to: self.id.clone(),
            sdp,
        });
    }

    pub async fn create_answer(&self) -> ClientMessage {
        let sdp = self.create_local_sdp(SDP::Answer).await;

        self.on_data_channel();

        return ClientMessage::Answer(ClientAnswer {
            to: self.id.clone(),
            sdp,
        });
    }

    fn listen_to_channel_messages(channel: RtcDataChannel, messages: Rc<RefCell<Vec<String>>>) {
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |message: MessageEvent| {
            let message = message.data().as_string().unwrap();
            messages.borrow_mut().push(message);
        });

        channel.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();
    }

    fn add_channel_to_new_channels_when_ready(peer_id: String, p2p: P2P, channel: RtcDataChannel) {
        let on_open = Closure::<dyn FnMut()>::new(move || {
            p2p.new_connection(&peer_id);
        });

        channel.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        on_open.forget();
    }

    pub fn send(&self, data: &str) {
        if let Some(channel) = self.channel.borrow().as_ref() {
            channel.send_with_str(data).unwrap();
        }
    }

    pub fn receive(&self) -> IntoIter<String> {
        return std::mem::replace(&mut *self.messages.borrow_mut(), Vec::new()).into_iter();
    }

    fn on_data_channel(&self) {
        let messages = Rc::clone(&self.messages);
        let self_channel = Rc::clone(&self.channel);

        let peer_id = self.id.clone();
        let p2p = self.p2p.clone();

        let callback =
            Closure::<dyn FnMut(RtcDataChannelEvent)>::new(move |event: RtcDataChannelEvent| {
                let channel = event.channel();
                (*self_channel.borrow_mut()) = Some(channel.clone());

                P2PConnection::listen_to_channel_messages(channel.clone(), messages.clone());
                P2PConnection::add_channel_to_new_channels_when_ready(
                    peer_id.clone(),
                    p2p.clone(),
                    channel,
                );
            });

        let callback_option = Some(callback.as_ref().unchecked_ref());
        self.connection.set_ondatachannel(callback_option);
        callback.forget();
    }
}
