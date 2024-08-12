use crate::decode::{Limits, RasterDecoder, RasterDecoderConsumer, RasterDecoderExt};
use crate::factory::RasterPageFactory;
use crate::reader::RasterPageReader;
use futures::ready;
use futures::task::Context;
use futures::AsyncRead;
use pin_project::pin_project;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::Poll;

/// A common implementation of `RasterPageReader` for all raster formats.
///
/// # Type parameters
/// - `F`: The `RasterPageFactory` implementation for the raster format.
/// - `HS`: The type to store the header.
/// - `DS`: The type to store the decoder.
/// - `R`: The mutable pointer to AsyncRead.
pub struct CommonRasterPageReader<F, HS, DS, R>
where
    F: RasterPageFactory,
    HS: From<<F as RasterPageFactory>::Header>,
    DS: From<<F as RasterPageFactory>::Decoder<R>> + RasterDecoder<R>,
    R: DerefMut<Target: AsyncRead>,
{
    header: HS,
    content: DS,
    limits: Limits,
    _factory: PhantomData<F>,
    _reader: PhantomData<R>,
}

impl<F, HS, DS, R> CommonRasterPageReader<F, HS, DS, R>
where
    F: RasterPageFactory,
    HS: From<<F as RasterPageFactory>::Header>,
    DS: From<<F as RasterPageFactory>::Decoder<R>> + RasterDecoder<R> + Unpin,
    R: DerefMut<Target: AsyncRead>,
    F::Error: From<std::io::Error>,
{
    /// Consumes the header of next page and returns a reader for the next page.
    pub fn reader_for(reader: Pin<R>, limits: Limits) -> CommonRasterPageReaderFor<F, HS, DS, R> {
        CommonRasterPageReaderFor {
            reader: Some(reader),
            buffer: vec![0; F::HEADER_SIZE],
            limits,
            start: 0,
            _header_storage: PhantomData,
            _decoder_storage: PhantomData,
            _factory: PhantomData,
        }
    }
}

impl<F, HS, DS, R> RasterPageReader<R> for CommonRasterPageReader<F, HS, DS, R>
where
    F: RasterPageFactory,
    HS: From<<F as RasterPageFactory>::Header>,
    DS: From<<F as RasterPageFactory>::Decoder<R>> + RasterDecoder<R> + Unpin,
    R: DerefMut<Target: AsyncRead>,
    F::Error: From<std::io::Error>,
{
    type Header = HS;
    type Decoder = DS;
    type Error = <F as RasterPageFactory>::Error;
    type NextPageFuture = CommonRasterPageReaderNext<F, HS, DS, R>;

    fn next_page(self) -> Self::NextPageFuture {
        let limits = self.limits.clone();
        let content = self.into_content().consume();
        CommonRasterPageReaderNext::Consume(content, limits)
    }

    fn header(&self) -> &Self::Header {
        &self.header
    }

    fn content_mut(&mut self) -> &mut Self::Decoder {
        &mut self.content
    }

    fn into_content(self) -> Self::Decoder {
        self.content
    }
}

#[pin_project]
pub struct CommonRasterPageReaderFor<F, HS, DS, R>
where
    F: RasterPageFactory,
    HS: From<<F as RasterPageFactory>::Header>,
    DS: From<<F as RasterPageFactory>::Decoder<R>> + RasterDecoder<R>,
    R: DerefMut<Target: AsyncRead>,
    F::Error: From<std::io::Error>,
{
    reader: Option<Pin<R>>,
    buffer: Vec<u8>,
    limits: Limits,
    start: usize,
    _header_storage: std::marker::PhantomData<HS>,
    _decoder_storage: std::marker::PhantomData<DS>,
    _factory: std::marker::PhantomData<F>,
}

impl<F, HS, DS, R> Future for CommonRasterPageReaderFor<F, HS, DS, R>
where
    F: RasterPageFactory,
    HS: From<<F as RasterPageFactory>::Header>,
    DS: From<<F as RasterPageFactory>::Decoder<R>> + RasterDecoder<R>,
    R: DerefMut<Target: AsyncRead>,
    F::Error: From<std::io::Error>,
{
    type Output = Result<Option<CommonRasterPageReader<F, HS, DS, R>>, F::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        if this.reader.is_none() {
            return Poll::Ready(Ok(None));
        }
        let reader = this.reader.as_mut().unwrap();
        loop {
            let buf = &mut this.buffer[*this.start..];
            let num_read = ready!(reader.as_mut().poll_read(cx, buf))?;
            *this.start += num_read;
            if *this.start >= F::HEADER_SIZE {
                // header is read
                break;
            }
            if num_read == 0 {
                return Poll::Ready(Ok(None));
            }
        }
        let header = F::header_from_bytes(this.buffer)?;
        let content = F::decode(&header, this.reader.take().unwrap(), this.limits)?;
        Poll::Ready(Ok(Some(CommonRasterPageReader {
            header: header.into(),
            content: content.into(),
            limits: this.limits.clone(),
            _factory: PhantomData,
            _reader: PhantomData,
        })))
    }
}

#[pin_project(project = CommonRasterPageReaderNextProj)]
pub enum CommonRasterPageReaderNext<F, HS, DS, R>
where
    F: RasterPageFactory,
    HS: From<<F as RasterPageFactory>::Header>,
    DS: From<<F as RasterPageFactory>::Decoder<R>> + RasterDecoder<R> + Unpin,
    R: DerefMut<Target: AsyncRead>,
    F::Error: From<std::io::Error>,
{
    Consume(#[pin] RasterDecoderConsumer<DS, R>, Limits),
    ReaderFor(#[pin] CommonRasterPageReaderFor<F, HS, DS, R>),
}

impl<F, HS, DS, R> Future for CommonRasterPageReaderNext<F, HS, DS, R>
where
    F: RasterPageFactory,
    HS: From<<F as RasterPageFactory>::Header>,
    DS: From<<F as RasterPageFactory>::Decoder<R>> + RasterDecoder<R> + Unpin,
    R: DerefMut<Target: AsyncRead>,
    F::Error: From<std::io::Error>,
{
    type Output = Result<Option<CommonRasterPageReader<F, HS, DS, R>>, F::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            match self.as_mut().project() {
                CommonRasterPageReaderNextProj::Consume(consumer, limits) => {
                    let reader = ready!(consumer.poll(cx))?;
                    let future =
                        CommonRasterPageReader::<F, HS, DS, R>::reader_for(reader, limits.clone());
                    self.set(CommonRasterPageReaderNext::ReaderFor(future));
                }
                CommonRasterPageReaderNextProj::ReaderFor(future) => return future.poll(cx),
            }
        }
    }
}
