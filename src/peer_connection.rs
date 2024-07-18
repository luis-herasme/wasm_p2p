use crate::{ice_server::IceServer, utils::sleep};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    js_sys::Reflect, RtcConfiguration, RtcDataChannel, RtcDataChannelEvent, RtcDataChannelState,
    RtcIceGatheringState, RtcPeerConnection as OriginalRtcPeerConnection, RtcSdpType,
    RtcSessionDescriptionInit,
};

/// This is a thin wrapper around RtcPeerConnection,
/// with the goal of making it more ergonomic.
pub struct RtcPeerConnection {
    connection: OriginalRtcPeerConnection,
}

impl RtcPeerConnection {
    pub fn new(ice_servers: &Vec<IceServer>) -> RtcPeerConnection {
        let mut config = RtcConfiguration::new();
        let config = config.ice_servers(&serde_wasm_bindgen::to_value(ice_servers).unwrap());
        let connection = OriginalRtcPeerConnection::new_with_configuration(&config).unwrap();

        RtcPeerConnection { connection }
    }

    pub fn create_data_channel(&self, label: &str) -> RtcDataChannel {
        self.connection.create_data_channel(label)
    }

    pub async fn create_local_offer(&self) -> Option<String> {
        let offer = self.create_offer().await;
        self.set_local_description(offer).await;
        self.wait_ice_gathering_complete().await;
        let local_description = self.connection.local_description()?;
        return Some(local_description.sdp());
    }

    pub async fn create_local_answer(&self) -> Option<String> {
        let answer = self.create_answer().await;
        self.set_local_description(answer).await;
        self.wait_ice_gathering_complete().await;
        let local_description = self.connection.local_description()?;
        return Some(local_description.sdp());
    }

    pub async fn set_remote_answer(&self, sdp: String) {
        let mut session_description = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        session_description.sdp(&sdp);
        self.set_remote_description(session_description).await;
    }

    pub async fn set_remote_offer(&self, sdp: String) {
        let mut session_description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        session_description.sdp(&sdp);
        self.set_remote_description(session_description).await;
    }

    pub async fn create_offer(&self) -> RtcSessionDescriptionInit {
        let create_sdp_promise = self.connection.create_offer();
        let js_value = JsFuture::from(create_sdp_promise).await.unwrap();

        let sdp = &Reflect::get(&js_value, &JsValue::from_str("sdp"))
            .unwrap()
            .as_string()
            .unwrap();

        let mut session_description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        session_description.sdp(sdp);
        return session_description;
    }

    pub async fn create_answer(&self) -> RtcSessionDescriptionInit {
        let create_sdp_promise = self.connection.create_answer();
        let js_value = JsFuture::from(create_sdp_promise).await.unwrap();

        let sdp = &Reflect::get(&js_value, &JsValue::from_str("sdp"))
            .unwrap()
            .as_string()
            .unwrap();

        let mut session_description = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        session_description.sdp(sdp);
        return session_description;
    }

    pub async fn set_remote_description(&self, session_description: RtcSessionDescriptionInit) {
        JsFuture::from(self.connection.set_remote_description(&session_description))
            .await
            .unwrap();
    }

    pub async fn set_local_description(&self, session_description: RtcSessionDescriptionInit) {
        JsFuture::from(self.connection.set_local_description(&session_description))
            .await
            .unwrap();
    }

    pub async fn wait_ice_gathering_complete(&self) {
        loop {
            if self.connection.ice_gathering_state() == RtcIceGatheringState::Complete {
                break;
            }

            sleep(0).await;
        }
    }

    pub async fn on_channel(&self) -> RtcDataChannel {
        let channel: Rc<RefCell<Option<RtcDataChannel>>> = Rc::new(RefCell::new(None));
        let callback_channel = Rc::clone(&channel);

        let callback =
            Closure::<dyn FnMut(RtcDataChannelEvent)>::new(move |event: RtcDataChannelEvent| {
                *callback_channel.borrow_mut() = Some(event.channel());
            });

        let callback_option = Some(callback.as_ref().unchecked_ref());
        self.connection.set_ondatachannel(callback_option);
        callback.forget();

        loop {
            if let Some(channel) = channel.borrow_mut().take() {
                return channel;
            }

            sleep(0).await;
        }
    }
}

pub async fn wait_channel_open(channel: &RtcDataChannel) {
    loop {
        if channel.ready_state() == RtcDataChannelState::Open {
            return;
        }

        sleep(0).await;
    }
}
