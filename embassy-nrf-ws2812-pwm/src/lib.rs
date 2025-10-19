//! Use WS2812 LEDs (aka Neopixel) with nRFxx PWM and the embassy ecosystem.
//!
//! This crate is intended for usage with the `smart-leds`
//! crate it implements the `SmartLedsWrite` and `SmartLedsWriteAsync` traits.
//!
//! Based on [ws2812-nrf52833-pwm](https://github.com/BartMassey/ws2812-nrf52833-pwm).

#![no_std]

#[cfg(any(
    feature = "nrf51",
    feature = "nrf52805",
    feature = "nrf52810",
    feature = "nrf52811",
    feature = "nrf52820",
    feature = "nrf52832",
    feature = "nrf52833",
    feature = "nrf52840",
    feature = "nrf5340-app-s",
    feature = "nrf5340-app-ns",
    feature = "nrf5340-net",
    feature = "nrf54l15-app-s",
    feature = "nrf54l15-app-ns",
    feature = "nrf9160-s",
    feature = "nrf9160-ns",
    feature = "nrf9120-s",
    feature = "nrf9120-ns",
))]
mod ws2812;

#[cfg(any(
    feature = "nrf51",
    feature = "nrf52805",
    feature = "nrf52810",
    feature = "nrf52811",
    feature = "nrf52820",
    feature = "nrf52832",
    feature = "nrf52833",
    feature = "nrf52840",
    feature = "nrf5340-app-s",
    feature = "nrf5340-app-ns",
    feature = "nrf5340-net",
    feature = "nrf54l15-app-s",
    feature = "nrf54l15-app-ns",
    feature = "nrf9160-s",
    feature = "nrf9160-ns",
    feature = "nrf9120-s",
    feature = "nrf9120-ns",
))]
pub use ws2812::Ws2812;
