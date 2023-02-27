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

enum I2COp {
    Read(u8),
    Write(u8),
}

// Only addresses on 7 bits are supported
fn read_addr(scl: &Line, sda: &Line) -> Result<I2COp, gpio_cdev::Error> {
    let mut addr: u8 = 0;

    let sda_handle = sda.request(LineRequestFlags::INPUT, 0, I2C_CONSUMER)?;
    for (nr, _event) in scl
        .events(
            LineRequestFlags::INPUT,
            EventRequestFlags::RISING_EDGE,
            I2C_CONSUMER,
        )?
        // Don't skip the first irq here because clock should be low at this point, thus dont
        // trigger a new irq
        // .skip(1)
        .enumerate()
    {
        return match nr {
            0..=6 => {
                // We shift of (6 - nr) because we receive MSB first
                addr |= sda_handle.get_value()? << (6 - nr);
                continue;
            }
            7 => match sda_handle.get_value()? {
                1 => Ok(I2COp::Write(addr)),
                _ => Ok(I2COp::Read(addr)),
            },
            _ => panic!("read address overflow"),
        };
    }

    Ok(I2COp::Read(addr))
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
            I2COp::Read(addr) => {
                println!("Detected reading at address {addr}");
                anyhow::Context::context(ack(&scl, &sda), format!("ack failed"))?;
                println!("acked address");
                warn!("Reading is not implemented yet");
                anyhow::Context::context(nack(&scl), format!("ack failed"))?;
                println!("nacked message");
            }
            I2COp::Write(addr) => {
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
