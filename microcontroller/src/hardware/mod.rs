use core::{ 
    u16,
    option::{ Option },
    mem::{ replace }
};


mod motor;

use stm32f1::stm32f103;

use stm32f1xx_hal::{
    prelude::*,
    adc::{self, Adc, AdcDma, Scan, SetChannels },
    dma::{ Transfer, W},
    gpio::{ Alternate, Floating, Input, PushPull, Analog },
    gpio::gpioa::{ 
        PA0, // Quadrature ADC 
        PA1, // Quadrature ADC
        PA2, // Quadrature ADC
        PA3,  // Quadrature ADC
        // PA4, // * Voltage
        // PA5, // * Other ADC
        // PA6, // * Motor PWM, TIM3
        // PA7, // * Motor PWM, TIM3
        // PA8, // * Other ADC | TIM1 CH1
        PA9, // Serial Tx USART1
        PA10, // Serial Tx USART1
        // PA11, // * USB-
        // PA12, // * USB+
        // PA15, // * Power (SWIN)
    },
    gpio::gpiob::{ 
        // PB0, // * Motor PWM, TIM3
        // PB1, // * Motor PWM, TIM3
        // PB3, // * Power (SWOUT)
        // PB4, // * RF24 CE
        // PB5, // * RF24 CSN
        // PB6, // * Servo TIM4 CH1, I2C1
        // PB7, // * Servo TIM4 CH2, I2C1
        // PB8, // * Servo TIM4 CH3
        // PB9, // * Servo TIM4 CH4
        // PB10, // * I2C for expansion I2C2
        // PB11, // * I2C for expansion I2C2
        // PB12, // * RF24 IRQ
        PB13, // SCLK - RF24
        PB14, // MISO - RF24
        PB15, // MOSI - RF24
    },
    pac,
    pwm::{ PwmChannel, C1, C2, C3, C4 },
    serial::{self, Serial},
    spi::{ Spi },
    stm32::{ TIM3, ADC1, SPI2 },
    timer::{Tim3NoRemap, Timer},
};
use cortex_m::{ singleton};

use motor::{ 
    Differential, 
    DcMotor, 
    TwoPinDcMotorOut, 
    AnalogRotaryEncoder, 
    DifferentialQuadratureAnalogInput,
    DifferentialQuadratureSamples
 };

type CommandUsart = stm32f103::USART1;
type CommandSerial = Serial<CommandUsart, (PA9<Alternate<PushPull>>, PA10<Input<Floating>>)>;
pub type CommandTx = serial::Tx<CommandUsart>;
pub type CommandRx = serial::Rx<CommandUsart>;

type RF24Spi = Spi<SPI2, (
    PB13<Alternate<PushPull>>, 
    PB14<Alternate<Input<Floating>>>, 
    PB15<Alternate<PushPull>>)>;

type LeftMotor = TwoPinDcMotorOut<PwmChannel<TIM3, C1>, PwmChannel<TIM3, C2>>;
type RightMotor = TwoPinDcMotorOut<PwmChannel<TIM3, C3>, PwmChannel<TIM3, C4>>;

impl motor::Avg for u16 { 
    fn avg(a: u16, b: u16) -> u16 { (a+b)/2 }
}

impl motor::Sample for u16 { }

impl SetChannels<QuadratureAdcPins> for Adc<ADC1> {
    fn set_samples(&mut self) {
        self.set_channel_sample_time(0, adc::SampleTime::T_28);
        self.set_channel_sample_time(1, adc::SampleTime::T_28);
        self.set_channel_sample_time(2, adc::SampleTime::T_28);
        self.set_channel_sample_time(3, adc::SampleTime::T_28);
    }

    fn set_sequence(&mut self) {
        self.set_regular_sequence(&[0, 1, 2, 3]);
    }
}

pub struct DifferentialQuadratureIdle<PINS> {
    buf: &'static mut [u16; 4],
    adc_dma: AdcDma<PINS, Scan>,
}

pub struct DifferentialQuadratureScanning<PINS> {
    transfer: Transfer<W, &'static mut [u16; 4], AdcDma<PINS, Scan>>,
}

pub enum DifferentialQuadratureScan<PINS> {
    None,
    Idle(DifferentialQuadratureIdle<PINS>),
    Scanning(DifferentialQuadratureScanning<PINS>)
}

impl <PINS> DifferentialQuadratureAnalogInput<u16> for DifferentialQuadratureScan<PINS> {
    // This checks to see if there is a transfer already in progress, and then if
    // that transfer is done
    fn read_nb(&mut self) -> Option<DifferentialQuadratureSamples<u16>> {
        let state = replace(self, Self::None);
        match state {
            Self::None => None,

            Self::Idle(idle) => {
                *self = Self::Scanning( 
                    DifferentialQuadratureScanning { transfer: idle.adc_dma.read(idle.buf), });
                None
            },

            Self::Scanning(scanning) if scanning.transfer.is_done() => {
                let (buf, adc_dma) = scanning.transfer.wait();
                let samples = DifferentialQuadratureSamples {
                    left: (buf[0], buf[1]),
                    right: (buf[2], buf[3])
                };
                *self = Self::Scanning(
                    DifferentialQuadratureScanning { transfer: adc_dma.read(buf) });
                Some(samples)
            },

            Self::Scanning(scanning) => {
                *self = Self::Scanning( 
                    DifferentialQuadratureScanning { transfer: scanning.transfer });
                None
            }
        }
    }
}


pub type Quadrature = DifferentialQuadratureScan<QuadratureAdcPins>;
pub type Motors = Differential<LeftMotor, RightMotor, u16, Quadrature>;

pub struct QuadratureAdcPins(PA0<Analog>, PA1<Analog>, PA2<Analog>, PA3<Analog>);


pub fn hardware<'a>() -> (CommandSerial, Motors) {
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

    // USART1
    let tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crl);
    let rx = gpioa.pa10;
    
    let command_serial = Serial::usart1(
        peripherals.USART1,
        (tx, rx),
        &mut afio.mapr,
        serial::Config::default().baudrate(115200.bps()),
        clocks,
        &mut rcc.apb2,
    );

    let motor_pwm_pins = (
        gpioa.pa6.into_alternate_push_pull(&mut gpioa.crl),
        gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl),
        gpiob.pb0.into_alternate_push_pull(&mut gpiob.crl),
        gpiob.pb1.into_alternate_push_pull(&mut gpiob.crl),
    );

    let (c1, c2, c3, c4) = Timer::tim3(peripherals.TIM3, &clocks, &mut rcc.apb1)
        .pwm::<Tim3NoRemap, _, _, _>(motor_pwm_pins, &mut afio.mapr, 10.khz()).split();

    let quadrature_adc = adc::Adc::adc1(peripherals.ADC1, &mut rcc.apb2, clocks);
    let quadrature_channels = QuadratureAdcPins(
        gpioa.pa0.into_analog(&mut gpioa.crl),
        gpioa.pa1.into_analog(&mut gpioa.crl),
        gpioa.pa2.into_analog(&mut gpioa.crl),
        gpioa.pa3.into_analog(&mut gpioa.crl)
    );

    let dma_ch1 = peripherals.DMA1.split(&mut rcc.ahb).1;
    let quadrature = Quadrature::Idle(DifferentialQuadratureIdle {
            adc_dma: quadrature_adc.with_scan_dma(quadrature_channels, dma_ch1), 
            buf: singleton!(: [u16; 4] = [0; 4]).unwrap(),
        }
    );

    let motors = Motors {
        left: DcMotor { 
            out: LeftMotor { out1: c1, out2: c2 }, 
            encoder: AnalogRotaryEncoder::new(u16::MAX/2),
        }, 
        right: DcMotor { 
            out: RightMotor { out1: c3, out2: c4 },
            encoder: AnalogRotaryEncoder::new(u16::MAX/2),
        },
        input: quadrature,
    };

    return (command_serial, motors);
}
