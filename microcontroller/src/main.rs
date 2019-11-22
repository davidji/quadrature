
#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

extern crate panic_semihosting;
extern crate nb;

use nb::Error::WouldBlock;
use nb::block;

use stm32f1xx_hal::{
    prelude::*,
    pac,
    serial::{Serial},
    serial,
};

use arraydeque::{
    ArrayDeque,
    Wrapping
    };

use stm32f1xx_hal::gpio::gpioa::{ PA2, PA3 };
use stm32f1xx_hal::gpio::{ Alternate, Floating, Input, PushPull };
use stm32f1::stm32f103;
use postcard;
use protocol;

const BUFFER_LENGTH : usize = 512;
const DELIMITER : u8 = 0;

type Buffer = [u8; BUFFER_LENGTH];

type CommandUsart = stm32f103::USART2;
type CommandSerial = Serial<CommandUsart, (PA2<Alternate<PushPull>>, PA3<Input<Floating>>)>;
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
        command_rx_queue: CommandRxQueue
    }

    #[init]
    fn init(_: init::Context) -> init::LateResources {
        rtfm::pend(stm32f103::Interrupt::USART1);
        let command_serial = command_serial();
        let (mut tx, mut rx) = command_serial.split();
        command_write(&mut tx, "Hello\n");
        rx.listen();
        init::LateResources {
            command_tx: tx,
            command_rx: rx,
            command_tx_queue: ArrayDeque::new(),
            command_rx_queue: ArrayDeque::new()
        }
    }

    #[task(binds = USART2, 
           resources = [command_tx, command_rx, command_tx_queue, command_rx_queue],
           spawn = [ command_serial_rx_frame ])]
    fn command_serial_poll(c: command_serial_poll::Context) {
       loop {
            match c.resources.command_rx.read() {
                Ok(byte) => {
                    c.resources.command_rx_queue.push_back(byte);
                    if byte == DELIMITER {
                        c.spawn.command_serial_rx_frame(c.resources.command_rx_queue.len()).unwrap()
                    }
                },
                Err(WouldBlock) => break,
                Err(_) => panic!("Error reading from command serial"),
            }
        };

        write_from_queue_nb(c.resources.command_tx, c.resources.command_tx_queue);
    }

    #[task(resources = [command_rx_queue])]
    fn command_serial_rx_frame(c: command_serial_rx_frame::Context, length: usize) {
        let mut drain = c.resources.command_rx_queue.drain(..length);
        let mut packet : Buffer = [0; BUFFER_LENGTH];
        for i in 0..length {
            match drain.next() {
                Some(byte) => packet[i] = byte,
                None => break
            }
        }

        let result : postcard::Result<protocol::Request>  = postcard::from_bytes_cobs(&mut packet[0..length]);
        match result {
            Ok(request) => match request.body {
                protocol::RequestBody::Ping(_) => {

                }
            }
            Err(_) => {},
        }

    }

    extern "C" {
        fn USART1();
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
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    // Prepare the alternate function I/O registers
    let mut afio = p.AFIO.constrain(&mut rcc.apb2);

    // Prepare the GPIOB peripheral
    let mut gpioa = p.GPIOA.split(&mut rcc.apb2);
    
    // USART2
    let tx = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
    let rx = gpioa.pa3;

    return Serial::usart2(
        p.USART2,
        (tx, rx),
        &mut afio.mapr,
        serial::Config::default().baudrate(115200.bps()),
        clocks,
        &mut rcc.apb1,
    );
}

fn write_from_queue_nb(
    command_tx : &mut CommandTx, 
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

fn command_write(tx : &mut CommandTx, s : & str) {
    for c in s.chars() {
        block!(tx.write(c as u8)).unwrap();
    }
}

