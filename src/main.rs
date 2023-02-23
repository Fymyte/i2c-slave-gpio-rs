use gpio_cdev::{self, Chip, EventRequestFlags, LineRequestFlags};
use quicli::prelude::*;
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

fn do_main(args: Cli) -> Result<(), gpio_cdev::Error> {
    println!("i2c-gpio-sqn");
    let mut chip = Chip::new(args.chip)?;
    let sda = chip.get_line(args.sda)?;
    let scl = chip.get_line(args.scl)?;
    println!("chip: {:?}, sda: {:?}, scl: {:?}", chip, sda, scl);

    for event in scl.events(
        LineRequestFlags::INPUT,
        EventRequestFlags::RISING_EDGE,
        "i2c-gpio-sqn",
    )? {
        println!("clk edge rising ({:?})", event);
    }

    Ok(())
}

fn main() -> CliResult {
    println!("i2c-gpio-sqn");
    let args = Cli::from_args();
    do_main(args).or_else(|e| {
        println!("error: {}", e);
        Ok(())
    })
}
