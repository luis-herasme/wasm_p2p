use p2p::{ConnectionUpdate, P2P};
use wasm_bindgen::prelude::*;

mod console_log;
mod messages;
mod p2p;

#[wasm_bindgen]
extern "C" {
    pub fn alert(value: &str);
    pub fn prompt(value: &str) -> JsValue;
    pub fn confirm(value: &str) -> JsValue;
}

#[wasm_bindgen(module = "/src/utils.js")]
extern "C" {
    pub async fn delay(delay_ms: u32);
}

fn main() {
    wasm_bindgen_futures::spawn_local(main_async());
}

async fn main_async() {
    console_error_panic_hook::set_once();
    let mut p2p = P2P::new("ws://127.0.0.1:9001");

    let id = p2p.id().await;
    console_log!("Your peer id: {}", id);

    let answer = confirm("Do youa wnt to send a connection request?")
        .as_bool()
        .unwrap();

    if answer {
        let peer_id = prompt("What is the ID of the peer you want to connect to?")
            .as_string()
            .unwrap();

        p2p.connect(&peer_id).await;
    }

    loop {
        let (messages, connections) = p2p.update().await;

        for connection in connections {
            match connection {
                ConnectionUpdate::Connected(peer_id) => {
                    console_log!("Peer {} connected.", peer_id);
                }
                ConnectionUpdate::Disconnected(peer_id) => {
                    console_log!("Peer {} disconnected.", peer_id);
                }
            }
        }

        for (peer_id, message) in messages {
            console_log!("Peer {} says {}", peer_id, message);
        }
    }
}
