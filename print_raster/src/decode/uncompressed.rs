use super::Limits;
use super::RasterDecoder;
use futures::ready;
use futures::task::Context;
use futures::task::Poll;
use futures::AsyncRead;
use pin_project::pin_project;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;

#[pin_project]
pub struct UncompressedRasterDecoder<R> {
    reader: Pin<R>,
    bytes_remaining: u64,
}

impl<R> UncompressedRasterDecoder<R> {
    pub fn new(reader: Pin<R>, limits: &Limits, num_bytes: u64) -> io::Result<Self> {
        if num_bytes > limits.bytes_per_page {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "num_bytes exceeds limit",
            ));
        }
        Ok(Self {
            reader,
            bytes_remaining: num_bytes,
        })
    }
}

impl<R> RasterDecoder<R> for UncompressedRasterDecoder<R>
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
impl<R> AsyncRead for UncompressedRasterDecoder<R>
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
        let buf_size = (*this.bytes_remaining).min(buf.len() as u64) as usize;
        buf = &mut buf[..buf_size];
        if buf_size == 0 {
            return Poll::Ready(Ok(0));
        }
        let total_read = ready!(reader.as_mut().poll_read(cx, buf))?;
        *this.bytes_remaining = this.bytes_remaining.saturating_sub(total_read as u64);
        Poll::Ready(Ok(total_read))
    }
}
