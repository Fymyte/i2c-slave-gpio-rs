use std::{fmt::Display, mem::size_of};

use gpio_cdev::{
    Chip, EventRequestFlags, Line, LineEventHandle, LineHandle, LineRequestFlags,
};
use thiserror::Error;

const I2C_CONSUMER: &str = "i2c-gpio-sqn";

// #[derive(Debug)]
// struct InputLineHandle {
//     handle: LineHandle,
// }

// impl InputLineHandle {
//     fn get_value(&self) -> Result<u8, anyhow::Error> {
//         Ok(self.handle.get_value()?)
//     }
// }

// #[derive(Debug)]
// struct OutputLineHandle {
//     handle: LineHandle,
// }

// impl OutputLineHandle {
//     fn set_value(&self, value: u8) -> Result<(), anyhow::Error> {
//         self.handle.set_value(value)?;
//         Ok(())
//     }
// }

// #[derive(Debug)]
// struct RisingEdgeLineEventHandle {
//     handle: LineEventHandle,
// }

// impl Iterator for RisingEdgeLineEventHandle {
//     type Item = Result<LineEvent, gpio_cdev::Error>;

//     fn next(&mut self) -> Option<Self::Item> {
//         self.handle.next()
//     }
// }

// #[derive(Debug)]
// struct FallingEdgeLineEventHandle {
//     handle: LineEventHandle,
// }

// impl Iterator for FallingEdgeLineEventHandle {
//     type Item = Result<LineEvent, gpio_cdev::Error>;

//     fn next(&mut self) -> Option<Self::Item> {
//         self.handle.next()
//     }
// }

// #[derive(Debug)]
// enum I2cLineHandleInner {
//     Input(Rc<InputLineHandle>),
//     Output(Rc<OutputLineHandle>),
//     RisingEdge(Rc<RisingEdgeLineEventHandle>),
//     FallingEdge(Rc<FallingEdgeLineEventHandle>),
// }

// impl I2cLineHandleInner {
//     fn new_input(handle: LineHandle) -> Self {
//         Self::Input(Rc::new(InputLineHandle { handle }))
//     }
//     fn new_output(handle: LineHandle) -> Self {
//         Self::Output(Rc::new(OutputLineHandle { handle }))
//     }
// }

// #[derive(Debug)]
// struct I2cLineHandle {
//     handle: I2cLineHandleInner,
// }

// impl I2cLineHandle {
//     fn input(&mut self) -> Result<Rc<InputLineHandle>, anyhow::Error> {
//         match &self.handle {
//             I2cLineHandleInner::Input(line) => Ok(line.clone()),
//             I2cLineHandleInner::Output(line) => {
//                 let line = line.handle.line();
//                 let handle: Rc<_> = InputLineHandle {
//                     handle: line.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?,
//                 }
//                 .into();
//                 self.handle = I2cLineHandleInner::Input(handle.clone());
//                 Ok(handle)
//             }
//             I2cLineHandleInner::RisingEdge(line) => {
//                 let line = line.handle.line();
//                 let handle: Rc<_> = InputLineHandle {
//                     handle: line.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?,
//                 }
//                 .into();
//                 self.handle = I2cLineHandleInner::Input(handle.clone());
//                 Ok(handle)
//             }
//             I2cLineHandleInner::FallingEdge(line) => {
//                 let line = line.handle.line();
//                 let handle: Rc<_> = InputLineHandle {
//                     handle: line.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?,
//                 }
//                 .into();
//                 self.handle = I2cLineHandleInner::Input(handle.clone());
//                 Ok(handle)
//             }
//         }
//     }

//     fn output(&mut self) -> Result<Rc<OutputLineHandle>, anyhow::Error> {
//         match &self.handle {
//             I2cLineHandleInner::Input(line) => {
//                 let line = line.handle.line();
//                 let handle: Rc<_> = OutputLineHandle {
//                     handle: line.request(LineRequestFlags::OUTPUT, 0, I2C_CONSUMER)?,
//                 }
//                 .into();
//                 self.handle = I2cLineHandleInner::Output(handle.clone());
//                 Ok(handle)
//             }
//             I2cLineHandleInner::Output(line) => Ok(line.clone()),
//             I2cLineHandleInner::RisingEdge(line) => {
//                 let line = line.handle.line();
//                 let handle: Rc<_> = OutputLineHandle {
//                     handle: line.request(LineRequestFlags::OUTPUT, 0, I2C_CONSUMER)?,
//                 }
//                 .into();
//                 self.handle = I2cLineHandleInner::Output(handle.clone());
//                 Ok(handle)
//             }
//             I2cLineHandleInner::FallingEdge(line) => {
//                 let line = line.handle.line();
//                 let handle: Rc<_> = OutputLineHandle {
//                     handle: line.request(LineRequestFlags::OUTPUT, 0, I2C_CONSUMER)?,
//                 }
//                 .into();
//                 self.handle = I2cLineHandleInner::Output(handle.clone());
//                 Ok(handle)
//             }
//         }
//     }

//     fn rising_edge(&mut self) -> Result<Rc<RisingEdgeLineEventHandle>, anyhow::Error> {
//         match &self.handle {
//             I2cLineHandleInner::Input(line) => {
//                 let line = line.handle.line();
//                 let handle: Rc<_> = RisingEdgeLineEventHandle {
//                     handle: line.events(
//                         LineRequestFlags::INPUT,
//                         EventRequestFlags::RISING_EDGE,
//                         I2C_CONSUMER,
//                     )?,
//                 }
//                 .into();
//                 self.handle = I2cLineHandleInner::RisingEdge(handle.clone());
//                 Ok(handle)
//             }
//             I2cLineHandleInner::Output(line) => {
//                 let line = line.handle.line();
//                 let handle: Rc<_> = RisingEdgeLineEventHandle {
//                     handle: line.events(
//                         LineRequestFlags::INPUT,
//                         EventRequestFlags::RISING_EDGE,
//                         I2C_CONSUMER,
//                     )?,
//                 }
//                 .into();
//                 self.handle = I2cLineHandleInner::RisingEdge(handle.clone());
//                 Ok(handle)
//             }
//             I2cLineHandleInner::RisingEdge(line) => Ok(line.clone()),
//             I2cLineHandleInner::FallingEdge(line) => {
//                 let line = line.handle.line();
//                 let handle: Rc<_> = RisingEdgeLineEventHandle {
//                     handle: line.events(
//                         LineRequestFlags::INPUT,
//                         EventRequestFlags::RISING_EDGE,
//                         I2C_CONSUMER,
//                     )?,
//                 }
//                 .into();
//                 self.handle = I2cLineHandleInner::RisingEdge(handle.clone());
//                 Ok(handle.clone())
//             }
//         }
//     }

//     fn falling_edge(&mut self) -> Result<Rc<FallingEdgeLineEventHandle>, anyhow::Error> {
//         match &self.handle {
//             I2cLineHandleInner::Input(line) => {
//                 let line = line.handle.line();
//                 let handle: Rc<_> = FallingEdgeLineEventHandle {
//                     handle: line.events(
//                         LineRequestFlags::INPUT,
//                         EventRequestFlags::FALLING_EDGE,
//                         I2C_CONSUMER,
//                     )?,
//                 }
//                 .into();
//                 self.handle = I2cLineHandleInner::FallingEdge(handle.clone());
//                 Ok(handle)
//             }
//             I2cLineHandleInner::Output(line) => {
//                 let line = line.handle.line();
//                 let handle: Rc<_> = FallingEdgeLineEventHandle {
//                     handle: line.events(
//                         LineRequestFlags::INPUT,
//                         EventRequestFlags::FALLING_EDGE,
//                         I2C_CONSUMER,
//                     )?,
//                 }
//                 .into();
//                 self.handle = I2cLineHandleInner::FallingEdge(handle.clone());
//                 Ok(handle)
//             }
//             I2cLineHandleInner::RisingEdge(line) => {
//                 let line = line.handle.line();
//                 let handle: Rc<_> = FallingEdgeLineEventHandle {
//                     handle: line.events(
//                         LineRequestFlags::INPUT,
//                         EventRequestFlags::FALLING_EDGE,
//                         I2C_CONSUMER,
//                     )?,
//                 }
//                 .into();
//                 self.handle = I2cLineHandleInner::FallingEdge(handle.clone());
//                 Ok(handle)
//             }
//             I2cLineHandleInner::FallingEdge(line) => Ok(line.clone()),
//         }
//     }
// }

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
pub enum GpioLineDirection {
    INPUT,
    OUTPUT,
}

#[derive(Debug)]
pub struct I2cGpioLine {
    line: Line,
    handle: LineHandle,
    dir: GpioLineDirection,
}

impl I2cGpioLine {
    pub fn input(&mut self) -> Result<(), anyhow::Error> {
        match self.dir {
            GpioLineDirection::INPUT => (),
            GpioLineDirection::OUTPUT => {
                self.handle = self
                    .line
                    .request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;
                self.dir = GpioLineDirection::INPUT;
            }
        }

        Ok(())
    }

    pub fn output(&mut self, value: u8) -> Result<(), anyhow::Error> {
        match self.dir {
            GpioLineDirection::INPUT => {
                self.handle = self
                    .line
                    .request(LineRequestFlags::INPUT, value, I2C_CONSUMER)?;
                self.dir = GpioLineDirection::OUTPUT;
            }
            GpioLineDirection::OUTPUT => self.handle.set_value(value)?,
        }

        Ok(())
    }

    pub fn get_value(&self) -> Result<u8, anyhow::Error> {
        Ok(self.handle.get_value()?)
    }

    pub fn set_value(&self) -> Result<(), anyhow::Error> {
        self.handle.get_value()?;
        Ok(())
    }

    pub fn rising_edge(&mut self) -> Result<LineEventHandle, anyhow::Error> {
        self.dir = GpioLineDirection::INPUT;
        let event_handle = self.line.events(
            LineRequestFlags::INPUT,
            EventRequestFlags::RISING_EDGE,
            I2C_CONSUMER,
        )?;
        Ok(event_handle)
    }

    pub fn falling_edge(&mut self) -> Result<LineEventHandle, anyhow::Error> {
        self.dir = GpioLineDirection::INPUT;
        Ok(self.line.events(
            LineRequestFlags::INPUT,
            EventRequestFlags::FALLING_EDGE,
            I2C_CONSUMER,
        )?)
    }
}

#[derive(Debug)]
pub struct I2cGpioSlave {
    // scl: I2cLineHandle,
    // sda: I2cLineHandle,
    // scl: Line,
    // sda: Line,
    // scl_handle: LineHandle,
    // sda_handle: LineHandle,
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
                dir: GpioLineDirection::INPUT,
            },
            sda: I2cGpioLine {
                line: sda_line,
                handle: sda_handle,
                dir: GpioLineDirection::INPUT,
            },
        })

        // Ok(Self {
        //     scl: scl_line,
        //     sda: sda_line,
        //     scl_handle,
        //     sda_handle,
        // })
        // Ok(Self {
        //     scl: I2cLineHandle {
        //         handle: I2cLineHandleInner::new_input(scl_handle),
        //     },
        //     sda: I2cLineHandle {
        //         handle: I2cLineHandleInner::new_input(sda_handle),
        //     },
        // })
    }

    pub fn wait_start(&mut self) -> Result<(), anyhow::Error> {
        // let scl_handle = self.scl.input()?;
        // let falling_edge_handle = self.sda.falling_edge()?;

        // while let Some(_) = falling_edge_handle.next() {
        //     return match scl_handle.get_value() {
        //         Ok(1) => Ok(()),
        //         _ => continue,
        //     };
        // };

        // Ok(())

        self.scl.input()?;

        for _event in self.sda.falling_edge()? {
            return match self.scl.get_value() {
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
