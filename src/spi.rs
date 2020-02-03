use embedded_hal::blocking::spi;
use embedded_hal::spi::{Mode, Phase, Polarity};
use embedded_hal::digital::v2::OutputPin;
use crate::{Interface, Error};

/// SPI mode
pub const MODE: Mode = Mode {
    polarity: Polarity::IdleLow,
    phase: Phase::CaptureOnFirstTransition,
};

/// `Interface` implementation for SPI interfaces
pub struct SpiInterface<SPI, CS, DC> {
    spi: SPI,
    cs: CS,
    dc: DC,
}

impl<SPI, CS, DC, SpiE, PinE> SpiInterface<SPI, CS, DC>
    where SPI: spi::Transfer<u8, Error = SpiE> + spi::Write<u8, Error = SpiE>,
          CS: OutputPin<Error = PinE>,
          DC: OutputPin<Error = PinE>,
{
    pub fn new(spi: SPI, cs: CS, dc: DC) -> Self {
        Self {
            spi,
            cs,
            dc,
        }
    }
}

impl<SPI, CS, DC, SpiE, PinE> Interface for SpiInterface<SPI, CS, DC>
    where SPI: spi::Transfer<u8, Error = SpiE> + spi::Write<u8, Error = SpiE>,
          CS: OutputPin<Error = PinE>,
          DC: OutputPin<Error = PinE>,
{
    type Error = Error<SpiE, PinE>;

    fn write(&mut self, command: u8, data: &[u8]) -> Result<(), Self::Error> {
        self.cs.set_low().map_err(Error::OutputPin)?;

        self.dc.set_low().map_err(Error::OutputPin)?;
        self.spi.write(&[command]).map_err(Error::Interface)?;

        self.dc.set_high().map_err(Error::OutputPin)?;
        self.spi.write(data).map_err(Error::Interface)?;

        self.cs.set_high().map_err(Error::OutputPin)?;
        Ok(())
    }

    fn write_iter(&mut self, command: u8, data: impl IntoIterator<Item=u16>) -> Result<(), Self::Error> {
        self.cs.set_low().map_err(Error::OutputPin)?;

        self.dc.set_low().map_err(Error::OutputPin)?;
        self.spi.write(&[command]).map_err(Error::Interface)?;

        self.dc.set_high().map_err(Error::OutputPin)?;
        for w in data.into_iter() {
            self.spi.write(&w.to_be_bytes()).map_err(Error::Interface)?;
        }

        self.cs.set_high().map_err(Error::OutputPin)?;
        Ok(())
    }
}
