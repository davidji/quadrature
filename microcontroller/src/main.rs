
#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

extern crate panic_semihosting;
extern crate nb;

use nb::Error::WouldBlock;

use stm32f1xx_hal::{
    prelude::*,
    pac,
    serial::{Serial},
    serial,
};

use arraydeque::{
    ArrayDeque,
    Wrapping,
    };

use stm32f1xx_hal::gpio::gpiob::{ PB6, PB7 };
use stm32f1xx_hal::gpio::{ Alternate, Floating, Input, PushPull };
use stm32f1::stm32f103;
use postcard;
use protocol;

const BUFFER_LENGTH : usize = 512;
const DELIMITER : u8 = 0;

type Buffer = [u8; BUFFER_LENGTH];

type CommandUsart = stm32f103::USART1;
type CommandSerial = Serial<CommandUsart, (PB6<Alternate<PushPull>>, PB7<Input<Floating>>)>;
type CommandTx = serial::Tx<CommandUsart>;
type CommandRx = serial::Rx<CommandUsart>;
type CommandTxQueue = ArrayDeque<Buffer>;
// We can't block on receiving, so Wrapping is the right behaviour
type CommandRxQueue = ArrayDeque<Buffer,Wrapping>;


/* This is how many bytes might get bufferred while we
 * process an incoming message
 */

#[rtfm::app(device = stm32f1::stm32f103)]
const APP: () = {

    struct Resources {
        command_tx: CommandTx,
        command_rx: CommandRx,
        command_tx_queue: CommandTxQueue,
        command_rx_queue: CommandRxQueue,
        tx_counter: u32,
        rx_counter: u32,
    }

    #[init]
    fn init(_: init::Context) -> init::LateResources {
        rtfm::pend(stm32f103::Interrupt::USART1);
        let command_serial = command_serial();
        let (mut tx, mut rx) = command_serial.split();
        rx.listen();
        tx.listen();
        init::LateResources {
            command_tx: tx,
            command_rx: rx,
            command_tx_queue: ArrayDeque::new(),
            command_rx_queue: ArrayDeque::new(),
            tx_counter: 0,
            rx_counter: 0,
        }
    }

    #[task(binds = USART1, 
           resources = [command_tx, 
                        command_rx, 
                        command_tx_queue, 
                        command_rx_queue],
           spawn = [ command_serial_rx_frame ])]
    fn command_serial_poll(c: command_serial_poll::Context) {
        while read_to_queue_nb(c.resources.command_rx, c.resources.command_rx_queue) {
            c.spawn.command_serial_rx_frame().unwrap();
        }
        write_from_queue_nb(c.resources.command_tx, c.resources.command_tx_queue);
    }

    #[task(resources = [command_tx, command_tx_queue])]
    fn command_serial_tx(c: command_serial_tx::Context) {
        write_from_queue_nb(c.resources.command_tx, c.resources.command_tx_queue);
    }

    #[task(resources = [command_rx_queue, command_tx_queue],
            spawn = [command_serial_tx])]
    fn command_serial_rx_frame(c: command_serial_rx_frame::Context) {
        let mut frame : Buffer = [0; BUFFER_LENGTH];
        match pop_frame(c.resources.command_rx_queue, &mut frame) {
            Some(length) => {
                process_request_frame(&mut frame[..length], c.resources.command_tx_queue);
                c.spawn.command_serial_tx().unwrap();
            }
            None => {}
        }
    }

    extern "C" {
        fn USART2();
    }
};

fn command_serial () -> CommandSerial {
    // Get access to the device specific peripherals from the peripheral access crate
    let p = pac::Peripherals::take().unwrap();

    // Take ownership over the raw flash and rcc devices and convert them into the corresponding
    // HAL structs
    let mut flash = p.FLASH.constrain();
    let mut rcc = p.RCC.constrain();

    // Freeze the configuration of all the clocks in the system and store the frozen frequencies in
    // `clocks`
    let clocks = rcc.cfgr
         .use_hse(8.mhz())
    //    .sysclk(16.mhz())
    //    .pclk1(4.mhz())
    //    .adcclk(2.mhz())
         .freeze(&mut flash.acr);

    // Prepare the alternate function I/O registers
    let mut afio = p.AFIO.constrain(&mut rcc.apb2);

    // Prepare the GPIOB peripheral
    let mut gpiob = p.GPIOB.split(&mut rcc.apb2);
    
    // USART1
    let tx = gpiob.pb6.into_alternate_push_pull(&mut gpiob.crl);
    let rx = gpiob.pb7;

    return Serial::usart1(
        p.USART1,
        (tx, rx),
        &mut afio.mapr,
        serial::Config::default().baudrate(115200.bps()),
        clocks,
        &mut rcc.apb2,
    );
}

fn process_request_frame(request_frame: &mut [u8], response_queue: &mut CommandTxQueue) {
    let result : postcard::Result<protocol::Request>  = postcard::from_bytes_cobs(request_frame);
    match result {
        Ok(request) => process_request(request, response_queue),
        Err(_) => {},
    }
}

fn process_request(request : protocol::Request, response_queue: &mut CommandTxQueue) {
    match request.body {
        protocol::RequestBody::Ping => {
            let response = protocol::Response {
                correlation_id: request.correlation_id,
                body: protocol::ResponseBody::Ping
            };
            let mut buffer : Buffer = [0; BUFFER_LENGTH];
            match postcard::to_slice_cobs(&response, &mut buffer) {
                Ok(frame) => push_frame(response_queue, frame),
                Err(_) => panic!("Error serializing response")
            }               
        }
    }
}

fn push_frame(queue: &mut CommandTxQueue, packet: &[u8]) {
    queue.extend_back(packet.iter().cloned());
}

fn pop_frame(queue: &mut CommandRxQueue, packet: &mut Buffer) -> Option<usize> {
    let available = queue.len();
    for i in 0..available {
        match queue.get(i) {
            Some(byte) if *byte == DELIMITER => {
                packet[i] = DELIMITER;
                let length = i+1;
                queue.drain(..length);
                return Some(length);
            }
            Some(byte) => packet[i] = *byte,
            None => break
        }
    }
    return None;
}

fn read_to_queue_nb(
    command_rx: &mut CommandRx,
    command_rx_queue: &mut CommandRxQueue) -> bool {
       loop {
            match command_rx.read() {
                Ok(byte) => {
                    command_rx_queue.push_back(byte);
                    return byte == DELIMITER;
                },
                Err(WouldBlock) => break,
                Err(_) => panic!("Error reading from command serial"),
            }
        };
        return false;
}

fn write_from_queue_nb(
    command_tx: &mut CommandTx, 
    command_tx_queue: &mut CommandTxQueue) {
    loop {
        match command_tx_queue.front() {
            Some(byte) => {
                match command_tx.write(*byte) {
                    Ok(_) => assert!(command_tx_queue.pop_front().is_some()),
                    Err(WouldBlock) => break,
                    Err(_) => panic!("Error writing to command serial"),
                }
            }
            None => break
        }
    }
}


