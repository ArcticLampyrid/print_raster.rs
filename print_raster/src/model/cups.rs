use super::RasterByteOrder;
use num_enum::TryFromPrimitive;
use std::{array, hash::Hash};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
/// The sync word is a 32-bit value that identifies the version and byte order of the raster.
/// # Note
/// The enum underlying value is in native endianness, so it can be different if you print
/// it out as a number on different target platforms. But if you convert it to a byte array
/// using `u32::to_ne_bytes`, it will always be the same.
pub enum CupsSyncWord {
    V1BigEndian = u32::from_ne_bytes([b'R', b'a', b'S', b't']),
    V1LittleEndian = u32::from_ne_bytes([b't', b'S', b'a', b'R']),
    V2BigEndian = u32::from_ne_bytes([b'R', b'a', b'S', b'2']),
    V2LittleEndian = u32::from_ne_bytes([b'2', b'S', b'a', b'R']),
    V3BigEndian = u32::from_ne_bytes([b'R', b'a', b'S', b'3']),
    V3LittleEndian = u32::from_ne_bytes([b'3', b'S', b'a', b'R']),
}

impl CupsSyncWord {
    pub fn byte_order(&self) -> RasterByteOrder {
        match self {
            CupsSyncWord::V1BigEndian | CupsSyncWord::V2BigEndian | CupsSyncWord::V3BigEndian => {
                RasterByteOrder::BigEndian
            }
            CupsSyncWord::V1LittleEndian
            | CupsSyncWord::V2LittleEndian
            | CupsSyncWord::V3LittleEndian => RasterByteOrder::LittleEndian,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u32)]
pub enum CupsAdvance {
    Never = 0,
    AfterFile = 1,
    AfterJob = 2,
    AfterSet = 3,
    AfterPage = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u32)]
pub enum CupsCut {
    Never = 0,
    AfterFile = 1,
    AfterJob = 2,
    AfterSet = 3,
    AfterPage = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u32)]
pub enum CupsJog {
    Never = 0,
    AfterFile = 1,
    AfterJob = 2,
    AfterSet = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u32)]
pub enum CupsLeadingEdge {
    Top = 0,
    Right = 1,
    Bottom = 2,
    Left = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u32)]
pub enum CupsColorOrder {
    /// Chunky pixels (CMYK CMYK CMYK)
    Chunky = 0,
    /// Banded pixels (CCC MMM YYY KKK)
    Banded = 1,
    /// Planar pixels (CCC... MMM... YYY... KKK...)
    Planar = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u32)]
pub enum CupsColorSpace {
    /// Luminance (DeviceGray, gamma 2.2 by default)
    Gray = 0,
    /// Red, green, blue (DeviceRGB, sRGB by default)
    RGB,
    /// Red, green, blue, alpha (DeviceRGB, sRGB by default)
    RGBA,
    /// Black (DeviceK)
    Black,
    /// Cyan, magenta, yellow (DeviceCMY)
    CMY,
    /// Yellow, magenta, cyan
    /// (deprecated)
    YMC,
    /// Cyan, magenta, yellow, black (DeviceCMYK)
    CMYK,
    /// Yellow, magenta, cyan, black
    /// (deprecated)
    YMCK,
    /// Black, cyan, magenta, yellow
    /// (deprecated)
    KCMY,
    /// Black, cyan, magenta, yellow, light-cyan, light-magenta (deprecated)
    KCMYcm,
    /// Gold, magenta, yellow, black
    /// (deprecated)
    GMCK,
    /// Gold, magenta, yellow, silver
    /// (deprecated)
    GMCS,
    /// White ink (as black)
    /// (deprecated)
    White,
    /// Gold foil
    /// (deprecated)
    Gold,
    /// Silver foil
    /// (deprecated)
    Silver,
    /// CIE XYZ
    CIEXYZ,
    /// CIE Lab
    CIELab,
    /// Red, green, blue, white (DeviceRGB, sRGB by default)
    RGBW,
    /// Luminance (gamma 2.2)
    #[allow(non_camel_case_types)]
    sGray,
    /// Red, green, blue (sRGB)
    #[allow(non_camel_case_types)]
    sRGB,
    /// Red, green, blue (Adobe RGB)
    AdobeRGB,
    /// CIE Lab with hint for 1 color
    Icc1 = 32,
    /// CIE Lab with hint for 2 colors
    Icc2,
    /// CIE Lab with hint for 3 colors
    Icc3,
    /// CIE Lab with hint for 4 colors
    Icc4,
    /// CIE Lab with hint for 5 colors
    Icc5,
    /// CIE Lab with hint for 6 colors
    Icc6,
    /// CIE Lab with hint for 7 colors
    Icc7,
    /// CIE Lab with hint for 8 colors
    Icc8,
    /// CIE Lab with hint for 9 colors
    Icc9,
    /// CIE Lab with hint for 10 colors
    IccA,
    /// CIE Lab with hint for 11 colors
    IccB,
    /// CIE Lab with hint for 12 colors
    IccC,
    /// CIE Lab with hint for 13 colors
    IccD,
    /// CIE Lab with hint for 14 colors
    IccE,
    /// CIE Lab with hint for 15 colors
    IccF,
    /// Device color, 1 colorant
    Device1 = 48,
    /// Device color, 2 colorants
    Device2,
    /// Device color, 3 colorants
    Device3,
    /// Device color, 4 colorants
    Device4,
    /// Device color, 5 colorants
    Device5,
    /// Device color, 6 colorants
    Device6,
    /// Device color, 7 colorants
    Device7,
    /// Device color, 8 colorants
    Device8,
    /// Device color, 9 colorants
    Device9,
    /// Device color, 10 colorants
    DeviceA,
    /// Device color, 11 colorants
    DeviceB,
    /// Device color, 12 colorants
    DeviceC,
    /// Device color, 13 colorants
    DeviceD,
    /// Device color, 14 colorants
    DeviceE,
    /// Device color, 15 colorants
    DeviceF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u32)]
pub enum CupsOrientation {
    Portrait = 0,
    Landscape = 1,
    ReversePortrait = 2,
    ReverseLandscape = 3,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CupsResolution {
    pub cross_feed: u32,
    pub feed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CupsImagingBoundingBox<T> {
    pub left: T,
    pub bottom: T,
    pub right: T,
    pub top: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CupsMargins {
    pub left: u32,
    pub bottom: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CupsPageSize<T> {
    pub width: T,
    pub height: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CupsPageHeaderV1 {
    pub media_class: String,
    pub media_color: String,
    pub media_type: String,
    pub output_type: String,
    pub advance_distance: u32,
    pub advance_media: CupsAdvance,
    pub collate: bool,
    pub cut_media: CupsCut,
    pub duplex: bool,
    pub resolution: CupsResolution,
    /// The left, bottom, right, and top positions of the page bounding box in points
    pub imaging_bbox: CupsImagingBoundingBox<u32>,
    pub insert_sheet: bool,
    pub jog: CupsJog,
    pub leading_edge: CupsLeadingEdge,
    /// Left and bottom origin of image in points
    pub margins: CupsMargins,
    /// Manually feed media
    pub manual_feed: bool,
    pub media_position: u32,
    /// Media weight in grams per meter squared, 0 = printer default
    pub media_weight: u32,
    /// Mirror prints
    pub mirror_print: bool,
    /// Invert prints
    pub negative_print: bool,
    /// 0 = printer default
    pub num_copies: u32,
    pub orientation: CupsOrientation,
    /// `false` = Output face down, `true`` = Output face up
    pub output_face_up: bool,
    /// Width and length in points
    pub page_size: CupsPageSize<u32>,
    /// Print color separations
    pub separations: bool,
    /// Change trays if selected tray is empty
    pub tray_switch: bool,
    /// Rotate even pages when duplexing
    pub tumble: bool,
    /// Width of page image in pixels
    pub width: u32,
    /// Height of page image in pixels
    pub height: u32,
    /// Driver-specific
    pub cups_media_type: u32,
    pub bits_per_color: u32,
    pub bits_per_pixel: u32,
    pub bytes_per_line: u32,
    pub color_order: CupsColorOrder,
    pub color_space: CupsColorSpace,
    /// Driver-specific
    pub cups_compression: u32,
    /// Driver-specific
    pub cups_row_count: u32,
    /// Driver-specific
    pub cups_row_feed: u32,
    /// Driver-specific
    pub cups_row_step: u32,
}

impl CupsPageHeaderV1 {
    pub fn num_colors(&self) -> u32 {
        match self.color_space {
            CupsColorSpace::Gray
            | CupsColorSpace::White
            | CupsColorSpace::Black
            | CupsColorSpace::Gold
            | CupsColorSpace::Silver
            | CupsColorSpace::sGray => 1,
            CupsColorSpace::RGB
            | CupsColorSpace::CMY
            | CupsColorSpace::YMC
            | CupsColorSpace::CIEXYZ
            | CupsColorSpace::CIELab
            | CupsColorSpace::sRGB
            | CupsColorSpace::AdobeRGB
            | CupsColorSpace::Icc1
            | CupsColorSpace::Icc2
            | CupsColorSpace::Icc3
            | CupsColorSpace::Icc4
            | CupsColorSpace::Icc5
            | CupsColorSpace::Icc6
            | CupsColorSpace::Icc7
            | CupsColorSpace::Icc8
            | CupsColorSpace::Icc9
            | CupsColorSpace::IccA
            | CupsColorSpace::IccB
            | CupsColorSpace::IccC
            | CupsColorSpace::IccD
            | CupsColorSpace::IccE
            | CupsColorSpace::IccF => 3,
            CupsColorSpace::RGBA
            | CupsColorSpace::RGBW
            | CupsColorSpace::CMYK
            | CupsColorSpace::YMCK
            | CupsColorSpace::KCMY
            | CupsColorSpace::GMCK
            | CupsColorSpace::GMCS => 4,
            CupsColorSpace::KCMYcm => {
                if self.bits_per_pixel < 8 {
                    6
                } else {
                    4
                }
            }
            CupsColorSpace::Device1 => 1,
            CupsColorSpace::Device2 => 2,
            CupsColorSpace::Device3 => 3,
            CupsColorSpace::Device4 => 4,
            CupsColorSpace::Device5 => 5,
            CupsColorSpace::Device6 => 6,
            CupsColorSpace::Device7 => 7,
            CupsColorSpace::Device8 => 8,
            CupsColorSpace::Device9 => 9,
            CupsColorSpace::DeviceA => 10,
            CupsColorSpace::DeviceB => 11,
            CupsColorSpace::DeviceC => 12,
            CupsColorSpace::DeviceD => 13,
            CupsColorSpace::DeviceE => 14,
            CupsColorSpace::DeviceF => 15,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CupsPageHeaderV2 {
    pub v1: CupsPageHeaderV1,
    pub num_colors: u32,
    pub borderless_scaling_factor: f32,
    pub page_size_f32: CupsPageSize<f32>,
    pub imaging_bbox_f32: CupsImagingBoundingBox<f32>,
    pub vendor_u32: [u32; 16],
    pub vendor_f32: [f32; 16],
    pub vendor_str: [String; 16],
    pub marker_type: String,
    pub rendering_intent: String,
    pub page_size_name: String,
}

impl CupsPageHeaderV2 {
    pub fn num_colors(&self) -> u32 {
        match self.num_colors {
            0 => self.v1.num_colors(),
            _ => self.num_colors,
        }
    }
}

impl From<CupsPageHeaderV1> for CupsPageHeaderV2 {
    fn from(v1: CupsPageHeaderV1) -> Self {
        CupsPageHeaderV2 {
            num_colors: 0,
            borderless_scaling_factor: 0.0,
            page_size_f32: CupsPageSize {
                width: 0.0,
                height: 0.0,
            },
            imaging_bbox_f32: CupsImagingBoundingBox {
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
                top: 0.0,
            },
            vendor_u32: [0; 16],
            vendor_f32: [0.0; 16],
            vendor_str: array::from_fn(|_| String::new()),
            marker_type: String::new(),
            rendering_intent: String::new(),
            page_size_name: String::new(),
            v1,
        }
    }
}
