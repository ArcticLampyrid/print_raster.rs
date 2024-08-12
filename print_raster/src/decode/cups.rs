use super::{CompressedRasterDecoder, UncompressedRasterDecoder};
use crate::decode::RasterDecoder;
use derive_more::From;
use futures::AsyncRead;
use pin_project::pin_project;
use std::{ops::DerefMut, pin::Pin};

#[pin_project(project = CupsRasterDecoderProj)]
#[derive(From)]
pub enum CupsRasterUnifiedDecoder<R> {
    Uncompressed(#[pin] UncompressedRasterDecoder<R>),
    Compressed(#[pin] CompressedRasterDecoder<R>),
}

impl<R> RasterDecoder<R> for CupsRasterUnifiedDecoder<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    fn bytes_remaining(&self) -> u64 {
        match self {
            CupsRasterUnifiedDecoder::Uncompressed(decoder) => decoder.bytes_remaining(),
            CupsRasterUnifiedDecoder::Compressed(decoder) => decoder.bytes_remaining(),
        }
    }

    fn into_pin_mut(self) -> Pin<R> {
        match self {
            CupsRasterUnifiedDecoder::Uncompressed(decoder) => decoder.into_pin_mut(),
            CupsRasterUnifiedDecoder::Compressed(decoder) => decoder.into_pin_mut(),
        }
    }
}

impl<R> AsyncRead for CupsRasterUnifiedDecoder<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.project();
        match this {
            CupsRasterDecoderProj::Uncompressed(decoder) => decoder.poll_read(cx, buf),
            CupsRasterDecoderProj::Compressed(decoder) => decoder.poll_read(cx, buf),
        }
    }
}
