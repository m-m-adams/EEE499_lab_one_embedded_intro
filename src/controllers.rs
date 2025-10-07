use crate::led_states::{LedLevel, LedState, LedStateTransition, Off, PressType};
use embassy_futures::join::join;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::Input;
use embassy_rp::pwm::PwmOutput;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::{Duration, Timer};
use embedded_hal::pwm::SetDutyCycle;

pub struct LedController {
    state: LedState,
    output: PwmOutput<'static>,
}

pub struct ButtonController {
    input: Input<'static>,
}

impl ButtonController {
    pub fn new(input: Input<'static>) -> Self {
        todo!()

    }
    /// wait for a press
    async fn press(&mut self) {
        todo!()

    }
    /// wait for a release
    async fn release(&mut self) {
        todo!()

    }

    async fn detect_press(&mut self) -> PressType {
        todo!()

    }
}

impl LedController {
    pub fn new(mut output: PwmOutput<'static>) -> Self {
        todo!()

    }
    fn set_level(&mut self, level: LedLevel) {
        todo!()

    }

    pub async fn time(&mut self) {
        todo!()
    }

    pub fn button_pressed(&mut self, press: PressType) {
        todo!()
    }
}

pub type LedChannelReceiver<'a> = Receiver<'a, CriticalSectionRawMutex, PressType, 4>;
pub type LedChannelSender<'a> = Sender<'a, CriticalSectionRawMutex, PressType, 4>;
pub type LedChannel = Channel<CriticalSectionRawMutex, PressType, 4>;

async fn led_task<'a>(mut led_controller: LedController, channel: LedChannelReceiver<'a>) -> ! {
    todo!()

}

async fn button_task<'a>(
    mut button_controller: ButtonController,
    channel: LedChannelSender<'a>,
) -> ! {
    todo!()
}

#[embassy_executor::task(pool_size = 4)]
pub async fn run_led_state_machine(input: Input<'static>, output: PwmOutput<'static>) {
    todo!()

}
