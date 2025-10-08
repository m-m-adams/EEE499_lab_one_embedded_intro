#![no_std]
#![no_main]

#[allow(unused)]
mod controllers;
mod led_states;
mod pending;
mod wifi;

use core::cell::RefCell;
use cortex_m::peripheral::NVIC;
use critical_section::CriticalSection;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::interrupt::Interrupt;
use embassy_rp::pac::pwm::regs::Intr;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::InterruptHandler;
use embassy_rp::pwm::{Config, Pwm};
use embassy_rp::{bind_interrupts, interrupt, pac};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_time::Duration;
use {defmt_rtt as _, panic_probe as _};

// interrupts exist in what's called a vector table, which is a table of addresses that point to functions that are called when an interrupt occurs.
// This creates function called PIO0_IRQ_0 and sets it as an interrupt handler. That function will call InterruptHandler::on_interrupt.
// This is used in rust at compile time to prove to peripherals that interrupts they require are registered

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

// needs to be global to access it from interrupt context. Note that these are wrapped in mutexes.
// Because we're now also using interrupts we need to synchronize access properly
// It's a refcell to allow mutability, and it's an option since it won't be initialized until our main function runs

static PWM: Mutex<CriticalSectionRawMutex, RefCell<Option<Pwm>>> = Mutex::new(RefCell::new(None));
static OUT_PIN: Mutex<CriticalSectionRawMutex, RefCell<Option<embassy_rp::gpio::Output<'static>>>> =
    Mutex::new(RefCell::new(None));
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut config = Config::default();
    config.top = 52157;
    config.divider = 255.into();
    config.phase_correct = true;
    let pwm = Pwm::new_free(p.PWM_SLICE7, config);
    // stick the PWM into the static cell
    PWM.lock(|p| {
        p.borrow_mut().replace(pwm);
    });

    let mut p8 = embassy_rp::gpio::Output::new(p.PIN_4, embassy_rp::gpio::Level::Low);
    p8.set_high();
    OUT_PIN.lock(|p| {
        p.borrow_mut().replace(p8);
    });
    // set the interrupt to get triggered when the counter wraps
    embassy_rp::pac::PWM.inte().modify(|reg| reg.set_ch7(true));
    // unmask the interrupts
    unsafe {
        NVIC::unmask(Interrupt::PWM_IRQ_WRAP);
        NVIC::unmask(Interrupt::SWI_IRQ_0);
    }

    // For part 3
    // let input1 = Input::new(p.PIN_6, embassy_rp::gpio::Pull::Up);
    // let input2 = Input::new(p.PIN_7, embassy_rp::gpio::Pull::Up);
    // let (pwm_a, pwm_b) =
    //     pwm::Pwm::new_output_ab(p.PWM_SLICE1, p.PIN_2, p.PIN_3, pwm::Config::default()).split();
    // spawner
    //     .spawn(controllers::run_led_state_machine(input1, pwm_a.unwrap()))
    //     .unwrap();
    // spawner
    //     .spawn(controllers::run_led_state_machine(input2, pwm_b.unwrap()))
    //     .unwrap();
}
#[interrupt]
fn PWM_IRQ_WRAP() {
    critical_section::with(|cs| {
        // all PWM slices trigger the same interrupt so we need to check if it's ours
        // this is a bitfield where each bit represents a channel
        let s = pac::PWM.ints().read();
        // equivalent to if s.0 == 1 << 7        // equivalent to s.
        if s.ch7() {
            unsafe { pwm_channel_7_wrap(cs) };
        }
        // Clear the interrupt, so we don't immediately re-enter this irq handler
    });
}
#[interrupt]
fn SWI_IRQ_0() {
    trace!("SWI_IRQ_0");
}

unsafe fn pwm_channel_7_wrap(cs: CriticalSection) {
    // Accessing a mutable static can lead to race conditions. We have a critical section so it's fine
    // otherwise retriggering the interrupt would cause problems since this is non reentrant
    unsafe {
        static mut COUNT: u8 = 0;

        COUNT += 1;

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
            NVIC::pend(Interrupt::SWI_IRQ_0);
        }
        // clear the interrupt for PWM channel 7 wrap
        pac::PWM.intr().write_value(Intr(1 << 7));
        // could also clear through the PWM struct - this does the same thing
        //PWM.borrow(cs).borrow_mut().as_mut().unwrap().clear_wrapped();
    }
}
