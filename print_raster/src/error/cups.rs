use crate::model::cups::{
    CupsAdvance, CupsColorOrder, CupsColorSpace, CupsCut, CupsJog, CupsLeadingEdge, CupsOrientation,
};
use num_enum::TryFromPrimitiveError;
use std::str::Utf8Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CupsRasterError {
    #[error("IO error")]
    IoError(#[from] std::io::Error),
    #[error("Invalid sync word")]
    InvalidSyncWord,
    #[error("Invalid string")]
    InvalidString(#[from] Utf8Error),
    #[error("Unknown advance media")]
    UnknownAdvanceMedia(#[from] TryFromPrimitiveError<CupsAdvance>),
    #[error("Unknown cut media")]
    UnknownCutMedia(#[from] TryFromPrimitiveError<CupsCut>),
    #[error("Unknown jog")]
    UnknownJog(#[from] TryFromPrimitiveError<CupsJog>),
    #[error("Unknown leading edge")]
    UnknownLeadingEdge(#[from] TryFromPrimitiveError<CupsLeadingEdge>),
    #[error("Unknown orientation")]
    UnknownOrientation(#[from] TryFromPrimitiveError<CupsOrientation>),
    #[error("Unknown color order")]
    UnknownColorOrder(#[from] TryFromPrimitiveError<CupsColorOrder>),
    #[error("Unknown color space")]
    UnknownColorSpace(#[from] TryFromPrimitiveError<CupsColorSpace>),
    #[error("String too long")]
    StringTooLong,
    #[error("Data layout error")]
    DataLayoutError,
    #[error("Data too large")]
    DataTooLarge,
}
