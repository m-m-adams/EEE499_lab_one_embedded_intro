#![no_std]
#![no_main]

mod controllers;
mod led_states;
mod pending;

use crate::controllers::run_led_state_machine;
use cyw43::Control;
use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::{bind_interrupts, pwm, Peri};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[allow(dead_code)]
async fn setup_wifi(
    spawner: &Spawner,
    p23: Peri<'static, PIN_23>,
    p25: Peri<'static, PIN_25>,
    p24: Peri<'static, PIN_24>,
    p29: Peri<'static, PIN_29>,
    pio0: Peri<'static, PIO0>,
    dma_ch0: Peri<'static, DMA_CH0>,
) -> &'static mut Control<'static> {
    // let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    // let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download ./cyw43-firmware/43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download ./cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000

    let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p23, Level::Low);
    let cs = Output::new(p25, Level::High);
    let mut pio = Pio::new(pio0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p24,
        p29,
        dma_ch0,
    );
    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    static CONTROLLER: StaticCell<Control> = StaticCell::new();

    let state = STATE.init(cyw43::State::new());
    let (_net_device, control, runner) = cyw43::new(state, pwr, spi, fw).await;
    let ctrl = CONTROLLER.init(control);
    unwrap!(spawner.spawn(cyw43_task(runner)));
    ctrl.init(clm).await;
    ctrl.set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;
    ctrl
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let c = setup_wifi(&spawner, p.PIN_23, p.PIN_25, p.PIN_24, p.PIN_29, p.PIO0, p.DMA_CH0).await;
    loop {
        c.gpio_set(0, true).await;
        Timer::after(Duration::from_secs(1)).await;
        c.gpio_set(0, false).await;
        Timer::after(Duration::from_secs(1)).await;
    }
}
