pub mod config;
pub mod game;
pub mod rendering;
pub mod communication;
#[feature(async_closure)]
use rendering::Render;

use quicksilver::{
    geom::{Rectangle, Vector},
    graphics::Color,
    run, Graphics, Input, Result as QsResult, Settings, Window,
};
use wasm_bindgen::prelude::*;
use web_sys::console;

use wasm_bindgen::JsCast;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    async fn app(window: Window, mut gfx: Graphics, mut input: Input) -> QsResult<()> {
        let mut game_state: game::Game = Default::default();
        let player_handle = 1;

        let ws = WebSocket::new(format!("ws://{}", config::BACKEND_ADDRESS).as_str())
            .expect("failed to connect to ws server");
        // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        // create callback
        let cloned_ws = ws.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            // Handle difference Text/Binary,...
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                console_log!("message event, received arraybuffer: {:?}", abuf);
                let array = js_sys::Uint8Array::new(&abuf);
                let len = array.byte_length() as usize;
                console_log!("Arraybuffer received {}bytes: {:?}", len, array.to_vec());
                // here you can for example use Serde Deserialize decode the message
                // for demo purposes we switch back to Blob-type and send off another binary message
                cloned_ws.set_binary_type(web_sys::BinaryType::Blob);
                match cloned_ws.send_with_u8_array(&vec![5, 6, 7, 8]) {
                    Ok(_) => console_log!("binary message successfully sent"),
                    Err(err) => console_log!("error sending message: {:?}", err),
                }
            } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                console_log!("message event, received blob: {:?}", blob);
                // better alternative to juggling with FileReader is to use https://crates.io/crates/gloo-file
                let fr = web_sys::FileReader::new().unwrap();
                let fr_c = fr.clone();
                // create onLoadEnd callback
                let onloadend_cb = Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
                    let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
                    let len = array.byte_length() as usize;
                    console_log!("Blob received {}bytes: {:?}", len, array.to_vec());
                    // here you can for example use the received image/png data
                })
                    as Box<dyn FnMut(web_sys::ProgressEvent)>);
                fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
                fr.read_as_array_buffer(&blob).expect("blob not readable");
                onloadend_cb.forget();
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                console_log!("message event, received Text: {:?}", txt);
            } else {
                console_log!("message event, received Unknown: {:?}", e.data());
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        // set message event handler on WebSocket
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // forget the callback to keep it alive
        onmessage_callback.forget();

        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            console_log!("error event: {:?}", e);
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        let cloned_ws = ws.clone();
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            console_log!("socket opened");
            match cloned_ws.send_with_str("ping") {
                Ok(_) => console_log!("message successfully sent"),
                Err(err) => console_log!("error sending message: {:?}", err),
            }
            // send off binary message
        }) as Box<dyn FnMut(JsValue)>);
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        // Your code goes here!
        console::log_1(&JsValue::from_str("Hello world!"));

        // Clear the screen to a blank, white color
        game_state.add(player_handle);
        loop {
            while let Some(_) = input.next_event().await {}
            game_state.handle_quicksilver_input(&mut input, player_handle);
            game_state.step();
            gfx.clear(Color::WHITE);
            // Paint a blue square with a red outline in the center of our screen
            // It should have a top-left of (350, 100) and a size of (150, 100)
            game_state.render(&mut gfx);
            // Send the data to be drawn
            gfx.present(&window)?;
        }
    }

    run(
        Settings {
            title: "Square Example",
            ..Settings::default()
        },
        app,
    );
    Ok(())
}
