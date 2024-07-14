use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use crate::{
    messages::{ClientMessage, ServerMessage},
    utils::sleep,
};

pub struct Signaling {
    id: String,
    pub socket: WebSocket,
}

impl Signaling {
    pub async fn new(url: &str) -> Signaling {
        let socket = Signaling::create_socket(url).await;
        let id = Signaling::get_signaling_id(&socket).await;
        Signaling { id, socket }
    }

    pub fn id(&self) -> String {
        self.id.clone()
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

    async fn get_signaling_id(socket: &WebSocket) -> String {
        let get_my_id = serde_json::to_string(&ClientMessage::GetMyID).unwrap();
        socket.send_with_str(&get_my_id).unwrap();

        let id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let id_callback = Rc::clone(&id);

        let callback = Closure::<dyn FnMut(MessageEvent)>::new(move |message: MessageEvent| {
            let message = message.data().as_string().unwrap();
            if let Ok(message) = serde_json::from_str::<ServerMessage>(&message) {
                if let ServerMessage::ID(data) = message {
                    (*id_callback.borrow_mut()) = Some(data.id);
                }
            }
        });

        socket.set_onmessage(Some(callback.as_ref().unchecked_ref()));
        callback.forget();

        loop {
            if let Some(id) = id.take() {
                return id;
            }

            sleep(0).await
        }
    }
}
