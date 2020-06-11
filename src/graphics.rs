use crate::{Ili9341, OutputPin};

use core::fmt::Debug;

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::{
        raw::{RawData, RawU16},
        Rgb565,
    },
    primitives::{ContainsPoint, Primitive, Rectangle},
    DrawTarget, Pixel,
};

impl<PinE, IFACE, RESET> DrawTarget for Ili9341<IFACE, RESET>
where
    IFACE: display_interface::WriteOnlyDataCommand,
    RESET: OutputPin<Error = PinE>,
    PinE: Debug,
{
    type Color = Rgb565;
    type Error = crate::Error<PinE>;

    fn size(&self) -> Size {
        Size::new(self.width as u32, self.height as u32)
    }

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            // Only draw pixels that would be on screen
            if coord.x >= 0
                && coord.y >= 0
                && coord.x < self.width as i32
                && coord.y < self.height as i32
            {
                self.draw_raw(
                    coord.x as u16,
                    coord.y as u16,
                    coord.x as u16,
                    coord.y as u16,
                    &[RawU16::from(color).into_inner()],
                )?;
            }
        }

        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        // Clamp area to drawable part of the display target
        let drawable_area = area.intersection(&Rectangle::new(Point::zero(), self.size()));

        if drawable_area.size != Size::zero() {
            self.draw_rect_iter(
                drawable_area.top_left.x as u16,
                drawable_area.top_left.y as u16,
                (drawable_area.top_left.x + (drawable_area.size.width - 1) as i32) as u16,
                (drawable_area.top_left.y + (drawable_area.size.height - 1) as i32) as u16,
                area.points()
                    .zip(colors)
                    .filter(|(pos, _color)| drawable_area.contains(*pos))
                    .map(|(_pos, color)| RawU16::from(color).into_inner()),
            )?;
        }

        Ok(())
    }
}
