use arbitrary::Arbitrary;
use futures::io::{AsyncReadExt, AsyncWriteExt, Cursor};
use honggfuzz::fuzz;
use print_raster::decode::{CompressedRasterDecoder, Limits};
use print_raster::encode::CompressedRasterEncoder;
use std::pin::Pin;

#[derive(Clone, Debug, Arbitrary)]
pub struct DataInput {
    pub data: Vec<u8>,
    pub chunk_size: u8,
    pub bytes_per_line: u64,
}
fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    loop {
        fuzz!(|input: DataInput| {
            let _ = rt.block_on(async move {
                let mut compressed = Vec::<u8>::new();
                let mut encoder = CompressedRasterEncoder::new(
                    Pin::new(&mut compressed),
                    input.chunk_size,
                    input.bytes_per_line,
                    input.data.len() as u64,
                )?;
                encoder.write_all(input.data.as_slice()).await.unwrap();
                encoder.close().await.unwrap();

                let mut compressed = Cursor::new(compressed);
                let mut decoder = CompressedRasterDecoder::new(
                    Pin::new(&mut compressed),
                    Limits::NO_LIMITS,
                    input.chunk_size,
                    input.bytes_per_line,
                    input.data.len() as u64,
                    0,
                )
                .unwrap();
                let mut decoded = Vec::<u8>::new();
                decoder.read_to_end(&mut decoded).await.unwrap();
                assert_eq!(input.data, decoded.as_slice());

                Ok::<(), Box<dyn std::error::Error>>(())
            });
        });
    }
}
