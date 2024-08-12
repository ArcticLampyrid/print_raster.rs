use crate::{
    decode::CupsRasterUnifiedDecoder,
    error::CupsRasterError,
    factory::{CupsPageFactoryV1, CupsPageFactoryV2, CupsPageFactoryV3},
    model::{cups::CupsPageHeaderV2, RasterByteOrder},
    reader::common::CommonRasterPageReader,
    reader::RasterPageReader,
};
use byteorder::{BigEndian, LittleEndian};
use derive_more::From;
use futures::task::Poll;
use futures::{task::Context, AsyncRead};
use pin_project::pin_project;
use std::{future::Future, ops::DerefMut, pin::Pin};

pub type CupsRasterUnifiedPageReaderV1BE<R> = CommonRasterPageReader<
    CupsPageFactoryV1<BigEndian>,
    CupsPageHeaderV2,
    CupsRasterUnifiedDecoder<R>,
    R,
>;
pub type CupsRasterUnifiedPageReaderV1LE<R> = CommonRasterPageReader<
    CupsPageFactoryV1<LittleEndian>,
    CupsPageHeaderV2,
    CupsRasterUnifiedDecoder<R>,
    R,
>;
pub type CupsRasterUnifiedPageReaderV2BE<R> = CommonRasterPageReader<
    CupsPageFactoryV2<BigEndian>,
    CupsPageHeaderV2,
    CupsRasterUnifiedDecoder<R>,
    R,
>;
pub type CupsRasterUnifiedPageReaderV2LE<R> = CommonRasterPageReader<
    CupsPageFactoryV2<LittleEndian>,
    CupsPageHeaderV2,
    CupsRasterUnifiedDecoder<R>,
    R,
>;
pub type CupsRasterUnifiedPageReaderV3BE<R> = CommonRasterPageReader<
    CupsPageFactoryV3<BigEndian>,
    CupsPageHeaderV2,
    CupsRasterUnifiedDecoder<R>,
    R,
>;
pub type CupsRasterUnifiedPageReaderV3LE<R> = CommonRasterPageReader<
    CupsPageFactoryV3<LittleEndian>,
    CupsPageHeaderV2,
    CupsRasterUnifiedDecoder<R>,
    R,
>;

#[derive(From)]
pub enum CupsRasterUnifiedPageReader<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    V1BigEndian(CupsRasterUnifiedPageReaderV1BE<R>),
    V1LittleEndian(CupsRasterUnifiedPageReaderV1LE<R>),
    V2BigEndian(CupsRasterUnifiedPageReaderV2BE<R>),
    V2LittleEndian(CupsRasterUnifiedPageReaderV2LE<R>),
    V3BigEndian(CupsRasterUnifiedPageReaderV3BE<R>),
    V3LittleEndian(CupsRasterUnifiedPageReaderV3LE<R>),
}

#[pin_project(project = CupsRasterUnifiedNextPageProj)]
pub enum CupsRasterUnifiedNextPage<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    V1BigEndian(#[pin] <CupsRasterUnifiedPageReaderV1BE<R> as RasterPageReader<R>>::NextPageFuture),
    V1LittleEndian(
        #[pin] <CupsRasterUnifiedPageReaderV1LE<R> as RasterPageReader<R>>::NextPageFuture,
    ),
    V2BigEndian(#[pin] <CupsRasterUnifiedPageReaderV2BE<R> as RasterPageReader<R>>::NextPageFuture),
    V2LittleEndian(
        #[pin] <CupsRasterUnifiedPageReaderV2LE<R> as RasterPageReader<R>>::NextPageFuture,
    ),
    V3BigEndian(#[pin] <CupsRasterUnifiedPageReaderV3BE<R> as RasterPageReader<R>>::NextPageFuture),
    V3LittleEndian(
        #[pin] <CupsRasterUnifiedPageReaderV3LE<R> as RasterPageReader<R>>::NextPageFuture,
    ),
}

impl<R> Future for CupsRasterUnifiedNextPage<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    type Output = Result<Option<CupsRasterUnifiedPageReader<R>>, CupsRasterError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        match this {
            CupsRasterUnifiedNextPageProj::V1BigEndian(fut) => fut
                .poll(cx)
                .map(|result| result.map(|reader| reader.map(CupsRasterUnifiedPageReader::from))),
            CupsRasterUnifiedNextPageProj::V1LittleEndian(fut) => fut
                .poll(cx)
                .map(|result| result.map(|reader| reader.map(CupsRasterUnifiedPageReader::from))),
            CupsRasterUnifiedNextPageProj::V2BigEndian(fut) => fut
                .poll(cx)
                .map(|result| result.map(|reader| reader.map(CupsRasterUnifiedPageReader::from))),
            CupsRasterUnifiedNextPageProj::V2LittleEndian(fut) => fut
                .poll(cx)
                .map(|result| result.map(|reader| reader.map(CupsRasterUnifiedPageReader::from))),
            CupsRasterUnifiedNextPageProj::V3BigEndian(fut) => fut
                .poll(cx)
                .map(|result| result.map(|reader| reader.map(CupsRasterUnifiedPageReader::from))),
            CupsRasterUnifiedNextPageProj::V3LittleEndian(fut) => fut
                .poll(cx)
                .map(|result| result.map(|reader| reader.map(CupsRasterUnifiedPageReader::from))),
        }
    }
}

impl<R> CupsRasterUnifiedPageReader<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    pub fn byte_order(&self) -> RasterByteOrder {
        match self {
            CupsRasterUnifiedPageReader::V1BigEndian(_) => RasterByteOrder::BigEndian,
            CupsRasterUnifiedPageReader::V1LittleEndian(_) => RasterByteOrder::LittleEndian,
            CupsRasterUnifiedPageReader::V2BigEndian(_) => RasterByteOrder::BigEndian,
            CupsRasterUnifiedPageReader::V2LittleEndian(_) => RasterByteOrder::LittleEndian,
            CupsRasterUnifiedPageReader::V3BigEndian(_) => RasterByteOrder::BigEndian,
            CupsRasterUnifiedPageReader::V3LittleEndian(_) => RasterByteOrder::LittleEndian,
        }
    }
}

impl<R> RasterPageReader<R> for CupsRasterUnifiedPageReader<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    type Header = CupsPageHeaderV2;
    type Decoder = CupsRasterUnifiedDecoder<R>;
    type Error = CupsRasterError;
    type NextPageFuture = CupsRasterUnifiedNextPage<R>;

    fn next_page(self) -> Self::NextPageFuture {
        match self {
            CupsRasterUnifiedPageReader::V1BigEndian(reader) => {
                CupsRasterUnifiedNextPage::V1BigEndian(reader.next_page())
            }
            CupsRasterUnifiedPageReader::V1LittleEndian(reader) => {
                CupsRasterUnifiedNextPage::V1LittleEndian(reader.next_page())
            }
            CupsRasterUnifiedPageReader::V2BigEndian(reader) => {
                CupsRasterUnifiedNextPage::V2BigEndian(reader.next_page())
            }
            CupsRasterUnifiedPageReader::V2LittleEndian(reader) => {
                CupsRasterUnifiedNextPage::V2LittleEndian(reader.next_page())
            }
            CupsRasterUnifiedPageReader::V3BigEndian(reader) => {
                CupsRasterUnifiedNextPage::V3BigEndian(reader.next_page())
            }
            CupsRasterUnifiedPageReader::V3LittleEndian(reader) => {
                CupsRasterUnifiedNextPage::V3LittleEndian(reader.next_page())
            }
        }
    }

    fn header(&self) -> &Self::Header {
        match self {
            CupsRasterUnifiedPageReader::V1BigEndian(reader) => reader.header(),
            CupsRasterUnifiedPageReader::V1LittleEndian(reader) => reader.header(),
            CupsRasterUnifiedPageReader::V2BigEndian(reader) => reader.header(),
            CupsRasterUnifiedPageReader::V2LittleEndian(reader) => reader.header(),
            CupsRasterUnifiedPageReader::V3BigEndian(reader) => reader.header(),
            CupsRasterUnifiedPageReader::V3LittleEndian(reader) => reader.header(),
        }
    }

    fn content_mut(&mut self) -> &mut Self::Decoder {
        match self {
            CupsRasterUnifiedPageReader::V1BigEndian(reader) => reader.content_mut(),
            CupsRasterUnifiedPageReader::V1LittleEndian(reader) => reader.content_mut(),
            CupsRasterUnifiedPageReader::V2BigEndian(reader) => reader.content_mut(),
            CupsRasterUnifiedPageReader::V2LittleEndian(reader) => reader.content_mut(),
            CupsRasterUnifiedPageReader::V3BigEndian(reader) => reader.content_mut(),
            CupsRasterUnifiedPageReader::V3LittleEndian(reader) => reader.content_mut(),
        }
    }

    fn into_content(self) -> Self::Decoder {
        match self {
            CupsRasterUnifiedPageReader::V1BigEndian(reader) => reader.into_content(),
            CupsRasterUnifiedPageReader::V1LittleEndian(reader) => reader.into_content(),
            CupsRasterUnifiedPageReader::V2BigEndian(reader) => reader.into_content(),
            CupsRasterUnifiedPageReader::V2LittleEndian(reader) => reader.into_content(),
            CupsRasterUnifiedPageReader::V3BigEndian(reader) => reader.into_content(),
            CupsRasterUnifiedPageReader::V3LittleEndian(reader) => reader.into_content(),
        }
    }
}
