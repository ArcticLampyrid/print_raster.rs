use crate::decode::RasterDecoder;
use futures::AsyncRead;
use std::{future::Future, ops::DerefMut};

pub trait RasterPageReader<R>: Sized
where
    R: DerefMut<Target: AsyncRead>,
{
    type Header;
    type Decoder: RasterDecoder<R>;
    type Error;
    type NextPageFuture: Future<Output = Result<Option<Self>, Self::Error>>;
    fn next_page(self) -> Self::NextPageFuture;
    fn header(&self) -> &Self::Header;
    fn content_mut(&mut self) -> &mut Self::Decoder;
    fn into_content(self) -> Self::Decoder;
}

pub trait RasterReader<R>: Sized
where
    R: DerefMut<Target: AsyncRead>,
{
    type PageHeader;
    type PageReader: RasterPageReader<R, Header = Self::PageHeader>;
    type Error;
    type NextPageFuture: Future<Output = Result<Option<Self::PageReader>, Self::Error>>;
    fn next_page(self) -> Self::NextPageFuture;
}
