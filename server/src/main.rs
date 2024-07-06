use signaling_server::SignalingServer;

mod messages;
mod signaling_server;

fn main() {
    let socket_manager = SignalingServer::new();
    socket_manager.init("127.0.0.1:9001");
}
