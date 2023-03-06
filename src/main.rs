use anyhow::Context;
use gpio_cdev::Chip;
use i2c_slave_gpio::{I2CSlaveOp, I2cGpioSlave};
use quicli::prelude::CliResult;
use structopt::StructOpt;

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
    let mut chip = Chip::new(args.chip.clone()).context(format!("unable to create chip from {}", args.chip))?;

    let mut last_read_byte = 1_u8;

    let mut i2c_slave = I2cGpioSlave::new(&mut chip, args.sda, args.scl)?;

    log::debug!("slave: {i2c_slave:?}");

    // Message loop
    loop {
        log::info!("Waiting for start condition...");
        i2c_slave.wait_start()?;
        log::debug!("Starting transaction");

        match i2c_slave.read_addr()? {
            I2CSlaveOp::Read(addr) => {
                log::info!("Detected reading at address {addr}");
                i2c_slave.ack()?;
                log::debug!("acked address");
                last_read_byte = i2c_slave.read_byte()?;
                log::info!(
                    "received byte: {} (char: {})",
                    last_read_byte,
                    last_read_byte as char
                );
                i2c_slave.ack()?;
                log::debug!("acked message");
            }
            I2CSlaveOp::Write(addr) => {
                i2c_slave.ack()?;
                log::info!("Detected writting at address {addr}");
                i2c_slave.write_byte(last_read_byte)?;
                // Continue sending byte while master request it
                while i2c_slave.read_master_ack()? == 0 {
                    i2c_slave.write_byte(last_read_byte)?
                }
            }
        }

        i2c_slave.wait_stop()?;
    }
}

fn main() -> CliResult {
    env_logger::init();
    let args = Cli::from_args();
    do_main(args).or_else(|e| {
        log::error!("error: {:?}", e);
        Ok(())
    })
}
