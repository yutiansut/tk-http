#[allow(unused_imports)]
use std::ascii::AsciiExt;
use std::str::{from_utf8};

use super::{Head};
use websocket::Accept;


/// Contains all the imporant parts of a websocket handshake
#[derive(Debug)]
pub struct WebsocketHandshake {
    /// The destination value of `Sec-WebSocket-Accept`
    pub accept: Accept,
    /// List of `Sec-WebSocket-Protocol` tokens
    pub protocols: Vec<String>,
    /// List of `Sec-WebSocket-Extensions` tokens
    pub extensions: Vec<String>,
}


fn bytes_trim(mut x: &[u8]) -> &[u8] {
    while x.len() > 0 && matches!(x[0], b'\r' | b'\n' | b' ' | b'\t') {
        x = &x[1..];
    }
    while x.len() > 0 && matches!(x[x.len()-1],  b'\r' | b'\n' | b' ' | b'\t')
    {
        x = &x[..x.len()-1];
    }
    return x;
}

pub fn get_handshake(req: &Head) -> Result<Option<WebsocketHandshake>, ()> {
    let conn_upgrade = req.connection_header().map(|x| {
        x.split(',').any(|tok| tok.trim().eq_ignore_ascii_case("upgrade"))
    });
    if !conn_upgrade.unwrap_or(false) {
        return Ok(None);
    }
    if req.path().is_none() {
        debug!("Invalid request-target for websocket request");
        return Err(());
    }
    let mut upgrade = false;
    let mut version = false;
    let mut accept = None;
    let mut protocols = Vec::new();
    let mut extensions = Vec::new();
    for h in req.all_headers() {
        if h.name.eq_ignore_ascii_case("Sec-WebSocket-Key") {
            if accept.is_some() {
                debug!("Duplicate Sec-WebSocket-Key");
                return Err(());
            }
            accept = Some(Accept::from_key_bytes(bytes_trim(h.value)));
        } else if h.name.eq_ignore_ascii_case("Sec-WebSocket-Version") {
            // Only version 13 is supported
            if bytes_trim(h.value) != b"13" {
                debug!("Bad websocket version {:?}",
                    String::from_utf8_lossy(h.value));
                return Err(());
            } else {
                version = true;
            }
        } else if h.name.eq_ignore_ascii_case("Sec-WebSocket-Protocol") {
            let tokens = from_utf8(h.value)
                .map_err(|_| debug!("Bad utf-8 in Sec-Websocket-Protocol"))?;
            protocols.extend(tokens.split(",")
                .map(|x| x.trim())
                .filter(|x| x.len() > 0)
                .map(|x| x.to_string()));
        } else if h.name.eq_ignore_ascii_case("Sec-WebSocket-Extensions") {
            let tokens = from_utf8(h.value)
                .map_err(|_| debug!("Bad utf-8 in Sec-Websocket-Extensions"))?;
            extensions.extend(tokens.split(",")
                .map(|x| x.trim())
                .filter(|x| x.len() > 0)
                .map(|x| x.to_string()));
        } else if h.name.eq_ignore_ascii_case("Upgrade") {
            if !h.value.eq_ignore_ascii_case(b"websocket") {
                return Ok(None); // Consider this not a websocket
            } else {
                upgrade = true;
            }
        }
    }
    if req.has_body() {
        debug!("Websocket handshake has payload");
        return Err(());
    }
    if !upgrade {
        debug!("No upgrade header for a websocket");
        return Err(());
    }
    if !version || accept.is_none() {
        debug!("No required headers for a websocket");
        return Err(());
    }
    Ok(Some(WebsocketHandshake {
        accept: accept.take().unwrap(),
        protocols: protocols,
        extensions: extensions,
    }))
}
