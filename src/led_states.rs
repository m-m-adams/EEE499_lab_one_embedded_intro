use defmt::{dbg, debug, info, trace, Format};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::{Input, Level, Output};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::{Duration, Timer};
#[derive(Debug, Format)]
struct Off;
#[derive(Debug, Format)]
struct On;
#[derive(Debug, Format)]
struct Blinking(bool);

#[derive(Debug, Format)]
pub enum PressType {
    Short,
    Long,
    Double,
}

trait LedStateTransition {
    async fn next_state(&self, controller: &mut LedChannelReceiver) -> LedState;
    fn get_level(&self) -> Level;
}

impl LedStateTransition for Off {
    async fn next_state(&self, controller: &mut LedChannelReceiver) -> LedState {
        controller.receive().await;
        LedState::On(On)
    }
    fn get_level(&self) -> Level {
        Level::Low
    }
}

impl LedStateTransition for On {
    async fn next_state(&self, controller: &mut LedChannelReceiver) -> LedState {
        match controller.receive().await {
            PressType::Long => LedState::Blinking(Blinking(true)),
            _ => LedState::Off(Off),
        }
    }
    fn get_level(&self) -> Level {
        Level::High
    }
}

impl LedStateTransition for Blinking {
    async fn next_state(&self, controller: &mut LedChannelReceiver) -> LedState {
        let press = controller.receive();
        let time = Timer::after(Duration::from_millis(1000));
        match select(press, time).await {
            Either::First(press_type) => LedState::Off(Off),
            _ => LedState::Blinking(Blinking(!self.0)),
        }
    }
    fn get_level(&self) -> Level {
        if self.0 {
            Level::High
        } else {
            Level::Low
        }
    }
}
#[derive(Debug, Format)]
enum LedState {
    Off(Off),
    On(On),
    Blinking(Blinking),
}

impl LedStateTransition for LedState {
    async fn next_state(&self, controller: &mut LedChannelReceiver) -> LedState {
        match self {
            LedState::Off(state) => state.next_state(controller).await,
            LedState::On(state) => state.next_state(controller).await,
            LedState::Blinking(state) => state.next_state(controller).await,
        }
    }
    fn get_level(&self) -> Level {
        match self {
            LedState::Off(state) => state.get_level(),
            LedState::On(state) => state.get_level(),
            LedState::Blinking(state) => state.get_level(),
        }
    }
}
pub type LedChannelReceiver = Receiver<'static, CriticalSectionRawMutex, PressType, 4>;
pub type LedChannelSender = Sender<'static, CriticalSectionRawMutex, PressType, 4>;
pub type LedChannel = Channel<CriticalSectionRawMutex, PressType, 4>;
pub struct LedController {
    state: LedState,
    status: u8,
    input: LedChannelReceiver,
    output: Output<'static>,
}

pub struct ButtonController {
    input: Input<'static>,
    channel: LedChannelSender,
}

impl ButtonController {
    pub fn new(
        input: Input<'static>,
        channel: Sender<'static, CriticalSectionRawMutex, PressType, 4>,
    ) -> Self {
        Self { input, channel }
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

    pub(crate) async fn run(&mut self) -> ! {
        loop {
            let press = self.wait_for_press().await;
            self.channel.send(press).await;
        }
    }
}

impl LedController {
    pub fn new(input: LedChannelReceiver, mut output: Output<'static>) -> Self {
        output.set_inversion(true);
        Self {
            state: LedState::Off(Off),
            status: 0,
            input,
            output,
        }
    }
    async fn set_level(&mut self, level: Level) {
        // Set the LED on or off
        self.output.set_level(level);
    }

    pub async fn run(&mut self) -> ! {
        loop {
            let level = self.state.get_level();
            self.set_level(level).await;
            let next_state = self.state.next_state(&mut self.input).await;
            self.state = dbg!(next_state);
        }
    }
}

#[embassy_executor::task(pool_size = 4)]
async fn led_task(mut led_controller: LedController) -> ! {
    loop {
        led_controller.run().await;
    }
}
#[embassy_executor::task(pool_size = 4)]
async fn button_task(mut button_controller: ButtonController) -> ! {
    loop {
        button_controller.run().await;
    }
}

pub async fn setup_led_button_tasks(
    spawner: Spawner,
    input: Input<'static>,
    output: Output<'static>,
    channel: &'static LedChannel,
) {
    let receiver = channel.receiver();
    let sender = channel.sender();
    let led_controller = LedController::new(receiver, output);
    let button_controller = ButtonController::new(input, sender);
    spawner.spawn(led_task(led_controller)).unwrap();
    spawner.spawn(button_task(button_controller)).unwrap();
}