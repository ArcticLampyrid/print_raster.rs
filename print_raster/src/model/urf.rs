use num_enum::TryFromPrimitive;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UrfHeader {
    pub page_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum UrfColorSpace {
    /// Luminance (gamma 2.2)
    #[allow(non_camel_case_types)]
    sGray,
    /// Red, green, blue (sRGB)
    #[allow(non_camel_case_types)]
    sRGB,
    /// CIE Lab
    CIELab,
    /// Red, green, blue (Adobe RGB)
    AdobeRGB,
    /// Luminance (DeviceGray, gamma 2.2 by default)
    Gray,
    /// Red, green, blue (DeviceRGB, sRGB by default)
    RGB,
    /// Cyan, magenta, yellow, black (DeviceCMYK)
    CMYK,
}

impl UrfColorSpace {
    pub fn num_colors(&self) -> usize {
        match self {
            UrfColorSpace::sGray | UrfColorSpace::Gray => 1,
            UrfColorSpace::sRGB
            | UrfColorSpace::RGB
            | UrfColorSpace::CIELab
            | UrfColorSpace::AdobeRGB => 3,
            UrfColorSpace::CMYK => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum UrfMediaType {
    Auto,
    Stationery,
    Transparency,
    Envelope,
    Cardstock,
    Labels,
    StationeryLetterhead,
    Disc,
    PhotographicMatte,
    PhotographicSatin,
    PhotographicSemiGloss,
    PhotographicGlossy,
    PhotographicHighGloss,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum UrfDuplex {
    NoDuplex = 1,
    ShortSide,
    LongSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum UrfQuality {
    Default = 0,
    Draft = 3,
    Normal,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum UrfMediaPosition {
    Auto = 0,
    Main,
    Alternate,
    LargeCapacity,
    Manual,
    Envelope,
    Disc,
    Photo,
    Hagaki,
    MainRoll,
    AlternateRoll,
    Top,
    Middle,
    Bottom,
    Side,
    Left,
    Right,
    Center,
    Rear,
    ByPassTray,
    Tray1,
    Tray2,
    Tray3,
    Tray4,
    Tray5,
    Tray6,
    Tray7,
    Tray8,
    Tray9,
    Tray10,
    Tray11,
    Tray12,
    Tray13,
    Tray14,
    Tray15,
    Tray16,
    Tray17,
    Tray18,
    Tray19,
    Tray20,
    Roll1,
    Roll2,
    Roll3,
    Roll4,
    Roll5,
    Roll6,
    Roll7,
    Roll8,
    Roll9,
    Roll10,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UrfPageHeader {
    pub bits_per_pixel: u8,
    pub color_space: UrfColorSpace,
    pub duplex: UrfDuplex,
    pub quality: UrfQuality,
    pub media_position: UrfMediaPosition,
    pub media_type: UrfMediaType,
    pub width: u32,
    pub height: u32,
    pub dot_per_inch: u32,
}
