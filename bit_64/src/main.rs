#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf_ws2812_pwm::Ws2812;
use embassy_time::Timer;
use smart_leds::colors;
use smart_leds::{SmartLedsWriteAsync as _, brightness};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

const NUM_LEDS: usize = 64;
const BUFFER_SIZE: usize = NUM_LEDS * 24;
static LED_BUFFER: StaticCell<[u16; BUFFER_SIZE]> = StaticCell::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    let buf = LED_BUFFER.init([0u16; BUFFER_SIZE]);
    let mut ws: Ws2812<_> = Ws2812::new(p.PWM0, p.P0_13, buf);

    loop {
        let data = [colors::BLUE; NUM_LEDS];
        ws.write(brightness(data.into_iter(), 20)).await.unwrap();

        Timer::after_secs(5).await
    }
}
