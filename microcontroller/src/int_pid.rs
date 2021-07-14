
use core::cmp::{ min, max };
use core::{ i32, i16 };

pub struct IntPid {
    // configuration
    kp: u32, 
    ki: u32, 
    kd: u32,
    out_max: i64, 
    out_min: i64,

    // state
    last_sp: i16, 
    last_out: i16,
    sum: i64,
    last_err: i32,
}

const PARAM_BITS: u64 = 16;
const PARAM_SHIFT: u64 = 8;
const PARAM_MAX: f32 = (((0x1 << PARAM_BITS)-1) >> PARAM_SHIFT) as f32;
const PARAM_MULT: f32 = (((0x1 << PARAM_BITS)) >> (PARAM_BITS - PARAM_SHIFT)) as f32;

#[derive(Debug)]
enum PidError {
    Overflow,
    Underflow,
}

fn float_to_param(f_p: f32) -> Result<u32, PidError> {
    if f_p > PARAM_MAX || f_p < 0.0 {
        return Err(PidError::Overflow);
    } else {
        let p : u32 = (f_p * PARAM_MULT) as u32;
        if p == 0 && f_p != 0.0 {
            return Err(PidError::Underflow);
        }
        return Ok(p);
    }
}

impl IntPid {
    pub fn new() -> IntPid {
        IntPid {
            kp: 0, ki: 0, kd: 0,
            out_min: 0, 
            out_max: 0,
            last_sp: 0, last_out: 0, sum: 0, last_err: 0
        }.with_output_range(i16::MIN, i16::MAX)
    }

    pub fn with_coefficients(self, f_kp: f32, f_ki: f32, f_kd: f32, hz:f32) -> IntPid {
        return IntPid {
            kp: float_to_param(f_kp).unwrap(),
            ki: float_to_param(f_ki/hz).unwrap(),
            kd: float_to_param(f_kd*hz).unwrap(),
            ..self
        };
    }

    pub fn with_output_range(self, min: i16, max: i16) -> IntPid {
        return IntPid {
            out_min: (min as i64) * (PARAM_MULT as i64),
            out_max: (max as i64) * (PARAM_MULT as i64), 
            ..self
        }
    }

    pub fn step(&mut self, sp : i32, fb : i16) -> i16 {
        // int16 + int16 = int17
        let err : i32 = sp as i32 - fb as i32;
        
        // uint16 * int16 = int32
        let p : i32 = match self.kp {
            0 => 0,
            kp => kp as i32 * err
        };
        

        let i : i32 = match self.ki {
            0 => 0,
            ki => {
                // int17 * int16 = int33
                self.sum += err as i64 * ki as i64;
                // Limit sum to 32-bit signed value so that it saturates, never overflows.
                self.sum = min(i32::MAX as i64, self.sum);
                self.sum as i32
            }
        };

        let d : i32 = match self.kd {
            0 => 0,
            kd => {
                // (int17 - int16) - (int16 - int16) = int19
                // Limit the derivative to 16-bit signed value.
                let derivative : i32 = max(i16::MIN as i32, min(i16::MAX as i32, 
                    (err - self.last_err) - (sp - self.last_sp as i32)));
                self.last_sp = sp as i16;
                self.last_err = err;
                kd as i32 * derivative
            }
        };

        let out = max(self.out_min, min(self.out_max, p as i64 + i as i64 + d as i64));
        let scaled : i16 = (out >> PARAM_SHIFT) as i16;
        match out & (0x1 << (PARAM_SHIFT - 1)) {
            0 => scaled,
            _ => scaled + 1
        }
     }
}