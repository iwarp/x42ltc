use ltc_sys as ffi;
use std::convert::TryInto;

#[derive(Debug)]
pub enum Error {
    OutOfMemoryError,
}

pub struct Decoder {
    pointer: *mut ffi::LTCDecoder,
}

impl Decoder {
    /// # Example
    ///
    /// ```
    /// let decoder = ltc::Decoder::new(32, 1920).unwrap();
    /// ```
    pub fn new(audio_frames_per_video_frame: i32, queue_size: i32) -> Result<Decoder, Error> {
        let pointer = unsafe { ffi::ltc_decoder_create(audio_frames_per_video_frame, queue_size) };

        if pointer.is_null() {
            Err(Error::OutOfMemoryError)
        } else {
            Ok(Decoder { pointer })
        }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe {
            ffi::ltc_decoder_free(self.pointer);
        }
    }
}

pub struct Encoder {
    pointer: *mut ffi::LTCEncoder,
}

impl Encoder {
    /// # Example
    ///
    /// ```
    /// let encoder = ltc::Encoder::new(48000, 25).unwrap();
    /// ```
    pub fn new(sample_rate: u32, fps: u32) -> Result<Encoder, Error> {
        let pointer = unsafe {
            ffi::ltc_encoder_create(
                sample_rate as f64,
                fps as f64,
                // Position of binary group flags is only different for 25 fps
                if fps == 25 {
                    ffi::LTC_TV_STANDARD_LTC_TV_625_50
                } else {
                    ffi::LTC_TV_STANDARD_LTC_TV_525_60
                },
                ffi::LTC_BG_FLAGS_LTC_USE_DATE as i32,
            )
        };

        if pointer.is_null() {
            Err(Error::OutOfMemoryError)
        } else {
            Ok(Encoder { pointer })
        }
    }

    /// Resets the write-pointer of the encoded buffer
    pub fn flush_buffer(&mut self) {
        unsafe {
            ffi::ltc_encoder_buffer_flush(self.pointer);
        }
    }

    pub fn decrease_timecode(&mut self) {
        unsafe {
            ffi::ltc_encoder_dec_timecode(self.pointer);
        }
    }

    /// # Example
    ///
    /// ```
    /// let mut encoder = ltc::Encoder::new(48000, 25).unwrap();
    /// encoder.set_user_bits(12345);
    /// assert_eq!(encoder.get_user_bits(), 12345);
    /// ```
    pub fn get_user_bits(&self) -> u32 {
        unsafe {
            let mut frame = self.get_frame();
            // We can unwrap here, since user bits is actually u32 in ltc_sys
            ffi::ltc_frame_get_user_bits(&mut frame.frame)
                .try_into()
                .unwrap()
        }
    }

    /// # Example
    ///
    /// ```
    /// let mut encoder = ltc::Encoder::new(48000, 25).unwrap();
    /// encoder.set_user_bits(98765);
    /// assert_eq!(encoder.get_user_bits(), 98765);
    /// ```
    pub fn set_user_bits(&mut self, user_bits: u32) {
        unsafe {
            ffi::ltc_encoder_set_user_bits(self.pointer, user_bits as u64);
        }
    }

    fn get_frame(&self) -> Frame {
        unsafe {
            let mut frame = ffi::LTCFrame {
                __bindgen_padding_0: 0,
                _bitfield_1: ffi::__BindgenBitfieldUnit::new([0u8; 10]),
            };
            ffi::ltc_encoder_get_frame(self.pointer, &mut frame);
            Frame { frame }
        }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            ffi::ltc_encoder_free(self.pointer);
        }
    }
}

struct Frame {
    frame: ffi::LTCFrame,
}

#[cfg(test)]
mod tests {}
