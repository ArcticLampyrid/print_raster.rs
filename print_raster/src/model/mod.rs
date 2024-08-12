pub mod cups;
pub mod urf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RasterByteOrder {
    BigEndian,
    LittleEndian,
}
