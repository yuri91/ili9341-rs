use crate::{Error, Interface};
use embedded_hal::digital::v2::OutputPin;

/// `Interface` implementation for GPIO interfaces
pub struct Gpio8Interface<'a, DATA, CSX, WRX, RDX, DCX> {
	data_pins: &'a mut [DATA; 8],
	csx: CSX,
	wrx: WRX,
	rdx: RDX,
	dcx: DCX,
}

impl<'a, CSX, WRX, RDX, DCX, PinE>
	Gpio8Interface<'_, &'a mut dyn OutputPin<Error = PinE>, CSX, WRX, RDX, DCX>
where
	CSX: OutputPin<Error = PinE>,
	WRX: OutputPin<Error = PinE>,
	RDX: OutputPin<Error = PinE>,
	DCX: OutputPin<Error = PinE>,
{
	/// Create a new Gpio8Interface
	///
	/// Example useage:
	///
	/// let csx = gpioc.pc2.into_push_pull_output();
	/// let wrx = gpiod.pd13.into_push_pull_output();
	/// let rdx = gpiod.pd12.into_push_pull_output();
	/// let dcx = gpiof.pf7.into_push_pull_output();
	///
	/// let mut data_pins: [&mut dyn OutputPin<Error = _>; 8] = [
	/// 	&mut gpiod.pd6.into_push_pull_output(),
	/// 	&mut gpiog.pg11.into_push_pull_output(),
	/// 	...
	/// ];
	///
	/// let if_gpio = ili9341::gpio::Gpio8Interface::new(&mut data_pins, csx, wrx, rdx, dcx);
	pub fn new(
		data_pins: &'a mut [&'a mut dyn OutputPin<Error = PinE>; 8],
		csx: CSX,
		wrx: WRX,
		rdx: RDX,
		dcx: DCX,
	) -> Self {
		Self {
			data_pins,
			csx,
			wrx,
			rdx,
			dcx,
		}
	}

	/// Sets the gpio data pins used in the parallel interface
	fn set_data_bus(&mut self, data: u8) -> Result<(), Error<PinE, PinE>> {
		for (i, d) in self.data_pins.iter_mut().enumerate() {
			if ((data >> i) & 0b1) == 0b1 {
				d.set_high().map_err(Error::OutputPin)?;
			} else {
				d.set_low().map_err(Error::OutputPin)?;
			}
		}
		Ok(())
	}
}

impl<'a, CSX, WRX, RDX, DCX, PinE> Interface
	for Gpio8Interface<'_, &mut dyn OutputPin<Error = PinE>, CSX, WRX, RDX, DCX>
where
	CSX: OutputPin<Error = PinE>,
	WRX: OutputPin<Error = PinE>,
	RDX: OutputPin<Error = PinE>,
	DCX: OutputPin<Error = PinE>,
{
	type Error = Error<PinE, PinE>;

	fn write(&mut self, command: u8, data: &[u8]) -> Result<(), Self::Error> {
		self.csx.set_low().map_err(Error::OutputPin)?;
		self.rdx.set_high().map_err(Error::OutputPin)?;
		self.dcx.set_low().map_err(Error::OutputPin)?;
		self.wrx.set_low().map_err(Error::OutputPin)?;

		self.set_data_bus(command)?;
		self.wrx.set_high().map_err(Error::OutputPin)?;

		self.dcx.set_high().map_err(Error::OutputPin)?;
		for val in data.iter() {
			self.wrx.set_low().map_err(Error::OutputPin)?;
			self.set_data_bus(*val)?;
			self.wrx.set_high().map_err(Error::OutputPin)?;
		}

		self.csx.set_high().map_err(Error::OutputPin)?;

		Ok(())
	}

	fn write_iter(
		&mut self,
		command: u8,
		data: impl IntoIterator<Item = u16>,
	) -> Result<(), Self::Error> {
		self.csx.set_low().map_err(Error::OutputPin)?;
		self.rdx.set_high().map_err(Error::OutputPin)?;
		self.dcx.set_low().map_err(Error::OutputPin)?;
		self.wrx.set_low().map_err(Error::OutputPin)?;

		self.set_data_bus(command)?;
		self.wrx.set_high().map_err(Error::OutputPin)?;

		self.dcx.set_high().map_err(Error::OutputPin)?;
		for val in data.into_iter() {
			for b in &val.to_be_bytes() {
				self.wrx.set_low().map_err(Error::OutputPin)?;
				self.set_data_bus(*b)?;
				self.wrx.set_high().map_err(Error::OutputPin)?;
			}
		}

		self.csx.set_high().map_err(Error::OutputPin)?;
		Ok(())
	}
}
