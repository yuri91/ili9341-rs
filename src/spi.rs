use embedded_hal::spi::{Mode, Phase, Polarity};

/// SPI mode
pub const MODE: Mode = Mode {
    polarity: Polarity::IdleLow,
    phase: Phase::CaptureOnFirstTransition,
};
