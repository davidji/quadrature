
use embedded_hal::PwmPin;
// use stm32f1xx_hal::prelude::_embedded_hal_PwmPin as PwmPin;

pub enum Mode {
    Free, Brake
}

pub trait DcMotorOut {
    fn free(&mut self);
    fn brake(&mut self);
    fn drive(&mut self, duty: f32, mode: Mode);
}

pub struct TwoPinDcMotorOut<P1, P2>
where P1: PwmPin, P2: PwmPin
{
    pub out1: P1,
    pub out2: P2
}

pub trait DutyPair {
    fn set_duty(&mut self, duty1: f32, duty2: f32);
}

impl <P1, P2> DutyPair for TwoPinDcMotorOut<P1, P2>
where 
    P1: PwmPin<Duty = u16>, 
    P2: PwmPin<Duty = u16>,
{
    fn set_duty(&mut self, duty1: f32, duty2: f32) {
        self.out1.set_duty((duty1*(self.out1.get_max_duty() as f32)) as u16);
        self.out2.set_duty((duty2*(self.out2.get_max_duty() as f32)) as u16);
    }
}

impl <O> DcMotorOut for O
where O: DutyPair
{
    fn free(&mut self) {
        self.set_duty(0.0, 0.0);
    }

    fn brake(&mut self) {
        self.set_duty(1.0, 1.0);
    }

    fn drive(&mut self, duty: f32, mode: Mode) {
        match (duty, mode) {
            (duty, Mode::Free) if duty > 0.0 => self.set_duty(0.0, duty),
            (duty, Mode::Free) => self.set_duty(-duty, 0.0),
            (duty, Mode::Brake) if duty > 0.0 => self.set_duty(1.0, 1.0 - duty),
            (duty, Mode::Brake) => self.set_duty(1.0 + duty, 1.0),
        }
    }
}

pub struct DcMotor<O>
where O: DcMotorOut
{
    pub out: O,
}

pub struct Differential<O1,O2>
where O1: DcMotorOut, O2: DcMotorOut
{
    pub left: DcMotor<O1>,
    pub right: DcMotor<O2>,
}

