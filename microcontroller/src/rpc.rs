use heapless::{ ArrayLength, Vec };
use heapless::spsc::{ Queue, Producer, Consumer };
use embedded_hal::serial::{Read,Write};
use nb::Error::WouldBlock;
use postcard::{ self };
use serde::{ Serialize, de::DeserializeOwned };

pub struct Service<'a, Nin, Nout, Nb> 
where
    Nin: ArrayLength<u8>,
    Nout: ArrayLength<u8>,
    Nb: ArrayLength<u8> {
    requests: Consumer<'a, u8, Nin>,
    responses: Producer<'a, u8, Nout>,
    incomplete: Vec<u8, Nb>
}

pub struct Transport<'a, Nin, Nout>
where
    Nin: ArrayLength<u8>,
    Nout: ArrayLength<u8> {
    requests: Producer<'a, u8, Nin>,
    responses: Consumer<'a, u8, Nout>,
}

pub struct Rpc<Nin, Nout>
where
    Nin: ArrayLength<u8>,
    Nout: ArrayLength<u8> {
    requests: Queue<u8, Nin>,
    responses: Queue<u8, Nout>,
}

impl <Nin, Nout> Default for Rpc<Nin, Nout>
where
    Nin: ArrayLength<u8>,
    Nout: ArrayLength<u8>
{
    fn default() -> Self {
        Rpc {
            requests: Queue::new(),
            responses: Queue::new(),
        }
    }
}

impl <Nin, Nout>Rpc<Nin, Nout>
where
    Nin: ArrayLength<u8>,
    Nout: ArrayLength<u8>
{
    pub fn new() -> Self {
        Rpc {
            requests: Queue::new(),
            responses: Queue::new(),
        }
    }

    pub fn split<'a, Nb>(&'a mut self) -> (Transport<'a, Nin, Nout>, Service<'a,Nin,Nout, Nb>)
    where Nb: ArrayLength<u8> {
        let (requests_producer, requests_consumer) = self.requests.split();
        let (responses_producer, responses_consumer) = self.responses.split();
        return (
            Transport { 
                requests: requests_producer, 
                responses: responses_consumer,
            },
            Service {
                requests: requests_consumer, 
                responses: responses_producer, 
                incomplete: Vec::new(),
            });
    }
}

impl <Nin, Nout>Transport<'_, Nin, Nout>
where
    Nin: ArrayLength<u8>,
    Nout: ArrayLength<u8>,
{
    pub fn read_nb<R> (
        &mut self,
        command_rx: &mut R) -> bool
    where R: Read<u8> {
        let mut read = false;
           loop {
                match command_rx.read() {
                    Ok(byte) => {
                        read = self.requests.enqueue(byte).is_ok();
                        if !read { break };
                    },
                    Err(WouldBlock) => break,
                    Err(_) => panic!("Error reading from command serial"),
                }
            };
            return read;
    }
    
    pub fn write_nb<W>(
        &mut self,
        command_tx: &mut W)
    where W: Write<u8> {
        loop {
            match self.responses.peek() {
                Some(byte) => {
                    match command_tx.write(*byte) {
                        Ok(_) => assert!(self.responses.dequeue().is_some()),
                        Err(WouldBlock) => break,
                        Err(_) => panic!("Error writing to command serial"),
                    }
                }
                None => break
            }
        }
    }
}

impl <Nin, Nout, Nb> Service<'_, Nin, Nout, Nb>
where
    Nin: ArrayLength<u8>,
    Nout: ArrayLength<u8>,
    Nb: ArrayLength<u8>
{
    pub fn send(&mut self, packet: &[u8]) {
        for byte in packet {
            self.responses.enqueue(*byte).unwrap();
        }
    }
    
    pub fn response<R>(&mut self, r: &R)
    where R: Serialize {
        let encoded: Vec<u8, Nb> = postcard::to_vec_cobs(r).unwrap();
        self.send(&encoded[..]);
    }

    pub fn recv<'a>(&'a mut self) -> Option<&'a mut [u8]> {
        // This is how I tell that the frame currently in
        // incomplete has already been returned
        if self.incomplete.last() == Some(&0) {
            self.incomplete.clear();
        }

        loop {
            match self.requests.dequeue() {
                Some(byte) if byte == 0 => {
                    // Just throw away empty frames
                    if self.incomplete.len() > 0 {
                        self.incomplete.push(0).unwrap();
                        return Some(&mut self.incomplete[..]);
                    }
                }
                Some(byte) => self.incomplete.push(byte).unwrap(),
                None => return None
            }
        }
    }

    pub fn process<Request, Response, Service>(&mut self, service: Service)
    where 
        Request: DeserializeOwned,
        Response: Serialize,
        Service: Fn(Request) -> Option<Response>
    {
        loop {
            match self.requests.dequeue() {
                Some(byte) if byte == 0 => {
                    // Just throw away empty frames
                    if self.incomplete.len() > 0 {
                        self.incomplete.push(0).unwrap();
                        let result : postcard::Result<Request> = postcard::from_bytes_cobs(&mut self.incomplete[..]);
                        match result {
                            Ok(request) => {
                                service(request).map(|x| self.response(&x));
                            },
                            Err(_) => {}
                        }
                        self.incomplete.clear();
                    }
                }
                Some(byte) => self.incomplete.push(byte).unwrap(),
                None => break,
            }
        }
    }
    
}
