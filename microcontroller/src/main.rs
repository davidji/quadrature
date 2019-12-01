
#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

mod rpc;

extern crate panic_semihosting;
extern crate nb;

use stm32f1xx_hal::{
    prelude::*,
    pac,
    serial::{Serial},
    serial,
};

use stm32f1xx_hal::gpio::gpiob::{ PB6, PB7 };
use stm32f1xx_hal::gpio::{ Alternate, Floating, Input, PushPull };
use stm32f1::stm32f103;
use protocol;
use heapless::{ consts::* };

type Transport = rpc::Transport<'static, U256, U256>;
type Service = rpc::Service<'static, U256, U256, U256>;

type CommandUsart = stm32f103::USART1;
type CommandSerial = Serial<CommandUsart, (PB6<Alternate<PushPull>>, PB7<Input<Floating>>)>;
type CommandTx = serial::Tx<CommandUsart>;
type CommandRx = serial::Rx<CommandUsart>;

#[rtfm::app(device = stm32f1::stm32f103)]
const APP: () = {

    struct Resources {
        transport: Transport,
        service: Service,
        command_tx: CommandTx,
        command_rx: CommandRx,
    }

    #[init]
    fn init(_: init::Context) -> init::LateResources {
        static mut RPC: Option<rpc::Rpc<U256, U256>> = None;
        *RPC = Some(rpc::Rpc::new());
        let (transport, service) = RPC.as_mut().unwrap().split();

        rtfm::pend(stm32f103::Interrupt::USART1);
        let command_serial = command_serial();
        let (mut tx, mut rx) = command_serial.split();
        rx.listen();
        tx.listen();

        init::LateResources {
            transport: transport,
            service: service,
            command_tx: tx,
            command_rx: rx,
        }
    }

    #[task(binds = USART1, 
           resources = [command_tx, 
                        command_rx,
                        transport],
           spawn = [ command_serial_rx_frame ])]
    fn command_serial_poll(c: command_serial_poll::Context) {
        while c.resources.transport.read_nb(c.resources.command_rx) {
            c.spawn.command_serial_rx_frame().unwrap();
        }
        c.resources.transport.write_nb(c.resources.command_tx);
    }

    #[task(resources = [command_tx, transport])]
    fn command_serial_tx(c: command_serial_tx::Context) {
        c.resources.transport.write_nb(c.resources.command_tx);
    }

    #[task(resources = [service],
            spawn = [command_serial_tx])]
    fn command_serial_rx_frame(c: command_serial_rx_frame::Context) {
        c.resources.service.process(process_request);
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


fn process_request(request : protocol::Request) -> Option<protocol::Response> {
    match request.body {
        protocol::RequestBody::Ping => {
            return Some(protocol::Response {
                correlation_id: request.correlation_id,
                body: protocol::ResponseBody::Ping
            });
        }
    }
}




