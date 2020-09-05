// libltc: src/bin/ltcgenerator.rs
//
// Copyright 2019-2020 Johannes Maibaum <jmaibaum@gmail.com>
//
// This file is free software; you can redistribute it and/or modify it
// under the terms of the GNU Lesser General Public License as
// published by the Free Software Foundation; either version 3 of the
// License, or (at your option) any later version.
//
// This file is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public
// License along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: LGPL-3.0-or-later
use std::fs::File;
use std::io::Write;
use x42ltc::*;

fn to_bcd(n: u32) -> u32 {
    let mut remainder = n;
    let mut bcd = 0;
    for i in 0..8 {
        bcd |= (remainder % 10) << (i * 4);
        remainder /= 10;
        if remainder == 0 {
            break;
        }
    }
    bcd
}

fn main() {
    let length = 10; // in seconds
    let sample_rate = 48_000;
    let frames_per_second = 25;
    let mut encoder = Encoder::new(sample_rate, frames_per_second as f64).unwrap();
    let bcd = to_bcd(123);
    encoder.set_user_bits(bcd);

    let mut output_file = File::create("output.raw").unwrap();

    for _frame in 0..(length * frames_per_second) {
        encoder.encode_frame();
        output_file.write_all(encoder.get_buffer()).unwrap();
        encoder.increase_timecode();
    }
}
