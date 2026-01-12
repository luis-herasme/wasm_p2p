use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    js_sys::{self, Function, Promise},
    window, DedicatedWorkerGlobalScope,
};

pub async fn sleep(ms: i32) {
    let mut callback = |resolve: Function, _| {
        match window() {
            Some(window) => window
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
                .unwrap(),
            None => {
                let scope = DedicatedWorkerGlobalScope::from(JsValue::from(js_sys::global()));
                scope
                    .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
                    .unwrap()
            }
        };
    };

    let promise = Promise::new(&mut callback);
    JsFuture::from(promise).await.unwrap();
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(value: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($expressions:expr),+) => {
        $crate::utils::log(&format!($($expressions),+));
    };
}
