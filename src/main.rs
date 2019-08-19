//! examples/init.rs

#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

extern crate panic_semihosting;

use cortex_m_semihosting::{debug, hprintln};
use stm32f1::stm32f103;

#[rtfm::app(device = stm32f1::stm32f103)]
const APP: () = {
    #[init]
    fn init(c: init::Context) {
        static mut X: u32 = 0;

        // Cortex-M peripherals
        let _core: rtfm::Peripherals = c.core;

        // Device specific peripherals
        let _device: stm32f103::Peripherals = c.device;

        // Safe access to local `static mut` variable
        let _x: &'static mut u32 = X;

        hprintln!("init").unwrap();

        debug::exit(debug::EXIT_SUCCESS);
    }
};
