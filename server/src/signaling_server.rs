use crate::messages::{ClienMessage, ServerAnswer, ServerMessage, ServerOffer, ID};
use core::time;
use std::net::{TcpListener, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread::{self, spawn};
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
        let id = uuid::Uuid::new_v4().to_string();
        let websocket = Arc::new(Mutex::new(websocket));

        let mut sockets = self.sockets.lock().unwrap();
        sockets.insert(id.clone(), Arc::clone(&websocket));

        return (websocket, id);
    }

    fn send(&self, to: &str, msg: Message) {
        let sockets = self.sockets.lock().unwrap();
        let destination_option = sockets.get(to);

        println!("destination_option: {:?}", destination_option);

        if let Some(destination) = destination_option {
            println!("destination: {:?}", destination);
            let mut destination_socket = destination.lock().unwrap();
            println!("Sending message: {}", msg);
            destination_socket.send(msg).unwrap();
        }
    }

    fn handle_msg(&self, msg: String, socket_id: String) {
        if let Ok(value) = serde_json::from_str::<ClienMessage>(&msg) {
            println!("Parsed messaged: {:?}", value);
            match value {
                ClienMessage::Answer(answer) => {
                    let destination_id = answer.to.clone();

                    let message = ServerMessage::Answer(ServerAnswer {
                        from: socket_id,
                        to: answer.to,
                        sdp: answer.sdp,
                    });

                    let message = Message::from(serde_json::to_string(&message).unwrap());
                    self.send(&destination_id, message);
                }
                ClienMessage::Offer(offer) => {
                    let detination_id = offer.to.clone();

                    println!("Offer destination id: {}", detination_id);

                    let message = ServerMessage::Offer(ServerOffer {
                        from: socket_id,
                        to: offer.to,
                        sdp: offer.sdp,
                    });

                    let message = Message::from(serde_json::to_string(&message).unwrap());
                    self.send(&detination_id, message);
                }
                ClienMessage::GetMyID => {
                    let message = ServerMessage::ID(ID {
                        id: socket_id.clone(),
                    });

                    let message =
                        Message::from(serde_json::to_string::<ServerMessage>(&message).unwrap());

                    self.send(&socket_id, message);
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
                println!("Creating new thread!");
                let websocket = accept(stream.unwrap()).unwrap();
                let (websocket, socket_id) = socket_manager.add(websocket);

                loop {
                    println!("Here 1");

                    let msg = {
                        println!("Here 2");
                        let mut websocket = websocket.lock().unwrap();
                        println!("Here 3");
                        let msg = websocket.read().unwrap();
                        println!("Here 4");
                        msg.to_string()
                    };

                    println!("Here 5");

                    println!("Received message: {}", msg);
                    socket_manager.handle_msg(msg, socket_id.clone());

                    println!("Sleeping start");
                    thread::sleep(time::Duration::from_millis(100));
                    println!("Sleeping end");
                }
            });
        }
    }
}
