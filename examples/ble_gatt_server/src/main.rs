#![no_std]
#![no_main]

use ble_gatt_server::gatt_server::NOTIFIER;
use ble_gatt_server::{gatt_server::run, led_mode::LedMode};
use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_nrf::mode::Async;
use embassy_nrf::peripherals;
use embassy_nrf::{bind_interrupts, rng};
use embassy_nrf_ws2812_pwm::Ws2812;
use embassy_time::{Duration, Timer};
use nrf_sdc::mpsl::MultiprotocolServiceLayer;
use nrf_sdc::{self as sdc, mpsl};
use smart_leds::colors;
use smart_leds::{
    RGB8, SmartLedsWriteAsync as _, brightness,
    hsv::{Hsv, hsv2rgb},
};
use static_cell::StaticCell;
use trouble_host::prelude::*;
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

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await
}

/// How many outgoing L2CAP buffers per link
const L2CAP_TXQ: u8 = 3;

/// How many incoming L2CAP buffers per link
const L2CAP_RXQ: u8 = 3;

fn build_sdc<'d, const N: usize>(
    p: nrf_sdc::Peripherals<'d>,
    rng: &'d mut rng::Rng<Async>,
    mpsl: &'d MultiprotocolServiceLayer,
    mem: &'d mut sdc::Mem<N>,
) -> Result<nrf_sdc::SoftdeviceController<'d>, nrf_sdc::Error> {
    sdc::Builder::new()?
        .support_adv()?
        .support_peripheral()?
        .peripheral_count(1)?
        .buffer_cfg(
            DefaultPacketPool::MTU as u16,
            DefaultPacketPool::MTU as u16,
            L2CAP_TXQ,
            L2CAP_RXQ,
        )?
        .build(p, rng, mpsl, mem)
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    let mpsl_p =
        mpsl::Peripherals::new(p.RTC0, p.TIMER0, p.TEMP, p.PPI_CH19, p.PPI_CH30, p.PPI_CH31);
    let lfclk_cfg = mpsl::raw::mpsl_clock_lfclk_cfg_t {
        source: mpsl::raw::MPSL_CLOCK_LF_SRC_RC as u8,
        rc_ctiv: mpsl::raw::MPSL_RECOMMENDED_RC_CTIV as u8,
        rc_temp_ctiv: mpsl::raw::MPSL_RECOMMENDED_RC_TEMP_CTIV as u8,
        accuracy_ppm: mpsl::raw::MPSL_DEFAULT_CLOCK_ACCURACY_PPM as u16,
        skip_wait_lfclk_started: mpsl::raw::MPSL_DEFAULT_SKIP_WAIT_LFCLK_STARTED != 0,
    };
    static MPSL: StaticCell<MultiprotocolServiceLayer> = StaticCell::new();
    let mpsl = MPSL.init(unwrap!(mpsl::MultiprotocolServiceLayer::new(
        mpsl_p, Irqs, lfclk_cfg
    )));
    spawner.must_spawn(mpsl_task(&*mpsl));

    let sdc_p = sdc::Peripherals::new(
        p.PPI_CH17, p.PPI_CH18, p.PPI_CH20, p.PPI_CH21, p.PPI_CH22, p.PPI_CH23, p.PPI_CH24,
        p.PPI_CH25, p.PPI_CH26, p.PPI_CH27, p.PPI_CH28, p.PPI_CH29,
    );

    let mut rng = rng::Rng::new(p.RNG, Irqs);

    let mut sdc_mem = sdc::Mem::<4720>::new();
    let sdc = unwrap!(build_sdc(sdc_p, &mut rng, mpsl, &mut sdc_mem));

    let buf = LED_BUFFER.init([0u16; BUFFER_SIZE]);
    let ws: Ws2812<_> = Ws2812::new(p.PWM0, p.P0_13, buf);
    let _ = join(run(sdc, "WLED BLE", LedMode::Off), led_manager(ws)).await;

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
        ws.write(brightness(colors.into_iter(), 64)).await.unwrap();

        // Advance the rainbow
        hue_offset = hue_offset.wrapping_add(4);

        // Wait before next frame
        Timer::after(Duration::from_millis(25)).await;
    }
    */
}

async fn led_manager(mut ws: Ws2812<BUFFER_SIZE>) -> ! {
    loop {
        let mode = NOTIFIER.wait().await;
        defmt::info!("mode: {}", mode);

        match mode {
            LedMode::Off => {
                let data = [RGB8::new(0, 0, 0); 8];
                ws.write(data.into_iter()).await.unwrap();
            }
            LedMode::Red => {
                let data = [colors::RED; 8];
                ws.write(data.into_iter()).await.unwrap();
            }
            LedMode::Green => {
                let data = [colors::GREEN; 8];
                ws.write(data.into_iter()).await.unwrap();
            }
            LedMode::Blue => {
                let data = [colors::BLUE; 8];
                ws.write(data.into_iter()).await.unwrap();
            }
        }
    }
}
