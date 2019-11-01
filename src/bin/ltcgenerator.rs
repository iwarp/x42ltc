use ltc::*;
use std::fs::File;
use std::io::Write;

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
        output_file.write(encoder.get_buffer()).unwrap();
        encoder.increase_timecode();
    }
}
