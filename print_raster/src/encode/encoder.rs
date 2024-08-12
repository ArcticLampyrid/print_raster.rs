use futures::AsyncWrite;
use pin_project::pin_project;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;

pub trait RasterEncoder<W>: AsyncWrite
where
    W: DerefMut<Target: AsyncWrite>,
{
    fn bytes_remaining(&self) -> u64;
    fn into_pin_mut(self) -> Pin<W>;
}

#[pin_project]
pub struct RasterEncoderConsumer<E, W>
where
    E: RasterEncoder<W> + Unpin,
    W: DerefMut<Target: AsyncWrite>,
{
    content: Option<E>,
    _phantom: std::marker::PhantomData<W>,
}

pub trait RasterEncoderExt<W>: RasterEncoder<W>
where
    W: DerefMut<Target: AsyncWrite>,
{
    /// Consumes the encoder and returns the underlying writer if all bytes have been written.
    fn try_consume(self) -> io::Result<Pin<W>>
    where
        Self: Unpin + Sized,
    {
        if self.bytes_remaining() == 0 {
            Ok(self.into_pin_mut())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not all bytes have been written",
            ))
        }
    }
}

impl<E, W> RasterEncoderExt<W> for E
where
    E: RasterEncoder<W>,
    W: DerefMut<Target: AsyncWrite>,
{
}
