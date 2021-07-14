extern crate clap;
use clap::{ Arg, App };
use serialport;
use serialport::{ SerialPortSettings, FlowControl,DataBits,Parity,StopBits };
use postcard;
use protocol;
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;
use std::{ thread };

const DELIMITER : u8 = 0;

fn main() {
    let matches = App::new("quadrature-ping")
    .version("0.1")
    .about("Test connectivity to the microcontroller")
    .author("David Ireland")
    .arg(Arg::with_name("serial-device-path")
    .short("d")
    .long("device")
    .help("Path to serial device")
    .required(true)
    .takes_value(true))
    .arg(Arg::with_name("serial-baud")
    .short("b")
    .long("baud")
    .help("Serial baud rate")
    .takes_value(true))
    .get_matches();
    
    let serial_device_path = matches.value_of("serial-device-path").unwrap();
    let serial_baud = matches.value_of("serial-baud").unwrap_or("115200");
    
    let settings = SerialPortSettings {
        baud_rate: serial_baud.parse::<u32>().unwrap(),
        data_bits: DataBits::Eight,
        flow_control: FlowControl::None,
        parity: Parity::None,
        stop_bits: StopBits::One,
        timeout: Duration::from_millis(2000)
    };
    
    let mut serial_port = serialport::open_with_settings(serial_device_path, &settings).unwrap();
    let mut sending_port = serial_port.try_clone().expect("Failed to clone");
    
    thread::spawn(move || for id in 0.. {
        let request = protocol::Request {
            correlation_id: id,
            body: protocol::RequestBody::Ping
        };
        
        let mut buffer : [u8; 256] = [0; 256];
        let frame = postcard::to_slice_cobs(&request, &mut buffer).unwrap();
        sending_port.write_all(&frame[..]).unwrap();
        thread::sleep(Duration::from_millis(1000));
    });
    
    let buffered = BufReader::new(&mut serial_port);
    for unterminated in buffered.split(DELIMITER) {
        let mut frame = unterminated.unwrap().clone();
        frame.push(DELIMITER);
        let result : 
            postcard::Result<(protocol::Response, &mut [u8])> = 
            postcard::take_from_bytes_cobs(&mut frame[..]);
        match result {
            Ok((response, _)) => {
                println!("Response: {:?}", response.correlation_id);
            },
            Err(e) =>  {
                eprintln!("Deserialisation error: {:?}", e);
                eprintln!("{:?}", &frame[..]);
            }
        }

    }

}
