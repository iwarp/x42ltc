use ltc_sys::*;
use std::ffi::CStr;
use std::fs::File;
use std::io::Read;

/// Adapted C test code from vendor/tests/ltcdecode.c
fn decode_raw(mut data: &[u8], apv: Option<i32>) -> String {
    unsafe {
        let apv = apv.unwrap_or(1920);
        let mut sound = [0u8; 1024];

        let mut total: usize = 0;
        let decoder = ltc_decoder_create(apv, 32);
        let mut output = String::new();
        loop {
            let n = data.read(&mut sound).unwrap();
            ltc_decoder_write(decoder, sound.as_mut_ptr(), n, total as i64);

            loop {
                let mut frame = LTCFrameExt {
                    biphase_tics: [0.0f32; 80],
                    ltc: LTCFrame {
                        __bindgen_padding_0: 0,
                        _bitfield_1: __BindgenBitfieldUnit::new([0u8; 10]),
                    },
                    off_start: 0,
                    off_end: 0,
                    reverse: 0,
                    sample_max: 0,
                    sample_min: 0,
                    volume: 0.0,
                };
                let read_ret = ltc_decoder_read(decoder, &mut frame as *mut LTCFrameExt);
                if read_ret == 0 {
                    break;
                }
                let mut stime = SMPTETimecode {
                    timezone: [0; 6],
                    years: 0,
                    months: 0,
                    days: 0,
                    hours: 0,
                    mins: 0,
                    secs: 0,
                    frame: 0,
                };
                ltc_frame_to_time(
                    &mut stime as *mut SMPTETimecode,
                    &mut frame.ltc as *mut LTCFrame,
                    1,
                );
                output.push_str(&format!(
                    "{:04}-{:02}-{:02} {} {:02}:{:02}:{:02}{}{:02} | {:8} {:8}{}\n",
                    if stime.years < 67 {
                        2000 + stime.years as i32
                    } else {
                        1900 + stime.years as i32
                    },
                    stime.months,
                    stime.days,
                    CStr::from_ptr(&stime.timezone as *const _)
                        .to_str()
                        .unwrap(),
                    stime.hours,
                    stime.mins,
                    stime.secs,
                    if frame.ltc.dfbit() != 0 { '.' } else { ':' },
                    stime.frame,
                    frame.off_start,
                    frame.off_end,
                    if frame.reverse != 0 { "  R" } else { "" },
                ));
            }

            total += n;
            if n == 0 {
                break;
            }
        }

        output
    }
}

/// Adapted C test code from vendor/tests/ltcencode.c
fn encode_raw(sample_rate: Option<f64>) -> Vec<u8> {
    let length = 2.0;
    let fps = 25.0;
    let sample_rate = sample_rate.unwrap_or(48000.0);
    let last_frame = (length * fps) as i32;

    let mut st = SMPTETimecode {
        timezone: [43, 48, 49, 48, 48, 0], // b"+0100\0"
        years: 8,
        months: 12,
        days: 31,
        hours: 23,
        mins: 59,
        secs: 59,
        frame: 0,
    };

    let mut output_buffer = Vec::new();

    unsafe {
        let encoder = ltc_encoder_create(1.0, 1.0, 0, LTC_BG_FLAGS_LTC_USE_DATE as i32);
        ltc_encoder_set_bufsize(encoder, sample_rate, fps);
        ltc_encoder_reinit(
            encoder,
            sample_rate,
            fps,
            LTC_TV_STANDARD_LTC_TV_625_50,
            LTC_BG_FLAGS_LTC_USE_DATE as i32,
        );
        ltc_encoder_set_filter(encoder, 0.0);
        ltc_encoder_set_filter(encoder, 25.0);
        ltc_encoder_set_volume(encoder, -18.0);
        ltc_encoder_set_timecode(encoder, &mut st as *mut SMPTETimecode);

        let mut frame_cnt = 0;

        loop {
            if frame_cnt < last_frame {
                frame_cnt += 1;
            } else {
                break;
            }
            let buf: *mut ltcsnd_sample_t;
            let mut len = 0;

            ltc_encoder_encode_frame(encoder);
            buf = ltc_encoder_get_bufptr(encoder, &mut len, 1);

            if len > 0 {
                output_buffer.extend_from_slice(std::slice::from_raw_parts_mut(buf, len as usize));
            }

            ltc_encoder_inc_timecode(encoder);
        }

        ltc_encoder_free(encoder);
    }
    output_buffer
}

#[test]
/// Adapted C test code from vendor/tests/ltc{de,en}code.c
fn ltc_encode_then_decode_48000() {
    let encoded_timecode = encode_raw(None);
    let decoded_output = decode_raw(&encoded_timecode, None);

    let mut expect_file = File::open("vendor/tests/expect_48k_2sec.txt").unwrap();
    let mut expected_output = String::new();
    expect_file.read_to_string(&mut expected_output).unwrap();
    assert_eq!(decoded_output, expected_output);
}

#[test]
/// Adapted C test code from vendor/tests/ltc{de,en}code.c
fn ltc_encode_then_decode_192000() {
    let encoded_timecode = encode_raw(Some(192_000.0));
    let decoded_output = decode_raw(&encoded_timecode, Some(7680));

    let mut expect_file = File::open("vendor/tests/expect_96k_2sec.txt").unwrap();
    let mut expected_output = String::new();
    expect_file.read_to_string(&mut expected_output).unwrap();
    assert_eq!(decoded_output, expected_output);
}

#[test]
/// Decode raw test file provided in vendor/tests/timecode.raw
fn ltc_decode_timecode_dot_raw() {
    let mut raw_file = File::open("vendor/tests/timecode.raw").unwrap();
    let mut raw_contents = Vec::new();
    raw_file.read_to_end(&mut raw_contents).unwrap();
    let decoded_output = decode_raw(&raw_contents, Some(882));

    let mut expect_file = File::open("vendor/tests/timecode.txt").unwrap();
    let mut expected_output = String::new();
    expect_file.read_to_string(&mut expected_output).unwrap();

    assert_eq!(decoded_output, expected_output);
}
