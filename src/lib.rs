use std::{fmt::Display, mem::size_of};

use anyhow::{anyhow, Context};
use gpio_cdev::{
    Chip, EventRequestFlags, EventType, Line, LineDirection, LineEventHandle, LineHandle,
    LineRequestFlags,
};

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
    fn request_error(line_info: LineErrorInfo, additional_info: Option<String>) -> Self {
        I2cGpioErrorKind::LineRequestError(
            line_info,
            additional_info.or(Some("".to_owned())).unwrap(),
        )
        .into()
    }

    fn info_error(line_info: LineErrorInfo) -> Self {
        I2cGpioErrorKind::LineInfoError(line_info).into()
    }

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
enum HandleVariant {
    Normal(LineHandle, LineDirection),
    Events(LineEventHandle, EventType),
}

#[derive(Debug)]
pub struct GpioLine {
    name: String,
    line: Line,
    handle: HandleVariant,
}

impl GpioLine {
    fn new(line: Line, name: String) -> Result<Self, gpio_cdev::Error> {
        let handle = line.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;
        Ok(Self {
            name,
            line,
            handle: HandleVariant::Normal(handle, LineDirection::In),
        })
    }

    // pub fn direction(&self) -> Option<LineDirection> {
    //     self.dir
    // }

    pub fn input(&mut self) -> Result<(), anyhow::Error> {
        match &mut self.handle {
            HandleVariant::Normal(_, LineDirection::In) => {
                log::debug!("line {} already in input", self.name)
            }
            handle @ _ => {
                log::debug!("switching line {} to input", self.name);
                *handle = HandleVariant::Normal(
                    self.line
                        .request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)
                        .with_context(|| {
                            I2cGpioError::request_error(
                                (self.name.as_str(), self.line.offset()).into(),
                                Some(" (requested output)".to_owned()),
                            )
                        })?,
                    LineDirection::In,
                );
            }
        }

        Ok(())
    }

    pub fn output(&mut self, value: u8) -> Result<(), anyhow::Error> {
        match &mut self.handle {
            HandleVariant::Normal(inner, LineDirection::Out) => {
                log::debug!(
                    "driving line {} {}",
                    self.name,
                    if value > 0 { "high" } else { "low" }
                );
                inner.set_value(value)?
            }
            handle @ _ => {
                log::debug!(
                    "start driving line {} {}",
                    self.name,
                    if value > 0 { "high" } else { "low" }
                );
                *handle = HandleVariant::Normal(
                    self.line
                        .request(LineRequestFlags::OUTPUT, value, I2C_CONSUMER)
                        .with_context(|| {
                            I2cGpioError::request_error(
                                (self.name.as_str(), self.line.offset()).into(),
                                Some(" (requested output)".to_owned()),
                            )
                        })?,
                    LineDirection::Out,
                );
            }
        }

        Ok(())
    }

    pub fn get_value(&self) -> Result<u8, anyhow::Error> {
        Ok(match &self.handle {
            HandleVariant::Normal(inner, _) => inner.get_value()?,
            HandleVariant::Events(inner, _) => inner.get_value()?,
        })
    }

    pub fn set_value(&self, value: u8) -> Result<(), anyhow::Error> {
        match &self.handle {
            HandleVariant::Normal(inner, LineDirection::Out) => {
                log::debug!(
                    "driving line {} {}",
                    self.name,
                    if value > 0 { "high" } else { "low" }
                );
                inner.set_value(value)?;
                Ok(())
            }
            _ => Err(I2cGpioError::request_error(
                (self.name.as_str(), self.line.offset()).into(),
                None,
            ))?,
        }
    }

    pub fn rising_edge(&mut self) -> Result<&mut LineEventHandle, anyhow::Error> {
        Ok(match &mut self.handle {
            HandleVariant::Events(inner, EventType::RisingEdge) => {
                log::debug!("line {} already in rising edge", self.name);
                inner
            }
            handle @ _ => {
                log::debug!("watching rising edge events on line {}", self.name);
                *handle = HandleVariant::Events(
                    self.line
                        .events(
                            LineRequestFlags::INPUT,
                            EventRequestFlags::RISING_EDGE,
                            I2C_CONSUMER,
                        )
                        .with_context(|| {
                            I2cGpioError::request_error(
                                (self.name.as_str(), self.line.offset()).into(),
                                Some(" (requested rising edge)".to_owned()),
                            )
                        })?,
                    EventType::RisingEdge,
                );
                if let HandleVariant::Events(inner, EventType::RisingEdge) = handle {
                    inner
                } else {
                    panic!("")
                }
            }
        })
    }

    pub fn falling_edge(&mut self) -> Result<&mut LineEventHandle, anyhow::Error> {
        Ok(match &mut self.handle {
            HandleVariant::Events(inner, EventType::FallingEdge) => {
                log::debug!("line {} already in falling edge", self.name);
                inner
            }
            handle @ _ => {
                log::debug!("watching falling edge events on line {}", self.name);
                *handle = HandleVariant::Events(
                    self.line
                        .events(
                            LineRequestFlags::INPUT,
                            EventRequestFlags::FALLING_EDGE,
                            I2C_CONSUMER,
                        )
                        .with_context(|| {
                            I2cGpioError::request_error(
                                (self.name.as_str(), self.line.offset()).into(),
                                Some(" (requested rising edge)".to_owned()),
                            )
                        })?,
                    EventType::FallingEdge,
                );
                if let HandleVariant::Events(inner, EventType::FallingEdge) = handle {
                    inner
                } else {
                    panic!("")
                }
            }
        })
    }
}

#[derive(Debug)]
pub struct I2cGpioSlave {
    scl: GpioLine,
    sda: GpioLine,
    // buffer: Vec<u8>,
}

impl I2cGpioSlave {
    pub fn new(chip: &mut Chip, sda: u32, scl: u32) -> Result<Self, anyhow::Error> {
        let scl_line = chip.get_line(scl)?;
        let sda_line = chip.get_line(sda)?;
        Ok(Self {
            scl: GpioLine::new(scl_line, String::from("scl"))?,
            sda: GpioLine::new(sda_line, String::from("sda"))?,
            // buffer: String::from("Hello, World").into(),
        })
    }

    pub fn wait_start(&mut self) -> Result<(), anyhow::Error> {
        log::debug!("before scl input");
        self.scl
            .input()
            .with_context(|| I2cGpioError::wait_start_error())?;
        log::debug!("after scl input");

        // Wait for sda to drop to low with scl still high
        for _event in self
            .sda
            .falling_edge()
            .with_context(|| I2cGpioError::wait_start_error())?
        {
            log::debug!("in loop for falling edge");
            return match self.scl.get_value() {
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
        self.sda.output(line_value)?;

        for (nr, _event) in self
            .scl
            .rising_edge()
            .with_context(|| I2cGpioError::write_byte_error(byte))?
            // Only seven, we already sent the first bit earlier
            .take(7)
            .enumerate()
        {
            match (line_value, (byte >> (6 - nr)) & 1) {
                // Don't call set_value if we continue to drive the same
                (before, now) if before == now => (),
                (_, now) => {
                    line_value = now;
                    self.sda.set_value(now)?
                }
            };
        }

        // Release sda. Stop driving value
        self.sda.input()?;

        Ok(())
    }

    pub fn read_byte(&mut self) -> Result<u8, anyhow::Error> {
        let mut byte: u8 = 0;
        let byte_size = size_of::<u8>() * 8;

        self.sda.input()?;

        // Read sda on the next 8 scl rising edge
        for (nr, _event) in self
            .scl
            .rising_edge()
            .with_context(|| I2cGpioError::read_byte_error())?
            .take(byte_size)
            .enumerate()
        {
            let value = self.sda.get_value()?;
            // We shift of (7 - nr) because we receive MSB first
            byte |= value << (byte_size - 1 - nr);
            log::info!("read bit: {value} (byte: {byte:x?})");
        }

        Ok(byte)
    }

    fn wait_up_down_cycle(&mut self) -> Result<(), anyhow::Error> {
        // Wait for the next clock edge
        self.scl
            .rising_edge()?
            .next()
            .ok_or(I2cGpioError::wait_next_edge_error(String::from("raising")))??;

        // And now wait for scl to return to low
        self.scl
            .falling_edge()?
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
        self.sda.output(0).with_context(|| {
            I2cGpioError::ack_error(String::from("failed to switch sda to output"))
        })?;

        self.wait_up_down_cycle()
            .with_context(|| I2cGpioError::ack_error(String::from("wait up down cycle failed")))?;

        // Move sda back to open drain. Stop driving value
        self.sda.input().with_context(|| {
            I2cGpioError::ack_error(String::from("failed to switch sda back to input"))
        })
    }

    pub fn nack(&mut self) -> Result<(), anyhow::Error> {
        self.wait_up_down_cycle()
            .with_context(|| I2cGpioError::nack_error(String::from("wait up down cycle failed")))
    }

    pub fn read_master_ack(&mut self) -> Result<u8, anyhow::Error> {
        self.sda.input()?;

        self.scl
            .rising_edge()?
            .next()
            .ok_or(I2cGpioError::wait_next_edge_error(String::from("raising")))??;

        self.sda.get_value()
    }

    pub fn wait_stop(&mut self) -> Result<(), anyhow::Error> {
        self.scl.input()?;

        self.sda
            .rising_edge()?
            .next()
            .ok_or(I2cGpioError::wait_next_edge_error(String::from("falling")))
            .with_context(|| I2cGpioError::wait_stop_error())??;

        match self.scl.get_value()? {
            1 => Ok(()),
            _ => Err(anyhow!(
                "scl was not low when sda droped low and waiting for stop condition"
            ))
            .with_context(|| I2cGpioError::wait_stop_error()),
        }
    }
}
