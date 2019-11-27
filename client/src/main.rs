extern crate clap;
use clap::{ Arg, App };
use serialport;
use serialport::{ SerialPortSettings, FlowControl,DataBits,Parity,StopBits };
use postcard;
use protocol;
use std::io;
use std::io::Write;
use std::time::Duration;
use std::{ thread };

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
        timeout: Duration::from_millis(1)
    };
    
    let mut serial_port = serialport::open_with_settings(serial_device_path, &settings).unwrap();
    let mut sending_port = serial_port.try_clone().expect("Failed to clone");
    
    thread::spawn(move || for id in 0..1000 {
        let request = protocol::Request {
            correlation_id: id%32,
            body: protocol::RequestBody::Ping
        };
        
        let mut buffer : [u8; 256] = [0; 256];
        let frame = postcard::to_slice_cobs(&request, &mut buffer).unwrap();
        sending_port.write(&frame[..]).unwrap();
        // println!("Sending: {:?}", &frame[..]);
        thread::sleep(Duration::from_millis(100));
    });
    
    let mut accumulator : Vec<u8> = Vec::new();
    for expected in 0..1000 {
        loop {
            let mut buffer: [u8; 32] = [0; 32];
            match serial_port.read(&mut buffer) {
                Ok(bytes) if bytes > 0 => {
                    accumulator.extend_from_slice(&buffer[..bytes]);
                    let mut frame = accumulator.clone();
                    let nonzero = frame.iter().position(|&x| x != 0).unwrap_or(0);
                    let zero = frame.iter().position(|&x| x != 0);
                    if zero.is_some() && zero.unwrap() > nonzero {
                        let result : 
                        postcard::Result<(protocol::Response, &mut [u8])> = 
                        postcard::take_from_bytes_cobs(&mut frame[nonzero..]);
                        match result {
                            Ok((response, unused)) => {
                                if expected%32 != response.correlation_id {
                                    println!("Incorrect response: {:?} for {:?}", 
                                    response.correlation_id, expected%32);
                                    eprintln!("{:?}", accumulator);
                                }
                                accumulator.clear();
                                accumulator.extend_from_slice(&unused[1..]);
                                break;
                            },
                            Err(postcard::Error::DeserializeUnexpectedEnd) => {},
                            Err(e) =>  {
                                eprintln!("Deserialisation error: {:?}", e);
                                eprintln!("{:?}", &buffer[..bytes]);
                            }
                        }
                    }
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                Err(e) => eprintln!("Serial IO error: {:?}", e),
            }
        }
    }
}
