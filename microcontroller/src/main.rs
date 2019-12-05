
#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

mod rpc;
mod motor;

extern crate panic_semihosting;
extern crate nb;

use stm32f1xx_hal::{
    prelude::*,
    pac,
    pwm::{ Pwm, C1, C2, C3, C4 },
    serial::{Serial},
    serial,
    stm32::{ TIM3 },
};

use stm32f1xx_hal::gpio::gpiob::{ PB6, PB7 };
use stm32f1xx_hal::gpio::{ Alternate, Floating, Input, PushPull };
use stm32f1::stm32f103;
use protocol;
use heapless::{ consts::* };
use motor::{ Differential, DcMotor, TwoPinDcMotorOut};

type Transport = rpc::Transport<'static, U256, U256>;
type Service = rpc::Service<'static, U256, U256, U256>;

type CommandUsart = stm32f103::USART1;
type CommandSerial = Serial<CommandUsart, (PB6<Alternate<PushPull>>, PB7<Input<Floating>>)>;
type CommandTx = serial::Tx<CommandUsart>;
type CommandRx = serial::Rx<CommandUsart>;

type LeftMotor = TwoPinDcMotorOut<Pwm<TIM3, C1>, Pwm<TIM3, C2>>;
type RightMotor = TwoPinDcMotorOut<Pwm<TIM3, C3>, Pwm<TIM3, C4>>;
type Motors = Differential<LeftMotor, RightMotor>;

#[rtfm::app(device = stm32f1::stm32f103)]
const APP: () = {

    struct Resources {
        transport: Transport,
        service: Service,
        command_tx: CommandTx,
        command_rx: CommandRx,
        motors : Motors,
   }

    #[init]
    fn init(_: init::Context) -> init::LateResources {
        static mut RPC: Option<rpc::Rpc<U256, U256>> = None;
        *RPC = Some(rpc::Rpc::new());

        let (command_serial, motors) = hardware();

        let (transport, service) = RPC.as_mut().unwrap().split();

        rtfm::pend(stm32f103::Interrupt::USART1);
        let (mut tx, mut rx) = command_serial.split();
        rx.listen();
        tx.listen();

        init::LateResources {
            transport: transport,
            service: service,
            command_tx: tx,
            command_rx: rx,
            motors: motors,
        }
    }

    #[task(binds = USART1,
           resources = [command_tx, command_rx, transport],
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

    #[task(resources = [service], spawn = [command_serial_tx])]
    fn command_serial_rx_frame(c: command_serial_rx_frame::Context) {
        c.resources.service.process(process_request);
    }

    extern "C" {
        fn USART2();
    }
};

fn hardware() -> (CommandSerial, Motors) {
        // Get access to the device specific peripherals from the peripheral access crate
        let peripherals = pac::Peripherals::take().unwrap();
        // Take ownership over the raw flash and rcc devices and convert them into the corresponding
        // HAL structs
        let mut flash = peripherals.FLASH.constrain();
        let mut rcc = peripherals.RCC.constrain();

        // Freeze the configuration of all the clocks in the system and store the frozen frequencies in
        // `clocks`
        let clocks = rcc.cfgr.use_hse(8.mhz()).freeze(&mut flash.acr);

        // Prepare the alternate function I/O registers
        let mut afio = peripherals.AFIO.constrain(&mut rcc.apb2);

        // Prepare the GPIO peripherals
        let mut gpioa = peripherals.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = peripherals.GPIOB.split(&mut rcc.apb2);

        let pins = (
            gpioa.pa6.into_alternate_push_pull(&mut gpioa.crl),
            gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl),
            gpiob.pb0.into_alternate_push_pull(&mut gpiob.crl),
            gpiob.pb1.into_alternate_push_pull(&mut gpiob.crl),
        );

        let (c1, c2, c3, c4)  = peripherals.TIM3.pwm(
            pins,
            &mut afio.mapr,
            10.khz(),
            clocks,
            &mut rcc.apb1
        );

        let motors = Motors {
            left: DcMotor { out: LeftMotor { out1: c1, out2: c2 } }, 
            right: DcMotor { out: RightMotor { out1: c3, out2: c4 } },
        };

        // USART1
        let tx = gpiob.pb6.into_alternate_push_pull(&mut gpiob.crl);
        let rx = gpiob.pb7;

        let command_serial = Serial::usart1(
            peripherals.USART1,
            (tx, rx),
            &mut afio.mapr,
            serial::Config::default().baudrate(115200.bps()),
            clocks,
            &mut rcc.apb2,
        );

        return (command_serial, motors);
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




