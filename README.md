# I2C slave emulation using GPIOs

This slave will respond to any address.

## Usage
```
i2c-slave-gpio <gpiochip> <scl> <sda>

gpiochip: char device (e.g. /dev/gpiochip0)
sda: sda gpio line (e.g. 0)
scl: scl gpio line (e.g. 1)
```
To enable logs, you need to pass the env variable `RUST_LOG=<debug,info,warn,error>`.

To use this slave, you need to enable GPIO support for your platform (since linux 5, char device is exposed by default for GPIO).
To use the I2C char device, you need to enable char device support with
```defconfig
CONFIG_I2C_CHARDEV=y
```
in your kernel compile options

## Building
```sh
cargo build
```

### YOCTO
You can produce a YOCTO receipe using `cargo bitbake`.
```sh
cargo install --locked cargo-bitbake
cargo bitbake
```
This will generate a file `i2c-slave-gpio_<ver>.bb`.

Then in your list of packages add `i2c-slave-gpio`
```bb
PACKAGE_INSTALL:append = " \
    i2c-tools \ # usefull for testing purposes (i2cset, i2cget)
    i2c-slave-gpio \
"
```
