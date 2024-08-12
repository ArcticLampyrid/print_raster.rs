//! A crate for processing print raster images in Rust.
//! # Example
//! ## Reading
//! First, pin a `AsyncRead` instance to the raster file you want to read. Then, create a suitable `RasterReader` instance and fetch the file header. After that, call `next_page` to read the raster image page by page.
//!
//! Note that it will consume the `RasterReader` instance after reading the first page, and it will consume the previous page after reading the next page. This is because the raster image is read in a streaming manner.
//!
//! ```rust
//! use futures::{io::BufReader, AsyncReadExt};
//! use print_raster::reader::{
//!     cups::unified::CupsRasterUnifiedReader, RasterPageReader, RasterReader,
//! };
//! use std::{path::Path, pin::pin};
//! use tokio_util::compat::TokioAsyncReadCompatExt;
//!
//! # let _ = tokio::runtime::Runtime::new().unwrap().block_on(async {
//! # let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/test_inputs/cups_v3_sRGB.ras"));
//! let file = tokio::fs::File::open(path).await?;
//! let pinned_file_reader = pin!(BufReader::new(file.compat()));
//! // Here we use CupsRasterUnifiedReader for CUPS Raster V1, V2, and V3.
//! // You may also use UrfPageReader for URF (Apple Raster).
//! let reader = CupsRasterUnifiedReader::new(pinned_file_reader).await?;
//! let mut page_index = 0;
//! let mut page_next = reader.next_page().await?;
//! while let Some(mut page) = page_next {
//!     // Read the metadata of page.
//!     println!(
//!         "Page {}, ByteOrder = {:?}, Header = {:#?}",
//!         page_index,
//!         page.byte_order(),
//!         page.header()
//!     );
//!     // Read the image data of page.
//!     let mut data = Vec::<u8>::new();
//!     page.content_mut().read_to_end(&mut data).await?;
//!     // Continue to read the next page.
//!     page_next = page.next_page().await?;
//!     page_index += 1;
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! # });
//! ```
//!
//! You may notice that the original `AsyncRead` instance is wrapped by `BufReader`. It is a common practice because the process of reading raster images will make small and repeated read calls to the underlying reader, which will cause a significant performance drop if the underlying reader is not buffered.
//!
//! ## Writing
//! Almost the same as reading, but you need to call `finish` after last page is written.
//!
//! ```rust
//! use futures::AsyncWriteExt;
//! use print_raster::{
//!     model::urf::{
//!         UrfColorSpace, UrfDuplex, UrfHeader, UrfMediaPosition, UrfMediaType, UrfPageHeader,
//!         UrfQuality,
//!     },
//!     writer::{urf::UrfWriter, RasterPageWriter, RasterWriter},
//! };
//! use std::pin::Pin;
//!
//! # let _ = tokio::runtime::Runtime::new().unwrap().block_on(async {
//! # const PIXEL_DATA: &[u8] = &[
//! #     0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff, 0xff,
//! #     0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0xff,
//! #     0xff, 0xff, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0xff, 0x00,
//! #     0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
//! #     0xff, 0xff, 0xff, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0xff, 0xff, 0x00,
//! #     0xff, 0xff, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
//! #     0x00, 0xff, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0xff, 0xff, 0x00,
//! #     0xff, 0xff, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
//! #     0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
//! #     0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00,
//! #     0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00,
//! #     0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00,
//! #     0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00,
//! # ];
//! // const PIXEL_DATA: &[u8] = <pixel data>;
//! assert_eq!(PIXEL_DATA.len(), 8 * 8 * 3);
//! let mut data = Vec::<u8>::new();
//! let writer = UrfWriter::new(Pin::new(&mut data), &UrfHeader { page_count: 2 })
//!     .await
//!     .unwrap();
//! let page_header = UrfPageHeader {
//!     bits_per_pixel: 24,
//!     color_space: UrfColorSpace::sRGB,
//!     width: 8,
//!     height: 8,
//!     duplex: UrfDuplex::NoDuplex,
//!     quality: UrfQuality::Normal,
//!     media_position: UrfMediaPosition::Auto,
//!     media_type: UrfMediaType::Auto,
//!     dot_per_inch: 300,
//! };
//! let mut page_writer = writer.next_page(&page_header).await.unwrap();
//! page_writer
//!     .content_mut()
//!     .write_all(PIXEL_DATA)
//!     .await
//!     .unwrap();
//! page_writer = page_writer.next_page(&page_header).await.unwrap();
//! page_writer
//!     .content_mut()
//!     .write_all(PIXEL_DATA)
//!     .await
//!     .unwrap();
//! page_writer.finish().await.unwrap();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! # });
//! ```

pub mod decode;
pub mod encode;
pub mod error;
pub mod factory;
pub mod model;
pub mod reader;
pub mod writer;
// Re-export byteorder crate.
pub use byteorder;
