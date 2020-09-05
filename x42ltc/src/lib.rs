// x42ltc: src/lib.rs
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
use std::convert::TryInto;
use x42ltc_sys as ffi;

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
    ///   dynamically. Setting this into the right ballpark is needed to properly decode the first
    ///   LTC frame.
    /// - `queue_size` sets the length of the internal queue to store decoded frames.
    ///
    /// # Example
    ///
    /// ```
    /// let decoder = x42ltc::Decoder::new(32, 1920).unwrap();
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
    /// Calls [`libltc_sys::ltc_encoder_reinit()`](../libltc_sys/fn.ltc_encoder_reinit.html) inside, see
    /// notes there or see notes for [`.reinitialize()`](#method.reinitialize).
    ///
    /// # Example
    ///
    /// ```
    /// let encoder = x42ltc::Encoder::new(48_000, 25.0).unwrap();
    /// ```
    pub fn new(sample_rate: u32, fps: f64) -> Result<Encoder, Error> {
        let pointer = unsafe {
            ffi::ltc_encoder_create(
                f64::from(sample_rate),
                fps,
                // Position of binary group flags is only different for 25 fps
                if (fps - 25.0).abs() < std::f64::EPSILON {
                    ffi::LTC_TV_STANDARD_LTC_TV_625_50
                } else {
                    ffi::LTC_TV_STANDARD_LTC_TV_525_60
                },
                ffi::LTC_BG_FLAGS_LTC_BGF_DONT_TOUCH as i32,
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

    /// Encode a full LTC frame at fixed speed.  This is equivalent to calling
    /// [`.encode_byte()`](#method.encode_byte) 10 times for bytes 0..=9 with speed 1.0.
    ///
    /// # Note
    ///
    /// The internal buffer must be empty before calling this function.  Otherwise it may overflow.
    /// This is usually the case if it is read with [`.get_buffer()`](#method.get_buffer) after
    /// calling this function.
    ///
    /// The default internal buffer size is exactly one full LTC frame at speed 1.0.
    pub fn encode_frame(&mut self) {
        unsafe {
            ffi::ltc_encoder_encode_frame(self.pointer);
        }
    }

    /// Resets the write-pointer of the encoded buffer.
    pub fn flush_buffer(&mut self) {
        unsafe {
            ffi::ltc_encoder_buffer_flush(self.pointer);
        }
    }

    /// Copy the accumulated encoded audio to the given sample buffer and flush the internal buffer.
    ///
    /// # Undefined behaviour warning
    ///
    /// It is the caller's responsibility to make sure that the capacity of the supplied buffer is
    /// large enough to hold all of the accumulated data, otherwise the program will most likely be
    /// aborted due to the internal FFI call causing a segmentation fault, which is UB.
    ///
    /// # Example
    ///
    /// ```
    /// let sample_rate = 48_000;
    /// let frames_per_second = 25;
    ///
    /// // Create a large enough buffer
    /// let mut audio_buffer = Vec::with_capacity((sample_rate / frames_per_second as u32) as usize);
    ///
    /// let mut encoder = x42ltc::Encoder::new(sample_rate, frames_per_second as f64).unwrap();
    /// encoder.encode_frame();
    /// assert_eq!(
    ///     encoder.copy_audio_to_buffer(&mut audio_buffer),
    ///     (sample_rate / frames_per_second as u32) as usize,
    /// );
    /// ```
    pub fn copy_audio_to_buffer(&mut self, buffer: &mut [u8]) -> usize {
        let copied_len = unsafe { ffi::ltc_encoder_get_buffer(self.pointer, buffer.as_mut_ptr()) };
        copied_len as usize
    }

    /// Returns a slice to the internal buffer of accumulated audio samples, and flushes buffer
    /// afterwards.
    ///
    /// # Example
    ///
    /// ```
    /// let mut encoder = x42ltc::Encoder::new(48_000, 25.0).unwrap();
    /// encoder.encode_frame();
    /// let buffer = encoder.get_buffer();
    /// assert_eq!(buffer.len(), 48_000 / 25);
    /// ```
    pub fn get_buffer(&self) -> &[u8] {
        let mut buf_len = 0;
        let buf_ptr = unsafe { ffi::ltc_encoder_get_bufptr(self.pointer, &mut buf_len, 1) };
        unsafe { std::slice::from_raw_parts(buf_ptr, buf_len as usize) }
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

    /// Query the length of the internal buffer. It is allocated to hold audio frames for exactly
    /// one LTC frame for the given sample rate and frame rate, i.e. (1 + sample_rate / fps) bytes.
    ///
    /// # Note
    ///
    /// This returns the total size of the buffer, not the used/free part. See also
    /// [`.get_buffer()`](#method.get_buffer).
    ///
    /// # Example
    ///
    /// ```
    /// let encoder = x42ltc::Encoder::new(48_000, 25.0).unwrap();
    /// assert_eq!(encoder.get_buffer_size(), (1 + 48_000 / 25) as usize);
    /// ```
    pub fn get_buffer_size(&self) -> usize {
        unsafe { ffi::ltc_encoder_get_buffersize(self.pointer) }
    }

    /// Get the 32 bit unsigned integer from the user data bits of the current frame. The data
    /// should have been written LSB first into the frame.
    ///
    /// # Example
    ///
    /// ```
    /// let mut encoder = x42ltc::Encoder::new(48_000, 25.0).unwrap();
    /// encoder.set_user_bits(12345);
    /// assert_eq!(encoder.get_user_bits(), 12345);
    /// ```
    pub fn get_user_bits(&self) -> u32 {
        let mut frame = self.get_frame();
        unsafe {
            // We can unwrap here, since user bits is actually u32 in libltc_sys
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
    /// let mut encoder = x42ltc::Encoder::new(48_000, 25.0).unwrap();
    /// let result = encoder.reinitialize(44_100, 25.0);
    /// assert!(result.is_ok());
    /// ```
    pub fn reinitialize(&mut self, sample_rate: u32, fps: f64) -> Result<(), Error> {
        let rv = unsafe {
            ffi::ltc_encoder_reinit(
                self.pointer,
                f64::from(sample_rate),
                fps, // Position of binary group flags is only different for 25 fps
                if (fps - 25.0).abs() < std::f64::EPSILON {
                    ffi::LTC_TV_STANDARD_LTC_TV_625_50
                } else {
                    ffi::LTC_TV_STANDARD_LTC_TV_525_60
                },
                ffi::LTC_BG_FLAGS_LTC_BGF_DONT_TOUCH as i32,
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
    /// let mut encoder = x42ltc::Encoder::new(48_000, 25.0).unwrap();
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
    ///
    /// # Example
    ///
    /// Generate a perfect square wave LTC signal:
    ///
    /// ```
    /// let mut encoder = x42ltc::Encoder::new(48_000, 25.0).unwrap();
    /// encoder.set_volume(0.0);  // so that logical one == 255u8
    /// encoder.set_filter(0.0);  // perfect square wave
    /// encoder.encode_frame();
    /// assert_eq!(encoder.get_buffer()[0], 255u8);  // First sample is alsways logical 1
    /// ```
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
    /// let mut encoder = x42ltc::Encoder::new(48_000, 25.0).unwrap();
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

unsafe impl Send for Encoder {}

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
