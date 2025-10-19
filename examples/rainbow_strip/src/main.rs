#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::peripherals;
use embassy_nrf::{bind_interrupts, rng};
use embassy_nrf_ws2812_pwm::Ws2812;
use embassy_time::{Duration, Timer};
use smart_leds::{
    RGB8, SmartLedsWriteAsync as _, brightness,
    hsv::{Hsv, hsv2rgb},
};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    RNG => rng::InterruptHandler<peripherals::RNG>;
    EGU0_SWI0 => nrf_sdc::mpsl::LowPrioInterruptHandler;
    CLOCK_POWER => nrf_sdc::mpsl::ClockInterruptHandler;
    RADIO => nrf_sdc::mpsl::HighPrioInterruptHandler;
    TIMER0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
    RTC0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
});

const NUM_LEDS: usize = 8;
const BUFFER_SIZE: usize = NUM_LEDS * 24;
static LED_BUFFER: StaticCell<[u16; BUFFER_SIZE]> = StaticCell::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    let buf = LED_BUFFER.init([0u16; BUFFER_SIZE]);
    let mut ws: Ws2812<_> = Ws2812::new(p.PWM0, p.P0_13, buf);

    let mut hue_offset = 0u8;
    loop {
        let mut colors = [RGB8::default(); 8];

        for (i, color) in colors.iter_mut().enumerate() {
            let hue = hue_offset.wrapping_add((i as u8) * 32);
            let hsv = Hsv {
                hue,
                sat: 255,
                val: 50, // Keep brightness reasonable
            };
            *color = hsv2rgb(hsv);
        }

        ws.write(brightness(colors.into_iter(), 64)).await.unwrap();
        hue_offset = hue_offset.wrapping_add(4);
        Timer::after(Duration::from_millis(25)).await;
    }
}
