#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_stm32::{bind_interrupts, Config};
use embassy_stm32::gpio::{Input, Pull};
use embassy_stm32::i2c::{I2c, Config as I2cConfig};
use embassy_stm32::peripherals::{I2C1, DMA1_CH6, DMA1_CH7};
use embassy_stm32::rcc::Hertz;
use embassy_time::{Delay, Timer};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};

use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306, mode::BufferedGraphicsMode, size::DisplaySize128x64};
use dht_sensor::dht11::{Dht11, Reading as DhtReading};

use heapless::String;
use core::fmt::Write;

bind_interrupts!(struct Irqs {
    I2C1_EV => embassy_stm32::i2c::EventInterruptHandler<I2C1>;
    I2C1_ER => embassy_stm32::i2c::ErrorInterruptHandler<I2C1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Config::default());

    // Setup DHT11 input pin
    let mut dht_pin = Input::new(p.PA1, Pull::Up);

    // Setup I2C peripheral with DMA channels and 400kHz frequency
    let i2c = I2c::new(
        p.I2C1,
        p.PB8,
        p.PB9,
        Irqs,
        p.DMA1_CH6,
        p.DMA1_CH7,
        Hertz(400_000),
        I2cConfig::default(),
    );

    // Create SSD1306 display interface over I2C
    let interface = I2CDisplayInterface::new(i2c);
    let mut display: Ssd1306<_, _, BufferedGraphicsMode<_>> = Ssd1306::new(
        interface,
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    ).into_buffered_graphics_mode();

    display.init().unwrap();
    display.flush().unwrap();

    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

    loop {
        match Dht11::read(&mut Delay, &mut dht_pin) {
            Ok(DhtReading { temperature, relative_humidity }) => {
                let mut temp_line: String<32> = String::new();
                let mut hum_line: String<32> = String::new();

                write!(temp_line, "Temp: {}°C", temperature).unwrap();
                write!(hum_line, "Humidity: {}%", relative_humidity).unwrap();

                display.clear().unwrap();
                Text::new(&temp_line, Point::new(0, 16), style).draw(&mut display).unwrap();
                Text::new(&hum_line, Point::new(0, 32), style).draw(&mut display).unwrap();
                display.flush().unwrap();
            }
            Err(_) => {
                display.clear().unwrap();
                Text::new("DHT11 Error!", Point::new(0, 16), style).draw(&mut display).unwrap();
                display.flush().unwrap();
            }
        }

        Timer::after_millis(1000).await;
    }
}
