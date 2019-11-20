
use serde::{Serialize, Deserialize};

struct Ping { }

enum RequestBody {
    Ping(Ping)
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct Request {
    correlation_id : u64;
    body : RequestBody;
}

enum ResponseBody {
    Ping(Ping)
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct Response {
    u64 correlation_id;
    body : ResponseBody;
}
