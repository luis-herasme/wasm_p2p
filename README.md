<div align="center">
    <h1>WASM P2P</h1>
    <big>Simple peer-to-peer library for Rust + WASM, built on top of WebRTC</big>
    <div>
    <br/>
        <a href="https://github.com/luis-herasme/wasm_p2p/pulse"><img src="https://img.shields.io/github/last-commit/luis-herasme/wasm_p2p.svg"/></a>
        <a href="https://github.com/luis-herasme/wasm_p2p/pulls"><img src="https://img.shields.io/github/issues-pr/luis-herasme/wasm_p2p.svg"/></a>
        <a href="https://github.com/luis-herasme/wasm_p2p/issues"><img src="https://img.shields.io/github/issues-closed/luis-herasme/wasm_p2p.svg"/></a>
    </div>
</div>
<br/>
</div>

## Introduction
This is a simple peer-to-peer library for Rust + WASM, built on top of WebRTC. In the following example, we will connect to another peer and send it `Hello world`:
```Rust
use wasm_p2p::{wasm_bindgen_futures, P2P};

fn main() {
    wasm_bindgen_futures::spawn_local(main_async());
}

async fn main_async() {
    let p2p = P2P::new("wss://signaling.luisherasme.com").await;
    let peer = p2p.connect("other-peer-id").await;
    peer.send("Hello world");
}
```
## Installation

```bash
cargo add wasm_p2p
```

## Usage

### Setup
To establish a peer-to-peer connection, we need to send information like our IP address to the other peer so that it knows how to reach us. The server that we use to exchange this information is called a signaling server.

To initialize the `P2P` client, you need to pass the URL of the signaling server:
```Rust
let mut p2p = P2P::new("wss://signaling.luisherasme.com").await;
```

In the previous example, we used `wss://signaling.luisherasme.com` as the signaling server. This server is free, and the code is open source so that you can create your own version. The code is available [here](https://github.com/luis-herasme/signaling-server.rs).

#### Peer ID
The signaling server assigns a random, unique ID to each peer:
```Rust
let id = p2p.id;
```
#### Receive meesages
To receive messages from the other peers that are connected to you, you can call the update method:
```Rust
let (messages, connections) = p2p.update().await;
```

#### Send message
To send a message to another peer you have to use the `send` method:
```Rust
let data = "EXAMPLE DATA YOU CAN SEND ANY &STR";
peer.send(data);
```

#### Connections
You can start a connection by calling `p2p.connect` with the peer ID of the destination peer.
```Rust
let peer = p2p.connect("OTHER_PEER_ID").await;
```
Inspect the connection array received in the update function to check for a new peer connection.
```Rust
let (messages, connections) = p2p.update().await;

for connection in connections {
    match connection {
        ConnectionUpdate::Connected(peer_id) => {
            println!("Peer {} connected", peer_id);
        }
        ConnectionUpdate::Disconnected(peer_id) => {
            println!("Peer {} disconnected", peer_id);
        }
    }
}
```

#### Custom ICE Servers
You can set your own ICE servers:

```Rust
use wasm_p2p::{wasm_bindgen_futures, ConnectionUpdate, P2P, IceServer};

fn main() {
    wasm_bindgen_futures::spawn_local(main_async());
}

async fn main_async() {
    let mut p2p = P2P::new("wss://signaling.luisherasme.com");

    let ice_servers = vec![IceServer {
        urls: String::from("stun:stun.l.google.com:19302"),
        credential: None,
        credential_type: None,
        username: None,
    }];

    p2p.set_ice_servers(ice_servers);
}
```

Furthermore, you can create an ICE server from a `&str`:

```Rust
use wasm_p2p::{wasm_bindgen_futures, ConnectionUpdate, P2P, IceServer};

fn main() {
    wasm_bindgen_futures::spawn_local(main_async());
}

async fn main_async() {
    let mut p2p = P2P::new("wss://signaling.luisherasme.com");
    let ice_servers = vec![IceServer::from("stun:stun.l.google.com:19302")];
    p2p.set_ice_servers(ice_servers);
}
```
## Examples
- P2P chat. [Demo](https://p2pexample.luisherasme.com/), [Repository](https://github.com/luis-herasme/p2p-example).
