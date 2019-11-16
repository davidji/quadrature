
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

use stm32f1xx_hal::gpio::gpioa::{ PA2, PA3 };
use stm32f1xx_hal::gpio::{ Alternate, Floating, Input, PushPull };
use stm32f1::stm32f103;

use serial_line_ip::{ Decoder };

type CommandUsart = stm32f103::USART2;
type CommandSerial = Serial<CommandUsart, (PA2<Alternate<PushPull>>, PA3<Input<Floating>>)>;
type CommandTx = serial::Tx<CommandUsart>;
type CommandRx = serial::Rx<CommandUsart>;

#[rtfm::app(device = stm32f1::stm32f103)]
const APP: () = {

    struct Resources {
        command_tx: CommandTx,
        command_rx: CommandRx,
        decoder: Decoder
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
            decoder: Decoder::new() }
    }

    #[task(binds = USART2, resources = [command_tx, command_rx])]
    fn command_serial_interrupt(c: command_serial_interrupt::Context) {
        command_poll(c.resources.command_tx, c.resources.command_rx);
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

fn command_write(tx : &mut CommandTx, s : & str) {
    for c in s.chars() {
        block!(tx.write(c as u8)).unwrap();
    }
}

fn command_poll(tx : &mut CommandTx, rx : &mut CommandRx) {
    'nonblockrx: loop {
        match rx.read() {
            Ok(byte) => block!(tx.write(to_upper(byte))).unwrap(),
            Err(err) => {
                match err {
                    WouldBlock => break 'nonblockrx,
                    _ => panic!("Error reading from command serial"),
                }
            },
        };
    }
}

fn to_upper(byte: u8) -> u8 {
    match byte as char {
        'a'..='z' => return byte - ('a' as u8 - 'A' as u8),
        _ => return byte
    }
}