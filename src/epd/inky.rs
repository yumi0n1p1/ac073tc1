use std::cmp::min;
use std::thread;
use std::time::Duration;

use log::{info, warn};
use ndarray::{Array1, Array2};
use rppal::gpio::{self, Gpio};
use rppal::i2c::{self, I2c};
use rppal::spi::{self, Spi};

use crate::epd;

const RESET_PIN: u8 = 27;
const BUSY_PIN: u8 = 17;
const DC_PIN: u8 = 22;
const _MOSI_PIN: u8 = 10;
const _SCLK_PIN: u8 = 11;
const CS0_PIN: u8 = 8;

const AC073TC1_PSR: u8 = 0x00;
const AC073TC1_PWR: u8 = 0x01;
const AC073TC1_POF: u8 = 0x02;
const AC073TC1_POFS: u8 = 0x03;
const AC073TC1_PON: u8 = 0x04;
const AC073TC1_BTST1: u8 = 0x05;
const AC073TC1_BTST2: u8 = 0x06;
const _AC073TC1_DSLP: u8 = 0x07;
const AC073TC1_BTST3: u8 = 0x08;
const AC073TC1_DTM: u8 = 0x10;
const _AC073TC1_DSP: u8 = 0x11;
const AC073TC1_DRF: u8 = 0x12;
const AC073TC1_IPC: u8 = 0x13;
const AC073TC1_PLL: u8 = 0x30;
const _AC073TC1_TSC: u8 = 0x40;
const AC073TC1_TSE: u8 = 0x41;
const _AC073TC1_TSW: u8 = 0x42;
const _AC073TC1_TSR: u8 = 0x43;
const AC073TC1_CDI: u8 = 0x50;
const _AC073TC1_LPD: u8 = 0x51;
const AC073TC1_TCON: u8 = 0x60;
const AC073TC1_TRES: u8 = 0x61;
const _AC073TC1_DAM: u8 = 0x65;
const _AC073TC1_REV: u8 = 0x70;
const _AC073TC1_FLG: u8 = 0x71;
const _AC073TC1_AMV: u8 = 0x80;
const _AC073TC1_VV: u8 = 0x81;
const AC073TC1_VDCS: u8 = 0x82;
const AC073TC1_T_VDCS: u8 = 0x84;
const AC073TC1_AGID: u8 = 0x86;
const AC073TC1_CMDH: u8 = 0xAA;
const AC073TC1_CCSET: u8 = 0xE0;
const AC073TC1_PWS: u8 = 0xE3;
const AC073TC1_TSSET: u8 = 0xE6;

pub struct Inky {
    spi: Spi,
    // i2c: I2c,
    // gpio: Gpio,
    pub eeprom: epd::EPDType,

    pub cs_pin: gpio::OutputPin,
    pub dc_pin: gpio::OutputPin,
    pub reset_pin: gpio::OutputPin,
    pub busy_pin: gpio::InputPin,

    buf: Array2<u8>,
}

#[derive(Debug)]
pub enum InkyError {
    SpiError(spi::Error),
    GpioError(gpio::Error),
    I2cError(i2c::Error),
}

impl From<i2c::Error> for InkyError {
    fn from(value: i2c::Error) -> Self {
        InkyError::I2cError(value)
    }
}

impl From<gpio::Error> for InkyError {
    fn from(value: gpio::Error) -> Self {
        InkyError::GpioError(value)
    }
}

impl From<spi::Error> for InkyError {
    fn from(value: spi::Error) -> Self {
        InkyError::SpiError(value)
    }
}

impl Inky {
    fn initialize_inky() -> Result<Inky, InkyError> {
        info!("Initializing I2C");
        let mut i2c = I2c::new()?;
        let eeprom = epd::read_eeprom(&mut i2c)?;
        info!("EPD Type: {eeprom:?}");

        info!("Initializing GPIO");
        let gpio = Gpio::new()?;
        info!("Chip Select @ PIN {CS0_PIN}");
        let cs_pin = gpio.get(CS0_PIN)?.into_output_high();
        info!("Data/Command @ PIN {DC_PIN}");
        let dc_pin = gpio.get(DC_PIN)?.into_output_low();
        info!("Reset @ PIN {RESET_PIN}");
        let reset_pin = gpio.get(RESET_PIN)?.into_output_high();
        info!("Busy @ PIN {BUSY_PIN}");
        let mut busy_pin = gpio.get(BUSY_PIN)?.into_input_pullup();
        busy_pin.set_interrupt(gpio::Trigger::Both, Some(Duration::from_millis(10)))?;
        info!("Busy pin initial state: {}", busy_pin.read());

        info!("Initializing SPI");
        let cs_channel = match CS0_PIN {
            0 => spi::SlaveSelect::Ss8,
            1 => spi::SlaveSelect::Ss7,
            _ => spi::SlaveSelect::Ss0,
        };
        let spi = Spi::new(spi::Bus::Spi0, cs_channel, 5000000, spi::Mode::Mode0)?;

        info!("Finished initialization");
        let width = eeprom.width as usize;
        let height = eeprom.height as usize;
        Ok(Inky {
            spi,
            // i2c,
            // gpio,
            eeprom,
            cs_pin,
            dc_pin,
            reset_pin,
            busy_pin,
            buf: Array2::zeros((height, width)),
        })
    }

    fn setup(&mut self) -> Result<(), InkyError> {
        info!("Entering setup sequence");

        self.reset_pin.set_low();
        thread::sleep(Duration::from_millis(100));
        self.reset_pin.set_high();
        thread::sleep(Duration::from_millis(100));

        self.reset_pin.set_low();
        thread::sleep(Duration::from_millis(100));
        self.reset_pin.set_high();

        self.busy_wait(Duration::from_secs(10))?;

        self.send_command(AC073TC1_CMDH, &[0x49, 0x55, 0x20, 0x08, 0x09, 0x18])?;
        self.send_command(AC073TC1_PWR, &[0x3F, 0x00, 0x32, 0x2A, 0x0E, 0x2A])?;
        self.send_command(AC073TC1_PSR, &[0x5F, 0x69])?;
        self.send_command(AC073TC1_POFS, &[0x00, 0x54, 0x00, 0x44])?;
        self.send_command(AC073TC1_BTST1, &[0x40, 0x1F, 0x1F, 0x2C])?;
        self.send_command(AC073TC1_BTST2, &[0x6F, 0x1F, 0x16, 0x25])?;
        self.send_command(AC073TC1_BTST3, &[0x6F, 0x1F, 0x1F, 0x22])?;
        self.send_command(AC073TC1_IPC, &[0x00, 0x04])?;
        self.send_command(AC073TC1_PLL, &[0x02])?;
        self.send_command(AC073TC1_TSE, &[0x00])?;
        self.send_command(AC073TC1_CDI, &[0x3F])?;
        self.send_command(AC073TC1_TCON, &[0x02, 0x00])?;
        self.send_command(AC073TC1_TRES, &[0x03, 0x20, 0x01, 0xE0])?;
        self.send_command(AC073TC1_VDCS, &[0x1E])?;
        self.send_command(AC073TC1_T_VDCS, &[0x00])?;
        self.send_command(AC073TC1_AGID, &[0x00])?;
        self.send_command(AC073TC1_PWS, &[0x2F])?;
        self.send_command(AC073TC1_CCSET, &[0x00])?;
        self.send_command(AC073TC1_TSSET, &[0x00])?;

        Ok(())
    }

    fn busy_wait(&mut self, timeout: Duration) -> Result<(), InkyError> {
        if self.busy_pin.is_high() {
            warn!("Busy Wait: Held high. Waiting for {timeout:?}");
            thread::sleep(timeout);
        }

        while self.busy_pin.is_low() {}

        return Ok(());
    }

    fn update(&mut self, buf: &[u8]) -> Result<(), InkyError> {
        self.setup()?;

        info!("Transmitting image");
        self.send_command(AC073TC1_DTM, buf)?;

        self.send_command(AC073TC1_PON, &[])?;
        self.busy_wait(Duration::from_millis(400))?;

        self.send_command(AC073TC1_DRF, &[0x00])?;
        self.busy_wait(Duration::from_secs(45))?;

        self.send_command(AC073TC1_POF, &[0x00])?;
        self.busy_wait(Duration::from_millis(400))?;

        info!("Update complete");
        return Ok(());
    }

    fn spi_write(&mut self, dc: bool, values: &[u8]) -> Result<(), InkyError> {
        self.cs_pin.set_low();
        if dc {
            self.dc_pin.set_high();
        } else {
            self.dc_pin.set_low();
        }

        let mut written = 0;

        while written != values.len() {
            written += self
                .spi
                .write(&values[written..min(written + 64, values.len())])?;
        }
        self.cs_pin.set_high();
        Ok(())
    }

    fn send_data(&mut self, data: &[u8]) -> Result<(), InkyError> {
        self.spi_write(true, data)?;
        Ok(())
    }

    fn send_command(&mut self, command: u8, data: &[u8]) -> Result<(), InkyError> {
        self.spi_write(false, &[command])?;
        self.send_data(data)
    }

    pub fn new() -> Result<Inky, InkyError> {
        let mut inky = Self::initialize_inky()?;
        inky.setup()?;
        Ok(inky)
    }

    pub fn show(&mut self) -> Result<(), InkyError> {
        let mut internal_buf: Array1<u8> =
            Array1::zeros(self.eeprom.width as usize * self.eeprom.height as usize / 2);
        for (ix, px) in self.buf.iter().enumerate() {
            let actual_px = if *px == 7 { 1 } else { *px } & 0xF;
            if ix % 2 == 0 {
                internal_buf[ix / 2] |= actual_px << 4;
            } else {
                internal_buf[ix / 2] |= actual_px;
            }
        }
        self.update(internal_buf.as_slice().unwrap())?;

        return Ok(());
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, v: u8) {
        self.buf[[y, x]] = v;
    }
}
