use defmt::{dbg, debug, info, trace, Format};
use embassy_futures::select::Either;
use embassy_rp::gpio::{Input, Level, Output};
use embassy_time::{Duration, Timer};
#[derive(Debug, Format)]
struct Off;
#[derive(Debug, Format)]
struct On;
#[derive(Debug, Format)]
struct Blinking(bool);

#[derive(Debug, Format)]
enum PressType {
    Short,
    Long,
}

trait LedStateTransition {
    async fn next_state(&self, controller: &mut Button) -> LedState;
    fn get_level(&self) -> Level;
}

impl LedStateTransition for Off {
    async fn next_state(&self, controller: &mut Button) -> LedState {
        controller.wait_for_press().await;
        LedState::On(On)
    }
    fn get_level(&self) -> Level {
        Level::Low
    }
}

impl LedStateTransition for On {
    async fn next_state(&self, controller: &mut Button) -> LedState {
        match controller.wait_for_press().await {
            PressType::Long => LedState::Blinking(Blinking(true)),
            _ => LedState::Off(Off),
        }
    }
    fn get_level(&self) -> Level {
        Level::High
    }
}

impl LedStateTransition for Blinking {
    async fn next_state(&self, controller: &mut Button) -> LedState {
        let press = controller.wait_for_press();
        let time = Timer::after(Duration::from_millis(1000));
        match embassy_futures::select::select(press, time).await {
            embassy_futures::select::Either::First(press_type) => LedState::Off(Off),
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
    async fn next_state(&self, controller: &mut Button) -> LedState {
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

pub struct LedController {
    state: LedState,
    status: u8,
    input: Button,
    output: Output<'static>,
}

struct Button {
    input: Input<'static>,
}

impl Button {
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

    pub fn new(input: Input<'static>, mut output: Output<'static>) -> Self {
        output.set_inversion(true);
        Self {
            state: LedState::Off(Off),
            status: 0,
            input: Button { input },
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