

use super::openssl::ssl::{SslMethod,SslContext};
use super::websocket::Client;
use super::websocket::stream::WebSocketStream;
use super::websocket::dataframe::{DataFrame,Opcode};
use super::websocket::sender::Sender;
use super::websocket::receiver::Receiver;
use super::websocket::client::request::Url;

use std::fmt::Debug;

use std::mem;

//
//Constant Strings for sending/recieving messages
//
const WEBSOCKET: &'static str =
    "wss://ws-feed.gdax.com";
const SUBSCRIBE: &'static str =
    r#"{"type":"subscribe","product_id":"BTC-USD"}"#;
const HEARTBEAT: &'static str =
    r#"{"type":"heartbeat","on":true}"#;

//
//Convert an error to a message
//
pub fn err2str<D: Debug>( err: &D ) -> String {
    format!("{:?}", err)
}

//
//Borrow packet as a read only string
//
pub fn df2txt<'a>( df: &'a DataFrame ) -> Result<& str, ()> {
    match df.opcode {
        Opcode::Text => { },
        _ => return Err(())
    };
    Ok(unsafe{mem::transmute(df.data.as_slice())})
}

//
//Create Method to connect to GDAX
//
pub fn connect_gdax()
-> Result<
Client<DataFrame,Sender<WebSocketStream>,Receiver<WebSocketStream>>,
String>
{
    //connect to OpenSSL
    let sslcontext = match SslContext::new( SslMethod::Sslv23 ) {
        Ok(x) => x,
        Err(ref e) => return Err(err2str(e))
    };
    //parse URL
    let url = match Url::parse( WEBSOCKET ){
        Ok(x) => x,
        Err(ref e) => return Err(err2str(e))
    };
    //build client connection request
    let req = match Client::connect_ssl_context(url, &sslcontext) {
        Ok(x) => x,
        Err(ref e) => return Err(err2str(e))
    };
    //send request, get response
    let res = match req.send() {
        Ok(x) => x,
        Err(ref e) => return Err(err2str(e))
    };
    //validate connection response
    match res.validate() {
        Ok(_) => { },
        Err(ref e) => return Err(err2str(e))
    };
    //build heartbeat data frame
    let heartbeat = DataFrame {
        finished: true,
        reserved: [false,false,false],
        opcode: Opcode::Text,
        data: HEARTBEAT.to_string().into_bytes()
    };
    //build subscription frame
    let subscribe = DataFrame {
        finished: true,
        reserved: [false,false,false],
        opcode: Opcode::Text,
        data: SUBSCRIBE.to_string().into_bytes()
    };
    //create client
    let mut client = res.begin();
    //subscribe to heart beats
    match client.send_dataframe( &heartbeat ) {
        Ok(_) => { },
        Err(ref e) => return Err(err2str(e))
    };
    //subscribe to BTC-USD
    match client.send_dataframe( &subscribe ) {
        Ok(_) => { },
        Err(ref e) => return Err(err2str(e))
    };
    //return client
    Ok(client)
}
