#![no_std]

extern crate embedded_hal as hal;

#[cfg(feature = "graphics")]
extern crate embedded_graphics;

use hal::blocking::delay::DelayMs;
use hal::blocking::spi::{Write, Transfer};
use hal::digital::v2::OutputPin;

use core::fmt::Debug;
use core::iter::IntoIterator;

pub mod spi;
use spi::SpiInterface;

/// Trait representing the interface to the hardware.
///
/// Intended to abstract the various buses (SPI, MPU 8/9/16-bit) from the Controller code.
pub trait Interface {
    type Error;

    /// Sends a command with a sequence of 8-bit arguments
    ///
    /// Mostly used for sending configuration commands
    fn write(&mut self, command: u8, data: &[u8]) -> Result<(), Self::Error>;

    /// Sends a command with a sequence of 16-bit data words
    ///
    /// Mostly used for sending MemoryWrite command and other commands
    /// with 16-bit arguments
    fn write_iter(&mut self, command: u8, data: impl IntoIterator<Item = u16>) -> Result<(), Self::Error>;
}

const WIDTH: usize = 240;
const HEIGHT: usize = 320;

#[derive(Debug)]
pub enum Error<IfaceE, PinE> {
    Interface(IfaceE),
    OutputPin(PinE),
}

impl<IfaceE, PinE> From<IfaceE> for Error<IfaceE, PinE> {
    fn from(e: IfaceE) -> Self {
        Error::Interface(e)
    }
}

/// The default orientation is Portrait
pub enum Orientation {
    Portrait,
    PortraitFlipped,
    Landscape,
    LandscapeFlipped,
}

/// There are two method for drawing to the screen:
/// [draw_raw](struct.Ili9341.html#method.draw_raw) and
/// [draw_iter](struct.Ili9341.html#method.draw_iter).
///
/// In both cases the expected pixel format is rgb565.
///
/// The hardware makes it efficient to draw rectangles on the screen.
///
/// What happens is the following:
///
/// - A drawing window is prepared (with the 2 opposite corner coordinates)
/// - The starting point for drawint is the top left corner of this window
/// - Every pair of bytes received is intepreted as a pixel value in rgb565
/// - As soon as a pixel is received, an internal counter is incremented,
///   and the next word will fill the next pixel (the adjacent on the right, or
///   the first of the next row if the row ended)
pub struct Ili9341<IFACE, RESET> {
    interface: IFACE,
    reset: RESET,
    width: usize,
    height: usize,
}

impl<SpiE, PinE, SPI, CS, DC, RESET> Ili9341<SpiInterface<SPI, CS, DC>, RESET>
where
    SPI: Transfer<u8, Error = SpiE> + Write<u8, Error = SpiE>,
    CS: OutputPin<Error = PinE>,
    DC: OutputPin<Error = PinE>,
    RESET: OutputPin<Error = PinE>,
{
    pub fn new_spi<DELAY: DelayMs<u16>>(
        spi: SPI,
        cs: CS,
        dc: DC,
        reset: RESET,
        delay: &mut DELAY,
    ) -> Result<Self, Error<SpiE, PinE>> {
        let interface = SpiInterface::new(spi, cs, dc);
        Self::new(interface, reset, delay).map_err(|e| match e {
            Error::Interface(inner) => inner,
            Error::OutputPin(inner) => Error::OutputPin(inner),
        })
    }
}

impl<IfaceE, PinE, IFACE, RESET> Ili9341<IFACE, RESET>
    where
        IFACE: Interface<Error=IfaceE>,
        RESET: OutputPin<Error = PinE>,
{
    pub fn new<DELAY: DelayMs<u16>>(
        interface: IFACE,
        reset: RESET,
        delay: &mut DELAY,
    ) -> Result<Self, Error<IfaceE, PinE>> {
        let mut ili9341 = Ili9341 {
            interface,
            reset,
            width: WIDTH,
            height: HEIGHT,
        };

        ili9341.hard_reset(delay).map_err(Error::OutputPin)?;
        ili9341.command(Command::SoftwareReset, &[])?;
        delay.delay_ms(200);

        ili9341.command(Command::PowerControlA, &[0x39, 0x2c, 0x00, 0x34, 0x02])?;
        ili9341.command(Command::PowerControlB, &[0x00, 0xc1, 0x30])?;
        ili9341.command(Command::DriverTimingControlA, &[0x85, 0x00, 0x78])?;
        ili9341.command(Command::DriverTimingControlB, &[0x00, 0x00])?;
        ili9341.command(Command::PowerOnSequenceControl, &[0x64, 0x03, 0x12, 0x81])?;
        ili9341.command(Command::PumpRatioControl, &[0x20])?;
        ili9341.command(Command::PowerControl1, &[0x23])?;
        ili9341.command(Command::PowerControl2, &[0x10])?;
        ili9341.command(Command::VCOMControl1, &[0x3e, 0x28])?;
        ili9341.command(Command::VCOMControl2, &[0x86])?;
        ili9341.command(Command::MemoryAccessControl, &[0x48])?;
        ili9341.command(Command::PixelFormatSet, &[0x55])?;
        ili9341.command(Command::FrameControlNormal, &[0x00, 0x18])?;
        ili9341.command(Command::DisplayFunctionControl, &[0x08, 0x82, 0x27])?;
        ili9341.command(Command::Enable3G, &[0x00])?;
        ili9341.command(Command::GammaSet, &[0x01])?;
        ili9341.command(
            Command::PositiveGammaCorrection,
            &[
                0x0f, 0x31, 0x2b, 0x0c, 0x0e, 0x08, 0x4e, 0xf1, 0x37, 0x07, 0x10, 0x03, 0x0e, 0x09,
                0x00,
            ],
        )?;
        ili9341.command(
            Command::NegativeGammaCorrection,
            &[
                0x00, 0x0e, 0x14, 0x03, 0x11, 0x07, 0x31, 0xc1, 0x48, 0x08, 0x0f, 0x0c, 0x31, 0x36,
                0x0f,
            ],
        )?;
        ili9341.command(Command::SleepOut, &[])?;
        delay.delay_ms(120);
        ili9341.command(Command::DisplayOn, &[])?;

        Ok(ili9341)
    }

    fn hard_reset<DELAY: DelayMs<u16>>(
        &mut self,
        delay: &mut DELAY,
    ) -> Result<(), PinE> {
        // set high if previously low
        self.reset.set_high()?;
        delay.delay_ms(200);
        // set low for reset
        self.reset.set_low()?;
        delay.delay_ms(200);
        // set high for normal operation
        self.reset.set_high()?;
        delay.delay_ms(200);
        Ok(())
    }

    fn command(&mut self, cmd: Command, args: &[u8]) -> Result<(), IFACE::Error> {
        self.interface.write(cmd as u8, args)
    }

    fn write_iter<I: IntoIterator<Item = u16>>(
        &mut self,
        data: I,
    ) -> Result<(), IFACE::Error> {
        self.interface.write_iter(Command::MemoryWrite as u8, data)
    }

    fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), IFACE::Error> {
        self.command(
            Command::ColumnAddressSet,
            &[
                (x0 >> 8) as u8,
                (x0 & 0xff) as u8,
                (x1 >> 8) as u8,
                (x1 & 0xff) as u8,
            ],
        )?;
        self.command(
            Command::PageAddressSet,
            &[
                (y0 >> 8) as u8,
                (y0 & 0xff) as u8,
                (y1 >> 8) as u8,
                (y1 & 0xff) as u8,
            ],
        )?;
        Ok(())
    }

    /// Draw a rectangle on the screen, represented by top-left corner (x0, y0)
    /// and bottom-right corner (x1, y1).
    ///
    /// The border is included.
    ///
    /// This method accepts an iterator of rgb565 pixel values.
    ///
    /// The iterator is useful to avoid wasting memory by holding a buffer for
    /// the whole screen when it is not necessary.
    pub fn draw_iter<I: IntoIterator<Item = u16>>(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        data: I,
    ) -> Result<(), IFACE::Error> {
        self.set_window(x0, y0, x1, y1)?;
        self.write_iter(data)
    }

    /// Draw a rectangle on the screen, represented by top-left corner (x0, y0)
    /// and bottom-right corner (x1, y1).
    ///
    /// The border is included.
    ///
    /// This method accepts a raw buffer of words that will be copied to the screen
    /// video memory.
    ///
    /// The expected format is rgb565.
    pub fn draw_raw(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        data: &[u16],
    ) -> Result<(), IFACE::Error> {
        self.set_window(x0, y0, x1, y1)?;
        self.write_iter(data.iter().cloned())
    }

    /// Change the orientation of the screen
    pub fn set_orientation(&mut self, mode: Orientation) -> Result<(), IFACE::Error> {
        match mode {
            Orientation::Portrait => {
                self.width = WIDTH;
                self.height = HEIGHT;
                self.command(Command::MemoryAccessControl, &[0x40 | 0x08])
            }
            Orientation::Landscape => {
                self.width = HEIGHT;
                self.height = WIDTH;
                self.command(Command::MemoryAccessControl, &[0x20 | 0x08])
            }
            Orientation::PortraitFlipped => {
                self.width = WIDTH;
                self.height = HEIGHT;
                self.command(Command::MemoryAccessControl, &[0x80 | 0x08])
            }
            Orientation::LandscapeFlipped => {
                self.width = HEIGHT;
                self.height = WIDTH;
                self.command(Command::MemoryAccessControl, &[0x40 | 0x80 | 0x20 | 0x08])
            }
        }
    }

    /// Get the current screen width. It can change based on the current orientation
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get the current screen heighth. It can change based on the current orientation
    pub fn height(&self) -> usize {
        self.height
    }
}

#[cfg(feature = "graphics")]
use embedded_graphics::drawable;
#[cfg(feature = "graphics")]
use embedded_graphics::{drawable::Pixel, pixelcolor::Rgb565, Drawing};

#[cfg(feature = "graphics")]
impl<IfaceE, PinE, IFACE, RESET> Drawing<Rgb565> for Ili9341<IFACE, RESET>
where
    IFACE: Interface<Error = IfaceE>,
    RESET: OutputPin<Error = PinE>,
    IfaceE: Debug,
    PinE: Debug,
{
    fn draw<T>(&mut self, item_pixels: T)
    where
        T: IntoIterator<Item = drawable::Pixel<Rgb565>>,
    {
        const BUF_SIZE: usize = 32;

        let mut row: [u16; BUF_SIZE] = [0; BUF_SIZE];
        let mut i = 0;
        let mut lasty = 0;
        let mut startx = 0;
        let mut endx = 0;
        let width = self.width as i32;
        let height = self.height as i32;

        // Filter out pixels that are off the screen
        let on_screen_pixels = item_pixels.into_iter().filter(|drawable::Pixel(point, _)| {
            point.x >= 0 && point.y >= 0 && point.x < width && point.y < height
        });

        for Pixel(pos, color) in on_screen_pixels {
            use embedded_graphics::pixelcolor::raw::RawData;
            // Check if pixel is contiguous with previous pixel
            if i == 0 || (pos.y == lasty && (pos.x == endx + 1) && i < BUF_SIZE - 1) {
                if i == 0 {
                    // New line of pixels
                    startx = pos.x;
                }
                // Add pixel color to buffer
                row[i] = embedded_graphics::pixelcolor::raw::RawU16::from(color).into_inner();
                i += 1;
                lasty = pos.y;
                endx = pos.x;
            } else {
                // Line of contiguous pixels has ended, so draw it now
                self.draw_raw(
                    startx as u16,
                    lasty as u16,
                    endx as u16,
                    lasty as u16,
                    &row[0..i],
                )
                .expect("Failed to communicate with device");

                // Start new line of contiguous pixels
                i = 0;
                startx = pos.x;
                row[i] = embedded_graphics::pixelcolor::raw::RawU16::from(color).into_inner();
                i += 1;
                lasty = pos.y;
                endx = pos.x;
            }
        }
        if i > 0 {
            // Draw remaining pixels in buffer
            self.draw_raw(
                startx as u16,
                lasty as u16,
                endx as u16,
                lasty as u16,
                &row[0..i],
            )
            .expect("Failed to communicate with device");
        }
    }
}

#[derive(Clone, Copy)]
enum Command {
    SoftwareReset = 0x01,
    PowerControlA = 0xcb,
    PowerControlB = 0xcf,
    DriverTimingControlA = 0xe8,
    DriverTimingControlB = 0xea,
    PowerOnSequenceControl = 0xed,
    PumpRatioControl = 0xf7,
    PowerControl1 = 0xc0,
    PowerControl2 = 0xc1,
    VCOMControl1 = 0xc5,
    VCOMControl2 = 0xc7,
    MemoryAccessControl = 0x36,
    PixelFormatSet = 0x3a,
    FrameControlNormal = 0xb1,
    DisplayFunctionControl = 0xb6,
    Enable3G = 0xf2,
    GammaSet = 0x26,
    PositiveGammaCorrection = 0xe0,
    NegativeGammaCorrection = 0xe1,
    SleepOut = 0x11,
    DisplayOn = 0x29,
    ColumnAddressSet = 0x2a,
    PageAddressSet = 0x2b,
    MemoryWrite = 0x2c,
}
