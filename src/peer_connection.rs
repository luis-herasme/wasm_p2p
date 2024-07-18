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

pub enum SDP {
    Offer,
    Answer,
}

impl RtcPeerConnection {
    pub fn new(ice_servers: &Vec<IceServer>) -> Result<RtcPeerConnection, JsValue> {
        let mut config = RtcConfiguration::new();
        let config = config.ice_servers(&serde_wasm_bindgen::to_value(ice_servers)?);
        let connection = OriginalRtcPeerConnection::new_with_configuration(&config)?;
        Ok(RtcPeerConnection { connection })
    }

    pub fn create_data_channel(&self, label: &str) -> RtcDataChannel {
        self.connection.create_data_channel(label)
    }

    pub async fn set_local_sdp(&self, sdp: RtcSessionDescriptionInit) -> Result<String, JsValue> {
        self.set_local_description(sdp).await?;
        self.wait_ice_gathering_complete().await;
        let local_description = self.connection.local_description().unwrap();
        return Ok(local_description.sdp());
    }

    pub async fn set_remote_sdp(&self, sdp: String, sdp_type: SDP) -> Result<JsValue, JsValue> {
        let sdp_type = match sdp_type {
            SDP::Answer => RtcSdpType::Answer,
            SDP::Offer => RtcSdpType::Offer,
        };

        let mut session_description = RtcSessionDescriptionInit::new(sdp_type);
        session_description.sdp(&sdp);
        return self.set_remote_description(session_description).await;
    }

    pub async fn create_sdp(&self, sdp_type: SDP) -> Result<RtcSessionDescriptionInit, JsValue> {
        let create_sdp_promise = match sdp_type {
            SDP::Answer => self.connection.create_answer(),
            SDP::Offer => self.connection.create_offer(),
        };

        let js_value = JsFuture::from(create_sdp_promise).await?;

        let sdp = &Reflect::get(&js_value, &JsValue::from_str("sdp"))?
            .as_string()
            .unwrap();

        let sdp_type = match sdp_type {
            SDP::Answer => RtcSdpType::Answer,
            SDP::Offer => RtcSdpType::Offer,
        };

        let mut session_description = RtcSessionDescriptionInit::new(sdp_type);
        session_description.sdp(sdp);
        return Ok(session_description);
    }

    pub async fn set_remote_description(
        &self,
        session_description: RtcSessionDescriptionInit,
    ) -> Result<JsValue, JsValue> {
        return JsFuture::from(self.connection.set_remote_description(&session_description)).await;
    }

    pub async fn set_local_description(
        &self,
        session_description: RtcSessionDescriptionInit,
    ) -> Result<JsValue, JsValue> {
        return JsFuture::from(self.connection.set_local_description(&session_description)).await;
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
