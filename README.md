# DC motor with quadrature encoder

Really, this project is just me starting an experiement using Rust and RTFM. I've done
something like this in C++ both in Chibios and MBed, so I can compare the experience.

This is mostly following instructions in [RTFM by example, Starting a new project][rtfm-by-example-new], and the [the embedded Rust book][book]

This currently suffers from [4463], so you can only build it with

    cargo build -p microcontroller

[book]: https://rust-embedded.github.io/book
[rtfm-by-example-new]: https://rtfm.rs/0.5/book/en/
[4463]: https://github.com/rust-lang/cargo/issues/4463
