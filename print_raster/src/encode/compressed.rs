use super::RasterEncoder;
use futures::ready;
use futures::task::Context;
use futures::task::Poll;
use futures::AsyncWrite;
use pin_project::pin_project;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;
use std::slice;

#[derive(Debug)]
enum FlushLineBufferState {
    None,
    Begin {
        ret: usize,
        line_repeat: u8,
    },
    BeginInlineBlock {
        ret: usize,
        start: usize,
    },
    WriteInlineBlock {
        ret: usize,
        tag: u8,
        start: usize,
        end: usize,
    },
    WriteInlineBlockData {
        ret: usize,
        start: usize,
        end: usize,
    },
}

impl FlushLineBufferState {
    fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

#[pin_project(project = CompressedRasterEncoderProj)]
pub struct CompressedRasterEncoder<W> {
    writer: Pin<W>,
    chunk_size: u8,
    bytes_per_line: u64,
    bytes_remaining: u64,
    line_buffer: Vec<u8>,
    line_repeat: Option<u8>,
    pos_in_line: usize,
    flush_line_buffer_state: FlushLineBufferState,
}

impl<W> CompressedRasterEncoder<W> {
    pub fn new(
        writer: Pin<W>,
        chunk_size: u8,
        bytes_per_line: u64,
        num_bytes: u64,
    ) -> io::Result<Self> {
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
        Ok(Self {
            writer,
            chunk_size,
            bytes_per_line,
            bytes_remaining: num_bytes,
            line_buffer,
            line_repeat: None,
            pos_in_line: 0,
            flush_line_buffer_state: FlushLineBufferState::None,
        })
    }
}

impl<W> RasterEncoder<W> for CompressedRasterEncoder<W>
where
    W: DerefMut<Target: AsyncWrite>,
{
    fn bytes_remaining(&self) -> u64 {
        self.bytes_remaining
    }

    fn into_pin_mut(self) -> Pin<W> {
        self.writer
    }
}

fn poll_flush_line_buffer<W>(
    state: &mut FlushLineBufferState,
    cx: &mut Context<'_>,
    writer: &mut Pin<W>,
    chunk_size: u8,
    line_buffer: &[u8],
) -> Poll<io::Result<usize>>
where
    W: DerefMut<Target: AsyncWrite>,
{
    loop {
        match *state {
            FlushLineBufferState::None => return Poll::Ready(Ok(0)),
            FlushLineBufferState::Begin { ret, line_repeat } => {
                let n_written = ready!(writer
                    .as_mut()
                    .poll_write(cx, slice::from_ref(&line_repeat)))?;
                if n_written == 0 {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "failed to write to writer",
                    )));
                }
                *state = FlushLineBufferState::BeginInlineBlock { ret, start: 0 };
            }
            FlushLineBufferState::BeginInlineBlock { ret, start } => {
                let mut chunks = line_buffer[start..].chunks(chunk_size as usize);
                let first_chunk = if let Some(chunk) = chunks.next() {
                    chunk
                } else {
                    *state = FlushLineBufferState::None;
                    return Poll::Ready(Ok(ret));
                };
                if let Some(second_chunk) = chunks.next() {
                    if first_chunk == second_chunk {
                        let mut tag = 1u8;
                        for chunk in chunks {
                            if chunk != first_chunk || tag >= 0x7f {
                                break;
                            }
                            tag += 1;
                        }
                        *state = FlushLineBufferState::WriteInlineBlock {
                            ret,
                            tag,
                            start: start + chunk_size as usize * tag as usize,
                            end: start + chunk_size as usize * (tag + 1) as usize,
                        };
                    } else {
                        let mut count = 1u8;
                        let mut prev_chunk = second_chunk;
                        for chunk in chunks {
                            if chunk == prev_chunk {
                                break;
                            }
                            count += 1;
                            prev_chunk = chunk;
                            if count >= 0x7f {
                                break;
                            }
                        }
                        let tag = (!count).wrapping_add(2);
                        *state = FlushLineBufferState::WriteInlineBlock {
                            ret,
                            tag,
                            start,
                            end: start + chunk_size as usize * count as usize,
                        };
                    }
                } else {
                    // only one chunk remaining
                    *state = FlushLineBufferState::WriteInlineBlock {
                        ret,
                        tag: 0,
                        start,
                        end: start + chunk_size as usize,
                    };
                };
            }
            FlushLineBufferState::WriteInlineBlock {
                ret,
                tag,
                start,
                end,
            } => {
                let n_written = ready!(writer.as_mut().poll_write(cx, &[tag]))?;
                if n_written == 0 {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "failed to write to writer",
                    )));
                }
                *state = FlushLineBufferState::WriteInlineBlockData { ret, start, end };
            }
            FlushLineBufferState::WriteInlineBlockData { ret, start, end } => {
                let n_written = ready!(writer.as_mut().poll_write(cx, &line_buffer[start..end]))?;
                if n_written == 0 {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "failed to write to writer",
                    )));
                }
                if start + n_written >= end {
                    *state = FlushLineBufferState::BeginInlineBlock { ret, start: end };
                } else {
                    *state = FlushLineBufferState::WriteInlineBlockData {
                        ret,
                        start: start + n_written,
                        end,
                    };
                }
            }
        }
    }
}

impl<W> AsyncWrite for CompressedRasterEncoder<W>
where
    W: DerefMut<Target: AsyncWrite>,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.as_mut().project();
        let writer = this.writer;
        let mut total_write = 0;
        buf = &buf[..(*this.bytes_remaining).min(buf.len() as u64) as usize];

        if !this.flush_line_buffer_state.is_none() {
            total_write = ready!(poll_flush_line_buffer(
                this.flush_line_buffer_state,
                cx,
                writer,
                *this.chunk_size,
                this.line_buffer,
            ))?;
            buf = &buf[total_write..];
        }

        while !buf.is_empty() {
            match *this.line_repeat {
                None => {
                    let bytes_to_write =
                        (buf.len()).min(this.line_buffer.len() - *this.pos_in_line);
                    this.line_buffer[*this.pos_in_line..*this.pos_in_line + bytes_to_write]
                        .copy_from_slice(&buf[..bytes_to_write]);

                    buf = &buf[bytes_to_write..];
                    *this.pos_in_line += bytes_to_write;
                    total_write += bytes_to_write;

                    if *this.pos_in_line == this.line_buffer.len() {
                        // One line is full, reset the pointer for the next line
                        *this.pos_in_line = 0;

                        if total_write as u64 >= *this.bytes_remaining {
                            // Flush immediately if all bytes are written
                            *this.flush_line_buffer_state = FlushLineBufferState::Begin {
                                ret: total_write,
                                line_repeat: 0,
                            };
                            total_write = ready!(poll_flush_line_buffer(
                                this.flush_line_buffer_state,
                                cx,
                                writer,
                                *this.chunk_size,
                                this.line_buffer
                            ))?;
                        } else {
                            this.line_repeat.replace(0);
                        }
                    }
                }
                Some(line_repeat) => {
                    let bytes_to_write =
                        (buf.len()).min(this.line_buffer.len() - *this.pos_in_line);
                    let diff_pos = buf[..bytes_to_write]
                        .iter()
                        .zip(
                            &this.line_buffer
                                [*this.pos_in_line..*this.pos_in_line + bytes_to_write],
                        )
                        .position(|(a, b)| a != b);
                    if let Some(diff_pos) = diff_pos {
                        this.line_repeat.take();
                        *this.pos_in_line += diff_pos;
                        buf = &buf[diff_pos..];
                        *this.flush_line_buffer_state = FlushLineBufferState::Begin {
                            ret: total_write + diff_pos,
                            line_repeat,
                        };
                        total_write = ready!(poll_flush_line_buffer(
                            this.flush_line_buffer_state,
                            cx,
                            writer,
                            *this.chunk_size,
                            this.line_buffer
                        ))?;
                    } else {
                        // update pointer
                        buf = &buf[bytes_to_write..];
                        *this.pos_in_line += bytes_to_write;
                        total_write += bytes_to_write;
                        // if this line is full
                        if *this.pos_in_line == this.line_buffer.len() {
                            *this.pos_in_line = 0;
                            this.line_repeat.replace(line_repeat + 1);
                            let flush_line_buffer = (line_repeat + 1) == u8::MAX
                                || total_write as u64 >= *this.bytes_remaining;
                            if flush_line_buffer {
                                this.line_repeat.take();
                                *this.flush_line_buffer_state = FlushLineBufferState::Begin {
                                    ret: total_write,
                                    line_repeat: line_repeat + 1,
                                };
                                total_write = ready!(poll_flush_line_buffer(
                                    this.flush_line_buffer_state,
                                    cx,
                                    writer,
                                    *this.chunk_size,
                                    this.line_buffer
                                ))?;
                            }
                        }
                    }
                }
            }
        }

        *this.bytes_remaining = this.bytes_remaining.saturating_sub(total_write as u64);
        Poll::Ready(Ok(total_write))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.project();
        let writer = this.writer;
        writer.as_mut().poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.project();
        let writer = this.writer;
        writer.as_mut().poll_close(cx)
    }
}

#[cfg(test)]
mod tests {
    use futures::AsyncWriteExt;
    use std::pin::Pin;

    #[tokio::test]
    async fn test_compress() {
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
        let mut writer = Vec::<u8>::new();
        let mut encoder =
            super::CompressedRasterEncoder::new(Pin::new(&mut writer), 3, 3 * 8, 3 * 8 * 8)
                .unwrap();
        encoder.write_all(UNCOMPRESSED_DATA).await.unwrap();
        encoder.flush().await.unwrap();
        assert_eq!(writer, COMPRESSED_DATA);
    }

    #[tokio::test]
    async fn test_compress_highly_repetitive_data() {
        const WIDTH: u64 = 512;
        const HEIGHT: u64 = 512;
        const UNCOMPRESSED_DATA: &[u8] = &[0xcc; WIDTH as usize * HEIGHT as usize * 3];
        const COMPRESSED_DATA: &[u8] = &[
            0xff, 0x7f, 0xcc, 0xcc, 0xcc, 0x7f, 0xcc, 0xcc, 0xcc, 0x7f, 0xcc, 0xcc, 0xcc, 0x7f,
            0xcc, 0xcc, 0xcc, 0xff, 0x7f, 0xcc, 0xcc, 0xcc, 0x7f, 0xcc, 0xcc, 0xcc, 0x7f, 0xcc,
            0xcc, 0xcc, 0x7f, 0xcc, 0xcc, 0xcc,
        ];
        let mut writer = Vec::<u8>::new();
        let mut encoder = super::CompressedRasterEncoder::new(
            Pin::new(&mut writer),
            3,
            3 * WIDTH,
            3 * WIDTH * HEIGHT,
        )
        .unwrap();
        encoder.write_all(UNCOMPRESSED_DATA).await.unwrap();
        encoder.flush().await.unwrap();
        assert_eq!(writer, COMPRESSED_DATA);
    }

    #[tokio::test]
    async fn test_compress_zero() {
        const UNCOMPRESSED_DATA: &[u8] = &[0; 0];
        const COMPRESSED_DATA: &[u8] = &[];
        let mut writer = Vec::<u8>::new();
        let mut encoder =
            super::CompressedRasterEncoder::new(Pin::new(&mut writer), 0, 0, 0).unwrap();
        encoder.write_all(UNCOMPRESSED_DATA).await.unwrap();
        encoder.flush().await.unwrap();
        assert_eq!(writer, COMPRESSED_DATA);
    }
}
