
#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

mod rpc;
mod hardware;
// mod int_pid;

extern crate panic_semihosting;
extern crate nb;

use stm32f1::stm32f103;
use protocol;
use heapless::{ consts::* };
use hardware::{ CommandTx, CommandRx, Motors, hardware };
use rtfm::cyccnt::{ Instant, U32Ext };

type Transport = rpc::Transport<'static, U256, U256>;
type Service = rpc::Service<'static, U256, U256, U256>;

const PERIOD: u32 = 8_000_000;

#[rtfm::app(device = stm32f1::stm32f103, monotonic = rtfm::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        transport: Transport,
        service: Service,
        command_tx: CommandTx,
        command_rx: CommandRx,
        motors : Motors
   }

    #[init(schedule=[quadrature])]
    fn init(c: init::Context) -> init::LateResources {
        static mut RPC: Option<rpc::Rpc<U256, U256>> = None;
        *RPC = Some(rpc::Rpc::new());

        let (command_serial, motors) = hardware();

        let (transport, service) = RPC.as_mut().unwrap().split();

        rtfm::pend(stm32f103::Interrupt::USART1);
        let (mut tx, mut rx) = command_serial.split();
        rx.listen();
        tx.listen();

		c.schedule.quadrature(Instant::now() + PERIOD.cycles()).unwrap();

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
        if c.resources.transport.read_nb(c.resources.command_rx) {
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

    #[task(resources = [ motors])]
    fn quadrature(c: quadrature::Context) {
        c.resources.motors.update();
    }

    extern "C" {
        fn USART2();
    }
};

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




