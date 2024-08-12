use futures::{io::BufReader, AsyncReadExt};
use image::{ImageBuffer, Rgb};
use print_raster::{
    model::cups::{CupsColorOrder, CupsColorSpace},
    reader::{cups::unified::CupsRasterUnifiedReader, RasterPageReader, RasterReader},
};
use std::{path::Path, pin::pin};
use tokio_util::compat::TokioAsyncReadCompatExt;

async fn cups_srgb(name: &str) {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/test_inputs/{}.ras", name));
    let file = tokio::fs::File::open(path).await.unwrap();
    let pinned_file_reader = pin!(BufReader::new(file.compat()));
    let reader = CupsRasterUnifiedReader::new(pinned_file_reader)
        .await
        .unwrap();

    let mut page_index = 0;
    let mut page_next = reader.next_page().await.unwrap();
    while let Some(mut page) = page_next {
        println!(
            "Page {}, ByteOrder = {:?}, Header = {:#?}",
            page_index,
            page.byte_order(),
            page.header()
        );
        assert_eq!(page.header().v1.bits_per_color, 8);
        assert_eq!(page.header().v1.bits_per_pixel, 24);
        assert_eq!(page.header().v1.color_order, CupsColorOrder::Chunky);
        assert_eq!(page.header().v1.color_space, CupsColorSpace::sRGB);

        let mut data = Vec::<u8>::new();
        page.content_mut().read_to_end(&mut data).await.unwrap();
        println!("{} bytes pixels read", data.len());

        let img = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_vec(
            page.header().v1.width as u32,
            page.header().v1.height as u32,
            data,
        )
        .unwrap();
        // make sure directory exists
        std::fs::create_dir_all(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/test_outputs/{}", name)),
        )
        .unwrap();
        img.save(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/test_outputs/{}/{}.png", name, page_index)),
        )
        .unwrap();

        page_next = page.next_page().await.unwrap();
        page_index += 1;
    }
}

#[tokio::test]
async fn pwg_srgb() {
    cups_srgb("pwg_sRGB").await;
}

#[tokio::test]
async fn cups_v3_srgb() {
    cups_srgb("cups_v3_sRGB").await;
}
