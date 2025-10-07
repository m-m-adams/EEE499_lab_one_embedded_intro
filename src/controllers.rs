use crate::led_states::{LedLevel, LedState, LedStateTransition, Off, PressType};
use defmt::{dbg, trace};
use embassy_futures::join::join;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::{Input, Level, Output};
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
        Self { input }
    }
    /// wait for a press
    async fn press(&mut self) {
        self.input.wait_for_rising_edge().await;
        Timer::after(Duration::from_millis(5)).await;
        self.input.wait_for_high().await; // debounce
    }
    /// wait for a release
    async fn release(&mut self) {
        self.input.wait_for_falling_edge().await;
        Timer::after(Duration::from_millis(5)).await;
        self.input.wait_for_low().await;
    }

    async fn detect_press(&mut self) -> PressType {
        self.press().await;
        trace!("Button pressed");
        let t = Timer::after(Duration::from_millis(200)); // check for hold
        let p = self.release();
        trace!("waiting for hold or release");
        if let Either::First(_) = select(t, p).await {
            // Timer completed first, button is held
            return dbg!(PressType::Long);
        };
        let t = Timer::after(Duration::from_millis(200)); // check for hold
        let p = self.release();
        match select(t, p).await {
            Either::First(_) => {
                // Timer first, short press
                dbg!(PressType::Short)
            }
            Either::Second(_) => {
                // second press first, double press
                dbg!(PressType::Double)
            }
        }
    }
}

impl LedController {
    pub fn new(mut output: PwmOutput<'static>) -> Self {
        output
            .set_duty_cycle_percent(0)
            .expect("definitely shouldn't fail");
        Self {
            state: LedState::Off(Off),
            output,
        }
    }
    fn set_level(&mut self, level: LedLevel) {
        let l: u8 = level.into();
        // Set the LED on or off
        self.output.set_duty_cycle_percent(100 - l).expect("level not between 0 and 100");
    }

    pub async fn time(&mut self) {
        let level = self.state.get_level();
        self.set_level(level);
        let next_state = self.state.time_transition().await;
        self.state = dbg!(next_state)
    }

    pub fn button_pressed(&mut self, press: PressType) {
        self.state = self.state.press_transition(press);
    }
}

pub type LedChannelReceiver<'a> = Receiver<'a, CriticalSectionRawMutex, PressType, 4>;
pub type LedChannelSender<'a> = Sender<'a, CriticalSectionRawMutex, PressType, 4>;
pub type LedChannel = Channel<CriticalSectionRawMutex, PressType, 4>;

async fn led_task<'a>(mut led_controller: LedController, channel: LedChannelReceiver<'a>) -> ! {
    loop {
        if let Either::Second(press) = select(led_controller.time(), channel.receive()).await {
            led_controller.button_pressed(press);
        }
    }
}

async fn button_task<'a>(
    mut button_controller: ButtonController,
    channel: LedChannelSender<'a>,
) -> ! {
    loop {
        let p = button_controller.detect_press().await;
        channel.send(p).await;
    }
}

#[embassy_executor::task(pool_size = 4)]
pub async fn run_led_state_machine(input: Input<'static>, output: PwmOutput<'static>) {
    let channel = LedChannel::new();

    let receiver = channel.receiver();
    let sender = channel.sender();
    let led_controller = LedController::new(output);
    let button_controller = ButtonController::new(input);
    join(
        led_task(led_controller, receiver),
        button_task(button_controller, sender),
    )
    .await;
}
