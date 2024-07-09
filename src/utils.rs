use wasm_bindgen_futures::JsFuture;
use web_sys::{
    js_sys::{Function, Promise},
    window,
};

pub async fn sleep(ms: i32) {
    let mut callback = |resolve: Function, _| {
        window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    };

    let promise = Promise::new(&mut callback);
    JsFuture::from(promise).await.unwrap();
}
