use std::thread::sleep;
use std::time::Duration;

use rppal::gpio::{self, Gpio};
use rppal::i2c::{self, I2c};
use rppal::spi::{self, Spi};

use crate::epd;

const RESET_PIN: u8 = 27;
const BUSY_PIN: u8 = 17;
const DC_PIN: u8 = 22;
const MOSI_PIN: u8 = 10;
const SCLK_PIN: u8 = 11;
const CS0_PIN: u8 = 8;

const AC073TC1_PSR: u8 = 0x00;
const AC073TC1_PWR: u8 = 0x01;
const AC073TC1_POF: u8 = 0x02;
const AC073TC1_POFS: u8 = 0x03;
const AC073TC1_PON: u8 = 0x04;
const AC073TC1_BTST1: u8 = 0x05;
const AC073TC1_BTST2: u8 = 0x06;
const AC073TC1_DSLP: u8 = 0x07;
const AC073TC1_BTST3: u8 = 0x08;
const AC073TC1_DTM: u8 = 0x10;
const AC073TC1_DSP: u8 = 0x11;
const AC073TC1_DRF: u8 = 0x12;
const AC073TC1_IPC: u8 = 0x13;
const AC073TC1_PLL: u8 = 0x30;
const AC073TC1_TSC: u8 = 0x40;
const AC073TC1_TSE: u8 = 0x41;
const AC073TC1_TSW: u8 = 0x42;
const AC073TC1_TSR: u8 = 0x43;
const AC073TC1_CDI: u8 = 0x50;
const AC073TC1_LPD: u8 = 0x51;
const AC073TC1_TCON: u8 = 0x60;
const AC073TC1_TRES: u8 = 0x61;
const AC073TC1_DAM: u8 = 0x65;
const AC073TC1_REV: u8 = 0x70;
const AC073TC1_FLG: u8 = 0x71;
const AC073TC1_AMV: u8 = 0x80;
const AC073TC1_VV: u8 = 0x81;
const AC073TC1_VDCS: u8 = 0x82;
const AC073TC1_T_VDCS: u8 = 0x84;
const AC073TC1_AGID: u8 = 0x86;
const AC073TC1_CMDH: u8 = 0xAA;
const AC073TC1_CCSET: u8 = 0xE0;
const AC073TC1_PWS: u8 = 0xE3;
const AC073TC1_TSSET: u8 = 0xE6;

pub struct Inky {
    spi: Spi,
    i2c: I2c,
    gpio: Gpio,

    pub eeprom: epd::EPDType,

    pub cs_pin: gpio::OutputPin,
    pub dc_pin: gpio::OutputPin,
    pub reset_pin: gpio::OutputPin,
    pub busy_pin: gpio::InputPin,

    pub h_flip: bool,
    pub v_flip: bool,
}

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
    fn initialize_inky(h_flip: bool, v_flip: bool) -> Result<Inky, InkyError> {
        let mut i2c = I2c::new()?;
        let eeprom = epd::read_eeprom(&mut i2c)?;

        let gpio = Gpio::new()?;
        let cs_pin = gpio.get(CS0_PIN)?.into_output_high();
        let dc_pin = gpio.get(DC_PIN)?.into_output_low();
        let reset_pin = gpio.get(RESET_PIN)?.into_output_high();
        let mut busy_pin = gpio.get(BUSY_PIN)?.into_input();
        busy_pin.set_interrupt(gpio::Trigger::RisingEdge, Some(Duration::from_millis(10)))?;

        let cs_channel = match CS0_PIN {
            0 => spi::SlaveSelect::Ss8,
            1 => spi::SlaveSelect::Ss7,
            _ => spi::SlaveSelect::Ss0,
        };
        let spi = Spi::new(spi::Bus::Spi0, cs_channel, 5000000, spi::Mode::Mode0)?;

        Ok(Inky {
            spi,
            i2c,
            gpio,
            eeprom,
            cs_pin,
            dc_pin,
            reset_pin,
            busy_pin,
            h_flip,
            v_flip,
        })
    }

    fn setup(&mut self) -> Result<(), InkyError> {
        self.reset_pin.set_low();
        sleep(Duration::from_millis(100));
        self.reset_pin.set_high();
        sleep(Duration::from_millis(100));
        self.reset_pin.set_low();
        sleep(Duration::from_millis(100));
        self.reset_pin.set_high();

        self.busy_wait(Duration::from_millis(100))?;

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
        self.busy_pin.poll_interrupt(false, Some(timeout))?;
        Ok(())
    }

    fn spi_write(&mut self, dc: bool, values: &[u8]) -> Result<usize, InkyError> {
        self.cs_pin.set_low();
        if dc {
            self.dc_pin.set_high();
        } else {
            self.dc_pin.set_low();
        }
        let result = self.spi.write(values);
        self.cs_pin.set_high();
        Ok(result?)
    }

    fn send_data(&mut self, data: &[u8]) -> Result<(), InkyError> {
        self.spi_write(true, data)?;
        Ok(())
    }

    fn send_command(&mut self, command: u8, data: &[u8]) -> Result<(), InkyError> {
        self.spi_write(false, &[command])?;
        self.send_data(data)
    }

    pub fn new(h_flip: bool, v_flip: bool) -> Result<Inky, InkyError> {
        let mut inky = Self::initialize_inky(h_flip, v_flip)?;
        inky.setup()?;
        Ok(inky)
    }
}

struct InkyImage {}
