use futures::ready;
use futures::task::Context;
use futures::task::Poll;
use futures::AsyncRead;
use pin_project::pin_project;
use std::ops::DerefMut;
use std::pin::Pin;
use std::{future::Future, io};

pub trait RasterDecoder<R>: AsyncRead
where
    R: DerefMut<Target: AsyncRead>,
{
    fn bytes_remaining(&self) -> u64;
    fn into_pin_mut(self) -> Pin<R>;
}

#[pin_project]
pub struct RasterDecoderConsumer<D, R>
where
    D: RasterDecoder<R> + Unpin,
    R: DerefMut<Target: AsyncRead>,
{
    content: Option<D>,
    buf: Vec<u8>,
    _phantom: std::marker::PhantomData<R>,
}

impl<D, R> Future for RasterDecoderConsumer<D, R>
where
    D: RasterDecoder<R> + Unpin,
    R: DerefMut<Target: AsyncRead>,
{
    type Output = io::Result<Pin<R>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        if this.content.is_none() {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::Other,
                "content is already consumed",
            )));
        }
        let content = this.content.as_mut().unwrap();
        let mut remaining = content.bytes_remaining();
        if remaining > 0 {
            loop {
                let num_read = ready!(Pin::new(&mut *content).poll_read(cx, &mut *this.buf))?;
                remaining = remaining.saturating_sub(num_read as u64);
                if remaining == 0 {
                    break;
                }
                if num_read == 0 {
                    // more data of raster page is expected
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "unexpected eof, more data of raster page is expected",
                    )));
                }
            }
        }
        Poll::Ready(Ok(this.content.take().unwrap().into_pin_mut()))
    }
}

pub trait RasterDecoderExt<R>: RasterDecoder<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    /// Consumes the decoder and returns the underlying reader if all bytes have been read.
    fn try_consume(self) -> io::Result<Pin<R>>
    where
        Self: Unpin + Sized,
    {
        if self.bytes_remaining() == 0 {
            Ok(self.into_pin_mut())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not all bytes have been read",
            ))
        }
    }

    /// Consumes the decoder and returns a future that reads all remaining bytes.
    fn consume(self) -> RasterDecoderConsumer<Self, R>
    where
        Self: Unpin + Sized,
    {
        RasterDecoderConsumer {
            content: Some(self),
            buf: vec![0; 4096],
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<D, R> RasterDecoderExt<R> for D
where
    D: RasterDecoder<R>,
    R: DerefMut<Target: AsyncRead>,
{
}
