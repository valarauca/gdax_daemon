extern crate websocket;
extern crate unix_socket;
extern crate openssl;
extern crate regex;
#[macro_use]
extern crate lazy_static;

use regex::Regex;
use std::io::prelude::*;
use std::fs::{File, OpenOptions};
use std::fmt::Debug;

mod connector;

//Path to FIFO
const FIFOPATH: &'static str = 
"/root/Documents/named_pipes/gdax_raw";

//message to send when reseting
const RESETMSG: &'static str =
r#"{"type":"reset"}"#;

//message to send when daemon closes
const ENDMSG: &'static str =
r#"{"type":"closing"}"#;


//
//This handle un-recoverable errors
//
pub fn alert_client<D: Debug>( fd: &mut File, err: &D ) -> ! {
    //send client end message
    match fd.write_all(ENDMSG.as_bytes()){
        Ok(_) => { },
        Err(e) => println!("Error while closing. {:?}", e)
    };
    panic!("{:?}", err);
}

lazy_static!{

static ref SEQVAL: Regex = Regex::new(
r#"[\s\S]*sequence.:.(\d+)[\s\S]*"#).unwrap();
static ref PACKETTYPE: Regex = Regex::new(
r#"[\s\S]*type.:.([a-z]+)[\s\S]*"#).unwrap();
}

//
//Enum concerning packet data
//
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
enum Packet {
    HeartBeat(u64),
    Next(u64),
    Reconnect
}
impl Packet {

    //
    //Check packet
    //
    fn new(msg: &str) -> Packet {
        //get sequence number
        let seq = match SEQVAL.captures(msg) {
            Option::Some(caps) => match caps.at(1) {
                Option::Some(val) => match u64::from_str_radix(val,10){
                    Ok(x) => x,
                    _ => return Packet::Reconnect
                },
                _ => return Packet::Reconnect
            },
            _ => return Packet::Reconnect
        };
        //get packet type
        match PACKETTYPE.captures(msg) {
            Option::Some(caps) => match caps.at(1) {
                Option::Some(x) if x == "heartbeat" => Packet::HeartBeat(seq),
                Option::Some(_) => Packet::Next(seq),
                _ => Packet::Reconnect
            },
            _ => Packet::Reconnect
        }
    }

    //
    //Handle seq value
    //
    fn handle(self, old_seq: &mut u64 ) -> bool {
        match self {
            Packet::Reconnect => false,
            Packet::HeartBeat(seq) => {
                let o_s = old_seq.clone();
                if o_s == 0u64 {
                    *old_seq = seq;
                    true
                } else {
                    seq == o_s
                }
            },
            Packet::Next(seq) => {
                let o_s = old_seq.clone();
                let next_seq = o_s.wrapping_add(1);
                if o_s == 0u64 {
                    *old_seq = seq;
                    true
                } else if seq == next_seq {
                    *old_seq = next_seq;
                    true
                } else {
                    false
                }
            }
        }
    }
}
fn main() {
    
    //open FIFO
    let mut fifo: File  = match OpenOptions::new()
                                .read(false)
                                .write(true)
                                .create(false)
                                .open(FIFOPATH) {
        Ok(x) => x,
        Err(e) => panic!("Couldn't open FIFO. Error: {:?}", e)
    };

    //main connection loop
    loop {
        //set an initial sequence value
        let mut seq_val = 0u64;

        //send client a reset message
        match fifo.write_all(RESETMSG.as_bytes()) {
            Ok(x) => x,
            Err(ref e) => alert_client(&mut fifo, e)
        };

        //open client
        let mut client = match connector::connect_gdax() {
            Ok(x) => x,
            Err(ref e) => alert_client(&mut fifo, e)
        };

        //loop over messages
        for result_msg in client.incoming_dataframes() {
            
            //get the dataframe
            let df = match result_msg {
                Ok(x) => x,
                Err(_) => break 
            };

            //get string
            let msg = match connector::df2txt(&df) {
                Ok(x) => x,
                Err(_) => break
            };

            //ensure packet is correct 
            if Packet::new(msg).handle(&mut seq_val) {
                match fifo.write_all(msg.as_bytes()){
                    Ok(_) => { },
                    Err(ref e) => alert_client(&mut fifo, e)
                };
            } else {
                break;
            }
        }
    }
}
