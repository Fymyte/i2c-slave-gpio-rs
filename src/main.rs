mod i2cslave;

use anyhow::Context;
use gpio_cdev::Chip;
use quicli::prelude::CliResult;
use structopt::StructOpt;

use crate::i2cslave::{I2cGpioSlave, I2CSlaveOp, AckError};

#[derive(Debug, StructOpt)]
struct Cli {
    /// The gpiochip device (e.g. /dev/gpiochip0)
    chip: String,
    /// The offset of the GPIO line for the i2c sda line
    sda: u32,
    /// The offset of the GPIO line for the i2c scl line
    scl: u32,
}

fn do_main(args: Cli) -> Result<(), anyhow::Error> {
    let mut chip = Chip::new(args.chip)?;

    let mut i2c_slave = I2cGpioSlave::new(&mut chip, args.sda, args.scl)?;

    log::debug!("slave: {i2c_slave:?}");

    // Message loop
    loop {
        log::info!("Waiting for start condition...");
        i2c_slave.wait_start().context("wait start error")?;
        log::debug!("Starting transaction");

        match i2c_slave.read_addr().context("read address failed")? {
            I2CSlaveOp::Read(addr) => {
                log::info!("Detected reading at address {addr}");
                i2c_slave.ack().context("ack address failed")?;
                log::debug!("acked address");
                let byte = i2c_slave.read_byte().context("reading requested byte failed")?;
                log::info!("received byte: {} (str: {})", byte, byte.to_string());
                i2c_slave.ack().context("ack failed")?;
                log::debug!("acked message");
            }
            I2CSlaveOp::Write(addr) => {
                log::info!("Detected writting at address {addr}");
                i2c_slave.ack().context("ack failed")?;
                log::debug!("acked address");
                log::warn!("Writting is not implemented yet");
                i2c_slave.nack().map_err(|_| AckError::Nack).context("nack failed")?;
                log::debug!("nacked message");
            }
        }
    }
}

fn main() -> CliResult {
    env_logger::init();
    let args = Cli::from_args();
    do_main(args).or_else(|e| {
        log::error!("error: {}", e);
        Ok(())
    })
}
