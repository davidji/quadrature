# DC motor with quadrature encoder

Really, this project is just me starting an experiement using Rust and RTFM. I've done
something like this in C++ both in Chibios and MBed, so I can compare the experience.

This is mostly following instructions in [RTFM by example, Starting a new project][rtfm-by-example-new], and the [the embedded Rust book][book]

This currently suffers from [4463], so you have to build the client and the microcontroller
part separately:

    cargo build

and

   (cd microcontroller; cargo build)

The point of this project is to use signal processing to allow you to connect the
output of a quadrature encoder strait to analog inputs, rather than have an external
circuit pre-processing it for digital inputs. That requires a fast ADC, so you can't,
for example, do it with a Raspberry PI. You can do it with something like an
STM32F103, with you can find in a blue pill. Pretty much any STM32 microcontroller
could be made to work: they all have fast multi-channel ADCs.

The basic set-up is to have a raspberry pi (or any other Linux machine) connected to a
microcontroller via a serial connection. It's not obvious if I'll manage to get this
working with USB. For now, it uses a UART for communication.

[book]: https://rust-embedded.github.io/book
[rtfm-by-example-new]: https://rtfm.rs/0.5/book/en/
[4463]: https://github.com/rust-lang/cargo/issues/4463
