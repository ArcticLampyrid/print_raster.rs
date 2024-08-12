use byteorder::{BigEndian, LittleEndian};
use futures::AsyncRead;
use pin_project::pin_project;
use std::io;
use std::task::{Context, Poll};
use std::{future::Future, ops::DerefMut, pin::Pin};
mod page;
use crate::decode::{CupsRasterUnifiedDecoder, Limits};
use crate::error::CupsRasterError;
use crate::factory::{CupsPageFactoryV1, CupsPageFactoryV2, CupsPageFactoryV3};
use crate::model::cups::{CupsPageHeaderV2, CupsSyncWord};
use crate::model::RasterByteOrder;
use crate::reader::common::CommonRasterPageReaderFor;
use crate::reader::RasterReader;
pub use page::*;

pub struct CupsRasterUnifiedReader<R> {
    sync_word: CupsSyncWord,
    reader: Pin<R>,
    limits: Limits,
}

impl<R> CupsRasterUnifiedReader<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    pub async fn new(reader: Pin<R>) -> Result<Self, CupsRasterError> {
        Self::new_with_limits(reader, Limits::default()).await
    }

    pub async fn new_with_limits(
        mut reader: Pin<R>,
        limits: Limits,
    ) -> Result<Self, CupsRasterError> {
        let sync_word = CupsRasterReaderReadSyncWord::new(reader.as_mut()).await?;
        Ok(CupsRasterUnifiedReader {
            sync_word,
            reader,
            limits,
        })
    }

    pub fn sync_word(&self) -> CupsSyncWord {
        self.sync_word
    }

    pub fn byte_order(&self) -> RasterByteOrder {
        self.sync_word.byte_order()
    }
}

impl<R> RasterReader<R> for CupsRasterUnifiedReader<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    type PageHeader = CupsPageHeaderV2;
    type PageReader = CupsRasterUnifiedPageReader<R>;
    type Error = CupsRasterError;
    type NextPageFuture = CupsRasterUnifiedReaderNextPage<R>;

    fn next_page(self) -> CupsRasterUnifiedReaderNextPage<R> {
        match self.sync_word {
            CupsSyncWord::V1BigEndian => CupsRasterUnifiedReaderNextPage::V1BigEndian(
                CupsRasterUnifiedPageReaderV1BE::reader_for(self.reader, self.limits),
            ),
            CupsSyncWord::V1LittleEndian => CupsRasterUnifiedReaderNextPage::V1LittleEndian(
                CupsRasterUnifiedPageReaderV1LE::reader_for(self.reader, self.limits),
            ),
            CupsSyncWord::V2BigEndian => CupsRasterUnifiedReaderNextPage::V2BigEndian(
                CupsRasterUnifiedPageReaderV2BE::reader_for(self.reader, self.limits),
            ),
            CupsSyncWord::V2LittleEndian => CupsRasterUnifiedReaderNextPage::V2LittleEndian(
                CupsRasterUnifiedPageReaderV2LE::reader_for(self.reader, self.limits),
            ),
            CupsSyncWord::V3BigEndian => CupsRasterUnifiedReaderNextPage::V3BigEndian(
                CupsRasterUnifiedPageReaderV3BE::reader_for(self.reader, self.limits),
            ),
            CupsSyncWord::V3LittleEndian => CupsRasterUnifiedReaderNextPage::V3LittleEndian(
                CupsRasterUnifiedPageReaderV3LE::reader_for(self.reader, self.limits),
            ),
        }
    }
}

#[pin_project(project = CupsRasterUnifiedReaderNextPageProj)]
pub enum CupsRasterUnifiedReaderNextPage<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    V1BigEndian(
        #[pin]
        CommonRasterPageReaderFor<
            CupsPageFactoryV1<BigEndian>,
            CupsPageHeaderV2,
            CupsRasterUnifiedDecoder<R>,
            R,
        >,
    ),
    V1LittleEndian(
        #[pin]
        CommonRasterPageReaderFor<
            CupsPageFactoryV1<LittleEndian>,
            CupsPageHeaderV2,
            CupsRasterUnifiedDecoder<R>,
            R,
        >,
    ),
    V2BigEndian(
        #[pin]
        CommonRasterPageReaderFor<
            CupsPageFactoryV2<BigEndian>,
            CupsPageHeaderV2,
            CupsRasterUnifiedDecoder<R>,
            R,
        >,
    ),
    V2LittleEndian(
        #[pin]
        CommonRasterPageReaderFor<
            CupsPageFactoryV2<LittleEndian>,
            CupsPageHeaderV2,
            CupsRasterUnifiedDecoder<R>,
            R,
        >,
    ),
    V3BigEndian(
        #[pin]
        CommonRasterPageReaderFor<
            CupsPageFactoryV3<BigEndian>,
            CupsPageHeaderV2,
            CupsRasterUnifiedDecoder<R>,
            R,
        >,
    ),
    V3LittleEndian(
        #[pin]
        CommonRasterPageReaderFor<
            CupsPageFactoryV3<LittleEndian>,
            CupsPageHeaderV2,
            CupsRasterUnifiedDecoder<R>,
            R,
        >,
    ),
}

impl<R> Future for CupsRasterUnifiedReaderNextPage<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    type Output = Result<Option<CupsRasterUnifiedPageReader<R>>, CupsRasterError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this {
            CupsRasterUnifiedReaderNextPageProj::V1BigEndian(reader) => reader
                .poll(cx)
                .map(|result| result.map(|reader| reader.map(CupsRasterUnifiedPageReader::from))),
            CupsRasterUnifiedReaderNextPageProj::V1LittleEndian(reader) => reader
                .poll(cx)
                .map(|result| result.map(|reader| reader.map(CupsRasterUnifiedPageReader::from))),
            CupsRasterUnifiedReaderNextPageProj::V2BigEndian(reader) => reader
                .poll(cx)
                .map(|result| result.map(|reader| reader.map(CupsRasterUnifiedPageReader::from))),
            CupsRasterUnifiedReaderNextPageProj::V2LittleEndian(reader) => reader
                .poll(cx)
                .map(|result| result.map(|reader| reader.map(CupsRasterUnifiedPageReader::from))),
            CupsRasterUnifiedReaderNextPageProj::V3BigEndian(reader) => reader
                .poll(cx)
                .map(|result| result.map(|reader| reader.map(CupsRasterUnifiedPageReader::from))),
            CupsRasterUnifiedReaderNextPageProj::V3LittleEndian(reader) => reader
                .poll(cx)
                .map(|result| result.map(|reader| reader.map(CupsRasterUnifiedPageReader::from))),
        }
    }
}

#[pin_project]
struct CupsRasterReaderReadSyncWord<R> {
    buffer: [u8; 4],
    num_read: usize,
    reader: Pin<R>,
}

impl<R> CupsRasterReaderReadSyncWord<R> {
    fn new(reader: Pin<R>) -> Self {
        CupsRasterReaderReadSyncWord {
            buffer: [0; 4],
            num_read: 0,
            reader,
        }
    }
}

impl<R> Future for CupsRasterReaderReadSyncWord<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    type Output = Result<CupsSyncWord, CupsRasterError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let mut buffer = &mut this.buffer[*this.num_read..];
        while !buffer.is_empty() {
            match this.reader.as_mut().poll_read(cx, buffer) {
                Poll::Ready(Ok(0)) => {
                    return Poll::Ready(Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()))
                }
                Poll::Ready(Ok(n)) => {
                    buffer = &mut buffer[n..];
                    *this.num_read += n;
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                Poll::Pending => return Poll::Pending,
            }
        }

        let sync_word = match this.buffer {
            [b'R', b'a', b'S', b't'] => CupsSyncWord::V1BigEndian,
            [b't', b'S', b'a', b'R'] => CupsSyncWord::V1LittleEndian,
            [b'R', b'a', b'S', b'2'] => CupsSyncWord::V2BigEndian,
            [b'2', b'S', b'a', b'R'] => CupsSyncWord::V2LittleEndian,
            [b'R', b'a', b'S', b'3'] => CupsSyncWord::V3BigEndian,
            [b'3', b'S', b'a', b'R'] => CupsSyncWord::V3LittleEndian,
            _ => return Poll::Ready(Err(CupsRasterError::InvalidSyncWord)),
        };
        Poll::Ready(Ok(sync_word))
    }
}
