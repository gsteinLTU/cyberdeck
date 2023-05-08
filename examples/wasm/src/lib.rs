use std::{collections::HashMap, cell::RefCell, rc::Rc};

use js_sys::{Reflect};
use wasm_bindgen::{prelude::{wasm_bindgen, Closure}, JsValue};
use web_sys::{RtcPeerConnection, RtcDataChannel, window };

extern crate console_error_panic_hook;
use std::panic;

thread_local! {
    static PEER_CONNECTION: RefCell<Option<Rc<RefCell<RtcPeerConnection>>>> = RefCell::new(None);
}

thread_local! {
    static DATA_CHANNELS: RefCell<HashMap<String, Rc<RefCell<RtcDataChannel>>>> = RefCell::new(HashMap::new());
}

#[wasm_bindgen(start)]
async fn main(){
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    run().await;
} 

#[wasm_bindgen]
pub fn log(text: &str){
    let logs = window().unwrap().document().unwrap().get_element_by_id("logs").expect("#logs not found");
    logs.set_inner_html(&(logs.inner_html().to_owned() + text + "<br>"));
}

#[wasm_bindgen]
pub async fn run() {
    let pc: Rc<RefCell<RtcPeerConnection>> = cyberdeck_client::create_peer_connection();
    let send_channel = cyberdeck_client::create_data_channel(pc.clone(), "foo");
    
    let onclose = Closure::<dyn Fn()>::new(|| {
        log("sendChannel has closed");
    });
    let onopen = Closure::<dyn Fn()>::new(|| {
        log("sendChannel has opened");
    });
    
    let send_channel_clone = send_channel.clone();
    let onmessage = Closure::<dyn Fn(JsValue)>::new(move |e: JsValue| {
        log(&format!("Message from DataChannel '{}' with payload '{}'", Reflect::get(&send_channel_clone.borrow(), &"label".into()).unwrap().as_string().unwrap(), Reflect::get(&e, &"data".into()).unwrap().as_string().unwrap()));
    });

    cyberdeck_client::init_data_channel(send_channel.clone(), onclose, onopen, onmessage);

    PEER_CONNECTION.with(|p| {
        p.replace(Some(pc.clone()));
    });

    DATA_CHANNELS.with(|d| {
        d.borrow_mut().insert("foo".to_owned(), send_channel.clone());
    });

    let pc_clone = pc.clone();
    let oniceconnectionstatechange = Closure::<dyn Fn(JsValue)>::new(move |_e: JsValue| {
        log(&Reflect::get(&pc_clone.borrow(), &"iceConnectionState".into()).unwrap().as_string().unwrap());
    });
    
    cyberdeck_client::init_peer_connection(pc.clone(), "http://localhost:3000/connect".to_string().into(), oniceconnectionstatechange).await;
}

#[wasm_bindgen]
pub fn send_message() {
    let messagebox = window().unwrap().document().unwrap().get_element_by_id("message").unwrap();

    let msg = Reflect::get(&messagebox, &"value".into()).unwrap().as_string().unwrap();
    DATA_CHANNELS.with(|d| {
        d.borrow().get("foo").unwrap().borrow().send_with_str(&msg).unwrap();
    });
}