use crate::messages::{ClientMessage, ServerAnswer, ServerMessage, ServerOffer, ID};
use futures_util::lock::Mutex;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

pub struct Sockets {
    sockets:
        Arc<Mutex<HashMap<String, Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>>>>,
}

impl Sockets {
    pub fn new() -> Arc<Mutex<Sockets>> {
        Arc::new(Mutex::new(Self {
            sockets: Arc::new(Mutex::new(HashMap::new())),
        }))
    }

    async fn add(
        &self,
        stream: WebSocketStream<TcpStream>,
    ) -> (SplitStream<WebSocketStream<TcpStream>>, String) {
        let id = uuid::Uuid::new_v4().to_string();
        let (write, read) = stream.split();
        let write = Arc::new(Mutex::new(write));
        self.sockets.lock().await.insert(id.clone(), write);
        return (read, id);
    }

    async fn send(&self, to: &str, msg: Message) {
        let sockets = self.sockets.lock().await;
        let destination_option = sockets.get(to);

        if let Some(destination) = destination_option {
            let mut destination_socket = destination.lock().await;
            destination_socket.send(msg).await.unwrap();
        }
    }
}

pub async fn init<A>(addr: A)
where
    A: ToSocketAddrs,
{
    let listener = TcpListener::bind(&addr).await.unwrap();
    let sockets = Sockets::new();

    while let Ok((stream, _)) = listener.accept().await {
        let sockets = Arc::clone(&sockets);
        tokio::spawn(handle_connection(sockets, stream));
    }
}

async fn handle_connection(sockets: Arc<Mutex<Sockets>>, stream: TcpStream) {
    let stream = tokio_tungstenite::accept_async(stream).await.unwrap();

    let (mut read, id) = {
        let sockets = sockets.lock().await;
        sockets.add(stream).await
    };

    while let Some(Ok(message)) = read.next().await {
        let sockets = Arc::clone(&sockets);
        handle_msg(message.to_string(), id.clone(), sockets).await;
    }
}

async fn handle_msg(msg: String, socket_id: String, sockets: Arc<Mutex<Sockets>>) {
    if let Ok(value) = serde_json::from_str::<ClientMessage>(&msg) {

        match value {
            ClientMessage::Answer(answer) => {
                let destination_id = answer.to.clone();

                let message = ServerMessage::Answer(ServerAnswer {
                    from: socket_id,
                    to: answer.to,
                    sdp: answer.sdp,
                });

                let message = Message::from(serde_json::to_string(&message).unwrap());
                sockets.lock().await.send(&destination_id, message).await;
            }
            ClientMessage::Offer(offer) => {
                let detination_id = offer.to.clone();

                let message = ServerMessage::Offer(ServerOffer {
                    from: socket_id,
                    to: offer.to,
                    sdp: offer.sdp,
                });

                let message = Message::from(serde_json::to_string(&message).unwrap());
                sockets.lock().await.send(&detination_id, message).await;
            }
            ClientMessage::GetMyID => {
                let message = ServerMessage::ID(ID {
                    id: socket_id.clone(),
                });

                let message =
                    Message::from(serde_json::to_string::<ServerMessage>(&message).unwrap());

                sockets.lock().await.send(&socket_id, message).await;
            }
        };
    }
}
