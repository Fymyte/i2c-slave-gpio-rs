use std::{fmt::Display, mem::size_of};

use gpio_cdev::{
    Chip, EventRequestFlags, Line, LineDirection, LineEventHandle, LineHandle, LineRequestFlags,
};
use thiserror::Error;

const I2C_CONSUMER: &str = "i2c-gpio-sqn";

#[derive(Debug, Error)]
pub enum AckError {
    Ack,
    Nack,
}

impl Display for AckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} failed",
            match &self {
                Self::Ack => "ack",
                Self::Nack => "nack",
            }
        )
    }
}

#[derive(Debug)]
pub enum I2CSlaveOp {
    Read(u8),
    Write(u8),
}

#[derive(Debug)]
pub struct I2cGpioLine {
    line: Line,
    handle: LineHandle,
}

impl I2cGpioLine {
    pub fn direction(&self) -> Result<LineDirection, anyhow::Error> {
        Ok(self.line.info()?.direction())
    }

    pub fn input(&mut self) -> Result<(), anyhow::Error> {
        match self.direction()? {
            LineDirection::In => (),
            LineDirection::Out => {
                self.handle = self
                    .line
                    .request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;
            }
        }

        Ok(())
    }

    pub fn output(&mut self, value: u8) -> Result<(), anyhow::Error> {
        match self.direction()? {
            LineDirection::In => {
                self.handle = self
                    .line
                    .request(LineRequestFlags::INPUT, value, I2C_CONSUMER)?;
            }
            LineDirection::Out => self.handle.set_value(value)?,
        }

        Ok(())
    }

    pub fn get_value(&self) -> Result<u8, anyhow::Error> {
        Ok(self.handle.get_value()?)
    }

    #[allow(dead_code)]
    pub fn set_value(&self) -> Result<(), anyhow::Error> {
        self.handle.get_value()?;
        Ok(())
    }

    pub fn rising_edge(&mut self) -> Result<LineEventHandle, anyhow::Error> {
        Ok(self.line.events(
            LineRequestFlags::INPUT,
            EventRequestFlags::RISING_EDGE,
            I2C_CONSUMER,
        )?)
    }

    pub fn falling_edge(&mut self) -> Result<LineEventHandle, anyhow::Error> {
        Ok(self.line.events(
            LineRequestFlags::INPUT,
            EventRequestFlags::FALLING_EDGE,
            I2C_CONSUMER,
        )?)
    }
}

#[derive(Debug)]
pub struct I2cGpioSlave {
    scl: I2cGpioLine,
    sda: I2cGpioLine,
}

impl I2cGpioSlave {
    pub fn new(chip: &mut Chip, sda: u32, scl: u32) -> Result<Self, anyhow::Error> {
        let scl_line = chip.get_line(scl)?;
        let sda_line = chip.get_line(sda)?;
        let scl_handle = scl_line.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;
        let sda_handle = sda_line.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;

        Ok(Self {
            scl: I2cGpioLine {
                line: scl_line,
                handle: scl_handle,
            },
            sda: I2cGpioLine {
                line: sda_line,
                handle: sda_handle,
            },
        })
    }

    pub fn wait_start(&mut self) -> Result<(), anyhow::Error> {
        self.scl.input()?;

        // Wait for sda to drop to low with  scl still high
        for _event in self.sda.falling_edge()? {
            return match self.scl.get_value() {
                // Sda dropped low and scl is still high => Start condition
                Ok(1) => Ok(()),
                _ => continue,
            };
        }

        Ok(())
    }

    pub fn read_byte(&mut self) -> Result<u8, anyhow::Error> {
        let mut byte: u8 = 0;
        let byte_size = size_of::<u8>() * 8;

        self.sda.input()?;

        // Read sda on the next 8 scl rising edge
        for (nr, _event) in self.scl.rising_edge()?.take(byte_size).enumerate() {
            let value = self.sda.get_value()?;
            // We shift of (7 - nr) because we receive MSB first
            byte |= value << (byte_size - 1 - nr);
            log::info!("read bit: {value} (byte: {byte:x?})");
        }

        Ok(byte)
    }

    fn wait_up_down_cycle(&mut self) -> Result<(), anyhow::Error> {
        // Wait for the next clock edge
        self.scl.rising_edge()?.next().ok_or(AckError::Ack)??;

        // And now wait for scl to return to low
        self.scl.falling_edge()?.next().ok_or(AckError::Ack)??;

        Ok(())
    }

    pub fn read_addr(&mut self) -> Result<I2CSlaveOp, anyhow::Error> {
        Ok(match self.read_byte()? {
            write_addr if (write_addr & 1) == 1 => I2CSlaveOp::Write(write_addr >> 1),
            read_addr => I2CSlaveOp::Read(read_addr >> 1),
        })
    }
    pub fn ack(&mut self) -> Result<(), anyhow::Error> {
        // Request the sda line to low
        self.sda.output(0)?;

        self.wait_up_down_cycle()?;

        // Move sda back to open drain. Stop driving value
        self.sda.input()
    }

    pub fn nack(&mut self) -> Result<(), anyhow::Error> {
        self.wait_up_down_cycle()
    }
}
