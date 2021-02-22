use std::io::{self, ErrorKind as IoErrorKind, Read};
use std::mem;

use byteorder::{LittleEndian, ReadBytesExt};
use log::error;
use songbird::constants::{MONO_FRAME_BYTE_SIZE, STEREO_FRAME_BYTE_SIZE, STEREO_FRAME_SIZE};

/// Extension trait to pull frames of audio from a byte source.
pub trait ReadAudioExt {
    fn add_float_pcm_frame(
        &mut self, float_buffer: &mut [f32; STEREO_FRAME_SIZE], true_stereo: bool, volume: f32,
    ) -> Option<usize>;

    fn consume(&mut self, amt: usize) -> usize
    where
        Self: Sized;
}

impl<R: Read + Sized> ReadAudioExt for R {
    fn add_float_pcm_frame(
        &mut self, float_buffer: &mut [f32; STEREO_FRAME_SIZE], stereo: bool, volume: f32,
    ) -> Option<usize> {
        // IDEA: Read in 8 floats at a time, then use iterator code
        // to gently nudge the compiler into vectorising for us.
        // Max SIMD float32 lanes is 8 on AVX, older archs use a divisor of this
        // e.g., 4.
        const SAMPLE_LEN: usize = mem::size_of::<f32>();
        const FLOAT_COUNT: usize = 512;
        let mut simd_float_bytes = [0_u8; FLOAT_COUNT * SAMPLE_LEN];
        let mut simd_float_buf = [0_f32; FLOAT_COUNT];

        let mut frame_pos = 0;

        // Code duplication here is because unifying these codepaths
        // with a dynamic chunk size is not zero-cost.
        if stereo {
            let mut max_bytes = STEREO_FRAME_BYTE_SIZE;

            while frame_pos < float_buffer.len() {
                let progress = self
                    .read(&mut simd_float_bytes[..max_bytes.min(FLOAT_COUNT * SAMPLE_LEN)])
                    .and_then(|byte_len| {
                        let target = byte_len / SAMPLE_LEN;
                        (&simd_float_bytes[..byte_len])
                            .read_f32_into::<LittleEndian>(&mut simd_float_buf[..target])
                            .map(|_| target)
                    })
                    .map(|f32_len| {
                        let new_pos = frame_pos + f32_len;
                        for (el, new_el) in float_buffer[frame_pos..new_pos]
                            .iter_mut()
                            .zip(&simd_float_buf[..f32_len])
                        {
                            *el += volume * new_el;
                        }
                        (new_pos, f32_len)
                    });

                match progress {
                    Ok((new_pos, delta)) => {
                        frame_pos = new_pos;
                        max_bytes -= delta * SAMPLE_LEN;

                        if delta == 0 {
                            break;
                        }
                    },
                    Err(ref e) => {
                        return if e.kind() == IoErrorKind::UnexpectedEof {
                            error!("EOF unexpectedly: {:?}", e);
                            Some(frame_pos)
                        } else {
                            error!("Input died unexpectedly: {:?}", e);
                            None
                        }
                    },
                }
            }
        } else {
            let mut max_bytes = MONO_FRAME_BYTE_SIZE;

            while frame_pos < float_buffer.len() {
                let progress = self
                    .read(&mut simd_float_bytes[..max_bytes.min(FLOAT_COUNT * SAMPLE_LEN)])
                    .and_then(|byte_len| {
                        let target = byte_len / SAMPLE_LEN;
                        (&simd_float_bytes[..byte_len])
                            .read_f32_into::<LittleEndian>(&mut simd_float_buf[..target])
                            .map(|_| target)
                    })
                    .map(|f32_len| {
                        let new_pos = frame_pos + (2 * f32_len);
                        for (els, new_el) in float_buffer[frame_pos..new_pos]
                            .chunks_exact_mut(2)
                            .zip(&simd_float_buf[..f32_len])
                        {
                            let sample = volume * new_el;
                            els[0] += sample;
                            els[1] += sample;
                        }
                        (new_pos, f32_len)
                    });

                match progress {
                    Ok((new_pos, delta)) => {
                        frame_pos = new_pos;
                        max_bytes -= delta * SAMPLE_LEN;

                        if delta == 0 {
                            break;
                        }
                    },
                    Err(ref e) => {
                        return if e.kind() == IoErrorKind::UnexpectedEof {
                            Some(frame_pos)
                        } else {
                            error!("Input died unexpectedly: {:?}", e);
                            None
                        }
                    },
                }
            }
        }

        Some(frame_pos * SAMPLE_LEN)
    }

    fn consume(&mut self, amt: usize) -> usize {
        io::copy(&mut self.by_ref().take(amt as u64), &mut io::sink()).unwrap_or(0) as usize
    }
}
