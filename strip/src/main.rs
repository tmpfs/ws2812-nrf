#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::peripherals::RNG;
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
    RNG => rng::InterruptHandler<RNG>;
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

    // let data = [
    //     RGB8::new(10, 0, 0), // Red
    //     RGB8::new(0, 10, 0), // Green
    //     RGB8::new(0, 0, 10), // Blue
    //     RGB8::new(0, 0, 10), // Blue
    //     RGB8::new(0, 0, 10), // Blue
    //     RGB8::new(0, 0, 10), // Blue
    //     RGB8::new(0, 0, 10), // Blue
    //     RGB8::new(0, 0, 10), // Blue
    // ];

    // let mut data = [RGB8::default(); 8];
    // data[0] = colors::RED;
    // data[1] = colors::GREEN;
    // data[2] = colors::BLUE;
    // data[3] = colors::WHITE;
    // data[4] = colors::YELLOW;
    // data[5] = colors::CYAN;
    // data[6] = colors::MAGENTA;
    // data[7] = RGB8 { r: 10, g: 0, b: 5 };

    // ws.write(data.iter().cloned()).await.unwrap();

    /*
    let level = 10;
    led.write(brightness([RED, GREEN, BLUE].into_iter(), level))
        .unwrap();
    loop {}
    */

    let mut hue_offset = 0u8;

    loop {
        // Create rainbow effect
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

        // Write colors with brightness control
        ws.write(brightness(colors.into_iter(), 64)).await.unwrap();

        // Advance the rainbow
        hue_offset = hue_offset.wrapping_add(4);

        // Wait before next frame
        Timer::after(Duration::from_millis(25)).await;
    }

    // loop {}
}
