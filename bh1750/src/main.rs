#![no_std]
#![no_main]

use bh1750::{BH1750, Resolution};
use embassy_executor::Spawner;
use embassy_nrf::peripherals;
use embassy_nrf::{bind_interrupts, twim};
use embassy_nrf_ws2812_pwm::Ws2812;
use embassy_time::{Delay, Timer};
use smart_leds::colors;
use smart_leds::{SmartLedsWriteAsync as _, brightness};
use static_cell::{ConstStaticCell, StaticCell};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    TWISPI0 => twim::InterruptHandler<peripherals::TWISPI0>;
});

const NUM_LEDS: usize = 1;
const BUFFER_SIZE: usize = NUM_LEDS * 24;
static LED_BUFFER: StaticCell<[u16; BUFFER_SIZE]> = StaticCell::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    let buf = LED_BUFFER.init([0u16; BUFFER_SIZE]);
    let mut ws: Ws2812<_> = Ws2812::new(p.PWM0, p.P0_13, buf);

    let sda = p.P0_03;
    let scl = p.P0_04;

    // Create I2C instance
    static RAM_BUFFER: ConstStaticCell<[u8; 16]> = ConstStaticCell::new([0; 16]);
    let i2c = twim::Twim::new(
        p.TWISPI0,
        Irqs,
        sda,
        scl,
        twim::Config::default(),
        RAM_BUFFER.take(),
    );

    let mut bh1750 = BH1750::new(i2c, Delay, false);
    loop {
        let lux = bh1750
            .get_one_time_measurement(Resolution::High)
            .expect("to read BH1750");
        defmt::info!("Lux = {:?}", lux);

        let data = [colors::BLUE; NUM_LEDS];
        ws.write(brightness(data.into_iter(), 20)).await.unwrap();

        Timer::after_secs(5).await
    }
}
