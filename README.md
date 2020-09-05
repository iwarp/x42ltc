[![Build Status](https://github.com/jmaibaum/libltc/workflows/CI/badge.svg)](https://github.com/jmaibaum/libltc/actions?query=workflow%3A%22CI%22)
# Rust wrapper crates for x42’s libltc

This repository holds the sources for the x42ltc and x42ltc-sys crates.
libltc-sys provides the FFI bindings to the C library, while libltc provides a
safe Rust wrapper.

[libltc](https://x42.github.io/libltc) by Robin Gareus (a.k.a x42) supports the
decoding and encoding of Linear/Longitudinal Time Code (LTC) signals, which are
used for synchronisation in audio/video workflows.


## Naming

The name x42ltc was chosen, because the name [ltc](https://crates.io/crates/ltc)
has already been taken by another project, which seems to be working on a pure
Rust LTC library.
