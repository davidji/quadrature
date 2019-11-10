//! examples/init.rs

#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

extern crate panic_semihosting;
extern crate nb;

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

#[rtfm::app(device = stm32f1::stm32f103, peripherals = true)]
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
        let (tx, rx) = command_serial.split();
        init::LateResources {
            command_tx: tx,
            command_rx: rx,
            decoder: Decoder::new() }
    }

    #[task(binds = USART2, resources = [command_tx, command_rx])]
    fn serial_interrupt(c: serial_interrupt::Context) {
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

fn command_poll(_tx : &CommandTx, _rx : &CommandRx) {

}