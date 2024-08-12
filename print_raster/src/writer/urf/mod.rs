use super::common::{CommonRasterPageWriter, CommonRasterPageWriterFor};
use super::RasterWriter;
use crate::error::UrfError;
use crate::factory::UrfPageFactory;
use crate::model::urf::{UrfHeader, UrfPageHeader};
use futures::{ready, AsyncWrite};
use pin_project::pin_project;
use std::future::Future;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct UrfWriter<W> {
    writer: Pin<W>,
}

pub type UrfPageWriter<W> = CommonRasterPageWriter<UrfPageFactory, W>;

impl<W> UrfWriter<W>
where
    W: DerefMut<Target: AsyncWrite>,
{
    pub async fn new(mut writer: Pin<W>, header: &UrfHeader) -> Result<Self, UrfError> {
        let mut buffer = [0u8; 12];
        buffer[..8].copy_from_slice(b"UNIRAST\0");
        buffer[8..12].copy_from_slice(&header.page_count.to_be_bytes());
        UrfWriteHeaderFuture {
            buffer,
            num_written: 0,
            writer: writer.as_mut(),
        }
        .await?;
        Ok(UrfWriter { writer })
    }
}

impl<W> RasterWriter<W> for UrfWriter<W>
where
    W: DerefMut<Target: AsyncWrite>,
{
    type PageHeader = UrfPageHeader;
    type PageWriter = CommonRasterPageWriter<UrfPageFactory, W>;
    type Error = UrfError;
    type NextPageFuture<'a> = CommonRasterPageWriterFor<'a, UrfPageFactory, W>
    where
        Self: 'a;
    type FinishFuture = futures::future::Ready<Result<(), UrfError>>;

    fn next_page<'a>(self, header: &'a UrfPageHeader) -> Self::NextPageFuture<'a>
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
struct UrfWriteHeaderFuture<W> {
    buffer: [u8; 12],
    num_written: usize,
    writer: Pin<W>,
}

impl<W> Future for UrfWriteHeaderFuture<W>
where
    W: DerefMut<Target: AsyncWrite>,
{
    type Output = Result<(), UrfError>;

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
