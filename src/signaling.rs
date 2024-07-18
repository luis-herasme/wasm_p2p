use crate::{
    messages::{
        ClientAnswer, ClientMessage, ClientOffer, ServerAnswer, ServerMessage, ServerOffer,
    },
    utils::sleep,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

pub struct Signaling {
    id: String,
    socket: WebSocket,
    answers: Rc<RefCell<HashMap<String, ServerAnswer>>>,
    offers: Rc<RefCell<Vec<ServerOffer>>>,
}

impl Signaling {
    pub async fn new(url: &str) -> Signaling {
        let socket = Signaling::create_socket(url).await;
        let id = Signaling::get_signaling_id(&socket).await;

        let signaling = Signaling {
            id,
            socket,
            answers: Rc::new(RefCell::new(HashMap::new())),
            offers: Rc::new(RefCell::new(Vec::new())),
        };

        signaling.listen();

        return signaling;
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn send_offer(&mut self, peer_id: &str, sdp: &str) {
        self.send(ClientMessage::Offer(ClientOffer {
            to: peer_id.to_string(),
            sdp: sdp.to_string(),
        }));
    }

    pub fn send_answer(&mut self, peer_id: &str, sdp: &str) {
        self.send(ClientMessage::Answer(ClientAnswer {
            to: peer_id.to_string(),
            sdp: sdp.to_string(),
        }));
    }

    pub async fn receive_answer_from(&mut self, peer_id: &str) -> String {
        loop {
            if let Some(answer) = self.answers.borrow_mut().remove(peer_id) {
                return answer.sdp;
            }

            sleep(0).await;
        }
    }

    pub async fn receive_offer(&self) -> Option<ServerOffer> {
        self.offers.borrow_mut().pop()
    }

    fn send(&self, message: ClientMessage) {
        let json = serde_json::to_string(&message).unwrap();
        self.socket.send_with_str(&json).unwrap();
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

    fn listen(&self) {
        let offers = Rc::clone(&self.offers);
        let answers = Rc::clone(&self.answers);

        let callback = Closure::<dyn FnMut(MessageEvent)>::new(move |message: MessageEvent| {
            let message = message.data().as_string().unwrap();

            if let Ok(message) = serde_json::from_str::<ServerMessage>(&message) {
                match message {
                    ServerMessage::Offer(offer) => {
                        offers.borrow_mut().push(offer);
                    }
                    ServerMessage::Answer(answer) => {
                        answers.borrow_mut().insert(answer.from.clone(), answer);
                    }
                    ServerMessage::ID(_) => {}
                }
            }
        });

        let callback_unchecked = Some(callback.as_ref().unchecked_ref());
        self.socket.set_onmessage(callback_unchecked);
        callback.forget();
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
