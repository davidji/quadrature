
mod motor;

use stm32f1::stm32f103;

use stm32f1xx_hal::{
    prelude::*,
    gpio::{ Alternate, Floating, Input, PushPull },
    gpio::gpiob::{ PB6, PB7 },
    pac,
    pwm::{ Pwm, C1, C2, C3, C4 },
    serial::{self, Serial},
    stm32::{ TIM3 },
};

use motor::{ Differential, DcMotor, TwoPinDcMotorOut};

type CommandUsart = stm32f103::USART1;
type CommandSerial = Serial<CommandUsart, (PB6<Alternate<PushPull>>, PB7<Input<Floating>>)>;
pub type CommandTx = serial::Tx<CommandUsart>;
pub type CommandRx = serial::Rx<CommandUsart>;

type LeftMotor = TwoPinDcMotorOut<Pwm<TIM3, C1>, Pwm<TIM3, C2>>;
type RightMotor = TwoPinDcMotorOut<Pwm<TIM3, C3>, Pwm<TIM3, C4>>;
pub type Motors = Differential<LeftMotor, RightMotor>;

pub fn hardware() -> (CommandSerial, Motors) {
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
