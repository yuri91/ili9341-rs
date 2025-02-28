use esp_hal::gpio::{Input, Level, Output, Pull};
use esp_hal::peripherals::{Peripherals, ADC1};
use display_interface_spi::{SPIInterface, *};
use embedded_graphics::mono_font::MonoFont;
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use embedded_graphics::{
    mono_font::{ascii::FONT_8X13, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, Triangle},
    text::{Alignment, Text},
};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_hal::{
    delay::Delay,
    gpio::{GpioPin, Level, Output},
    peripherals::SPI2,
    spi::{
        master::{Config, Spi},
        Mode,
    },
    time::RateExtU32,
    Blocking,
};
use ili9341::{DisplaySize240x320, Ili9341, Orientation};

type TFTSpiDevice<'spi> = ExclusiveDevice<Spi<'spi, Blocking>, Output<'spi>, NoDelay>;
type TFTSpiInterface<'spi> =
SPIInterface<ExclusiveDevice<Spi<'spi, Blocking>, Output<'spi>, NoDelay>, Output<'spi>>;

pub struct TFT<'spi> {
    display: Ili9341<TFTSpiInterface<'spi>, Output<'spi>>,
}

impl<'spi> TFT<'spi> {
    pub fn new(
        spi2: SPI2,
        sclk: GpioPin<19>,
        miso: GpioPin<20>,
        mosi: GpioPin<18>,
        cs: GpioPin<21>,
        rst: GpioPin<22>,
        dc: GpioPin<9>,
    ) -> TFT<'spi> {
        let rst_output = Output::new(rst, Level::Low);
        let dc_output = Output::new(dc, Level::Low);
        let spi = Spi::new(spi2, Self::create_config())
            .unwrap()
            .with_sck(sclk)
            .with_miso(miso) // order matters
            .with_mosi(mosi) // order matters
            ;
        let cs_output = Output::new(cs, Level::High);
        let spi_device = ExclusiveDevice::new_no_delay(spi, cs_output).unwrap();
        let interface = SPIInterface::new(spi_device, dc_output);

        let mut display = Ili9341::new(
            interface,
            rst_output,
            &mut Delay::new(),
            Orientation::Portrait,
            DisplaySize240x320,
        ).unwrap();

        TFT { display }
    }

    fn create_config() -> Config {
        Config::default()
            .with_frequency(100.kHz())
            .with_mode(Mode::_0)
    }

    pub fn clear(&mut self, color: Rgb565) {
        self.display.clear(color).unwrap();
    }

    pub fn part_clear(&mut self, x: i32, y: i32, w: u32, h: u32) {
        Rectangle::new(Point::new(x, y), Size::new(w, h))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(&mut self.display)
            .unwrap();
    }

    pub fn println(&mut self, text: &str, x: i32, y: i32) {
        let style = MonoTextStyle::new(&FONT_8X13, Rgb565::RED);
        Text::with_alignment(text, Point::new(x, y), style, Alignment::Center)
            .draw(&mut self.display)
            .unwrap();
    }
}

#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals: Peripherals = init(config);
    esp_alloc::heap_allocator!(72 * 1024);

    let dc = peripherals.GPIO9;
    let mosi = peripherals.GPIO18; // sdo -> MOSI
    let sclk = peripherals.GPIO19;
    let miso = peripherals.GPIO20; // sdi -> MISO
    let cs = peripherals.GPIO21;
    let rst = peripherals.GPIO22;

    let mut tft = TFT::new(peripherals.SPI2, sclk, miso, mosi, cs, rst, dc);
    tft.clear(Rgb565::WHITE);
    tft.println("Hello from ESP32-C6", 100, 40);

    loop {
        // your business logic
    }
}