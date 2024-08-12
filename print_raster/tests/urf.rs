use futures::{io::BufReader, AsyncReadExt};
use image::{ImageBuffer, Luma};
use print_raster::{
    model::urf::{UrfColorSpace, UrfDuplex, UrfQuality},
    reader::{urf::UrfReader, RasterPageReader, RasterReader},
};
use std::{path::Path, pin::pin};
use tokio_util::compat::TokioAsyncReadCompatExt;

#[tokio::test]
async fn urf_single_sgray() {
    let name = "urf_sGray";
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/test_inputs/{}.ras", name));
    let file = tokio::fs::File::open(path).await.unwrap();
    let pinned_file_reader = pin!(BufReader::new(file.compat()));
    let reader = UrfReader::new(pinned_file_reader).await.unwrap();

    let mut page_index = 0;
    let mut page_next = reader.next_page().await.unwrap();
    while let Some(mut page) = page_next {
        println!("Page {}, Header = {:#?}", page_index, page.header());
        assert_eq!(page.header().bits_per_pixel, 8);
        assert_eq!(page.header().color_space, UrfColorSpace::sGray);
        assert_eq!(page.header().duplex, UrfDuplex::NoDuplex);
        assert_eq!(page.header().quality, UrfQuality::Default);

        let mut data = Vec::<u8>::new();
        page.content_mut().read_to_end(&mut data).await.unwrap();

        let img = ImageBuffer::<Luma<u8>, Vec<u8>>::from_vec(
            page.header().width as u32,
            page.header().height as u32,
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
