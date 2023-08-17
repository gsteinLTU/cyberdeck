use std::{collections::HashMap, cell::RefCell, rc::Rc};

use js_sys::{Reflect, JSON, Object, Array, JsString};
use wasm_bindgen::{prelude::Closure, JsValue, JsCast};
use web_sys::{Request, RequestInit, RequestMode, Response, RtcPeerConnection, RtcDataChannel, RtcConfiguration, RtcSessionDescriptionInit, window };

/// Create an RtcPeerConnection with the given ICE/STUN server, defaulting to Google's STUN server
pub fn create_peer_connection(ice_server: Option<String>) -> Rc<RefCell<RtcPeerConnection>> {
    let mut config = RtcConfiguration::new();
    let config_servers = Array::new(); 
    let ice_server_js = Object::new();
    Reflect::set(&ice_server_js, &"urls".into(), &ice_server.unwrap_or("stun:stun.l.google.com:19302".to_string()).into()).unwrap();
    config_servers.push(&ice_server_js);
    config.ice_servers(&config_servers);

    Rc::new(RefCell::new(RtcPeerConnection::new_with_configuration(&config).expect("Failed to create RTCPeerConnection")))
}

/// Initialize RtcPeerConnection using selected signalling server endpoint, defaulting to "http://localhost:3000/connect"
pub async fn init_peer_connection(pc: Rc<RefCell<RtcPeerConnection>>, connect_url: Option<String>, oniceconnectionstatechange: Closure<dyn Fn(JsValue)>) {
    pc.borrow().set_oniceconnectionstatechange(Some(&oniceconnectionstatechange.into_js_value().unchecked_into()));

    let pc_clone = pc.clone();
    let connect_url = connect_url.unwrap_or("http://localhost:3000/connect".to_string()).clone();

    let onicecandidate = Closure::<dyn Fn(JsValue)>::new(move |event: JsValue| {    
        if Reflect::get(&event, &"candidate".into()).unwrap().is_null() {
            let local_description = get_local_description(&pc_clone);
            let mut opts = RequestInit::new();
            opts.method("POST");
            opts.mode(RequestMode::Cors);
            opts.body(Some(&JSON::stringify(&local_description.into()).unwrap()));
            
            let mut headers = HashMap::new();
            headers.insert("Content-Type", "application/json");

            opts.headers(&serde_wasm_bindgen::to_value(&headers).unwrap());
            let request = Request::new_with_str_and_init(&connect_url, &opts).unwrap();

            let pc_clone_2 = pc_clone.clone();
            let then = Closure::<dyn FnMut(JsValue)>::new(move |answer: JsValue| {
                let answer: Response = answer.unchecked_into();
                
                let pc_clone_3 = pc_clone_2.clone();
                let then = Closure::<dyn FnMut(JsValue)>::new(move |answer: JsValue| {
                    let answer: String = answer.unchecked_into::<JsString>().into();
                    let answer = answer.replace("\"", "");
                    let atob = window().unwrap().atob(&answer).unwrap();
                    let parsed = JSON::parse(&atob).unwrap();
                    pc_clone_3.borrow().set_remote_description(
                        &RtcSessionDescriptionInit::unchecked_from_js(parsed)
                    );
                });

                answer.text().unwrap().then(&then);
                then.into_js_value();
            });

            let _fetch = wasm_bindgen_futures::JsFuture::from(window().unwrap().fetch_with_request(&request).then(&then));
            then.into_js_value();
        }
    });

    pc.borrow().set_onicecandidate(Some(&onicecandidate.into_js_value().unchecked_into()));

    let pc_clone = pc.clone();
    let onnegotiationneeded = Closure::<dyn Fn()>::new(move || {
        let pc_clone_2 = pc_clone.clone();
        let then = Closure::<dyn FnMut(JsValue)>::new(move |d: JsValue| {
            pc_clone_2.borrow().set_local_description(&d.unchecked_into());
        });
        pc_clone.borrow().create_offer().then(&then);
        then.into_js_value();
    });
    pc.borrow().set_onnegotiationneeded(Some(&onnegotiationneeded.into_js_value().unchecked_into()));
}

/// Initialize RtcPeerConnection using offer already known
pub async fn init_peer_connection_from_offer(pc: Rc<RefCell<RtcPeerConnection>>, offer: String, oniceconnectionstatechange: Closure<dyn Fn(JsValue)>) {
    pc.borrow().set_oniceconnectionstatechange(Some(&oniceconnectionstatechange.into_js_value().unchecked_into()));

    let pc_clone = pc.clone();

    let onicecandidate = Closure::<dyn Fn(JsValue)>::new(move |event: JsValue| {    
        if Reflect::get(&event, &"candidate".into()).unwrap().is_null() {
            let pc_clone_2 = pc_clone.clone();
            let answer: String = offer.replace("\"", "");
            let atob = window().unwrap().atob(&answer).unwrap();
            let parsed = JSON::parse(&atob).unwrap();
            pc_clone_2.borrow().set_remote_description(
                &RtcSessionDescriptionInit::unchecked_from_js(parsed)
            );
        }
    });

    pc.borrow().set_onicecandidate(Some(&onicecandidate.into_js_value().unchecked_into()));

    let pc_clone = pc.clone();
    let onnegotiationneeded = Closure::<dyn Fn()>::new(move || {
        let pc_clone_2 = pc_clone.clone();
        let then = Closure::<dyn FnMut(JsValue)>::new(move |d: JsValue| {
            pc_clone_2.borrow().set_local_description(&d.unchecked_into());
        });
        pc_clone.borrow().create_offer().then(&then);
        then.into_js_value();
    });
    pc.borrow().set_onnegotiationneeded(Some(&onnegotiationneeded.into_js_value().unchecked_into()));
}

pub fn get_local_description(pc: &Rc<RefCell<RtcPeerConnection>>) -> String {
    let desc = pc.borrow().local_description().unwrap();
    let desc = &JSON::stringify(&desc.unchecked_into()).unwrap().as_string().unwrap();
    window().unwrap().btoa(desc).unwrap()
}

/// Create a data channel using the given RtcPeerConnection, assigned the given label
pub fn create_data_channel(pc: Rc<RefCell<RtcPeerConnection>>, label: &str) -> Rc<RefCell<RtcDataChannel>> {
    Rc::new(RefCell::new(pc.borrow().create_data_channel(label)))
}

/// Initialize an RtcDataChannel, with the given callback Closures
pub fn init_data_channel(channel: Rc<RefCell<RtcDataChannel>>, onclose: Closure<dyn Fn()>, onopen: Closure<dyn Fn()>, onmessage: Closure<dyn Fn(JsValue)>) {
    channel.borrow().set_onclose(Some(&onclose.into_js_value().unchecked_into()));
    channel.borrow().set_onopen(Some(&onopen.into_js_value().unchecked_into()));
    channel.borrow().set_onmessage(Some(&onmessage.into_js_value().unchecked_into()));
}
