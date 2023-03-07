use std::collections::HashMap;

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
    let mut chip = Chip::new(args.chip.clone())
        .context(format!("unable to create chip from {}", args.chip))?;

    let mut i2c_slave = I2cGpioSlave::new(&mut chip, args.sda, args.scl)?;
    let mut addr = 1u8;

    log::debug!("slave: {i2c_slave:?}");

    let mut data: HashMap<_, _> = vec![(1, 1), (2, 2), (3, 3)].into_iter().collect();

    // Message loop
    loop {
        log::info!("Waiting for start condition...");
        i2c_slave.wait_start()?;
        log::debug!("Starting transaction");

        match i2c_slave.read_addr()? {
            I2CSlaveOp::Read(slave_addr) => {
                log::info!("Detected reading for address 0x{slave_addr:x?}");
                i2c_slave.ack()?;
                log::debug!("acked address");

                addr = i2c_slave.read_byte()?;
                i2c_slave.ack()?;
                log::info!("data address: 0x{addr:x?}");
                let value = i2c_slave.read_byte()?;
                log::info!("data value: 0x{value:x?} (char: {})", value as char);
                i2c_slave.ack()?;

                log::debug!("storing {value:x?} at address {addr:x?}");
                data.insert(addr, value);
                dbg!(&data);
            }

            I2CSlaveOp::Write(slave_addr) => {
                i2c_slave.ack()?;
                log::info!("Detected writting for address {slave_addr}");
                let value = *data.get(&slave_addr).unwrap_or(&0);
                // Increase address to simulate cursor move
                addr += 1;
                i2c_slave.write_byte(value)?;
                // Continue sending byte while master requests it
                while i2c_slave.read_master_ack()? == 0 {
                    let value = *data.get(&addr).unwrap_or(&0);
                    addr += 1;
                    i2c_slave.write_byte(value)?
                }
            }
        }

        // We can't support restart condition and stop as the chip does not support waiting for
        // both rising and falling edge at the same time
        if let Err(e) = i2c_slave.wait_stop() {
            log::error!("{e:?}")
        }
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
