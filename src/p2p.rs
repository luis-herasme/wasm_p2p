use crate::p2p_connection::P2PConnection;
use crate::peer_connection::RtcPeerConnection;
use crate::signaling::Signaling;
use crate::{ice_server::IceServer, peer_connection::wait_channel_open};

pub struct P2P {
    signaling: Signaling,
    ice_servers: Vec<IceServer>,
}

impl P2P {
    pub async fn new(url: &str) -> P2P {
        let signaling = Signaling::new(url).await;

        let p2p = P2P {
            signaling,
            ice_servers: vec![IceServer::from("stun:stun.l.google.com:19302")],
        };

        return p2p;
    }

    pub fn id(&self) -> String {
        self.signaling.id()
    }

    pub async fn connect(&mut self, peer_id: &str) -> Option<P2PConnection> {
        // 1. Create Connection
        let connection = RtcPeerConnection::new(&self.ice_servers);

        // 2. Create channel
        let channel = connection.create_data_channel("channel");

        // 3. Create local SDP (offer)
        let local_sdp = connection.create_local_offer().await?;

        // 4. Send SDP (offer) to the other peer
        self.signaling.send_offer(peer_id, &local_sdp);

        // 5. Waif for the SDP (answer) from the other peer
        let remote_sdp = self.signaling.receive_sdp_from(peer_id).await;

        // 6. Set the remote SDP (answer) to the other peer SDP
        connection.set_remote_answer(remote_sdp).await;

        // 7. Wait for channel open
        wait_channel_open(&channel).await;

        return Some(P2PConnection::new(peer_id.to_string(), channel));
    }

    pub async fn receive_connection(&mut self) -> Option<P2PConnection> {
        // 1. Receive SDP (offer) from other peer
        let offer = self.signaling.receive_offer().await?;

        // 2. Create Connection
        let connection = RtcPeerConnection::new(&self.ice_servers);

        // 3. Set the remote SDP to the other peer SDP (offer)
        connection.set_remote_offer(offer.sdp).await;

        // 4. Create local SDP (answer)
        let local_sdp = connection.create_local_answer().await?;

        // 5. Send the SDP (answer) to the other peer
        self.signaling.send_answer(&offer.from, &local_sdp);

        // 6. Wait for the channel
        let channel = connection.on_channel().await;

        // 7. Wait for channel open
        wait_channel_open(&channel).await;

        return Some(P2PConnection::new(offer.from, channel));
    }
}
