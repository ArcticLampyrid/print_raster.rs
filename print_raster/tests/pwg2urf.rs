use futures::io::{BufReader, BufWriter};
use print_raster::{
    model::{
        cups::{CupsColorSpace, CupsPageHeaderV2},
        urf::{
            UrfColorSpace, UrfDuplex, UrfHeader, UrfMediaPosition, UrfMediaType, UrfPageHeader,
            UrfQuality,
        },
    },
    reader::{cups::unified::CupsRasterUnifiedReader, RasterPageReader, RasterReader},
    writer::{urf::UrfWriter, RasterPageWriter, RasterWriter},
};
use std::{path::Path, pin::pin};
use tokio_util::compat::TokioAsyncReadCompatExt;

fn cups_page_header_v2_to_urf_page_header(c: &CupsPageHeaderV2) -> UrfPageHeader {
    UrfPageHeader {
        bits_per_pixel: c.v1.bits_per_pixel.try_into().unwrap(),
        color_space: match c.v1.color_space {
            CupsColorSpace::sGray => UrfColorSpace::sGray,
            CupsColorSpace::sRGB => UrfColorSpace::sRGB,
            CupsColorSpace::CIELab => UrfColorSpace::CIELab,
            CupsColorSpace::AdobeRGB => UrfColorSpace::AdobeRGB,
            CupsColorSpace::Gray => UrfColorSpace::Gray,
            CupsColorSpace::RGB => UrfColorSpace::RGB,
            CupsColorSpace::CMYK => UrfColorSpace::CMYK,
            _ => panic!("Unsupported color space"),
        },
        width: c.v1.width,
        height: c.v1.height,
        duplex: UrfDuplex::NoDuplex,
        quality: UrfQuality::Normal,
        media_position: UrfMediaPosition::Auto,
        media_type: UrfMediaType::Auto,
        dot_per_inch: {
            assert_eq!(
                c.v1.resolution.cross_feed, c.v1.resolution.feed,
                "Cross feed and feed resolutions must be equal"
            );
            c.v1.resolution.cross_feed
        },
    }
}

#[tokio::test]
async fn pwg2urf() {
    let input_name = "pwg_sRGB";
    let input_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/test_inputs/{}.ras", input_name));
    let input_file = tokio::fs::File::open(input_path).await.unwrap();
    let pinned_file_reader = pin!(BufReader::new(input_file.compat()));
    let reader = CupsRasterUnifiedReader::new(pinned_file_reader)
        .await
        .unwrap();

    let output_name = "pwg2urf_sRGB";
    let output_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/test_outputs/{}.ras", output_name));
    let output_file = tokio::fs::File::create(output_path).await.unwrap();
    let pinned_file_writer = pin!(BufWriter::new(output_file.compat()));
    let writer = UrfWriter::new(pinned_file_writer, &UrfHeader { page_count: 0 })
        .await
        .unwrap();
    let mut page_index = 0;
    if let Some(mut page_reader) = reader.next_page().await.unwrap() {
        let mut page_writer = writer
            .next_page(&cups_page_header_v2_to_urf_page_header(
                page_reader.header(),
            ))
            .await
            .unwrap();
        let n_copied = futures::io::copy(page_reader.content_mut(), page_writer.content_mut())
            .await
            .unwrap();
        println!("Page {}: Copied {} bytes pixels", page_index, n_copied);
        page_index += 1;

        let mut page_next_to_read = page_reader.next_page().await.unwrap();
        while let Some(mut page_reader) = page_next_to_read {
            page_writer = page_writer
                .next_page(&cups_page_header_v2_to_urf_page_header(
                    page_reader.header(),
                ))
                .await
                .unwrap();
            let n_copied = futures::io::copy(page_reader.content_mut(), page_writer.content_mut())
                .await
                .unwrap();
            println!("Page {}: Copied {} bytes pixels", page_index, n_copied);
            page_index += 1;
            page_next_to_read = page_reader.next_page().await.unwrap();
        }
        page_writer.finish().await.unwrap();
    }
}
