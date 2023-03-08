use std::{fmt::Display, mem::size_of};

use anyhow::{anyhow, Context};
use gpio_cdev::{Line, Chip, LineRequestFlags, EventRequestFlags};

const I2C_CONSUMER: &str = "i2c-gpio-sqn";

#[derive(Debug)]
pub struct LineErrorInfo {
    name: String,
    offset: u32,
}

impl LineErrorInfo {
    fn new(name: &str, offset: u32) -> Self {
        Self {
            name: String::from(name),
            offset,
        }
    }
}

impl From<(&str, u32)> for LineErrorInfo {
    fn from(value: (&str, u32)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl Display for LineErrorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.offset)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct I2cGpioError(#[from] I2cGpioErrorKind);

impl I2cGpioError {

    fn wait_start_error() -> Self {
        I2cGpioErrorKind::WaitStartError.into()
    }

    fn wait_stop_error() -> Self {
        I2cGpioErrorKind::WaitStopError.into()
    }

    fn wait_next_edge_error(edge: String) -> Self {
        I2cGpioErrorKind::WaitNextEdge(edge).into()
    }

    fn read_byte_error() -> Self {
        I2cGpioErrorKind::ReadByteError.into()
    }

    fn write_byte_error(byte: u8) -> Self {
        I2cGpioErrorKind::WriteByteError(byte).into()
    }

    fn read_addr_error() -> Self {
        I2cGpioErrorKind::ReadAddrError.into()
    }

    fn ack_error(reason: String) -> Self {
        I2cGpioErrorKind::AckError(reason).into()
    }

    fn nack_error(reason: String) -> Self {
        I2cGpioErrorKind::NackError(reason).into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum I2cGpioErrorKind {
    #[error("request error for line {0}{1}")]
    LineRequestError(LineErrorInfo, String),
    #[error("failed to retrieve info for line {0}")]
    LineInfoError(LineErrorInfo),
    #[error("failed to wait for i2c start event")]
    WaitStartError,
    #[error("failed to wait for i2c stop event")]
    WaitStopError,
    #[error("failed to wait for next {0} edge")]
    WaitNextEdge(String),
    #[error("failed to read byte from master")]
    ReadByteError,
    #[error("failed to send byte {0} from master")]
    WriteByteError(u8),
    #[error("failed to read address from master")]
    ReadAddrError,
    #[error("failed to ack: {0}")]
    AckError(String),
    #[error("failed to nack: {0}")]
    NackError(String),
}

#[derive(Debug)]
pub enum I2CSlaveOp {
    Read(u8),
    Write(u8),
}

#[derive(Debug)]
pub struct I2cGpioSlave {
    scl: Line,
    sda: Line,
}

impl I2cGpioSlave {
    pub fn new(chip: &mut Chip, sda: u32, scl: u32) -> Result<Self, anyhow::Error> {
        let scl_line = chip.get_line(scl)?;
        let sda_line = chip.get_line(sda)?;
        Ok(Self {
            scl: scl_line,
            sda: sda_line,
        })
    }

    pub fn wait_start(&mut self) -> Result<(), anyhow::Error> {
        log::debug!("before scl input");
        let scl_handle = self
            .scl
            .request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)
            .with_context(|| I2cGpioError::wait_start_error())?;
        log::debug!("after scl input");

        // Wait for sda to drop to low with scl still high
        for _event in self
            .sda
            .events(
                LineRequestFlags::INPUT,
                EventRequestFlags::FALLING_EDGE,
                I2C_CONSUMER,
            )
            .with_context(|| I2cGpioError::wait_start_error())?
        {
            log::debug!("in loop for falling edge");
            return match scl_handle.get_value() {
                // Sda dropped low and scl is still high => Start condition
                Ok(1) => Ok(()),
                _ => continue,
            };
        }

        Ok(())
    }

    pub fn write_byte(&mut self, byte: u8) -> Result<(), anyhow::Error> {
        // Send MSB first
        let mut line_value = (byte >> 7) & 1;
        let sda_handle = self
            .sda
            .request(LineRequestFlags::OUTPUT, line_value, I2C_CONSUMER)?;

        for (nr, _event) in self
            .scl
            .events(
                LineRequestFlags::INPUT,
                EventRequestFlags::RISING_EDGE,
                I2C_CONSUMER,
            )
            .with_context(|| I2cGpioError::write_byte_error(byte))?
            // Only seven, we already sent the first bit earlier
            .take(7)
            .enumerate()
        {
            let value_to_drive = (byte >> (6 - nr)) & 1;
            log::info!("write bit: {value_to_drive} (nr {nr} for byte: 0x{byte:x?})");
            match (line_value, value_to_drive) {
                // Don't call set_value if we continue to drive the same
                (line, to_drive) if line == to_drive => (),
                (_, to_drive) => {
                    line_value = to_drive;
                    sda_handle.set_value(to_drive)?
                }
            };
        }

        drop(sda_handle);
        // Release sda. Stop driving value
        self.sda.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;

        Ok(())
    }

    pub fn read_byte(&mut self) -> Result<u8, anyhow::Error> {
        let mut byte: u8 = 0;
        let byte_size = size_of::<u8>() * 8;

        let sda_handle = self.sda.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;

        // Read sda on the next 8 scl rising edge
        for (nr, _event) in self
            .scl
            .events(
                LineRequestFlags::INPUT,
                EventRequestFlags::RISING_EDGE,
                I2C_CONSUMER,
            )
            .with_context(|| I2cGpioError::read_byte_error())?
            .take(byte_size)
            .enumerate()
        {
            let value = sda_handle.get_value()?;
            // We shift of (7 - nr) because we receive MSB first
            byte |= value << (byte_size - 1 - nr);
            log::info!("read bit: {value} (byte: 0x{byte:x?})");
        }

        Ok(byte)
    }

    fn wait_up_down_cycle(&mut self) -> Result<(), anyhow::Error> {
        // Wait for the next clock edge
        self.scl
            .events(
                LineRequestFlags::INPUT,
                EventRequestFlags::RISING_EDGE,
                I2C_CONSUMER,
            )?
            .next()
            .ok_or(I2cGpioError::wait_next_edge_error(String::from("raising")))??;

        // And now wait for scl to return to low
        self.scl
            .events(
                LineRequestFlags::INPUT,
                EventRequestFlags::FALLING_EDGE,
                I2C_CONSUMER,
            )?
            .next()
            .ok_or(I2cGpioError::wait_next_edge_error(String::from("falling")))??;

        Ok(())
    }

    pub fn read_addr(&mut self) -> Result<I2CSlaveOp, anyhow::Error> {
        Ok(
            match self
                .read_byte()
                .with_context(|| I2cGpioError::read_addr_error())?
            {
                write_addr if (write_addr & 1) == 1 => I2CSlaveOp::Write(write_addr >> 1),
                read_addr => I2CSlaveOp::Read(read_addr >> 1),
            },
        )
    }

    pub fn ack(&mut self) -> Result<(), anyhow::Error> {
        // Request the sda line to low
        let sda_handle = self
            .sda
            .request(LineRequestFlags::OUTPUT, 0, I2C_CONSUMER)
            .with_context(|| {
                I2cGpioError::ack_error(String::from("failed to switch sda to output"))
            })?;

        self.wait_up_down_cycle()
            .with_context(|| I2cGpioError::ack_error(String::from("wait up down cycle failed")))?;

        drop(sda_handle);
        // Move sda back to open drain. Stop driving value
        self.sda
            .request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)
            .with_context(|| {
                I2cGpioError::ack_error(String::from("failed to switch sda back to input"))
            })?;

        Ok(())
    }

    pub fn nack(&mut self) -> Result<(), anyhow::Error> {
        self.wait_up_down_cycle()
            .with_context(|| I2cGpioError::nack_error(String::from("wait up down cycle failed")))
    }

    pub fn read_master_ack(&mut self) -> Result<u8, anyhow::Error> {
        let sda_handle = self.sda.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;

        self.scl
            .events(
                LineRequestFlags::INPUT,
                EventRequestFlags::RISING_EDGE,
                I2C_CONSUMER,
            )?
            .next()
            .ok_or(I2cGpioError::wait_next_edge_error(String::from("raising")))??;

        Ok(sda_handle.get_value()?)
    }

    pub fn wait_stop(&mut self) -> Result<(), anyhow::Error> {
        let scl_handle = self.scl.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;

        self.sda
            .events(
                LineRequestFlags::INPUT,
                EventRequestFlags::RISING_EDGE,
                I2C_CONSUMER,
            )?
            .next()
            .ok_or(I2cGpioError::wait_next_edge_error(String::from("falling")))
            .with_context(|| I2cGpioError::wait_stop_error())??;

        match scl_handle.get_value()? {
            1 => Ok(()),
            _ => Err(anyhow!(
                "scl was not low when sda droped low and waiting for stop condition"
            ))
            .with_context(|| I2cGpioError::wait_stop_error()),
        }
    }
}
