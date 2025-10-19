//! Use WS2812 LEDs (aka Neopixel) with nRFxx PWM and the embassy ecosystem.
//!
//! This crate is intended for usage with the `smart-leds`
//! crate it implements the `SmartLedsWriteAsync` trait.
//!
//! Based on [ws2812-nrf52833-pwm](https://github.com/BartMassey/ws2812-nrf52833-pwm).

#![no_std]

use embassy_nrf::{Peri, gpio, pwm};
use embassy_time::{Timer, block_for};
use rgb::RGB8;
use smart_leds_trait::{SmartLedsWrite, SmartLedsWriteAsync};

/// WS2812 0-bit high time in ns.
const T0H_NS: u32 = 400;
/// WS2812 1-bit high time in ns.
const T1H_NS: u32 = 800;
/// WS2812 total frame time in ns.
const FRAME_NS: u32 = 1250;
/// WS2812 frame reset time in µs (minimum 250µs for some BC, plus slop).
const RESET_TIME: u32 = 270;
/// PWM clock in MHz.
const PWM_CLOCK: u32 = 16;
/// Size of the RGB color definition.
const RGB_SIZE: usize = 24;

/// Convert nanoseconds to PWM ticks, rounding.
const fn to_ticks(ns: u32) -> u32 {
    (ns * PWM_CLOCK + 500) / 1000
}

/// WS2812 frame reset time in PWM ticks.
const RESET_TICKS: u32 = to_ticks(RESET_TIME * 1000);

/// Samples for PWM array, with flip bits.
const BITS: [u16; 2] = [
    // 0-bit high time in ticks.
    to_ticks(T0H_NS) as u16 | 0x8000,
    // 1-bit high time in ticks.
    to_ticks(T1H_NS) as u16 | 0x8000,
];
/// Total PWM period in ticks.
const PWM_PERIOD: u16 = to_ticks(FRAME_NS) as u16;

/// Driver for a chain of WS2812-family devices using
/// PWM and a single GPIO.
///
/// The `N` value must be a multiple of 24.
pub struct Ws2812<const N: usize> {
    pwm: Option<pwm::SequencePwm<'static>>,
    buf: &'static mut [u16; N],
}

impl<const N: usize> Ws2812<N> {
    /// Set up WS2812 chain with PWM and an output pin.
    pub fn new<Pwm: pwm::Instance, P: gpio::Pin>(
        pwm: Peri<'static, Pwm>,
        pin: Peri<'static, P>,
        buf: &'static mut [u16; N],
    ) -> Self {
        assert!(
            N.is_multiple_of(RGB_SIZE),
            "N must be a multiple of {}",
            RGB_SIZE
        );

        let mut config = pwm::Config::default();
        config.counter_mode = pwm::CounterMode::Up;
        config.max_duty = PWM_PERIOD;
        config.prescaler = pwm::Prescaler::Div1;
        config.sequence_load = pwm::SequenceLoad::Common;
        config.ch0_drive = gpio::OutputDrive::HighDrive0Standard1;
        config.ch1_drive = gpio::OutputDrive::HighDrive0Standard1;
        config.ch2_drive = gpio::OutputDrive::HighDrive0Standard1;
        config.ch3_drive = gpio::OutputDrive::HighDrive0Standard1;
        let pwm = pwm::SequencePwm::new_1ch(pwm, pin, config).expect("to create sequence PWM");
        Self {
            pwm: Some(pwm),
            buf,
        }
    }

    /// Number of microseconds to wait for a sequence duty cycle to run once.
    #[inline(always)]
    fn delay_micros(&self) -> u64 {
        // Each bit takes FRAME_NS nanoseconds to transmit
        let active_time_ns = N as u32 * FRAME_NS;
        // Convert active time to microseconds
        let active_time_us = active_time_ns / 1000;
        // Add reset time (already in microseconds)
        let total_time_us = active_time_us + RESET_TIME;
        total_time_us as u64
    }

    #[inline(always)]
    fn write_buffer<T, I>(&mut self, iterator: T)
    where
        T: IntoIterator<Item = I>,
        I: Into<RGB8>,
    {
        for (item, locs) in iterator.into_iter().zip(self.buf.chunks_mut(RGB_SIZE)) {
            let item = item.into();
            let color = ((item.g as u32) << 16) | ((item.r as u32) << 8) | (item.b as u32);
            for (i, loc) in locs.iter_mut().enumerate() {
                let b = (color >> (24 - i - 1)) & 1;
                *loc = BITS[b as usize];
            }
        }
    }

    #[inline(always)]
    fn sequence_config(&self) -> pwm::SequenceConfig {
        let mut conf = pwm::SequenceConfig::default();
        conf.refresh = 0;
        conf.end_delay = RESET_TICKS;
        conf
    }
}

impl<const N: usize> SmartLedsWrite for Ws2812<N> {
    type Error = pwm::Error;
    type Color = RGB8;

    /// Write all the items of an iterator to a WS2812 strip
    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        self.write_buffer(iterator);
        let mut pwm = self.pwm.take().expect("to take sequence PWM");
        let seq = pwm::SingleSequencer::new(&mut pwm, &*self.buf, self.sequence_config());
        seq.start(pwm::SingleSequenceMode::Times(1))?;

        block_for(embassy_time::Duration::from_micros(self.delay_micros()));

        drop(seq);
        self.pwm = Some(pwm);

        Ok(())
    }
}

impl<const N: usize> SmartLedsWriteAsync for Ws2812<N> {
    type Error = pwm::Error;
    type Color = RGB8;

    /// Write all the items of an iterator to a WS2812 strip
    async fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        self.write_buffer(iterator);
        let mut pwm = self.pwm.take().expect("to take sequence PWM");
        let seq = pwm::SingleSequencer::new(&mut pwm, &*self.buf, self.sequence_config());
        seq.start(pwm::SingleSequenceMode::Times(1))?;
        Timer::after_micros(self.delay_micros()).await;

        drop(seq);
        self.pwm = Some(pwm);

        Ok(())
    }
}
