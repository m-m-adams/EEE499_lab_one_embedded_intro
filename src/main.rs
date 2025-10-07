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

    //unwrap!(spawner.spawn(blinky(ctrl)));
    let input1 = Input::new(p.PIN_6, embassy_rp::gpio::Pull::Up);
    let input2 = Input::new(p.PIN_7, embassy_rp::gpio::Pull::Up);
    let input3 = Input::new(p.PIN_8, embassy_rp::gpio::Pull::Up);
    let input4 = Input::new(p.PIN_9, embassy_rp::gpio::Pull::Up);
    let conf = pwm::Config::default();
    let (pwm_a, pwm_b) =
        pwm::Pwm::new_output_ab(p.PWM_SLICE1, p.PIN_2, p.PIN_3, conf.clone()).split();
    let (pwm_c, pwm_d) = pwm::Pwm::new_output_ab(p.PWM_SLICE2, p.PIN_4, p.PIN_5, conf).split();
    spawner
        .spawn(run_led_state_machine(input1, pwm_a.unwrap()))
        .unwrap();
    spawner
        .spawn(run_led_state_machine(input2, pwm_b.unwrap()))
        .unwrap();
    spawner
        .spawn(run_led_state_machine(input3, pwm_c.unwrap()))
        .unwrap();
    spawner
        .spawn(run_led_state_machine(input4, pwm_d.unwrap()))
        .unwrap();
}
