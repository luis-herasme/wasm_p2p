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
This is a simple peer-to-peer library for Rust + WASM, built on top of WebRTC.

## Installation

```bash
cargo add wasm_p2p
```

## Usage
```Rust
use wasm_p2p::{wasm_bindgen_futures, ConnectionUpdate, P2P};

fn main() {
    wasm_bindgen_futures::spawn_local(init());
}

async fn init() {
    let mut p2p = P2P::new("wss://signaling.luisherasme.com");

    let id = p2p.id().await;
    println!("Your id is {}", id);

    loop {
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

        for (peer_id, message) in messages {
            println!("{}: {}", peer_id, message);
        }
    }
}
```
