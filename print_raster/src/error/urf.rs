use crate::model::urf::{UrfColorSpace, UrfDuplex, UrfMediaPosition, UrfMediaType, UrfQuality};
use num_enum::TryFromPrimitiveError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UrfError {
    #[error("IO error")]
    IoError(#[from] std::io::Error),
    #[error("Invalid magic")]
    InvalidMagic,
    #[error("Unknown color space")]
    UnknownColorSpace(#[from] TryFromPrimitiveError<UrfColorSpace>),
    #[error("Unknown duplex")]
    UnknownDuplex(#[from] TryFromPrimitiveError<UrfDuplex>),
    #[error("Unknown quality")]
    UnknownQuality(#[from] TryFromPrimitiveError<UrfQuality>),
    #[error("Unknown media position")]
    UnknownMediaPosition(#[from] TryFromPrimitiveError<UrfMediaPosition>),
    #[error("Unknown media type")]
    UnknownMediaType(#[from] TryFromPrimitiveError<UrfMediaType>),
    #[error("Data too large")]
    DataTooLarge,
}
