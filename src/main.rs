use std::{fmt::Display, mem::size_of};

use anyhow::Context;
use gpio_cdev::*;
use log;
use quicli::prelude::CliResult;
use structopt::StructOpt;
use thiserror::Error;

const I2C_CONSUMER: &str = "i2c-gpio-sqn";

/// # IRQ quirks
///
///                 Line
///           |         | LOW | HIGH |
///           |---------|-----|------|
/// Requiered | RISING  |  -  |  x   |
///           | FALLING |  x  |  x   |

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
    let byte_size = size_of::<u8>() * 8;

    let sda_handle = sda.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;
    for (nr, _event) in scl
        .events(
            LineRequestFlags::INPUT,
            EventRequestFlags::RISING_EDGE,
            I2C_CONSUMER,
        )?
        .skip(if skip_first { 1 } else { 0 })
        // Only take the next 8 events for 1 byte
        .take(byte_size)
        .enumerate()
    {
        let value = sda_handle.get_value()?;
        // We shift of (7 - nr) because we receive MSB first
        byte |= value << (byte_size - 1 - nr);
        log::debug!("read bit: {value} (byte: {byte:x?})");
    }

    Ok(byte)
}

// Only addresses on 7 bits are supported
fn read_addr(scl: &Line, sda: &Line) -> Result<I2CSlaveOp, gpio_cdev::Error> {
    // Don't skip the first byte here because scl should be low at this point, and reading
    // a byte is triggered on RISING_EDGE.
    Ok(match read_byte(scl, sda, false)? {
        write_addr if (write_addr & 1) == 1 => I2CSlaveOp::Write(write_addr >> 1),
        read_addr => I2CSlaveOp::Read(read_addr >> 1),
    })
}

fn wait_start(scl: &Line, sda: &Line) -> Result<(), gpio_cdev::Error> {
    let scl_handle = scl_release(scl)?;

    for _event in sda
        .events(
            LineRequestFlags::INPUT,
            EventRequestFlags::FALLING_EDGE,
            I2C_CONSUMER,
        )?
        // Skip because falling edge
        .skip(1)
    {
        return match scl_handle.get_value() {
            Ok(1) => Ok(()),
            _ => continue,
        };
    }

    Ok(())
}

#[allow(unused)]
fn scl_low(scl: &Line) -> Result<LineHandle, gpio_cdev::Error> {
    scl.request(LineRequestFlags::OUTPUT, 0, I2C_CONSUMER)
}

fn scl_release(scl: &Line) -> Result<LineHandle, gpio_cdev::Error> {
    // Move scl back to open drain. Stop driving value
    scl.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)
}

#[derive(Debug, Error)]
enum AckError {
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

fn wait_up_down_cycle(scl: &Line) -> Result<(), anyhow::Error> {
    // Wait for the next clock edge
    scl.events(
        LineRequestFlags::INPUT,
        EventRequestFlags::RISING_EDGE,
        I2C_CONSUMER,
    )?
    // Don't skip because scl should be low at this point
    .next()
    .ok_or(AckError::Ack)
    .context("failed to wait for next rising edge")?
    .context("gpio error while waiting for rising edge")?;

    // And now wait for scl to return to low
    scl.events(
        LineRequestFlags::INPUT,
        EventRequestFlags::FALLING_EDGE,
        I2C_CONSUMER,
    )?
    // Skip because falling edge
    .skip(1)
    .next()
    .context("failed to wait for next rising edge")?
    .context("gpio error while waiting for rising edge")?;

    Ok(())
}

fn ack(scl: &Line, sda: &Line) -> Result<(), anyhow::Error> {
    // Request the sda line to low
    sda.request(LineRequestFlags::OUTPUT, 0, I2C_CONSUMER)?;

    wait_up_down_cycle(scl).with_context(|| AckError::Ack)?;

    // Move sda back to open drain. Stop driving value
    sda.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;

    Ok(())
}

fn nack(scl: &Line) -> Result<(), anyhow::Error> {
    wait_up_down_cycle(scl).with_context(|| AckError::Nack)
}

fn do_main(args: Cli) -> Result<(), anyhow::Error> {
    let mut chip = Chip::new(args.chip)?;

    // Configure lines as input by default
    let sda = chip.get_line(args.sda)?;
    let scl = chip.get_line(args.scl)?;
    sda.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;
    scl.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;
    log::debug!("chip: {:?}, sda: {:?}, scl: {:?}", chip, sda, scl);

    // Message loop
    loop {
        log::info!("Waiting for start condition...");
        wait_start(&scl, &sda).context("wait start failed")?;
        log::info!("Starting transaction");

        match read_addr(&scl, &sda).context("read address failed")? {
            I2CSlaveOp::Read(addr) => {
                log::debug!("Detected reading at address {addr}");
                ack(&scl, &sda).context("ack address failed")?;
                log::debug!("acked address");
                let byte = read_byte(&scl, &sda, true).context("reading requested byte failed")?;
                log::debug!("received byte: {} (str: {})", byte, byte.to_string());
                ack(&scl, &sda).context("ack failed")?;
                log::debug!("acked message");
                // println!("nacked message");
                // nack(&scl).context(format!("ack failed"))?;
            }
            I2CSlaveOp::Write(addr) => {
                log::debug!("Detected writting at address {addr}");
                ack(&scl, &sda).context("ack failed")?;
                log::debug!("acked address");
                log::warn!("Writting is not implemented yet");
                nack(&scl).context("ack failed")?;
                log::debug!("nacked message");
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
    let args = Cli::from_args();
    do_main(args).or_else(|e| {
        log::error!("error: {}", e);
        Ok(())
    })
}
