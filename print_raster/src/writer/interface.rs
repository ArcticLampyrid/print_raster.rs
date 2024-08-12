use crate::encode::RasterEncoder;
use futures::AsyncWrite;
use std::{future::Future, ops::DerefMut};

pub trait RasterPageWriter<W>: Sized
where
    W: DerefMut<Target: AsyncWrite>,
{
    type Header;
    type Encoder: RasterEncoder<W>;
    type Error;
    type NextPageFuture<'a>: Future<Output = Result<Self, Self::Error>> + 'a
    where
        Self: 'a;
    type FinishFuture: Future<Output = Result<(), Self::Error>>;
    fn next_page<'a>(self, header: &'a Self::Header) -> Self::NextPageFuture<'a>
    where
        Self: 'a;
    fn finish(self) -> Self::FinishFuture;
    fn content_mut(&mut self) -> &mut Self::Encoder;
    fn into_content(self) -> Self::Encoder;
}

pub trait RasterWriter<W>: Sized
where
    W: DerefMut<Target: AsyncWrite>,
{
    type PageHeader;
    type PageWriter: RasterPageWriter<W, Header = Self::PageHeader>;
    type Error;
    type NextPageFuture<'a>: Future<Output = Result<Self::PageWriter, Self::Error>> + 'a
    where
        Self: 'a;
    type FinishFuture: Future<Output = Result<(), Self::Error>>;
    fn next_page<'a>(self, header: &'a Self::PageHeader) -> Self::NextPageFuture<'a>
    where
        Self: 'a;
    fn finish(self) -> Self::FinishFuture;
}
