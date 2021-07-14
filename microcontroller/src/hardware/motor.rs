
// use super::super::int_pid::IntPid;
use core::cmp::{ min, max };
use core::option::Option;
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

pub trait Avg {
    fn avg(a: Self, b: Self) -> Self;
}

pub trait Sample: Ord + Copy + Avg { }

struct MinMax<S: Sample>  {
    min: S,
    max: S,
    zero: S,
}

impl <S: Sample> MinMax<S> {

    pub fn new(zero: S) -> Self {
        MinMax { min: zero, max: zero, zero: zero }
    }

    pub fn update(&mut self, value: S) -> bool {
        if self.max < value || self.min > value {
            self.min = min(self.min, value);
            self.max = max(self.max, value);
            self.zero = Avg::avg(self.min, self.max);
        }

        value > self.zero
    }
}

pub struct AnalogRotaryEncoder<S: Sample> {
    in1: MinMax<S>,
    in2: MinMax<S>,
    counter: u64,
    in1_prev_value: bool,
    delta_r: i64,
}

impl <S: Sample> AnalogRotaryEncoder<S> {
    pub fn new(zero: S) -> Self {
        AnalogRotaryEncoder {
            in1: MinMax::new(zero),
            in2: MinMax::new(zero),
            counter: 0,
            in1_prev_value: false,
            delta_r : 0,
        }
    }

    pub fn update(&mut self, values: (S, S)) {
        self.counter += 1;
        let in1_next_value = self.in1.update(values.0);
        if !self.in1_prev_value && in1_next_value {
            let in2_value = self.in2.update(values.1);
            self.delta_r += match in2_value { true => 1, false => -1 };
        }
    
        self.in1_prev_value = in1_next_value;
    }

    pub fn read(&mut self) -> i64 {
        let delta = self.delta_r;
        self.delta_r = 0;
        return delta;
    }

    pub fn peek(& self) -> i64 {
        return self.delta_r;
    }
}

pub struct DcMotor<O, S: Sample>
where O: DcMotorOut
{
    pub out: O,
    pub encoder:  AnalogRotaryEncoder<S>
}

pub struct DifferentialQuadratureSamples<S>
where S: Sample
{
    pub left: (S, S),
    pub right: (S, S)
}

pub trait DifferentialQuadratureAnalogInput<S> 
where S: Sample
{
    fn read_nb(&mut self) -> Option<DifferentialQuadratureSamples<S>>;
}

pub struct Differential<O1, O2, S, I>
where O1: DcMotorOut, O2: DcMotorOut, S: Sample, I: DifferentialQuadratureAnalogInput<S>
{
    pub left: DcMotor<O1, S>,
    pub right: DcMotor<O2, S>,
    pub input: I
}

impl <O1, O2, S, I> Differential<O1, O2, S, I>
where O1: DcMotorOut, O2: DcMotorOut, S: Sample, I: DifferentialQuadratureAnalogInput<S> {
    fn update_encoders(&mut self, input: DifferentialQuadratureSamples<S>) {
        self.left.encoder.update(input.left);
        self.right.encoder.update(input.right);
    }

    pub fn update(&mut self) {
        let result = self.input.read_nb();
        match result {
            Some(samples) => self.update_encoders(samples),
            None => {}
        }
    }
}

