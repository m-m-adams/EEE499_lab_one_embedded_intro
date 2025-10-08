#![no_std]
#![no_main]

#[allow(unused)]
mod controllers;
mod led_states;
mod pending;
mod wifi;

use core::cell::RefCell;
use cortex_m::delay::Delay;
use critical_section::CriticalSection;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::InterruptHandler;
use embassy_rp::{bind_interrupts, config, interrupt, pac};
use embassy_rp::pac::pwm::regs::Intr;
use embassy_rp::pwm::{Config, Pwm};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::Duration;
use {defmt_rtt as _, panic_probe as _};
use fixed::{FixedU16};


// interrupts exist in what's called a vector table, which is a table of addresses that point to functions that are called when an interrupt occurs.
// This creates function called PIO0_IRQ_0 and sets it as an interrupt handler. That function will call InterruptHandler::on_interrupt.
// This is used in rust at compile time to prove to peripherals that interrupts they require are registered
bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});
// needs to be global to access it from interrupt context. Note that these are wrapped in mutexes.
// Because we're now also using interrupts we need to synchronize access properly
static PWM: Mutex<CriticalSectionRawMutex, RefCell<Option<Pwm>>> = Mutex::new(RefCell::new(None));
static OUT_PIN: Mutex<CriticalSectionRawMutex, RefCell<Option<embassy_rp::gpio::Output<'static>>>> = Mutex::new(RefCell::new(None));
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut config = Config::default();
    config.top = 52157;
    config.divider = 255.into();
    config.phase_correct = true;
    let pwm = Pwm::new_free(p.PWM_SLICE7, config);
    // stick the PWM into the static cell
    PWM.lock(|p|{p.borrow_mut().replace(pwm);});

    let mut p8 = embassy_rp::gpio::Output::new(p.PIN_2, embassy_rp::gpio::Level::Low);
    p8.set_high();
    OUT_PIN.lock(|p|{p.borrow_mut().replace(p8);});
    // set the interrupt to get triggered when the counter wraps
    embassy_rp::pac::PWM.inte().modify(|reg|{reg.set_ch7(true)});
    // unmask the interrupt
    unsafe {
        cortex_m::peripheral::NVIC::unmask(embassy_rp::pac::Interrupt::PWM_IRQ_WRAP);
    }

}
#[interrupt]
fn PWM_IRQ_WRAP() {
    critical_section::with(|cs| {
        // all PWM slices trigger the same interrupt so we need to check if it's ours
        // this is a bitfield where each bit represents a channel
        let s= pac::PWM.ints().read();
        // equivalent to if s.0 == 1 << 7        // equivalent to s.
        if s.ch7() {
            unsafe {  pwm_channel_7_wrap(cs) };
        }
        // Clear the interrupt, so we don't immediately re-enter this irq handler
    });
}

unsafe fn pwm_channel_7_wrap(cs: CriticalSection) {
    // Accessing a mutable static can lead to race conditions. We have a critical section so it's fine
    // otherwise retriggering the interrupt would cause problems since this is non reentrant
    unsafe {
        static mut COUNT: u8 = 0;

        COUNT = COUNT + 1;

        if COUNT == 3 {
            let mut p = OUT_PIN.borrow(cs).borrow_mut();
            let out = p.as_mut().unwrap();
            if out.is_set_high() {
                out.set_low();
            } else {
                out.set_high();
            }
            COUNT = 0;
            trace!("delaying");
            embassy_time::block_for(Duration::from_millis(500));
            trace!("done delaying");



        }
        // clear the interrupt for PWM channel 7 wrap
        pac::PWM.intr().write_value(Intr(1 << 7));
        // could also clear through the PWM struct - this does the same thing
        //PWM.borrow(cs).borrow_mut().as_mut().unwrap().clear_wrapped();

    }
}