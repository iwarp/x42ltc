use ltc_sys as ffi;
use std::convert::TryInto;

#[derive(Debug)]
pub enum Error {
    AllocationFailed,
    ReinitializationFailed,
    ValueOutOfRange,
}

pub struct Decoder {
    pointer: *mut ffi::LTCDecoder,
}

impl Decoder {
    /// Create a new LTC decoder.
    ///
    /// - `audio_frames_per_video_frame` is only used for initial settings, the speed is tracked
    ///   dynamically. Settings this into the right ballpark is needed to properly decode the first
    ///   LTC frame.
    /// - `queue_size` sets the length of the internal queue to store decoded frames.
    ///
    /// # Example
    ///
    /// ```
    /// let decoder = ltc::Decoder::new(32, 1920).unwrap();
    /// ```
    pub fn new(audio_frames_per_video_frame: i32, queue_size: i32) -> Result<Decoder, Error> {
        let pointer = unsafe { ffi::ltc_decoder_create(audio_frames_per_video_frame, queue_size) };

        if pointer.is_null() {
            Err(Error::AllocationFailed)
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
    /// Allocate and initialize LTC audio encoder.
    ///
    /// Calls [`ltc_sys::ltc_encoder_reinit()`](../ltc_sys/fn.ltc_encoder_reinit.html) inside, see
    /// notes there or see notes for [`.reinitialize()`](#method.reinitialize).
    ///
    /// # Example
    ///
    /// ```
    /// let encoder = ltc::Encoder::new(48000, 25.0).unwrap();
    /// ```
    pub fn new(sample_rate: u32, fps: f64) -> Result<Encoder, Error> {
        let pointer = unsafe {
            ffi::ltc_encoder_create(
                f64::from(sample_rate),
                fps,
                // Position of binary group flags is only different for 25 fps
                if fps == 25.0 {
                    ffi::LTC_TV_STANDARD_LTC_TV_625_50
                } else {
                    ffi::LTC_TV_STANDARD_LTC_TV_525_60
                },
                ffi::LTC_BG_FLAGS_LTC_USE_DATE as i32,
            )
        };

        if pointer.is_null() {
            Err(Error::AllocationFailed)
        } else {
            Ok(Encoder { pointer })
        }
    }

    /// Move the encoder to the previous timecode frame. This is useful for encoding reverse LTC.
    pub fn decrease_timecode(&mut self) {
        unsafe {
            ffi::ltc_encoder_dec_timecode(self.pointer);
        }
    }

    /// Resets the write-pointer of the encoded buffer.
    pub fn flush_buffer(&mut self) {
        unsafe {
            ffi::ltc_encoder_buffer_flush(self.pointer);
        }
    }

    fn get_frame(&self) -> Frame {
        let mut frame = ffi::LTCFrame {
            _bitfield_1: ffi::LTCFrame::new_bitfield_1(
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ),
            ..Default::default()
        };
        unsafe {
            ffi::ltc_encoder_get_frame(self.pointer, &mut frame);
        }
        Frame { frame }
    }

    /// Get the 32 bit unsigned integer from the user data bits of the current frame. The data
    /// should have been written LSB first into the frame.
    ///
    /// # Example
    ///
    /// ```
    /// let mut encoder = ltc::Encoder::new(48000, 25.0).unwrap();
    /// encoder.set_user_bits(12345);
    /// assert_eq!(encoder.get_user_bits(), 12345);
    /// ```
    pub fn get_user_bits(&self) -> u32 {
        let mut frame = self.get_frame();
        unsafe {
            // We can unwrap here, since user bits is actually u32 in ltc_sys
            ffi::ltc_frame_get_user_bits(&mut frame.frame)
                .try_into()
                .unwrap()
        }
    }

    /// Move the encoder to the next timecode frame.
    pub fn increase_timecode(&mut self) {
        unsafe {
            ffi::ltc_encoder_inc_timecode(self.pointer);
        }
    }

    /// Change the encoder's settings without reallocating any library internal data structure
    /// (realtime safe). Changing the `fps` and/or `sample_rate` implies a buffer flush, and a
    /// biphase state reset.
    ///
    /// This call will fail if the internal buffer is too small to hold one full LTC frame. Use
    /// [`.set_buffer_size()`](#method.set_buffer_size) to prepare an internal buffer large enough
    /// to accomodate all `sample_rate` and `fps` combinations that you would like to reinitialize
    /// to.
    ///
    /// The LTC frame payload data is not modified by this call, however, the flag bits of the LTC
    /// frame are updated: If `fps` equals to `29.97` or `30000.0/1001.0` the `Frame`'s `dfbit` bit
    /// is set to `1` to indicate drop-frame timecode.
    ///
    /// # Example
    ///
    /// ```
    /// let mut encoder = ltc::Encoder::new(48_000, 25.0).unwrap();
    /// let result = encoder.reinitialize(44_100, 25.0);
    /// assert!(result.is_ok());
    /// ```
    pub fn reinitialize(&mut self, sample_rate: u32, fps: f64) -> Result<(), Error> {
        let rv = unsafe {
            ffi::ltc_encoder_reinit(
                self.pointer,
                f64::from(sample_rate),
                fps, // Position of binary group flags is only different for 25 fps
                if fps == 25.0 {
                    ffi::LTC_TV_STANDARD_LTC_TV_625_50
                } else {
                    ffi::LTC_TV_STANDARD_LTC_TV_525_60
                },
                ffi::LTC_BG_FLAGS_LTC_USE_DATE as i32,
            )
        };
        match rv {
            0 => Ok(()),
            _ => Err(Error::ReinitializationFailed),
        }
    }

    /// Reset encoder state. Flushes buffer and resets biphase state.
    pub fn reset(&mut self) {
        unsafe {
            ffi::ltc_encoder_reset(self.pointer);
        }
    }

    /// Configure a custom size for the internal buffer.
    ///
    /// This is needed if you are planning to call [`.reinitialize()`](#method.reinitialize) or if
    /// you want to keep more than one LTC frame's worth of data in the library's internal buffer.
    ///
    /// The buffer size is `(1 + sample_rate / fps) bytes. Resizing the internal buffer will flush
    /// all existing data in it, alike [`.flush_buffer()`](#method.flush_buffer).
    ///
    /// # Caution
    ///
    /// If this call returns `Error::AllocationFailed` the encoder is in a unusable state. Drop it
    /// or reallocate the buffer.
    ///
    /// # Example
    ///
    /// ```
    /// let mut encoder = ltc::Encoder::new(48_000, 25.0).unwrap();
    /// let result = encoder.set_buffer_size(192_000, 25.0);
    /// assert!(result.is_ok());
    /// ```
    pub fn set_buffer_size(&mut self, sample_rate: u32, fps: f64) -> Result<(), Error> {
        let rv = unsafe { ffi::ltc_encoder_set_bufsize(self.pointer, f64::from(sample_rate), fps) };
        match rv {
            0 => Ok(()),
            _ => Err(Error::AllocationFailed),
        }
    }

    /// Set encoder signal rise time / signal filtering.
    ///
    /// A LTC signal should have a rise time of 40µs +/- 10µs. By default the encoder honors this
    /// and low-pass filters the output depending on the sample rate.
    ///
    /// If you want a perfect square wave, set `rise_time` to `0.0`.
    ///
    /// # Note
    /// [`.reinitialize()`](#method.reinitialize) resets the filter time constant to use the default
    /// 40µs for the given sample rate, overriding any value previously set with this method.
    pub fn set_filter(&mut self, rise_time: f64) {
        unsafe {
            ffi::ltc_encoder_set_filter(self.pointer, rise_time);
        }
    }

    /// Set the user bits of the current frame to the given data. The data is written LSB first into
    /// the eight user bit fields.
    ///
    /// # Example
    ///
    /// ```
    /// let mut encoder = ltc::Encoder::new(48000, 25.0).unwrap();
    /// encoder.set_user_bits(98765);
    /// assert_eq!(encoder.get_user_bits(), 98765);
    /// ```
    pub fn set_user_bits(&mut self, user_bits: u32) {
        unsafe {
            ffi::ltc_encoder_set_user_bits(self.pointer, u64::from(user_bits));
        }
    }

    /// Set the volume of the generated LTC signal.
    ///
    /// Typically, LTC is sent at 0dBu; in EBU calibrated systems this corresponds to -18dBFS. By
    /// default, libltc creates a -3dBFS LTC signal.
    ///
    /// Since libltc generates 8 bit audio data, the minimum dBFS is about -42dB which corresponds
    /// to 1 bit.
    ///
    /// 0dB corresponds to a signal range of 127 1..255 with 128 at the center.
    ///
    /// # Return value
    ///
    /// Returns `Error::ValueOutOfRange` if `volume_in_dbfs` is > `0.0`.
    pub fn set_volume(&mut self, volume_in_dbfs: f64) -> Result<(), Error> {
        let rv = unsafe { ffi::ltc_encoder_set_volume(self.pointer, volume_in_dbfs) };
        match rv {
            0 => Ok(()),
            _ => Err(Error::ValueOutOfRange),
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
mod tests {
    use super::*;
    #[test]
    fn encoder_out_of_range_volume_errors() {
        let mut encoder = Encoder::new(48_000, 25.0).unwrap();
        assert!(encoder.set_volume(1.0).is_err());
    }

    #[test]
    fn encoder_reinitialization_fails_if_internal_buffer_is_too_small() {
        let mut encoder = Encoder::new(48_000, 25.0).unwrap();
        assert!(encoder.reinitialize(192_000, 25.0).is_err());
    }
}
