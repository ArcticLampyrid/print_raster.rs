use super::RasterEncoder;
use futures::ready;
use futures::task::Context;
use futures::task::Poll;
use futures::AsyncWrite;
use pin_project::pin_project;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;

#[pin_project]
pub struct UncompressedRasterEncoder<W> {
    writer: Pin<W>,
    bytes_remaining: u64,
}

impl<W> UncompressedRasterEncoder<W> {
    pub fn new(writer: Pin<W>, num_bytes: u64) -> Self {
        Self {
            writer,
            bytes_remaining: num_bytes,
        }
    }
}

impl<W> RasterEncoder<W> for UncompressedRasterEncoder<W>
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

impl<W> AsyncWrite for UncompressedRasterEncoder<W>
where
    W: DerefMut<Target: AsyncWrite>,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        let writer = this.writer;
        let buf_size = (*this.bytes_remaining).min(buf.len() as u64) as usize;
        buf = &buf[..buf_size];
        if buf_size == 0 {
            return Poll::Ready(Ok(0));
        }
        let total_write = ready!(writer.as_mut().poll_write(cx, buf))?;
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
