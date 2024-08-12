use crate::{
    decode::{Limits, RasterDecoder},
    encode::RasterEncoder,
};
use futures::{AsyncRead, AsyncWrite};
use std::{ops::DerefMut, pin::Pin};

pub trait RasterPageFactory
where
    Self: Sized,
{
    type Header;
    type Error;
    const HEADER_SIZE: usize;
    /// Parse the header from the given bytes, the bytes are guaranteed to be `HEADER_SIZE` long.
    fn header_from_bytes(content: &[u8]) -> Result<Self::Header, Self::Error>;
    /// Convert the header to bytes, the bytes will be `HEADER_SIZE` long.
    fn header_to_bytes(target: &mut [u8], header: &Self::Header) -> Result<(), Self::Error>;

    type Decoder<R>: RasterDecoder<R>
    where
        R: DerefMut<Target: AsyncRead>;
    /// Create a new decoder from the given reader, setting the correct parameters based on the header.
    fn decode<R>(
        header: &Self::Header,
        reader: Pin<R>,
        limits: &Limits,
    ) -> Result<Self::Decoder<R>, Self::Error>
    where
        R: DerefMut<Target: AsyncRead>;

    type Encoder<W>: RasterEncoder<W>
    where
        W: DerefMut<Target: AsyncWrite>;
    /// Create a new encoder from the given writer, setting the correct parameters based on the header.
    fn encode<W>(header: &Self::Header, writer: Pin<W>) -> Result<Self::Encoder<W>, Self::Error>
    where
        W: DerefMut<Target: AsyncWrite>;
}
