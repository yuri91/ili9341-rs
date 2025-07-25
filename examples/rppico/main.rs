//! Raspberry Pi Pico (RP2040) example
//! Tested on Raspberry Pi Pico and Raspberry Pi Pico W
//! Dependencies:
//!   rp-pico = "0.9"
//!   display-interface-spi = "0.4.1"
//!   embedded-graphics = "0.7.1"
//!   ili9341 = "0.5.0"
//! PIN ASSIGNMENTS
//!   GP10 (PIN14): DC
//!   GP11 (PIN15): RESET
//!   GP12 (PIN16): MISO
//!   GP13 (PIN17): CS
//!   GP14 (PIN19): SCL
//!   GP15 (PIN20): MOSI

#![no_std]
#![no_main]

use bsp::entry;
use defmt::*;
use defmt_rtt as _;
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle},
    text::Text,
};
use ili9341::{DisplayError, Ili9341};
use panic_probe as _;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};
use rp_pico::{
    self as bsp,
    hal::{fugit::RateExtU32, gpio::FunctionSpi, Spi},
};

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let spi_ch1: Spi<_, _, _, 8> = Spi::new(
        pac.SPI1,
        (
            pins.gpio15.into_function::<FunctionSpi>(),
            pins.gpio12.into_function::<FunctionSpi>(),
            pins.gpio14.into_function::<FunctionSpi>(),
        ),
    )
    .init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        16.MHz(),
        &embedded_hal::spi::MODE_0,
    );

    let spi_interface = SPIInterface::new(
        spi_ch1,
        pins.gpio10.into_push_pull_output(),
        pins.gpio13.into_push_pull_output(),
    );
    let mut display = Ili9341::new(
        spi_interface,
        pins.gpio11.into_push_pull_output(),
        &mut delay,
        ili9341::Orientation::Landscape,
        ili9341::DisplaySize240x320,
    )
    .unwrap();

    /* Draw to display */
    draw(&mut display);
    loop {}
}

/* embedded-graphics example art */
fn draw<T>(display: &mut T)
where
    T: DrawTarget<Color = Rgb565, Error = DisplayError>,
{
    let line_style = PrimitiveStyle::with_stroke(Rgb565::BLUE, 1);
    let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);

    Circle::new(Point::new(72, 8), 48)
        .into_styled(line_style)
        .draw(display)
        .unwrap();

    Line::new(Point::new(48, 16), Point::new(8, 16))
        .into_styled(line_style)
        .draw(display)
        .unwrap();

    Line::new(Point::new(48, 16), Point::new(64, 32))
        .into_styled(line_style)
        .draw(display)
        .unwrap();

    Rectangle::new(Point::new(79, 15), Size::new(34, 34))
        .into_styled(line_style)
        .draw(display)
        .unwrap();

    Text::new("Hello World!", Point::new(5, 5), text_style)
        .draw(display)
        .unwrap();
}
