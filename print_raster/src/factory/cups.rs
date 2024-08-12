use super::RasterPageFactory;
use crate::{
    decode::{CompressedRasterDecoder, Limits, UncompressedRasterDecoder},
    encode::{CompressedRasterEncoder, UncompressedRasterEncoder},
    error::CupsRasterError,
    model::cups::{
        CupsAdvance, CupsColorOrder, CupsColorSpace, CupsCut, CupsImagingBoundingBox, CupsJog,
        CupsLeadingEdge, CupsMargins, CupsOrientation, CupsPageHeaderV1, CupsPageHeaderV2,
        CupsPageSize, CupsResolution, CupsSyncWord,
    },
};
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use futures::{AsyncRead, AsyncWrite};
use num_enum::TryFromPrimitive;
use std::{array, ops::DerefMut, pin::Pin, str};

fn read_c_string(content: &[u8]) -> Result<String, CupsRasterError> {
    let len = content
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(content.len());
    Ok(str::from_utf8(&content[0..len])?.to_string())
}

fn write_c_string(content: &mut [u8], s: &str) -> Result<(), CupsRasterError> {
    if s.len() > content.len() {
        return Err(CupsRasterError::StringTooLong);
    }
    content[0..s.len()].copy_from_slice(s.as_bytes());
    content[s.len()..].fill(0);
    Ok(())
}

fn read_bool(content: &[u8]) -> bool {
    content.iter().any(|&b| b != 0)
}

fn write_bool<TOrder>(content: &mut [u8], b: bool)
where
    TOrder: ByteOrder,
{
    TOrder::write_u32(content, if b { 1 } else { 0 });
}

fn read_page_header_v1<TOrder>(content: &[u8]) -> Result<CupsPageHeaderV1, CupsRasterError>
where
    TOrder: ByteOrder,
{
    let header = CupsPageHeaderV1 {
        media_class: read_c_string(&content[0..64])?,
        media_color: read_c_string(&content[64..128])?,
        media_type: read_c_string(&content[128..192])?,
        output_type: read_c_string(&content[192..256])?,
        advance_distance: TOrder::read_u32(&content[256..260]),
        advance_media: CupsAdvance::try_from_primitive(TOrder::read_u32(&content[260..264]))?,
        collate: read_bool(&content[264..268]),
        cut_media: CupsCut::try_from_primitive(TOrder::read_u32(&content[268..272]))?,
        duplex: read_bool(&content[272..276]),
        resolution: CupsResolution {
            cross_feed: TOrder::read_u32(&content[276..280]),
            feed: TOrder::read_u32(&content[280..284]),
        },
        imaging_bbox: CupsImagingBoundingBox {
            left: TOrder::read_u32(&content[284..288]),
            bottom: TOrder::read_u32(&content[288..292]),
            right: TOrder::read_u32(&content[292..296]),
            top: TOrder::read_u32(&content[296..300]),
        },
        insert_sheet: read_bool(&content[300..304]),
        jog: CupsJog::try_from_primitive(TOrder::read_u32(&content[304..308]))?,
        leading_edge: CupsLeadingEdge::try_from_primitive(TOrder::read_u32(&content[308..312]))?,
        margins: CupsMargins {
            left: TOrder::read_u32(&content[312..316]),
            bottom: TOrder::read_u32(&content[316..320]),
        },
        manual_feed: read_bool(&content[320..324]),
        media_position: TOrder::read_u32(&content[324..328]),
        media_weight: TOrder::read_u32(&content[328..332]),
        mirror_print: read_bool(&content[332..336]),
        negative_print: read_bool(&content[336..340]),
        num_copies: TOrder::read_u32(&content[340..344]),
        orientation: CupsOrientation::try_from_primitive(TOrder::read_u32(&content[344..348]))?,
        output_face_up: read_bool(&content[348..352]),
        page_size: CupsPageSize {
            width: TOrder::read_u32(&content[352..356]),
            height: TOrder::read_u32(&content[356..360]),
        },
        separations: read_bool(&content[360..364]),
        tray_switch: read_bool(&content[364..368]),
        tumble: read_bool(&content[368..372]),
        width: TOrder::read_u32(&content[372..376]),
        height: TOrder::read_u32(&content[376..380]),
        cups_media_type: TOrder::read_u32(&content[380..384]),
        bits_per_color: TOrder::read_u32(&content[384..388]),
        bits_per_pixel: TOrder::read_u32(&content[388..392]),
        bytes_per_line: TOrder::read_u32(&content[392..396]),
        color_order: CupsColorOrder::try_from_primitive(TOrder::read_u32(&content[396..400]))?,
        color_space: CupsColorSpace::try_from_primitive(TOrder::read_u32(&content[400..404]))?,
        cups_compression: TOrder::read_u32(&content[404..408]),
        cups_row_count: TOrder::read_u32(&content[408..412]),
        cups_row_feed: TOrder::read_u32(&content[412..416]),
        cups_row_step: TOrder::read_u32(&content[416..420]),
    };
    let chunk_size = match header.color_order {
        CupsColorOrder::Chunky => u8::try_from((header.bits_per_pixel as u64 + 7) / 8)
            .map_err(|_| CupsRasterError::DataTooLarge)?,
        CupsColorOrder::Banded | CupsColorOrder::Planar => {
            u8::try_from((header.bits_per_color as u64 + 7) / 8)
                .map_err(|_| CupsRasterError::DataTooLarge)?
        }
    }
    .max(1);
    if header.bytes_per_line != 0 && header.bytes_per_line % chunk_size as u32 != 0 {
        return Err(CupsRasterError::DataLayoutError);
    }
    Ok(header)
}

fn write_page_header_v1<TOrder>(
    content: &mut [u8],
    header: &CupsPageHeaderV1,
) -> Result<(), CupsRasterError>
where
    TOrder: ByteOrder,
{
    write_c_string(&mut content[0..64], &header.media_class)?;
    write_c_string(&mut content[64..128], &header.media_color)?;
    write_c_string(&mut content[128..192], &header.media_type)?;
    write_c_string(&mut content[192..256], &header.output_type)?;
    TOrder::write_u32(&mut content[256..260], header.advance_distance);
    TOrder::write_u32(&mut content[260..264], header.advance_media as u32);
    write_bool::<TOrder>(&mut content[264..268], header.collate);
    TOrder::write_u32(&mut content[268..272], header.cut_media as u32);
    write_bool::<TOrder>(&mut content[272..276], header.duplex);
    TOrder::write_u32(&mut content[276..280], header.resolution.cross_feed);
    TOrder::write_u32(&mut content[280..284], header.resolution.feed);
    TOrder::write_u32(&mut content[284..288], header.imaging_bbox.left);
    TOrder::write_u32(&mut content[288..292], header.imaging_bbox.bottom);
    TOrder::write_u32(&mut content[292..296], header.imaging_bbox.right);
    TOrder::write_u32(&mut content[296..300], header.imaging_bbox.top);
    write_bool::<TOrder>(&mut content[300..304], header.insert_sheet);
    TOrder::write_u32(&mut content[304..308], header.jog as u32);
    TOrder::write_u32(&mut content[308..312], header.leading_edge as u32);
    TOrder::write_u32(&mut content[312..316], header.margins.left);
    TOrder::write_u32(&mut content[316..320], header.margins.bottom);
    write_bool::<TOrder>(&mut content[320..324], header.manual_feed);
    TOrder::write_u32(&mut content[324..328], header.media_position);
    TOrder::write_u32(&mut content[328..332], header.media_weight);
    write_bool::<TOrder>(&mut content[332..336], header.mirror_print);
    write_bool::<TOrder>(&mut content[336..340], header.negative_print);
    TOrder::write_u32(&mut content[340..344], header.num_copies);
    TOrder::write_u32(&mut content[344..348], header.orientation as u32);
    write_bool::<TOrder>(&mut content[348..352], header.output_face_up);
    TOrder::write_u32(&mut content[352..356], header.page_size.width);
    TOrder::write_u32(&mut content[356..360], header.page_size.height);
    write_bool::<TOrder>(&mut content[360..364], header.separations);
    write_bool::<TOrder>(&mut content[364..368], header.tray_switch);
    write_bool::<TOrder>(&mut content[368..372], header.tumble);
    TOrder::write_u32(&mut content[372..376], header.width);
    TOrder::write_u32(&mut content[376..380], header.height);
    TOrder::write_u32(&mut content[380..384], header.cups_media_type);
    TOrder::write_u32(&mut content[384..388], header.bits_per_color);
    TOrder::write_u32(&mut content[388..392], header.bits_per_pixel);
    TOrder::write_u32(&mut content[392..396], header.bytes_per_line);
    TOrder::write_u32(&mut content[396..400], header.color_order as u32);
    TOrder::write_u32(&mut content[400..404], header.color_space as u32);
    TOrder::write_u32(&mut content[404..408], header.cups_compression);
    TOrder::write_u32(&mut content[408..412], header.cups_row_count);
    TOrder::write_u32(&mut content[412..416], header.cups_row_feed);
    TOrder::write_u32(&mut content[416..420], header.cups_row_step);
    Ok(())
}

fn read_page_header_v2<TOrder>(content: &[u8]) -> Result<CupsPageHeaderV2, CupsRasterError>
where
    TOrder: ByteOrder,
{
    Ok(CupsPageHeaderV2 {
        v1: read_page_header_v1::<TOrder>(&content[0..420])?,
        num_colors: TOrder::read_u32(&content[420..424]),
        borderless_scaling_factor: TOrder::read_f32(&content[424..428]),
        page_size_f32: CupsPageSize {
            width: TOrder::read_f32(&content[428..432]),
            height: TOrder::read_f32(&content[432..436]),
        },
        imaging_bbox_f32: CupsImagingBoundingBox {
            left: TOrder::read_f32(&content[436..440]),
            bottom: TOrder::read_f32(&content[440..444]),
            right: TOrder::read_f32(&content[444..448]),
            top: TOrder::read_f32(&content[448..452]),
        },
        vendor_u32: array::from_fn(|i| TOrder::read_u32(&content[452 + i * 4..456 + i * 4])),
        vendor_f32: array::from_fn(|i| TOrder::read_f32(&content[516 + i * 4..520 + i * 4])),
        vendor_str: array::from_fn(|i| {
            read_c_string(&content[580 + i * 64..644 + i * 64])
                .unwrap_or_else(|_| String::default())
        }),
        marker_type: read_c_string(&content[1604..1668])?,
        rendering_intent: read_c_string(&content[1668..1732])?,
        page_size_name: read_c_string(&content[1732..1796])?,
    })
}

fn write_page_header_v2<TOrder>(
    content: &mut [u8],
    header: &CupsPageHeaderV2,
) -> Result<(), CupsRasterError>
where
    TOrder: ByteOrder,
{
    write_page_header_v1::<TOrder>(&mut content[0..420], &header.v1)?;
    TOrder::write_u32(&mut content[420..424], header.num_colors);
    TOrder::write_f32(&mut content[424..428], header.borderless_scaling_factor);
    TOrder::write_f32(&mut content[428..432], header.page_size_f32.width);
    TOrder::write_f32(&mut content[432..436], header.page_size_f32.height);
    TOrder::write_f32(&mut content[436..440], header.imaging_bbox_f32.left);
    TOrder::write_f32(&mut content[440..444], header.imaging_bbox_f32.bottom);
    TOrder::write_f32(&mut content[444..448], header.imaging_bbox_f32.right);
    TOrder::write_f32(&mut content[448..452], header.imaging_bbox_f32.top);
    for (i, &v) in header.vendor_u32.iter().enumerate() {
        TOrder::write_u32(&mut content[452 + i * 4..456 + i * 4], v);
    }
    for (i, &v) in header.vendor_f32.iter().enumerate() {
        TOrder::write_f32(&mut content[516 + i * 4..520 + i * 4], v);
    }
    for (i, s) in header.vendor_str.iter().enumerate() {
        write_c_string(&mut content[580 + i * 64..644 + i * 64], s)?;
    }
    write_c_string(&mut content[1604..1668], &header.marker_type)?;
    write_c_string(&mut content[1668..1732], &header.rendering_intent)?;
    write_c_string(&mut content[1732..1796], &header.page_size_name)?;
    Ok(())
}

pub struct CupsPageFactoryV1<TOrder>
where
    TOrder: ByteOrder,
{
    _phantom: std::marker::PhantomData<TOrder>,
}
pub struct CupsPageFactoryV2<TOrder>
where
    TOrder: ByteOrder,
{
    _phantom: std::marker::PhantomData<TOrder>,
}

pub struct CupsPageFactoryV3<TOrder>
where
    TOrder: ByteOrder,
{
    _phantom: std::marker::PhantomData<TOrder>,
}

impl<TOrder> RasterPageFactory for CupsPageFactoryV1<TOrder>
where
    TOrder: ByteOrder,
{
    type Header = CupsPageHeaderV1;
    type Error = CupsRasterError;
    const HEADER_SIZE: usize = 420;
    fn header_from_bytes(content: &[u8]) -> Result<Self::Header, Self::Error> {
        read_page_header_v1::<TOrder>(content)
    }
    fn header_to_bytes(target: &mut [u8], header: &Self::Header) -> Result<(), Self::Error> {
        write_page_header_v1::<TOrder>(target, header)
    }

    type Decoder<R> = UncompressedRasterDecoder<R>
    where R: DerefMut<Target: AsyncRead>;
    fn decode<R>(
        header: &Self::Header,
        reader: Pin<R>,
        limits: &Limits,
    ) -> Result<Self::Decoder<R>, Self::Error>
    where
        R: DerefMut<Target: AsyncRead>,
    {
        let num_bytes = match header.color_order {
            CupsColorOrder::Chunky | CupsColorOrder::Banded => {
                header.bytes_per_line as u64 * header.height as u64
            }
            CupsColorOrder::Planar => (header.bytes_per_line as u64 * header.height as u64)
                .checked_mul(header.num_colors() as u64)
                .ok_or(CupsRasterError::DataTooLarge)?,
        };
        Ok(UncompressedRasterDecoder::new(reader, limits, num_bytes)?)
    }

    type Encoder<W> = UncompressedRasterEncoder<W> where
    W: DerefMut<Target: AsyncWrite>;
    fn encode<W>(header: &Self::Header, writer: Pin<W>) -> Result<Self::Encoder<W>, Self::Error>
    where
        W: DerefMut<Target: AsyncWrite>,
    {
        let num_bytes = match header.color_order {
            CupsColorOrder::Chunky | CupsColorOrder::Banded => {
                header.bytes_per_line as u64 * header.height as u64
            }
            CupsColorOrder::Planar => (header.bytes_per_line as u64 * header.height as u64)
                .checked_mul(header.num_colors() as u64)
                .ok_or(CupsRasterError::DataTooLarge)?,
        };
        Ok(UncompressedRasterEncoder::new(writer, num_bytes))
    }
}

impl<TOrder> RasterPageFactory for CupsPageFactoryV2<TOrder>
where
    TOrder: ByteOrder,
{
    type Header = CupsPageHeaderV2;
    type Error = CupsRasterError;
    const HEADER_SIZE: usize = 1796;
    fn header_from_bytes(content: &[u8]) -> Result<Self::Header, Self::Error> {
        read_page_header_v2::<TOrder>(content)
    }
    fn header_to_bytes(target: &mut [u8], header: &Self::Header) -> Result<(), Self::Error> {
        write_page_header_v2::<TOrder>(target, header)
    }

    type Decoder<R> = CompressedRasterDecoder<R>
    where
        R: DerefMut<Target: AsyncRead>;
    fn decode<R>(
        header: &Self::Header,
        reader: Pin<R>,
        limits: &Limits,
    ) -> Result<Self::Decoder<R>, Self::Error>
    where
        R: DerefMut<Target: AsyncRead>,
    {
        let chunk_size = match header.v1.color_order {
            CupsColorOrder::Chunky => u8::try_from((header.v1.bits_per_pixel as u64 + 7) / 8)
                .map_err(|_| CupsRasterError::DataTooLarge)?,
            CupsColorOrder::Banded | CupsColorOrder::Planar => {
                u8::try_from((header.v1.bits_per_color as u64 + 7) / 8)
                    .map_err(|_| CupsRasterError::DataTooLarge)?
            }
        }
        .max(1);
        let bytes_per_line = header.v1.bytes_per_line as u64;
        let num_bytes = match header.v1.color_order {
            CupsColorOrder::Chunky | CupsColorOrder::Banded => {
                header.v1.bytes_per_line as u64 * header.v1.height as u64
            }
            CupsColorOrder::Planar => (header.v1.bytes_per_line as u64 * header.v1.height as u64)
                .checked_mul(header.num_colors() as u64)
                .ok_or(CupsRasterError::DataTooLarge)?,
        };
        let fill_byte = match header.v1.color_space {
            CupsColorSpace::sGray
            | CupsColorSpace::sRGB
            | CupsColorSpace::CIELab
            | CupsColorSpace::AdobeRGB
            | CupsColorSpace::Gray
            | CupsColorSpace::RGB
            | CupsColorSpace::RGBA
            | CupsColorSpace::RGBW => 0xffu8,
            _ => 0u8,
        };
        Ok(CompressedRasterDecoder::new(
            reader,
            limits,
            chunk_size,
            bytes_per_line,
            num_bytes,
            fill_byte,
        )?)
    }

    type Encoder<W> = CompressedRasterEncoder<W>
    where
        W: DerefMut<Target: AsyncWrite>;
    fn encode<W>(header: &Self::Header, writer: Pin<W>) -> Result<Self::Encoder<W>, Self::Error>
    where
        W: DerefMut<Target: AsyncWrite>,
    {
        let chunk_size = match header.v1.color_order {
            CupsColorOrder::Chunky => u8::try_from((header.v1.bits_per_pixel as u64 + 7) / 8)
                .map_err(|_| CupsRasterError::DataTooLarge)?,
            CupsColorOrder::Banded | CupsColorOrder::Planar => {
                u8::try_from((header.v1.bits_per_color as u64 + 7) / 8)
                    .map_err(|_| CupsRasterError::DataTooLarge)?
            }
        }
        .max(1);
        let bytes_per_line = header.v1.bytes_per_line as u64;
        let num_bytes = match header.v1.color_order {
            CupsColorOrder::Chunky | CupsColorOrder::Banded => {
                header.v1.bytes_per_line as u64 * header.v1.height as u64
            }
            CupsColorOrder::Planar => (header.v1.bytes_per_line as u64 * header.v1.height as u64)
                .checked_mul(header.num_colors() as u64)
                .ok_or(CupsRasterError::DataTooLarge)?,
        };
        Ok(CompressedRasterEncoder::new(
            writer,
            chunk_size,
            bytes_per_line,
            num_bytes,
        )?)
    }
}

impl<TOrder> RasterPageFactory for CupsPageFactoryV3<TOrder>
where
    TOrder: ByteOrder,
{
    type Header = CupsPageHeaderV2;
    type Error = CupsRasterError;
    const HEADER_SIZE: usize = 1796;
    fn header_from_bytes(content: &[u8]) -> Result<Self::Header, Self::Error> {
        read_page_header_v2::<TOrder>(content)
    }
    fn header_to_bytes(target: &mut [u8], header: &Self::Header) -> Result<(), Self::Error> {
        write_page_header_v2::<TOrder>(target, header)
    }

    type Decoder<R> = UncompressedRasterDecoder<R>
    where
        R: DerefMut<Target: AsyncRead>;
    fn decode<R>(
        header: &Self::Header,
        reader: Pin<R>,
        limits: &Limits,
    ) -> Result<Self::Decoder<R>, Self::Error>
    where
        R: DerefMut<Target: AsyncRead>,
    {
        let num_bytes = match header.v1.color_order {
            CupsColorOrder::Chunky | CupsColorOrder::Banded => {
                header.v1.bytes_per_line as u64 * header.v1.height as u64
            }
            CupsColorOrder::Planar => (header.v1.bytes_per_line as u64 * header.v1.height as u64)
                .checked_mul(header.num_colors() as u64)
                .ok_or(CupsRasterError::DataTooLarge)?,
        };
        Ok(UncompressedRasterDecoder::new(reader, limits, num_bytes)?)
    }

    type Encoder<W> = UncompressedRasterEncoder<W> where
    W: DerefMut<Target: AsyncWrite>;
    fn encode<W>(header: &Self::Header, writer: Pin<W>) -> Result<Self::Encoder<W>, Self::Error>
    where
        W: DerefMut<Target: AsyncWrite>,
    {
        let num_bytes = match header.v1.color_order {
            CupsColorOrder::Chunky | CupsColorOrder::Banded => {
                header.v1.bytes_per_line as u64 * header.v1.height as u64
            }
            CupsColorOrder::Planar => (header.v1.bytes_per_line as u64 * header.v1.height as u64)
                .checked_mul(header.num_colors() as u64)
                .ok_or(CupsRasterError::DataTooLarge)?,
        };
        Ok(UncompressedRasterEncoder::new(writer, num_bytes))
    }
}

pub trait WithCupsSyncWord {
    fn sync_word() -> CupsSyncWord;
}

impl WithCupsSyncWord for CupsPageFactoryV1<BigEndian> {
    fn sync_word() -> CupsSyncWord {
        CupsSyncWord::V1BigEndian
    }
}

impl WithCupsSyncWord for CupsPageFactoryV1<LittleEndian> {
    fn sync_word() -> CupsSyncWord {
        CupsSyncWord::V1LittleEndian
    }
}

impl WithCupsSyncWord for CupsPageFactoryV2<BigEndian> {
    fn sync_word() -> CupsSyncWord {
        CupsSyncWord::V2BigEndian
    }
}

impl WithCupsSyncWord for CupsPageFactoryV2<LittleEndian> {
    fn sync_word() -> CupsSyncWord {
        CupsSyncWord::V2LittleEndian
    }
}

impl WithCupsSyncWord for CupsPageFactoryV3<BigEndian> {
    fn sync_word() -> CupsSyncWord {
        CupsSyncWord::V3BigEndian
    }
}

impl WithCupsSyncWord for CupsPageFactoryV3<LittleEndian> {
    fn sync_word() -> CupsSyncWord {
        CupsSyncWord::V3LittleEndian
    }
}
