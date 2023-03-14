# I2C slave emulation using GPIOs

```
Usage:
i2c-slave-gpio <gpiochip> <scl> <sda>

gpiochip: char device (e.g. /dev/gpiochip0)
sda: sda gpio line (e.g. 0)
scl: scl gpio line (e.g. 1)
```

## Building
```sh
cargo build
```

You can produce a YOCTO receipe using `cargo bitbake`.
```sh
cargo install --locked cargo-bitbake
cargo bitbake
```

This will generate a file `i2c-slave-gpio_<ver>.bb`
