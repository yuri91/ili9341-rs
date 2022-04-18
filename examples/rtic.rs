//! cortex-m-rtic example
//! Tested on BlackPill dev board with stm32f411ceu microcontroller
//! The LCD RESET pin was hard puled to Vcc therefore
//! DummyOutputPin was used as the reset pin

#![no_main]
#![no_std]

#[rtic::app(device = stm32f4xx_hal::pac)]
mod app {
    use display_interface_spi::SPIInterface;
    use embedded_graphics::{
        mono_font::{ascii::FONT_6X10, MonoTextStyle},
        pixelcolor::Rgb565,
        prelude::*,
        text::{Alignment, Text},
    };
    use embedded_hal::digital::{blocking::OutputPin, ErrorType, PinState};
    use ili9341::{DisplaySize240x320, Ili9341, Orientation};
    use stm32f4xx_hal::{
        prelude::*,
        spi::{Mode, NoMiso, Phase, Polarity},
        timer::Channel,
    };

    #[derive(Default)]
    pub struct DummyOutputPin;
    impl ErrorType for DummyOutputPin {
        type Error = ();
    }

    impl OutputPin for DummyOutputPin {
        fn set_low(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn set_high(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn set_state(&mut self, _state: PinState) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let dp = ctx.device;

        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(100.MHz()).freeze();

        let gpioa = dp.GPIOA.split();
        let gpiob = dp.GPIOB.split();

        /*
         *  The ILI9341 driver
         */
        let lcd_clk = gpiob.pb0.into_alternate();
        let lcd_miso = NoMiso {};
        let lcd_mosi = gpioa.pa10.into_alternate().internal_pull_up(true);
        let lcd_dc = gpiob.pb1.into_push_pull_output();
        let lcd_cs = gpiob.pb2.into_push_pull_output();
        let mode = Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        };
        let lcd_spi = dp
            .SPI5
            .spi((lcd_clk, lcd_miso, lcd_mosi), mode, 2.MHz(), &clocks);
        let spi_iface = SPIInterface::new(lcd_spi, lcd_dc, lcd_cs);
        let dummy_reset = DummyOutputPin::default();
        let mut delay = dp.TIM1.delay_us(&clocks);
        let mut lcd = Ili9341::new(
            spi_iface,
            dummy_reset,
            &mut delay,
            Orientation::PortraitFlipped,
            DisplaySize240x320,
        )
        .unwrap();

        // Create a new character style
        let style = MonoTextStyle::new(&FONT_6X10, Rgb565::RED);

        // Create a text at position (20, 30) and draw it using the previously defined style
        Text::with_alignment(
            "First line\nSecond line",
            Point::new(20, 30),
            style,
            Alignment::Center,
        )
        .draw(&mut lcd)
        .unwrap();

        (Shared {}, Local {}, init::Monotonics())
    }

    #[idle(local = [])]
    fn idle(cx: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }
}
