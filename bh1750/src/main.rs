#![no_std]
#![no_main]

use bh1750::{BH1750, Resolution};
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::peripherals;
use embassy_nrf::{bind_interrupts, twim};
use embassy_nrf_ws2812_pwm::Ws2812;
use embassy_time::{Delay, Timer};
use libm::{logf, roundf};
use smart_leds::colors;
use smart_leds::{SmartLedsWriteAsync as _, brightness};
use static_cell::{ConstStaticCell, StaticCell};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    TWISPI0 => twim::InterruptHandler<peripherals::TWISPI0>;
});

pub fn lux_to_u8(lux: f32, min_lux: f32, max_lux: f32, min_out: u8, max_out: u8) -> u8 {
    let clamped = lux.clamp(min_lux, max_lux);

    // Precalculate constants with +1 to avoid log(0)
    let ln_min = logf(min_lux + 1.0);
    let ln_max = logf(max_lux + 1.0);
    let ln_val = logf(clamped + 1.0);

    // Normalize to 0..1 with logarithmic curve
    let norm = (ln_val - ln_min) / (ln_max - ln_min);
    let inv = 1.0 - norm;

    // Map to output range
    let out_range = (max_out - min_out) as f32;
    let out = min_out as f32 + inv * out_range;

    roundf(out).clamp(u8::MIN as f32, u8::MAX as f32) as u8
}

const NUM_LEDS: usize = 1;
const BUFFER_SIZE: usize = NUM_LEDS * 24;
static LED_BUFFER: StaticCell<[u16; BUFFER_SIZE]> = StaticCell::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    // Green LED pin on the Makerdiary nRF52840 connect kit
    // used to indicate firmware is running
    let mut led = Output::new(p.P1_11, Level::High, OutputDrive::Standard);
    led.set_low();

    // Prepare the WS2812 LED
    let buf = LED_BUFFER.init([0u16; BUFFER_SIZE]);
    let mut ws: Ws2812<_> = Ws2812::new(p.PWM0, p.P0_13, buf);

    // Create I2C instance
    static RAM_BUFFER: ConstStaticCell<[u8; 16]> = ConstStaticCell::new([0; 16]);
    let sda = p.P1_01;
    let scl = p.P1_02;
    let i2c = twim::Twim::new(
        p.TWISPI0,
        Irqs,
        sda,
        scl,
        twim::Config::default(),
        RAM_BUFFER.take(),
    );

    let mut bh1750 = BH1750::new(i2c, Delay, false);
    bh1750
        .start_continuous_measurement(Resolution::High)
        .expect("to start measuring light sensor");
    let mut smoothed_lux: Option<f32> = None;

    loop {
        match bh1750.get_current_measurement(Resolution::High) {
            Ok(lux) => {
                defmt::debug!("Lux: {}", lux);

                let s_lux = smoothed_lux.get_or_insert(lux);

                // 0.10–0.15 recommended
                let alpha = 0.12;
                *s_lux = *s_lux + alpha * (lux - *s_lux);

                let pwm = u8::MAX - lux_to_u8(*s_lux, 5.0, 2000.0, 5, 255);

                defmt::debug!("PWM: {}", pwm);

                let data = [colors::DARK_CYAN; NUM_LEDS];

                // Update the WS2812 LED
                ws.write(brightness(data.into_iter(), pwm))
                    .await
                    .expect("to write to LED");
            }
            Err(_) => {
                defmt::warn!("error getting measurement");
            }
        }
        Timer::after_millis(15).await;
    }
}
