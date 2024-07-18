use std::vec::IntoIter;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, RtcDataChannel};

pub struct P2PConnection {
    id: String,
    channel: RtcDataChannel,
    messages: Rc<RefCell<Vec<String>>>,
}

impl P2PConnection {
    pub(crate) fn new(id: String, channel: RtcDataChannel) -> P2PConnection {
        let connection = P2PConnection {
            id,
            channel,
            messages: Rc::new(RefCell::new(Vec::new())),
        };

        connection.listen_to_channel_messages();

        return connection;
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn send(&self, data: &str) -> Result<(), JsValue> {
        self.channel.send_with_str(data)
    }

    pub fn receive(&self) -> IntoIter<String> {
        return std::mem::replace(&mut *self.messages.borrow_mut(), Vec::new()).into_iter();
    }

    fn listen_to_channel_messages(&self) {
        let messages = Rc::clone(&self.messages);
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |message: MessageEvent| {
            if let Some(message) = message.data().as_string() {
                messages.borrow_mut().push(message);
            }
        });

        self.channel
            .set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();
    }
}
