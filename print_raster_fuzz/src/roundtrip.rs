use futures::{io::AsyncReadExt, AsyncRead, AsyncWrite, AsyncWriteExt};
use print_raster::{
    reader::{RasterPageReader, RasterReader},
    writer::{RasterPageWriter, RasterWriter},
};
use std::ops::DerefMut;

pub async fn roundtrip_raster<R, W, RR, RW, H>(
    reader: RR,
    writer: RW,
) -> Result<(), Box<dyn std::error::Error>>
where
    R: DerefMut<Target: AsyncRead>,
    W: DerefMut<Target: AsyncWrite>,
    RR: RasterReader<R, PageHeader = H>,
    RW: RasterWriter<W, PageHeader = H>,
    <RR as RasterReader<R>>::Error: std::error::Error + 'static,
    <RW as RasterWriter<W>>::Error: std::error::Error + 'static,
    <<RR as RasterReader<R>>::PageReader as RasterPageReader<R>>::Error:
        std::error::Error + 'static,
    <<RW as RasterWriter<W>>::PageWriter as RasterPageWriter<W>>::Error:
        std::error::Error + 'static,
    <<RR as RasterReader<R>>::PageReader as RasterPageReader<R>>::Decoder: Unpin,
    <<RW as RasterWriter<W>>::PageWriter as RasterPageWriter<W>>::Encoder: Unpin,
{
    // If any error occurs during reading, it will be returned as an error, since the input
    // data is not guaranteed to be valid.
    // If any error occurs during writing, it will be panic, because all data read successfully
    // should be written successfully.F
    let mut buffer = vec![0; 4096];
    let mut page_count = 0;
    if let Some(mut page_reader) = reader.next_page().await? {
        page_count += 1;
        let mut page_writer = writer.next_page(page_reader.header()).await.unwrap();

        loop {
            let n_read = page_reader.content_mut().read(&mut buffer).await?;
            if n_read == 0 {
                break;
            }
            page_writer
                .content_mut()
                .write_all(&buffer[..n_read])
                .await
                .unwrap();
        }

        let mut page_next_to_read = page_reader.next_page().await?;
        while let Some(mut page_reader) = page_next_to_read {
            page_count += 1;
            if page_count > 300 {
                return Err("page_count > 300".into());
            }

            page_writer = page_writer.next_page(page_reader.header()).await.unwrap();

            loop {
                let n_read = page_reader.content_mut().read(&mut buffer).await?;
                if n_read == 0 {
                    break;
                }
                page_writer
                    .content_mut()
                    .write_all(&buffer[..n_read])
                    .await
                    .unwrap();
            }

            page_next_to_read = page_reader.next_page().await?;
        }
    }
    Ok(())
}
