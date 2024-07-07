mod messages;
mod signaling_server;

#[tokio::main]
async fn main() {
    signaling_server::init("127.0.0.1:9001").await;
}
