use super::RasterPageFactory;
use crate::decode::{CompressedRasterDecoder, Limits};
use crate::encode::CompressedRasterEncoder;
use crate::error::UrfError;
use crate::model::urf::{
    UrfColorSpace, UrfDuplex, UrfMediaPosition, UrfMediaType, UrfPageHeader, UrfQuality,
};
use futures::{AsyncRead, AsyncWrite};
use num_enum::TryFromPrimitive;
use std::ops::DerefMut;
use std::pin::Pin;

pub enum UrfPageFactory {}

impl RasterPageFactory for UrfPageFactory {
    type Header = UrfPageHeader;
    type Error = UrfError;
    const HEADER_SIZE: usize = 32;
    fn header_from_bytes(content: &[u8]) -> Result<Self::Header, Self::Error> {
        Ok(UrfPageHeader {
            bits_per_pixel: content[0],
            color_space: UrfColorSpace::try_from_primitive(content[1])?,
            duplex: UrfDuplex::try_from_primitive(content[2])?,
            quality: UrfQuality::try_from_primitive(content[3])?,
            media_position: UrfMediaPosition::try_from_primitive(content[4])?,
            media_type: UrfMediaType::try_from_primitive(content[5])?,
            width: u32::from_be_bytes([content[12], content[13], content[14], content[15]]),
            height: u32::from_be_bytes([content[16], content[17], content[18], content[19]]),
            dot_per_inch: u32::from_be_bytes([content[20], content[21], content[22], content[23]]),
        })
    }
    fn header_to_bytes(target: &mut [u8], header: &Self::Header) -> Result<(), Self::Error> {
        target[0] = header.bits_per_pixel;
        target[1] = header.color_space as u8;
        target[2] = header.duplex as u8;
        target[3] = header.quality as u8;
        target[4] = header.media_position as u8;
        target[5] = header.media_type as u8;
        target[6..12].fill(0);
        target[12..16].copy_from_slice(&header.width.to_be_bytes());
        target[16..20].copy_from_slice(&header.height.to_be_bytes());
        target[20..24].copy_from_slice(&header.dot_per_inch.to_be_bytes());
        target[24..32].fill(0);
        Ok(())
    }

    type Decoder<R> = CompressedRasterDecoder<R>
        where R: DerefMut<Target: AsyncRead>;
    fn decode<R>(
        header: &Self::Header,
        reader: Pin<R>,
        limits: &Limits,
    ) -> Result<Self::Decoder<R>, Self::Error>
    where
        R: DerefMut<Target: AsyncRead>,
    {
        // for Apple Raster (urf), chunky pixels are used, so the chunk size is the pixel size.
        let chunk_size = header.bits_per_pixel / 8;
        let bytes_per_line = header.width as u64 * chunk_size as u64;
        let num_bytes = (header.width as u64 * header.height as u64)
            .checked_mul(chunk_size as u64)
            .ok_or(UrfError::DataTooLarge)?;
        let fill_byte = match header.color_space {
            UrfColorSpace::sGray
            | UrfColorSpace::sRGB
            | UrfColorSpace::CIELab
            | UrfColorSpace::AdobeRGB
            | UrfColorSpace::Gray
            | UrfColorSpace::RGB => 0xffu8,
            _ => 0u8,
        };
        Ok(CompressedRasterDecoder::new(
            reader,
            limits,
            chunk_size,
            bytes_per_line,
            num_bytes,
            fill_byte,
        )?)
    }

    type Encoder<W> = CompressedRasterEncoder<W>
    where
        W: DerefMut<Target: AsyncWrite>;
    fn encode<W>(header: &Self::Header, writer: Pin<W>) -> Result<Self::Encoder<W>, Self::Error>
    where
        W: DerefMut<Target: AsyncWrite>,
    {
        // for Apple Raster (urf), chunky pixels are used, so the chunk size is the pixel size.
        let chunk_size = header.bits_per_pixel / 8;
        let bytes_per_line = header.width as u64 * chunk_size as u64;
        let num_bytes = (header.width as u64 * header.height as u64)
            .checked_mul(chunk_size as u64)
            .ok_or(UrfError::DataTooLarge)?;
        Ok(CompressedRasterEncoder::new(
            writer,
            chunk_size,
            bytes_per_line,
            num_bytes,
        )?)
    }
}
