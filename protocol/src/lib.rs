#![no_std]
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum RequestBody {
    Ping
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Request {
    // pub message_id : u64,
    pub correlation_id : i32,
    pub body : RequestBody
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum ResponseBody {
    Ping
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Response {
    // pub message_id : u64,
    pub correlation_id : i32,
    pub body : ResponseBody
}
