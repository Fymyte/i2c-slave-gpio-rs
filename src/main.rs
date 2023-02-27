use std::mem::size_of;

use gpio_cdev::*;
use quicli::prelude::{warn, CliResult};
use structopt::StructOpt;

const I2C_CONSUMER: &str = "i2c-gpio-sqn";

#[derive(Debug, StructOpt)]
struct Cli {
    /// The gpiochip device (e.g. /dev/gpiochip0)
    chip: String,
    /// The offset of the GPIO line for the i2c sda line
    sda: u32,
    /// The offset of the GPIO line for the i2c scl line
    scl: u32,
}

enum I2CSlaveOp {
    Read(u8),
    Write(u8),
}

/// read_byte reads 8 bits from i2c
/// `skip_first` allows to consider the first irq received as spurious and skip it.
/// This is needed because the controller gpio controller raise an irq when enabled if the line is
/// already high and RISING_EDGE or LEVEL_HIGH is requested, or line is low and FALLING_EDGE or
/// LEVEL_LOW is requested.
fn read_byte(scl: &Line, sda: &Line, skip_first: bool) -> Result<u8, gpio_cdev::Error> {
    let mut byte: u8 = 0;
    let byte_size = size_of::<u8>();

    let sda_handle = sda.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;
    for (nr, _event) in scl
        .events(
            LineRequestFlags::INPUT,
            EventRequestFlags::RISING_EDGE,
            I2C_CONSUMER,
        )?
        // Only take the next 8 events for 1 byte
        .take(byte_size)
        .skip(if skip_first { 1 } else { 0 })
        .enumerate()
    {
        // We shift of (7 - nr) because we receive MSB first
        byte |= sda_handle.get_value()? << (byte_size - 1 - nr);
    }

    Ok(byte)
}

// Only addresses on 7 bits are supported
fn read_addr(scl: &Line, sda: &Line) -> Result<I2CSlaveOp, gpio_cdev::Error> {
    // Don't skip the first byte here because the this should be low at this point, and reading
    // a byte is triggered on RISING_EDGE.
    Ok(match read_byte(scl, sda, false)? {
        write_addr if (write_addr & 1) == 1 => I2CSlaveOp::Write(write_addr >> 1),
        read_addr => I2CSlaveOp::Read(read_addr >> 1),
    })
}

fn wait_start(scl: &Line, sda: &Line) -> Result<(), gpio_cdev::Error> {
    let scl_handle = scl.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;

    for _event in sda
        .events(
            LineRequestFlags::INPUT,
            EventRequestFlags::FALLING_EDGE,
            I2C_CONSUMER,
        )?
        .skip(1)
    {
        return match scl_handle.get_value() {
            Ok(1) => Ok(()),
            _ => continue,
        };
    }

    Ok(())
}

fn ack(scl: &Line, sda: &Line) -> Result<(), gpio_cdev::Error> {
    // Request the sda line to low
    sda.request(LineRequestFlags::OUTPUT, 0, I2C_CONSUMER)?;

    // Wait for the next clock edge
    scl.events(
        LineRequestFlags::INPUT,
        EventRequestFlags::RISING_EDGE,
        I2C_CONSUMER,
    )?
    .skip(1)
    .next()
    .unwrap()?;

    // Move sda back to open drain. Stop driving value
    sda.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;

    Ok(())
}

fn nack(scl: &Line) -> Result<(), gpio_cdev::Error> {
    // Just wait for the next clock edge, leaving sda to high
    scl.events(
        LineRequestFlags::INPUT,
        EventRequestFlags::RISING_EDGE,
        I2C_CONSUMER,
    )?
    .skip(1)
    .next()
    .unwrap()?;

    Ok(())
}

fn do_main(args: Cli) -> Result<(), anyhow::Error> {
    println!("i2c-gpio-sqn");
    let mut chip = Chip::new(args.chip)?;
    let sda = chip.get_line(args.sda)?;
    let scl = chip.get_line(args.scl)?;
    println!("chip: {:?}, sda: {:?}, scl: {:?}", chip, sda, scl);

    // Message loop
    loop {
        println!("Waiting for start condition...");
        anyhow::Context::context(wait_start(&scl, &sda), format!("wait start failed"))?;
        println!("Starting transaction");
        match anyhow::Context::context(read_addr(&scl, &sda), format!("read address failed"))? {
            I2CSlaveOp::Read(addr) => {
                println!("Detected reading at address {addr}");
                anyhow::Context::context(ack(&scl, &sda), format!("ack address failed"))?;
                println!("acked address");
                let byte = anyhow::Context::context(
                    read_byte(&scl, &sda, true),
                    format!("reading requested byte failed"),
                )?;
                println!("received byte: {} (str: {})", byte, byte.to_string());
                anyhow::Context::context(ack(&scl, &sda), format!("ack received failed"))?;
                println!("acked acked message");
                // anyhow::Context::context(nack(&scl), format!("ack failed"))?;
                // println!("nacked message");
            }
            I2CSlaveOp::Write(addr) => {
                println!("Detected writting at address {addr}");
                anyhow::Context::context(ack(&scl, &sda), format!("ack failed"))?;
                println!("acked address");
                warn!("Writting is not implemented yet");
                anyhow::Context::context(nack(&scl), format!("ack failed"))?;
                println!("nacked message");
            }
        }
    }

    // for event in scl.events(
    //     LineRequestFlags::INPUT,
    //     EventRequestFlags::RISING_EDGE,
    //     "i2c-gpio-sqn",
    // )? {
    //     println!("clk edge rising ({:?})", event);
    // }

    // Ok(())
}

fn main() -> CliResult {
    println!("i2c-gpio-sqn");
    let args = Cli::from_args();
    do_main(args).or_else(|e| {
        println!("error: {}", e);
        Ok(())
    })
}
