#![no_std]
#![no_main]

use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::mode::Async;
use embassy_nrf::peripherals::RNG;
use embassy_nrf::{bind_interrupts, rng};
use embassy_time::Delay;
use nrf_sdc::mpsl::MultiprotocolServiceLayer;
use nrf_sdc::{self as sdc, mpsl};
use smart_leds::colors;
use static_cell::StaticCell;
// use trouble_example_apps::ble_bas_peripheral;
// use trouble_host::prelude::*;
use smart_leds::{
    brightness,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite as _, RGB8,
};
use ws2812_nrf52_strip::ws2812_timer::Ws2812;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    RNG => rng::InterruptHandler<RNG>;
    EGU0_SWI0 => nrf_sdc::mpsl::LowPrioInterruptHandler;
    CLOCK_POWER => nrf_sdc::mpsl::ClockInterruptHandler;
    RADIO => nrf_sdc::mpsl::HighPrioInterruptHandler;
    TIMER0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
    RTC0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    let mut pin = Output::new(p.P0_13, Level::Low, OutputDrive::Standard);

    let mut ws = Ws2812::new(Delay, &mut pin);

    defmt::info!("Running...");

    // // Prepare some colors
    // let colors = [
    //     RGB8::new(10, 0, 0), // Red
    //     RGB8::new(0, 10, 0), // Green
    //     RGB8::new(0, 0, 10), // Blue
    //     RGB8::new(0, 0, 10), // Blue
    //     RGB8::new(0, 0, 10), // Blue
    //     RGB8::new(0, 0, 10), // Blue
    //     RGB8::new(0, 0, 10), // Blue
    //     RGB8::new(0, 0, 10), // Blue
    // ];

    // Define your 8 LEDs
    let mut data = [RGB8::default(); 8];

    // Set them to different colors
    data[0] = colors::RED;
    data[1] = colors::GREEN;
    data[2] = colors::BLUE;
    data[3] = colors::WHITE;
    data[4] = colors::YELLOW;
    data[5] = colors::CYAN;
    data[6] = colors::MAGENTA;
    data[7] = RGB8 { r: 10, g: 0, b: 5 };

    // Send them to the LED strip
    ws.write(data.iter().cloned()).unwrap();

    /*
    let mut led = {
        let frequency = Rate::from_mhz(80);
        let rmt = Rmt::new(p.RMT, frequency).expect("Failed to initialize RMT0");
        SmartLedsAdapter::new(rmt.channel0, p.GPIO17, smart_led_buffer!(8))
    };
    */

    /*
    let level = 10;
    led.write(brightness([RED, GREEN, BLUE].into_iter(), level))
        .unwrap();
    loop {}
    */

    /*
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
        led.write(brightness(colors.into_iter(), 64)).unwrap();

        // Advance the rainbow
        hue_offset = hue_offset.wrapping_add(4);

        // Wait before next frame
        Timer::after(Duration::from_millis(50)).await;
    }
    */

    loop {}
}
