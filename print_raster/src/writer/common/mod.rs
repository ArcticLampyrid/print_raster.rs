use super::RasterPageWriter;
use crate::{encode::RasterEncoder, factory::RasterPageFactory};
use futures::{ready, AsyncWrite};
use pin_project::pin_project;
use std::{
    future::Future,
    io,
    marker::PhantomData,
    ops::DerefMut,
    pin::Pin,
    task::{Context, Poll},
};

pub struct CommonRasterPageWriter<F, W>
where
    F: RasterPageFactory,
    W: DerefMut<Target: AsyncWrite>,
{
    content: <F as RasterPageFactory>::Encoder<W>,
}

impl<F, W> CommonRasterPageWriter<F, W>
where
    F: RasterPageFactory,
    W: DerefMut<Target: AsyncWrite>,
{
    /// Writes the header of the page and returns a writer for the page content.
    pub fn writer_for(
        header: &<F as RasterPageFactory>::Header,
        writer: Pin<W>,
    ) -> CommonRasterPageWriterFor<F, W> {
        CommonRasterPageWriterFor {
            header,
            writer: Some(writer),
            buffer: Vec::new(),
            start: 0,
            _factory: PhantomData,
        }
    }
}

#[pin_project]
pub struct CommonRasterPageWriterFor<'a, F, W>
where
    F: RasterPageFactory,
    W: DerefMut<Target: AsyncWrite>,
{
    header: &'a <F as RasterPageFactory>::Header,
    writer: Option<Pin<W>>,
    buffer: Vec<u8>,
    start: usize,
    _factory: PhantomData<F>,
}

impl<'a, F, W> Future for CommonRasterPageWriterFor<'a, F, W>
where
    F: RasterPageFactory,
    W: DerefMut<Target: AsyncWrite>,
    F::Error: From<io::Error>,
{
    type Output = Result<CommonRasterPageWriter<F, W>, <F as RasterPageFactory>::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        if this.writer.is_none() {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::Other,
                "writer is already taken",
            )
            .into()));
        }
        #[allow(clippy::uninit_vec)]
        if this.buffer.is_empty() {
            *this.buffer = unsafe {
                let mut buffer = Vec::new();
                buffer
                    .try_reserve(F::HEADER_SIZE)
                    .map_err(io::Error::from)?;
                buffer.set_len(F::HEADER_SIZE);
                buffer
            };
            F::header_to_bytes(this.buffer, this.header)?;
        }
        let writer = this.writer.as_mut().unwrap();
        loop {
            let buf = &mut this.buffer[*this.start..];
            let num_written = ready!(writer.as_mut().poll_write(cx, buf))?;
            *this.start += num_written;
            if *this.start >= F::HEADER_SIZE {
                // header is read
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
        let writer = this.writer.take().unwrap();
        Poll::Ready(Ok(CommonRasterPageWriter {
            content: F::encode(this.header, writer)?,
        }))
    }
}

impl<F, W> RasterPageWriter<W> for CommonRasterPageWriter<F, W>
where
    F: RasterPageFactory,
    W: DerefMut<Target: AsyncWrite>,
    F::Error: From<io::Error>,
{
    type Header = F::Header;
    type Encoder = F::Encoder<W>;
    type Error = F::Error;
    type NextPageFuture<'a> = CommonRasterPageWriterNext<'a, F, W>
    where
        Self: 'a;
    type FinishFuture = CommonRasterPageWriterFinish<W, Self::Error>;

    fn next_page<'a>(self, header: &'a Self::Header) -> Self::NextPageFuture<'a>
    where
        Self: 'a,
    {
        if self.content.bytes_remaining() > 0 {
            CommonRasterPageWriterNext::ErrorNotAllBytesWritten
        } else {
            CommonRasterPageWriterNext::NextPage(CommonRasterPageWriter::writer_for(
                header,
                self.into_content().into_pin_mut(),
            ))
        }
    }

    fn finish(self) -> Self::FinishFuture {
        CommonRasterPageWriterFinish {
            not_all_bytes_written: self.content.bytes_remaining() > 0,
            writer: self.content.into_pin_mut(),
            _error: PhantomData,
        }
    }

    fn content_mut(&mut self) -> &mut Self::Encoder {
        &mut self.content
    }

    fn into_content(self) -> Self::Encoder {
        self.content
    }
}

#[pin_project(project = CommonRasterPageWriterNextProj)]
pub enum CommonRasterPageWriterNext<'a, F, W>
where
    F: RasterPageFactory,
    W: DerefMut<Target: AsyncWrite>,
{
    ErrorNotAllBytesWritten,
    NextPage(#[pin] CommonRasterPageWriterFor<'a, F, W>),
}

impl<'a, F, W> Future for CommonRasterPageWriterNext<'a, F, W>
where
    F: RasterPageFactory,
    W: DerefMut<Target: AsyncWrite>,
    F::Error: From<io::Error>,
{
    type Output = Result<CommonRasterPageWriter<F, W>, <F as RasterPageFactory>::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match self.project() {
            CommonRasterPageWriterNextProj::ErrorNotAllBytesWritten => Poll::Ready(Err(
                io::Error::new(io::ErrorKind::Other, "not all bytes are written").into(),
            )),
            CommonRasterPageWriterNextProj::NextPage(fut) => fut.poll(cx),
        }
    }
}

#[pin_project(project = CommonRasterPageWriterFinishProj)]
pub struct CommonRasterPageWriterFinish<W, E> {
    writer: Pin<W>,
    not_all_bytes_written: bool,
    _error: PhantomData<E>,
}

impl<W, E> Future for CommonRasterPageWriterFinish<W, E>
where
    W: DerefMut<Target: AsyncWrite>,
    E: From<io::Error>,
{
    type Output = Result<(), E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        ready!(this.writer.as_mut().poll_close(cx))?;
        if *this.not_all_bytes_written {
            Poll::Ready(Err(io::Error::new(
                io::ErrorKind::Other,
                "not all bytes are written",
            )
            .into()))
        } else {
            Poll::Ready(Ok(()))
        }
    }
}
