use crate::messages::{ClienMessage, ServerAnswer, ServerOffer, ID};
use std::net::{TcpListener, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::{collections::HashMap, net::TcpStream};
use tungstenite::{accept, Message, WebSocket};

#[derive(Clone)]
pub struct SignalingServer {
    sockets: Arc<Mutex<HashMap<String, Arc<Mutex<WebSocket<TcpStream>>>>>>,
}

impl SignalingServer {
    pub fn new() -> Self {
        Self {
            sockets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn add(&self, websocket: WebSocket<TcpStream>) -> (Arc<Mutex<WebSocket<TcpStream>>>, String) {
        let mut sockets = self.sockets.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();

        let websocket = Arc::new(Mutex::new(websocket));
        sockets.insert(id.clone(), Arc::clone(&websocket));

        return (websocket, id);
    }

    fn send(&self, to: &str, msg: Message) {
        let sockets = self.sockets.lock().unwrap();
        let destination_option = sockets.get(to);

        if let Some(destination) = destination_option {
            let mut destination_socket = destination.lock().unwrap();
            destination_socket.send(msg).unwrap();
        }
    }

    fn handle_msg(&self, msg: String, socket_id: String) {
        if let Ok(value) = serde_json::from_str::<ClienMessage>(&msg) {
            match value {
                ClienMessage::Answer(answer) => {
                    let answer = ServerAnswer {
                        from: socket_id,
                        to: answer.to,
                        sdp: answer.sdp,
                        message_id: answer.message_id,
                    };

                    let msg = Message::from(serde_json::to_string(&answer).unwrap());
                    self.send(&answer.to, msg);
                }
                ClienMessage::Offer(offer) => {
                    let offer = ServerOffer {
                        from: socket_id,
                        to: offer.to,
                        sdp: offer.sdp,
                        message_id: offer.message_id,
                    };

                    let msg = Message::from(serde_json::to_string(&offer).unwrap());
                    self.send(&offer.to, msg);
                }
                ClienMessage::GetMyID => {
                    let message = ID {
                        id: socket_id.clone(),
                    };

                    let msg = Message::from(serde_json::to_string(&message).unwrap());
                    self.send(&socket_id, msg);
                }
            };
        }
    }

    pub fn init<A>(self, addr: A)
    where
        A: ToSocketAddrs,
    {
        let server = TcpListener::bind(addr).unwrap();

        for stream in server.incoming() {
            let socket_manager = self.clone();

            spawn(move || {
                let websocket = accept(stream.unwrap()).unwrap();
                let (websocket, socket_id) = socket_manager.add(websocket);

                loop {
                    let msg = {
                        let mut websocket = websocket.lock().unwrap();
                        let msg = websocket.read().unwrap();
                        msg.to_string()
                    };

                    socket_manager.handle_msg(msg, socket_id.clone());
                }
            });
        }
    }
}
