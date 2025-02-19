use std::mem::transmute;

use rppal::i2c::{self, I2c};

#[repr(u8)]
pub enum EPDColor {
    Black = 0x01,
    Red = 0x02,
    Yellow = 0x03,
    SevenColour = 0x05,
}

#[repr(C)]
pub struct EPDType {
    width: u16,
    height: u16,
    color: EPDColor,
    pcb_variant: u8,
    display_variant: u8,
    eeprom_write_time_length: u8,
    eeprom_write_time: [u8; 21],
}

const EEP_ADDRESS: u16 = 0x50;

pub fn read_eeprom(i2c: &mut I2c) -> Result<EPDType, i2c::Error> {
    i2c.set_slave_address(EEP_ADDRESS)?;
    i2c.block_write(0x00, &[0x00])?;

    let mut buffer: [u8; 30] = [0; 30];
    i2c.block_read(0x00, &mut buffer[..29])?;

    let epd_type: EPDType = unsafe { transmute(buffer) };
    return Ok(epd_type);
}
