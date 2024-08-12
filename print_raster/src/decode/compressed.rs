use super::decoder::RasterDecoder;
use super::Limits;
use futures::ready;
use futures::task::Context;
use futures::task::Poll;
use futures::AsyncRead;
use pin_project::pin_project;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;
use std::slice;

enum CompressedRasterDecoderState {
    Begin,
    BeginInlineBlock {
        start: usize,
    },
    ReadInlineBlock {
        repeat_last: u8,
        start: usize,
        remaining: usize,
    },
    UseBuffer {
        start: usize,
        remaining: usize,
    },
}

#[pin_project]
pub struct CompressedRasterDecoder<R> {
    reader: Pin<R>,
    chunk_size: u8,
    bytes_per_line: u64,
    fill_byte: u8,
    line_buffer: Vec<u8>,
    line_repeat: u8,
    state: CompressedRasterDecoderState,
    bytes_remaining: u64,
}

impl<R> CompressedRasterDecoder<R> {
    pub fn new(
        reader: Pin<R>,
        limits: &Limits,
        chunk_size: u8,
        bytes_per_line: u64,
        num_bytes: u64,
        fill_byte: u8,
    ) -> io::Result<Self> {
        if bytes_per_line > limits.bytes_per_line {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "bytes_per_line exceeds limit",
            ));
        }
        if num_bytes > limits.bytes_per_page {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "num_bytes exceeds limit",
            ));
        }
        if bytes_per_line != 0 && (chunk_size == 0 || bytes_per_line % chunk_size as u64 != 0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "bytes_per_line must be multiple of chunk_size",
            ));
        }
        if (num_bytes != 0) && (bytes_per_line == 0 || num_bytes % bytes_per_line != 0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "num_bytes must be multiple of bytes_per_line",
            ));
        }
        // note: when `num_bytes` = 0, `bytes_per_line` can be any value, but `line_buffer_size` must be 0
        let line_buffer_size = usize::try_from(bytes_per_line.min(num_bytes)).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "bytes_per_line is too large")
        })?;
        #[allow(clippy::uninit_vec)]
        let line_buffer = unsafe {
            let mut line_buffer = Vec::new();
            line_buffer.try_reserve(line_buffer_size)?;
            line_buffer.set_len(line_buffer_size);
            line_buffer
        };
        Ok(CompressedRasterDecoder {
            reader,
            chunk_size,
            bytes_per_line,
            fill_byte,
            line_buffer,
            line_repeat: 0,
            state: CompressedRasterDecoderState::Begin,
            bytes_remaining: num_bytes,
        })
    }
}

impl<R> RasterDecoder<R> for CompressedRasterDecoder<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    fn bytes_remaining(&self) -> u64 {
        self.bytes_remaining
    }

    fn into_pin_mut(self) -> Pin<R> {
        self.reader
    }
}

impl<R> AsyncRead for CompressedRasterDecoder<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        let reader = this.reader;
        let chunk_size = *this.chunk_size;
        let buf_size = (*this.bytes_remaining).min(buf.len() as u64) as usize;
        buf = &mut buf[..buf_size];
        if buf_size == 0 {
            return Poll::Ready(Ok(0));
        }
        let mut total_read: usize = 0;
        loop {
            match this.state {
                CompressedRasterDecoderState::Begin => {
                    let mut code = 0u8;
                    let read_code =
                        ready!(reader.as_mut().poll_read(cx, slice::from_mut(&mut code)));
                    match read_code {
                        Ok(0) => {
                            *this.bytes_remaining =
                                this.bytes_remaining.saturating_sub(total_read as u64);
                            return Poll::Ready(Ok(total_read));
                        }
                        Ok(_) => {
                            *this.line_repeat = code;
                            *this.state =
                                CompressedRasterDecoderState::BeginInlineBlock { start: 0 };
                        }
                        Err(e) => return Poll::Ready(Err(e)),
                    }
                }
                CompressedRasterDecoderState::BeginInlineBlock { start } => {
                    let mut code = 0u8;
                    let read_code =
                        ready!(reader.as_mut().poll_read(cx, slice::from_mut(&mut code)));
                    match read_code {
                        Ok(0) => {
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "unexpected eof while reading block header",
                            )))
                        }
                        Ok(_) => {
                            match code {
                                0x00..=0x7F => {
                                    // repeat single pixel
                                    let length_uncompressed =
                                        (code as usize + 1) * chunk_size as usize;
                                    // the code may be invalid, check and return error if invalid to avoid panic
                                    if (this.line_buffer.len() - *start) < length_uncompressed {
                                        return Poll::Ready(Err(io::Error::new(
                                            io::ErrorKind::InvalidData,
                                            "invalid block header",
                                        )));
                                    }
                                    *this.state = CompressedRasterDecoderState::ReadInlineBlock {
                                        repeat_last: code,
                                        start: *start,
                                        remaining: chunk_size as usize,
                                    }
                                }
                                0x80 => {
                                    // reset all remaining pixels to white (apple-specific)
                                    this.line_buffer[*start..].fill(*this.fill_byte);
                                    *this.state = CompressedRasterDecoderState::UseBuffer {
                                        start: *start,
                                        remaining: this.line_buffer.len() - *start,
                                    }
                                }
                                _ => {
                                    // pixel sequence
                                    let length = !code + 2;
                                    let length_in_bytes = length as usize * chunk_size as usize;
                                    // the length may be invalid, check and return error if invalid to avoid panic
                                    if (this.line_buffer.len() - *start) < length_in_bytes {
                                        return Poll::Ready(Err(io::Error::new(
                                            io::ErrorKind::InvalidData,
                                            "invalid block header",
                                        )));
                                    }
                                    *this.state = CompressedRasterDecoderState::ReadInlineBlock {
                                        repeat_last: 0,
                                        start: *start,
                                        remaining: length_in_bytes,
                                    }
                                }
                            }
                        }
                        Err(e) => return Poll::Ready(Err(e)),
                    }
                }
                CompressedRasterDecoderState::ReadInlineBlock {
                    repeat_last,
                    start,
                    remaining,
                } => {
                    let start_cur = *start;
                    let n_read = buf.len().min(*remaining);
                    let read_exact = ready!(reader
                        .as_mut()
                        .poll_read(cx, &mut this.line_buffer[start_cur..start_cur + n_read]));
                    match read_exact {
                        Ok(0) => {
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "unexpected eof while reading block content",
                            )))
                        }
                        Ok(n) => {
                            *start += n;
                            *remaining -= n;

                            if *remaining == 0 {
                                // block finished
                                let mut n_available = n;
                                let mut repeat_counter = *repeat_last;
                                if repeat_counter != 0 {
                                    n_available += repeat_counter as usize * chunk_size as usize;

                                    let (filled, mut rest) = this.line_buffer.split_at_mut(*start);
                                    let last_pixel = &filled[*start - (chunk_size as usize)..];
                                    while repeat_counter > 0 {
                                        rest[..chunk_size as usize].copy_from_slice(last_pixel);
                                        rest = &mut rest[chunk_size as usize..];
                                        repeat_counter -= 1;
                                    }
                                }
                                let read = buf.len().min(n_available);
                                buf[..read].copy_from_slice(
                                    &this.line_buffer[start_cur..start_cur + read],
                                );
                                buf = &mut buf[read..];
                                total_read += read;
                                // try to read more using buffer
                                // eg. if line is repeated, then we can read more.
                                // if no more data, `UseBuffer` will reset state and return current block
                                *this.state = CompressedRasterDecoderState::UseBuffer {
                                    start: start_cur + read,
                                    remaining: n_available - read,
                                };
                            } else {
                                // block not finished, keep state unchanged
                                buf[..n]
                                    .copy_from_slice(&this.line_buffer[start_cur..start_cur + n]);
                                total_read += n;
                                // for there is data available, return immediately
                                *this.bytes_remaining =
                                    this.bytes_remaining.saturating_sub(total_read as u64);
                                return Poll::Ready(Ok(total_read));
                            }
                        }
                        Err(e) => return Poll::Ready(Err(e)),
                    }
                }
                CompressedRasterDecoderState::UseBuffer { start, remaining } => {
                    let read = buf.len().min(*remaining);
                    buf[..read].copy_from_slice(&this.line_buffer[*start..*start + read]);
                    buf = &mut buf[read..];
                    *start += read;
                    *remaining -= read;
                    total_read += read;
                    if *remaining == 0 {
                        if *start == this.line_buffer.len() {
                            // line finished
                            if *this.line_repeat > 0 {
                                // repeat line
                                *this.line_repeat -= 1;
                                *this.state = CompressedRasterDecoderState::UseBuffer {
                                    start: 0,
                                    remaining: this.line_buffer.len(),
                                };
                                // continue to use buffer
                            } else {
                                // next line
                                *this.state = CompressedRasterDecoderState::Begin;
                                if total_read != 0 {
                                    // for there is data available, return immediately
                                    *this.bytes_remaining =
                                        this.bytes_remaining.saturating_sub(total_read as u64);
                                    return Poll::Ready(Ok(total_read));
                                }
                            }
                        } else {
                            // next block
                            *this.state =
                                CompressedRasterDecoderState::BeginInlineBlock { start: *start };
                            if total_read != 0 {
                                // for there is data available, return immediately
                                *this.bytes_remaining =
                                    this.bytes_remaining.saturating_sub(total_read as u64);
                                return Poll::Ready(Ok(total_read));
                            }
                        }
                    } else {
                        *this.bytes_remaining =
                            this.bytes_remaining.saturating_sub(total_read as u64);
                        return Poll::Ready(Ok(total_read));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::AsyncReadExt;
    use std::pin::Pin;

    use crate::decode::Limits;

    #[tokio::test]
    async fn test_decompress() {
        const UNCOMPRESSED_DATA: &[u8] = &[
            0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00,
            0x00, 0xff, 0xff, 0xff, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0x00, 0xff, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00,
            0xff, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0xff, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff,
            0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00,
            0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00,
            0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00,
        ];
        const COMPRESSED_DATA: &[u8] = &[
            0x00, 0x00, 0xff, 0xff, 0xff, 0x02, 0xff, 0xff, 0x00, 0x03, 0xff, 0xff, 0xff, 0x00,
            0xfe, 0xff, 0xff, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0x00, 0x02, 0xff, 0xff, 0xff,
            0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0xff, 0xff, 0x00, 0x01, 0xff, 0xff, 0x00, 0x02,
            0xff, 0xff, 0xff, 0x02, 0x00, 0xff, 0x00, 0x00, 0x02, 0xff, 0xff, 0x00, 0x02, 0xff,
            0xff, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0xff, 0xff, 0x00, 0x00, 0xff, 0xff,
            0xff, 0x02, 0xff, 0xff, 0x00, 0x03, 0xff, 0xff, 0xff, 0x00, 0x07, 0xff, 0xff, 0xff,
            0x01, 0x07, 0xff, 0x00, 0x00,
        ];
        let mut reader = futures::io::Cursor::new(COMPRESSED_DATA);
        let mut decoder = super::CompressedRasterDecoder::new(
            Pin::new(&mut reader),
            Limits::NO_LIMITS,
            3,
            3 * 8,
            3 * 8 * 8,
            0,
        )
        .unwrap();
        let mut uncompressed = Vec::new();
        decoder.read_to_end(&mut uncompressed).await.unwrap();
        assert_eq!(uncompressed, UNCOMPRESSED_DATA);
    }

    #[tokio::test]
    async fn test_uncompress_highly_repetitive_data() {
        const WIDTH: u64 = 512;
        const HEIGHT: u64 = 512;
        const UNCOMPRESSED_DATA: &[u8] = &[0xcc; WIDTH as usize * HEIGHT as usize * 3];
        const COMPRESSED_DATA: &[u8] = &[
            0xff, 0x7f, 0xcc, 0xcc, 0xcc, 0x7f, 0xcc, 0xcc, 0xcc, 0x7f, 0xcc, 0xcc, 0xcc, 0x7f,
            0xcc, 0xcc, 0xcc, 0xff, 0x7f, 0xcc, 0xcc, 0xcc, 0x7f, 0xcc, 0xcc, 0xcc, 0x7f, 0xcc,
            0xcc, 0xcc, 0x7f, 0xcc, 0xcc, 0xcc,
        ];
        let mut reader = futures::io::Cursor::new(COMPRESSED_DATA);
        let mut decoder = super::CompressedRasterDecoder::new(
            Pin::new(&mut reader),
            Limits::NO_LIMITS,
            3,
            WIDTH * 3,
            WIDTH * HEIGHT * 3,
            0,
        )
        .unwrap();
        let mut uncompressed = Vec::new();
        decoder.read_to_end(&mut uncompressed).await.unwrap();
        assert_eq!(uncompressed, UNCOMPRESSED_DATA);
    }

    #[tokio::test]
    async fn test_uncompress_zero() {
        const UNCOMPRESSED_DATA: &[u8] = &[];
        const COMPRESSED_DATA: &[u8] = &[];
        let mut reader = futures::io::Cursor::new(COMPRESSED_DATA);
        let mut decoder = super::CompressedRasterDecoder::new(
            Pin::new(&mut reader),
            Limits::NO_LIMITS,
            0,
            0,
            0,
            0,
        )
        .unwrap();
        let mut uncompressed = Vec::new();
        decoder.read_to_end(&mut uncompressed).await.unwrap();
        assert_eq!(uncompressed, UNCOMPRESSED_DATA);
    }
}
