#![cfg_attr(not(any(test, feature = "use-std")), no_std)]
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Ping { }

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum RequestBody {
    Ping(Ping)
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Request {
    pub correlation_id : u64,
    pub body : RequestBody
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum ResponseBody {
    Ping(Ping)
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Response {
    pub correlation_id : u64,
    pub body : ResponseBody
}
