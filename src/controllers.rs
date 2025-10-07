use defmt::{dbg, trace};
use embassy_rp::gpio::{Input, Level, Output};
use embassy_time::{Duration, Timer};
use embassy_futures::select::{select, Either};
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_futures::join::join;
use crate::led_states::{LedState, LedStateTransition, Off, PressType};

pub struct LedController {
    state: LedState,
    status: u8,
    output: Output<'static>,
}

pub struct ButtonController {
    input: Input<'static>,
}

impl ButtonController {
    pub fn new(input: Input<'static>) -> Self {
        Self { input }
    }
    async fn wait_for_press(&mut self) -> PressType {
        self.input.wait_for_rising_edge().await;
        Timer::after(Duration::from_millis(1)).await;
        self.input.wait_for_high().await; // debounce
        trace!("Button pressed");
        let t = Timer::after(Duration::from_millis(200)); // check for hold
        let p = self.input.wait_for_low(); // check for release
        trace!("waiting for hold or release");
        match embassy_futures::select::select(t, p).await {
            embassy_futures::select::Either::First(_) => {
                // Timer completed first, button is held
                dbg!(PressType::Long)
            }
            embassy_futures::select::Either::Second(_) => {
                dbg!(PressType::Short)
            }
        }
    }
}

impl LedController {
    pub fn new(mut output: Output<'static>) -> Self {
        output.set_inversion(true);
        Self {
            state: LedState::Off(Off),
            status: 0,
            output,
        }
    }
    fn set_level(&mut self, level: Level) {
        // Set the LED on or off
        self.output.set_level(level);
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
        let p = button_controller.wait_for_press().await;
        channel.send(p).await;
    }
}

pub type LedChannelReceiver<'a> = Receiver<'a, CriticalSectionRawMutex, PressType, 4>;
pub type LedChannelSender<'a> = Sender<'a, CriticalSectionRawMutex, PressType, 4>;
pub type LedChannel = Channel<CriticalSectionRawMutex, PressType, 4>;

#[embassy_executor::task(pool_size = 4)]
pub async fn run_led_state_machine(input: Input<'static>, output: Output<'static>) {
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