//! Use WS2812 LEDs (aka Neopixel) with nRF52xx PWM and the embassy ecosystem.
//!
//! This crate is intended for usage with the `smart-leds`
//! crate it implements the `SmartLedsWriteAsync` trait.
//!
//! Based on [ws2812-nrf52833-pwm](https://github.com/BartMassey/ws2812-nrf52833-pwm).

use core::ops::DerefMut;
use embassy_nrf::{
    gpio::{self, OutputDrive},
    pwm::{self, CounterMode, SequenceConfig, SingleSequencer},
    Peri,
};
use embassy_time::{block_for, Timer};
use smart_leds::{SmartLedsWrite, SmartLedsWriteAsync, RGB8};

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

struct DmaBuffer<const N: usize>([u16; N]);

impl<const N: usize> DmaBuffer<N> {
    fn new() -> Self {
        DmaBuffer([0; N])
    }
}

impl<const N: usize> core::ops::Deref for DmaBuffer<N> {
    type Target = [u16; N];

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
#[derive(Debug)]
pub enum Error {
    /// PWM error.
    PwmError(pwm::Error),
}

impl From<pwm::Error> for Error {
    fn from(value: pwm::Error) -> Self {
        Self::PwmError(value)
    }
}

/// Driver for a chain of WS2812-family devices using
/// PWM and a single GPIO.
///
/// The `N` value must be a multiple of 24.
pub struct Ws2812<Pwm: pwm::Instance, const N: usize> {
    num_leds: usize,
    pwm: Option<pwm::SequencePwm<'static, Pwm>>,
    buf: Option<DmaBuffer<N>>,
}

impl<Pwm: pwm::Instance, const N: usize> Ws2812<Pwm, N> {
    /// Set up WS2812 chain with PWM and an output pin.
    pub fn new<P: gpio::Pin>(pwm: Peri<'static, Pwm>, pin: Peri<'static, P>) -> Self {
        assert!(N % RGB_SIZE == 0, "N must be a multiple of 24");

        let num_leds = N / RGB_SIZE;
        let mut config = pwm::Config::default();
        config.counter_mode = CounterMode::Up;
        config.max_duty = PWM_PERIOD;
        config.prescaler = pwm::Prescaler::Div1;
        config.sequence_load = pwm::SequenceLoad::Common;
        config.ch0_drive = OutputDrive::HighDrive0Standard1;
        config.ch1_drive = OutputDrive::HighDrive0Standard1;
        config.ch2_drive = OutputDrive::HighDrive0Standard1;
        config.ch3_drive = OutputDrive::HighDrive0Standard1;
        let pwm = pwm::SequencePwm::new_1ch(pwm, pin, config).expect("to create sequence PWM");
        Self {
            num_leds,
            pwm: Some(pwm),
            buf: Some(DmaBuffer::<N>::new()),
        }
    }

    /// Number of microseconds to wait for a sequence duty cycle to run once.
    fn delay_micros(&self) -> u64 {
        // Each LED requires 24 bits (8 bits each for G, R, B)
        let num_bits = self.num_leds * RGB_SIZE;
        // Each bit takes FRAME_NS nanoseconds to transmit
        let active_time_ns = num_bits as u32 * FRAME_NS;
        // Convert active time to microseconds
        let active_time_us = active_time_ns / 1000;
        // Add reset time (already in microseconds)
        let total_time_us = active_time_us + RESET_TIME;
        total_time_us as u64
    }

    fn take_buffer<T, I>(&mut self, iterator: T) -> DmaBuffer<N>
    where
        T: IntoIterator<Item = I>,
        I: Into<RGB8>,
    {
        let mut buffer = self.buf.take().expect("to take DMA buffer");
        for (item, locs) in iterator.into_iter().zip(buffer.chunks_mut(RGB_SIZE)) {
            let item = item.into();
            let color = ((item.g as u32) << 16) | ((item.r as u32) << 8) | (item.b as u32);
            for (i, loc) in locs.iter_mut().enumerate() {
                let b = (color >> (24 - i - 1)) & 1;
                *loc = BITS[b as usize];
            }
        }
        buffer
    }

    fn sequence_config(&self) -> SequenceConfig {
        let mut conf = SequenceConfig::default();
        conf.refresh = 0;
        conf.end_delay = RESET_TICKS;
        conf
    }
}

impl<Pwm: pwm::Instance, const N: usize> SmartLedsWrite for Ws2812<Pwm, N> {
    type Error = Error;
    type Color = RGB8;

    /// Write all the items of an iterator to a WS2812 strip
    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        let buffer = self.take_buffer(iterator);
        let mut pwm = self.pwm.take().expect("to take sequence PWM");
        let seq = SingleSequencer::new(&mut pwm, buffer.as_ref(), self.sequence_config());
        seq.start(pwm::SingleSequenceMode::Times(1))?;

        block_for(embassy_time::Duration::from_micros(self.delay_micros()));

        drop(seq);
        self.pwm = Some(pwm);
        self.buf = Some(buffer);

        Ok(())
    }
}

impl<Pwm: pwm::Instance, const N: usize> SmartLedsWriteAsync for Ws2812<Pwm, N> {
    type Error = Error;
    type Color = RGB8;

    /// Write all the items of an iterator to a WS2812 strip
    async fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        let buffer = self.take_buffer(iterator);

        let mut pwm = self.pwm.take().expect("to take sequence PWM");
        let seq = SingleSequencer::new(&mut pwm, buffer.as_ref(), self.sequence_config());
        seq.start(pwm::SingleSequenceMode::Times(1))?;
        Timer::after_micros(self.delay_micros()).await;

        drop(seq);
        self.pwm = Some(pwm);
        self.buf = Some(buffer);

        Ok(())
    }
}
