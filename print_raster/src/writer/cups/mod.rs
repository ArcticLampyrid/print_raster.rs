use super::common::{CommonRasterPageWriter, CommonRasterPageWriterFor};
use super::RasterWriter;
use crate::error::CupsRasterError;
use crate::factory::{
    CupsPageFactoryV1, CupsPageFactoryV2, CupsPageFactoryV3, RasterPageFactory, WithCupsSyncWord,
};
use byteorder::{BigEndian, LittleEndian};
use futures::{ready, AsyncWrite};
use pin_project::pin_project;
use std::future::Future;
use std::io;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct CupsRasterWriter<F, W> {
    writer: Pin<W>,
    _factory: PhantomData<F>,
}

impl<W, F> CupsRasterWriter<F, W>
where
    F: RasterPageFactory + WithCupsSyncWord,
    F::Error: From<io::Error>,
    W: DerefMut<Target: AsyncWrite>,
{
    pub async fn new(mut writer: Pin<W>) -> Result<Self, CupsRasterError> {
        let buffer = (F::sync_word() as u32).to_ne_bytes();
        CupsRasterWriterWriteSyncWord {
            buffer,
            num_written: 0,
            writer: writer.as_mut(),
        }
        .await?;
        Ok(CupsRasterWriter {
            writer,
            _factory: PhantomData,
        })
    }
}

impl<W, F> RasterWriter<W> for CupsRasterWriter<F, W>
where
    F: RasterPageFactory<Error = CupsRasterError> + WithCupsSyncWord,
    W: DerefMut<Target: AsyncWrite>,
{
    type PageHeader = F::Header;
    type PageWriter = CommonRasterPageWriter<F, W>;
    type Error = CupsRasterError;
    type NextPageFuture<'a> = CommonRasterPageWriterFor<'a, F, W>
    where
        Self: 'a;
    type FinishFuture = futures::future::Ready<Result<(), CupsRasterError>>;

    fn next_page<'a>(self, header: &'a F::Header) -> Self::NextPageFuture<'a>
    where
        Self: 'a,
    {
        CommonRasterPageWriter::writer_for(header, self.writer)
    }

    fn finish(self) -> Self::FinishFuture {
        futures::future::ready(Ok(()))
    }
}

#[pin_project]
struct CupsRasterWriterWriteSyncWord<W> {
    buffer: [u8; 4],
    num_written: usize,
    writer: Pin<W>,
}

impl<W> Future for CupsRasterWriterWriteSyncWord<W>
where
    W: DerefMut<Target: AsyncWrite>,
{
    type Output = Result<(), CupsRasterError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        loop {
            let buf = &mut this.buffer[*this.num_written..];
            let num_written = ready!(this.writer.as_mut().poll_write(cx, buf))?;
            *this.num_written += num_written;
            if *this.num_written >= this.buffer.len() {
                // header is written
                break;
            }
            if num_written == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    "failed to write header",
                )
                .into()));
            }
        }
        Poll::Ready(Ok(()))
    }
}

pub type CupsRasterWriterV1BE<W> = CupsRasterWriter<CupsPageFactoryV1<BigEndian>, W>;
pub type CupsRasterWriterV1LE<W> = CupsRasterWriter<CupsPageFactoryV1<LittleEndian>, W>;
pub type CupsRasterWriterV2BE<W> = CupsRasterWriter<CupsPageFactoryV2<BigEndian>, W>;
pub type CupsRasterWriterV2LE<W> = CupsRasterWriter<CupsPageFactoryV2<LittleEndian>, W>;
pub type CupsRasterWriterV3BE<W> = CupsRasterWriter<CupsPageFactoryV3<BigEndian>, W>;
pub type CupsRasterWriterV3LE<W> = CupsRasterWriter<CupsPageFactoryV3<LittleEndian>, W>;

pub type CupsRasterPageWriterV1BE<W> = CommonRasterPageWriter<CupsPageFactoryV1<BigEndian>, W>;
pub type CupsRasterPageWriterV1LE<W> = CommonRasterPageWriter<CupsPageFactoryV1<LittleEndian>, W>;
pub type CupsRasterPageWriterV2BE<W> = CommonRasterPageWriter<CupsPageFactoryV2<BigEndian>, W>;
pub type CupsRasterPageWriterV2LE<W> = CommonRasterPageWriter<CupsPageFactoryV2<LittleEndian>, W>;
pub type CupsRasterPageWriterV3BE<W> = CommonRasterPageWriter<CupsPageFactoryV3<BigEndian>, W>;
pub type CupsRasterPageWriterV3LE<W> = CommonRasterPageWriter<CupsPageFactoryV3<LittleEndian>, W>;
