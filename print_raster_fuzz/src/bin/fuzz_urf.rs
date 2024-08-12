use futures::io::Cursor;
use honggfuzz::fuzz;
use print_raster::{decode::Limits, reader::urf::UrfReader, writer::urf::UrfWriter};
use print_raster_fuzz::roundtrip_raster;
use std::pin::Pin;

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    const LIMITS: Limits = Limits {
        bytes_per_line: 8000 * 3,
        bytes_per_page: 8000 * 8000 * 3,
    };

    loop {
        fuzz!(|input: &[u8]| {
            let _ = rt.block_on(async move {
                let mut input = Cursor::new(input);
                let reader = UrfReader::new_with_limits(Pin::new(&mut input), LIMITS).await?;
                let mut output = Vec::new();
                let writer = UrfWriter::new(Pin::new(&mut output), reader.header())
                    .await
                    .unwrap();
                roundtrip_raster(reader, writer).await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            });
        });
    }
}
