use crate::decode::{CompressedRasterDecoder, Limits};
use crate::error::UrfError;
use crate::factory::UrfPageFactory;
use crate::model::urf::{UrfHeader, UrfPageHeader};
use crate::reader::common::CommonRasterPageReader;
use futures::AsyncRead;
use pin_project::pin_project;
use std::future::Future;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::common::CommonRasterPageReaderFor;
use super::RasterReader;

pub struct UrfReader<R> {
    reader: Pin<R>,
    header: UrfHeader,
    limits: Limits,
}

pub type UrfPageReader<R> =
    CommonRasterPageReader<UrfPageFactory, UrfPageHeader, CompressedRasterDecoder<R>, R>;

impl<R> UrfReader<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    pub async fn new(reader: Pin<R>) -> Result<Self, UrfError> {
        Self::new_with_limits(reader, Limits::default()).await
    }

    pub async fn new_with_limits(mut reader: Pin<R>, limits: Limits) -> Result<Self, UrfError> {
        let header = UrfReaderReadHeaderFuture {
            buffer: [0; 12],
            num_read: 0,
            reader: reader.as_mut(),
        }
        .await?;
        Ok(UrfReader {
            reader,
            header,
            limits,
        })
    }

    pub fn header(&self) -> &UrfHeader {
        &self.header
    }
}

impl<R> RasterReader<R> for UrfReader<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    type PageHeader = UrfPageHeader;
    type PageReader = UrfPageReader<R>;
    type Error = UrfError;
    type NextPageFuture =
        CommonRasterPageReaderFor<UrfPageFactory, UrfPageHeader, CompressedRasterDecoder<R>, R>;

    fn next_page(self) -> Self::NextPageFuture {
        UrfPageReader::reader_for(self.reader, self.limits)
    }
}

#[pin_project]
struct UrfReaderReadHeaderFuture<R> {
    buffer: [u8; 12],
    num_read: usize,
    reader: Pin<R>,
}

impl<R> Future for UrfReaderReadHeaderFuture<R>
where
    R: DerefMut<Target: AsyncRead>,
{
    type Output = Result<UrfHeader, UrfError>;

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
        if this.buffer[0..8] != *b"UNIRAST\0" {
            Poll::Ready(Err(UrfError::InvalidMagic))
        } else {
            Poll::Ready(Ok(UrfHeader {
                page_count: u32::from_be_bytes([
                    this.buffer[8],
                    this.buffer[9],
                    this.buffer[10],
                    this.buffer[11],
                ]),
            }))
        }
    }
}
