use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(value: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($expressions:expr),+) => {
        $crate::console_log::log(&format!($($expressions),+));
    };
}
