use wasm_bindgen::JsValue;

use crate::messages::ServerOffer;
use crate::p2p_connection::P2PConnection;
use crate::peer_connection::{RtcPeerConnection, SDP};
use crate::signaling::Signaling;
use crate::{ice_server::IceServer, peer_connection::wait_channel_open};

pub struct P2P {
    signaling: Signaling,
    ice_servers: Vec<IceServer>,
}

impl P2P {
    pub async fn new(url: &str) -> Result<P2P, JsValue> {
        let p2p = P2P {
            signaling: Signaling::new(url).await?,
            ice_servers: vec![IceServer::from("stun:stun.l.google.com:19302")],
        };

        return Ok(p2p);
    }

    pub fn id(&self) -> String {
        self.signaling.id()
    }

    pub async fn connect(&mut self, peer_id: &str) -> Result<P2PConnection, JsValue> {
        // 1. Create Connection
        let connection = RtcPeerConnection::new(&self.ice_servers)?;

        // 2. Create channel
        let channel = connection.create_data_channel("channel");

        // 3. Create SDP (offer)
        let sdp = connection.create_sdp(SDP::Offer).await?;

        // 4. Set SDP to local
        let sdp = connection.set_local_sdp(sdp).await?;

        // 5. Send SDP (offer) to the other peer
        self.signaling.send_offer(peer_id, &sdp)?;

        // 6. Waif for the SDP (answer) from the other peer
        let remote_sdp = self.signaling.receive_answer_from(peer_id).await;

        // 7. Set the remote SDP (answer) to the other peer SDP
        connection.set_remote_sdp(remote_sdp, SDP::Answer).await?;

        // 8. Wait for channel open
        wait_channel_open(&channel).await;

        return Ok(P2PConnection::new(peer_id.to_string(), channel));
    }

    pub fn receive_offer(&mut self) -> Option<ServerOffer> {
        // 1. Receive SDP (offer) from other peer
        return self.signaling.receive_offer();
    }

    pub async fn create_connection(
        &mut self,
        offer: ServerOffer,
    ) -> Result<P2PConnection, JsValue> {
        // 2. Create Connection
        let connection = RtcPeerConnection::new(&self.ice_servers)?;

        // 3. Set the remote SDP to the other peer SDP (offer)
        connection.set_remote_sdp(offer.sdp, SDP::Offer).await?;

        // 4. Create local SDP (answer)
        let sdp = connection.create_sdp(SDP::Answer).await?;

        // 5. Set SDP to local
        let sdp = connection.set_local_sdp(sdp).await?;

        // 6. Send the SDP (answer) to the other peer
        self.signaling.send_answer(&offer.from, &sdp)?;

        // 7. Wait for the channel
        let channel = connection.on_channel().await;

        // 8. Wait for channel open
        wait_channel_open(&channel).await;

        return Ok(P2PConnection::new(offer.from, channel));
    }
}
