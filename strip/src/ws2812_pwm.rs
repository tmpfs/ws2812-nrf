/*! # Use ws2812 LEDs with nRF52840 PWM.

Based on https://github.com/BartMassey/ws2812-nrf52833-pwm

This crate is intended for usage with the `smart-leds`
crate: it implements the `SmartLedsWriteAsync` trait.
*/

use core::ops::DerefMut;
use embassy_nrf::{
    gpio::OutputDrive,
    peripherals::{P0_13, PWM0},
    pwm::{self, CounterMode, SequenceConfig, SingleSequencer},
    Peri,
};
use embassy_time::Timer;
use smart_leds::{SmartLedsWriteAsync, RGB8};

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

type Seq<const N: usize> = [u16; N];

struct DmaBuffer<const N: usize>(Seq<N>);

impl<const N: usize> Default for DmaBuffer<N> {
    fn default() -> Self {
        DmaBuffer([0; N])
    }
}

impl<const N: usize> core::ops::Deref for DmaBuffer<N> {
    type Target = Seq<N>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for DmaBuffer<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Error during WS2812 driver operation.
pub enum Error {
    /// PWM error.
    PwmError(pwm::Error),
}

impl core::fmt::Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::PwmError(err) => write!(f, "pwm error: {:?}", err),
        }
    }
}

/// Driver for a chain of WS2812-family devices using
/// PWM. The constant `N` should be 24 times the number of
/// chips in the chain.
pub struct Ws2812<const N: usize> {
    pwm: Option<pwm::SequencePwm<'static, PWM0>>,
    buf: Option<DmaBuffer<N>>,
}

impl<const N: usize> Ws2812<N> {
    /// Set up WS2812 chain with PWM and an output pin.
    pub fn new(pwm: Peri<'static, PWM0>, pin: Peri<'static, P0_13>) -> Self {
        // defmt::info!("PWM_PERIOD: {}", PWM_PERIOD);
        // defmt::info!("RESET_TICKS: {}", RESET_TICKS);
        // defmt::info!("RESET_TIME: {}", RESET_TIME);

        let mut config = pwm::Config::default();
        config.counter_mode = CounterMode::Up;
        config.max_duty = PWM_PERIOD;
        config.prescaler = pwm::Prescaler::Div1;
        config.sequence_load = pwm::SequenceLoad::Common;
        config.ch0_drive = OutputDrive::HighDrive0Standard1;
        config.ch1_drive = OutputDrive::HighDrive0Standard1;
        config.ch2_drive = OutputDrive::HighDrive0Standard1;
        config.ch3_drive = OutputDrive::HighDrive0Standard1;
        let pwm = pwm::SequencePwm::new_1ch(pwm, pin, config).expect("to create pwm");

        Self {
            pwm: Some(pwm),
            buf: Some(DmaBuffer::default()),
        }
    }

    /// Number of microseconds to wait for a sequence duty cycle
    /// to run once.
    fn delay_micros(&self) -> u64 {
        let num_leds: usize = 8;
        // Each LED requires 24 bits (8 bits each for G, R, B)
        let num_bits = num_leds * 24;
        // Each bit takes FRAME_NS nanoseconds to transmit
        let active_time_ns = num_bits as u32 * FRAME_NS;
        // Convert active time to microseconds
        let active_time_us = active_time_ns / 1000;
        // Add reset time (already in microseconds)
        let total_time_us = active_time_us + RESET_TIME;
        total_time_us as u64
    }
}

impl<const N: usize> SmartLedsWriteAsync for Ws2812<N> {
    type Error = Error;
    type Color = RGB8;

    /// Write all the items of an iterator to a ws2812 strip
    async fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        let mut buffer = self.buf.take().unwrap();
        for (item, locs) in iterator.into_iter().zip(buffer.chunks_mut(24)) {
            let item = item.into();
            let color = ((item.g as u32) << 16) | ((item.r as u32) << 8) | (item.b as u32);
            for (i, loc) in locs.iter_mut().enumerate() {
                let b = (color >> (24 - i - 1)) & 1;
                *loc = BITS[b as usize];
            }
        }

        let mut pwm = self.pwm.take().unwrap();
        let mut conf = SequenceConfig::default();
        conf.refresh = 0;
        conf.end_delay = RESET_TICKS;
        let seq = SingleSequencer::new(&mut pwm, buffer.as_ref(), conf);

        seq.start(pwm::SingleSequenceMode::Times(1)).unwrap();
        Timer::after_micros(self.delay_micros()).await;

        drop(seq);

        self.pwm = Some(pwm);
        self.buf = Some(buffer);

        Ok(())
    }
}
