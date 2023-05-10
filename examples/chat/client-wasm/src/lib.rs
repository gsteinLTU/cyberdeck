use std::{collections::HashMap, cell::RefCell, rc::Rc};

use js_sys::{Reflect};
use wasm_bindgen::{prelude::{wasm_bindgen, Closure}, JsValue};
use web_sys::{RtcPeerConnection, RtcDataChannel, window, Node };

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
    let pc: Rc<RefCell<RtcPeerConnection>> = cyberdeck_client_web_sys::create_peer_connection(None);
    let send_channel = cyberdeck_client_web_sys::create_data_channel(pc.clone(), "lobby");
    
    let onclose = Closure::<dyn Fn()>::new(|| {
        log("lobby channel has closed");
    });
    let onopen = Closure::<dyn Fn()>::new(|| {
        log("lobby channel has opened");
    });
    
    let send_channel_clone = send_channel.clone();
    let onmessage = Closure::<dyn Fn(JsValue)>::new(move |e: JsValue| {
        log(&format!("Message from DataChannel '{}' with payload '{}'", Reflect::get(&send_channel_clone.borrow(), &"label".into()).unwrap().as_string().unwrap(), Reflect::get(&e, &"data".into()).unwrap().as_string().unwrap()));
    });

    cyberdeck_client_web_sys::init_data_channel(send_channel.clone(), onclose, onopen, onmessage);

    PEER_CONNECTION.with(|p| {
        p.replace(Some(pc.clone()));
    });

    DATA_CHANNELS.with(|d| {
        d.borrow_mut().insert("lobby".to_owned(), send_channel.clone());
    });

    let pc_clone = pc.clone();
    let oniceconnectionstatechange = Closure::<dyn Fn(JsValue)>::new(move |_e: JsValue| {
        log(&Reflect::get(&pc_clone.borrow(), &"iceConnectionState".into()).unwrap().as_string().unwrap());
    });
    
    cyberdeck_client_web_sys::init_peer_connection(pc.clone(), "http://localhost:3000/connect".to_string().into(), oniceconnectionstatechange).await;
}

#[wasm_bindgen]
pub fn send_message() {
    let document = window().unwrap().document().unwrap();
    let messagebox = document.get_element_by_id("message").unwrap();
    let room_select = document.get_element_by_id("msg_room").unwrap();

    let msg = Reflect::get(&messagebox, &"value".into()).unwrap().as_string().unwrap();
    let msg_room = Reflect::get(&room_select, &"value".into()).unwrap().as_string().unwrap();

    DATA_CHANNELS.with(|d| {
        d.borrow().get(&msg_room).unwrap().borrow().send_with_str(&msg).unwrap();
    });
}

#[wasm_bindgen]
pub fn join_room() {
    let document = window().unwrap().document().unwrap();
    let messagebox = document.get_element_by_id("room").unwrap();
    let room_select = document.get_element_by_id("msg_room").unwrap();

    let msg = Reflect::get(&messagebox, &"value".into()).unwrap().as_string().unwrap();

    if msg.len() > 0 {
        DATA_CHANNELS.with(|d| {
            d.borrow().get("lobby").unwrap().borrow().send_with_str(&("/join ".to_owned() + &msg)).unwrap();
        });

        PEER_CONNECTION.with(|pc| {
            let send_channel = cyberdeck_client_web_sys::create_data_channel(pc.borrow().clone().unwrap().clone(), &msg);

            let onclose = Closure::<dyn Fn()>::new(|| {
                log("lobby channel has closed");
            });
            let onopen = Closure::<dyn Fn()>::new(|| {
                log("lobby channel has opened");
            });
            
            let send_channel_clone = send_channel.clone();
            let onmessage = Closure::<dyn Fn(JsValue)>::new(move |e: JsValue| {
                log(&format!("Message from DataChannel '{}' with payload '{}'", Reflect::get(&send_channel_clone.borrow(), &"label".into()).unwrap().as_string().unwrap(), Reflect::get(&e, &"data".into()).unwrap().as_string().unwrap()));
            });

            cyberdeck_client_web_sys::init_data_channel(send_channel.clone(), onclose, onopen, onmessage);

            DATA_CHANNELS.with(|d| {
                d.borrow_mut().insert(msg.to_owned(), send_channel.clone());
            });
        });

        let room_option = document.create_element("option").unwrap();
        room_option.set_text_content(Some(&msg));
        Reflect::set(&room_option, &"value".to_owned().into(), &msg.into());
        room_select.append_child(&room_option);
    }
}