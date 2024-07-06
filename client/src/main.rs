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
    let mut p2p = P2P::new("ws://127.0.0.1:9001");

    // let id = p2p.id().await;
    // console_log!("Your peer id: {}", id);

    // let answer = confirm("Do you want to send a connection request?")
    //     .as_bool()
    //     .unwrap();

    // if answer {
    //     let peer_id = prompt("What is the ID of the peer you want to connect to?")
    //         .as_string()
    //         .unwrap();

    //     p2p.connect(&peer_id).await;
    // }

    // loop {
    //     let (messages, connections) = p2p.update().await;

    //     for connection in connections {
    //         match connection {
    //             ConnectionUpdate::Connected(peer_id) => {
    //                 console_log!("Peer {} connected.", peer_id);
    //             }
    //             ConnectionUpdate::Disconnected(peer_id) => {
    //                 console_log!("Peer {} disconnected.", peer_id);
    //             }
    //         }
    //     }

    //     for (peer_id, message) in messages {
    //         console_log!("Peer {} says {}", peer_id, message);
    //     }

    //     delay(1).await;
    // }
}

// fn main() {
//     console_error_panic_hook::set_once();
//     let websocket = WebSocket::new("ws://127.0.0.1:9001").unwrap();

//     let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |message: MessageEvent| {
//         let data = message.data();
//         log(&format!("Data: {:?}", data));
//     });

//     let ws = websocket.clone();
//     let on_open = Closure::<dyn FnMut()>::new(move || {
//         log("Connected");
//         ws.send_with_str("Test").unwrap();
//     });

//     let connection = RtcPeerConnection::new().unwrap();

//     let on_change_conn = connection.clone();

//     let on_conn_change = Closure::<dyn FnMut()>::new(move || {
//         let ice_gathering_state = on_change_conn.ice_gathering_state();
//         if ice_gathering_state == RtcIceGatheringState::Complete {}
//     });

//     connection.set_onicegatheringstatechange(Some(on_conn_change.as_ref().unchecked_ref()));
//     on_conn_change.forget();

//     websocket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
//     on_open.forget();

//     websocket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
//     on_message.forget();

//     let document = window()
//         .and_then(|win| win.document())
//         .expect("Could not access the document");

//     let body = document.body().expect("Could not access document.body");
//     let text_node = document.create_text_node("Hello, world from Vanilla Rust!");
//     body.append_child(text_node.as_ref())
//         .expect("Failed to append text");
// }
