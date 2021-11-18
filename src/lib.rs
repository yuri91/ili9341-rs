#![no_std]

//! ILI9341 Display Driver
//!
//! ### Usage
//!
//! To control the display you need to set up:
//!
//! * Interface for communicating with display ([display-interface-spi crate] for SPI)
//! * Configuration (reset pin, delay, orientation and size) for display
//!
//! ```ignore
//! let iface = SPIInterface::new(spi, dc, cs);
//!
//! let mut display = Ili9341::new(
//!     iface,
//!     reset_gpio,
//!     &mut delay,
//!     Orientation::Landscape,
//!     ili9341::DisplaySize240x320,
//! )
//! .unwrap();
//!
//! display.clear(Rgb565::RED).unwrap()
//! ```
//!
//! [display-interface-spi crate]: https://crates.io/crates/display-interface-spi
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::OutputPin;

use core::iter::once;
use display_interface::DataFormat::{U16BEIter, U8Iter};
use display_interface::WriteOnlyDataCommand;

#[cfg(feature = "graphics")]
mod graphics_core;

pub use embedded_hal::spi::MODE_0 as SPI_MODE;

pub use display_interface::DisplayError;

type Result<T = (), E = DisplayError> = core::result::Result<T, E>;

/// Trait that defines display size information
pub trait DisplaySize {
    /// Width in pixels
    const WIDTH: usize;
    /// Height in pixels
    const HEIGHT: usize;
}

/// Generic display size of 240x320 pixels
pub struct DisplaySize240x320;

impl DisplaySize for DisplaySize240x320 {
    const WIDTH: usize = 240;
    const HEIGHT: usize = 320;
}

/// Generic display size of 320x480 pixels
pub struct DisplaySize320x480;

impl DisplaySize for DisplaySize320x480 {
    const WIDTH: usize = 320;
    const HEIGHT: usize = 480;
}

/// For quite a few boards (ESP32-S2-Kaluga-1, M5Stack, M5Core2 and others),
/// the ILI9341 initialization command arguments are slightly different
///
/// This trait provides the flexibility for users to define their own
/// initialization command arguments suitable for the particular board they are using
pub trait Mode {
    fn mode(&self) -> u8;

    fn is_landscape(&self) -> bool;
}

/// The default implementation of the Mode trait from above
/// Should work for most (but not all) boards
pub enum Orientation {
    Portrait,
    PortraitFlipped,
    Landscape,
    LandscapeFlipped,
}

impl Mode for Orientation {
    fn mode(&self) -> u8 {
        match self {
            Self::Portrait => 0x40 | 0x08,
            Self::Landscape => 0x20 | 0x08,
            Self::PortraitFlipped => 0x80 | 0x08,
            Self::LandscapeFlipped => 0x40 | 0x80 | 0x20 | 0x08,
        }
    }

    fn is_landscape(&self) -> bool {
        match self {
            Self::Landscape | Self::LandscapeFlipped => true,
            Self::Portrait | Self::PortraitFlipped => false,
        }
    }
}

/// There are two method for drawing to the screen:
/// [Ili9341::draw_raw_iter] and [Ili9341::draw_raw_slice]
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
    landscape: bool,
}

impl<IFACE, RESET> Ili9341<IFACE, RESET>
where
    IFACE: WriteOnlyDataCommand,
    RESET: OutputPin,
{
    pub fn new<DELAY, SIZE, MODE>(
        interface: IFACE,
        reset: RESET,
        delay: &mut DELAY,
        mode: MODE,
        _display_size: SIZE,
    ) -> Result<Self>
    where
        DELAY: DelayMs<u16>,
        SIZE: DisplaySize,
        MODE: Mode,
    {
        let mut ili9341 = Ili9341 {
            interface,
            reset,
            width: SIZE::WIDTH,
            height: SIZE::HEIGHT,
            landscape: false,
        };

        // Do hardware reset by holding reset low for at least 10us
        ili9341.reset.set_low().map_err(|_| DisplayError::RSError)?;
        delay.delay_ms(1);
        // Set high for normal operation
        ili9341
            .reset
            .set_high()
            .map_err(|_| DisplayError::RSError)?;

        // Wait 5ms after reset before sending commands
        // and 120ms before sending Sleep Out
        delay.delay_ms(5);

        // Do software reset
        ili9341.command(Command::SoftwareReset, &[])?;

        // Wait 5ms after reset before sending commands
        // and 120ms before sending Sleep Out
        delay.delay_ms(120);

        ili9341.set_orientation(mode)?;

        // Set pixel format to 16 bits per pixel
        ili9341.command(Command::PixelFormatSet, &[0x55])?;

        ili9341.command(Command::SleepOut, &[])?;

        // Wait 5ms after Sleep Out before sending commands
        delay.delay_ms(5);

        ili9341.command(Command::DisplayOn, &[])?;

        Ok(ili9341)
    }
}

impl<IFACE, RESET> Ili9341<IFACE, RESET>
where
    IFACE: WriteOnlyDataCommand,
{
    fn command(&mut self, cmd: Command, args: &[u8]) -> Result {
        self.interface.send_commands(U8Iter(&mut once(cmd as u8)))?;
        self.interface.send_data(U8Iter(&mut args.iter().cloned()))
    }

    fn write_iter<I: IntoIterator<Item = u16>>(&mut self, data: I) -> Result {
        self.command(Command::MemoryWrite, &[])?;
        self.interface.send_data(U16BEIter(&mut data.into_iter()))
    }

    fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result {
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
        )
    }

    /// Configures the screen for hardware-accelerated vertical scrolling.
    pub fn configure_vertical_scroll(
        &mut self,
        fixed_top_lines: u16,
        fixed_bottom_lines: u16,
    ) -> Result<Scroller> {
        let height = if self.landscape {
            self.width
        } else {
            self.height
        } as u16;
        let scroll_lines = height as u16 - fixed_top_lines - fixed_bottom_lines;

        self.command(
            Command::VerticalScrollDefine,
            &[
                (fixed_top_lines >> 8) as u8,
                (fixed_top_lines & 0xff) as u8,
                (scroll_lines >> 8) as u8,
                (scroll_lines & 0xff) as u8,
                (fixed_bottom_lines >> 8) as u8,
                (fixed_bottom_lines & 0xff) as u8,
            ],
        )?;

        Ok(Scroller::new(fixed_top_lines, fixed_bottom_lines, height))
    }

    pub fn scroll_vertically(&mut self, scroller: &mut Scroller, num_lines: u16) -> Result {
        scroller.top_offset += num_lines;
        if scroller.top_offset > (scroller.height - scroller.fixed_bottom_lines) {
            scroller.top_offset = scroller.fixed_top_lines
                + (scroller.top_offset + scroller.fixed_bottom_lines - scroller.height)
        }

        self.command(
            Command::VerticalScrollAddr,
            &[
                (scroller.top_offset >> 8) as u8,
                (scroller.top_offset & 0xff) as u8,
            ],
        )
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
    pub fn draw_raw_iter<I: IntoIterator<Item = u16>>(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        data: I,
    ) -> Result {
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
    pub fn draw_raw_slice(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, data: &[u16]) -> Result {
        self.draw_raw_iter(x0, y0, x1, y1, data.iter().copied())
    }

    /// Change the orientation of the screen
    pub fn set_orientation<MODE>(&mut self, mode: MODE) -> Result
    where
        MODE: Mode,
    {
        self.command(Command::MemoryAccessControl, &[mode.mode()])?;

        if self.landscape ^ mode.is_landscape() {
            core::mem::swap(&mut self.height, &mut self.width);
        }
        self.landscape = mode.is_landscape();
        Ok(())
    }
}

impl<IFACE, RESET> Ili9341<IFACE, RESET> {
    /// Get the current screen width. It can change based on the current orientation
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get the current screen heighth. It can change based on the current orientation
    pub fn height(&self) -> usize {
        self.height
    }
}

/// Scroller must be provided in order to scroll the screen. It can only be obtained
/// by configuring the screen for scrolling.
pub struct Scroller {
    top_offset: u16,
    fixed_bottom_lines: u16,
    fixed_top_lines: u16,
    height: u16,
}

impl Scroller {
    fn new(fixed_top_lines: u16, fixed_bottom_lines: u16, height: u16) -> Scroller {
        Scroller {
            top_offset: fixed_top_lines,
            fixed_top_lines,
            fixed_bottom_lines,
            height,
        }
    }
}

#[derive(Clone, Copy)]
enum Command {
    SoftwareReset = 0x01,
    MemoryAccessControl = 0x36,
    PixelFormatSet = 0x3a,
    SleepOut = 0x11,
    DisplayOn = 0x29,
    ColumnAddressSet = 0x2a,
    PageAddressSet = 0x2b,
    MemoryWrite = 0x2c,
    VerticalScrollDefine = 0x33,
    VerticalScrollAddr = 0x37,
}
