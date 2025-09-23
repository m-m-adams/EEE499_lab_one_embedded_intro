use defmt::{dbg, debug, info, Format};
use embassy_futures::select::Either;
use embassy_rp::gpio::{Input, Level, Output};
use embassy_time::{Duration, Timer};

struct Off;
struct On;
struct Blinking(bool);
struct Fading(u8); // u8 represents brightness level from 0 to 255

#[derive(Debug, Format)]
enum PressType {
    Short,
    Long,
    Double,
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

enum LedState {
    Off(Off),
    On(On),
    Blinking(Blinking),
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
        debug!("Button pressed");
        let t = Timer::after(Duration::from_millis(200)); // check for hold
        let p = self.input.wait_for_falling_edge(); // check for release
        debug!("waiting for hold or release");
        match embassy_futures::select::select(t, p).await {
            embassy_futures::select::Either::First(_) => {
                // Timer completed first, button is held
                dbg!(PressType::Long)
            }
            embassy_futures::select::Either::Second(_) => {
                // Button released before timer, short press
                let t = Timer::after(Duration::from_millis(200)); // check for double press
                let p = self.input.wait_for_rising_edge(); // check for another press
                match embassy_futures::select::select(t, p).await {
                    embassy_futures::select::Either::First(_) => dbg!(PressType::Short), // Timer completed first, no second press
                    embassy_futures::select::Either::Second(_) => {
                        self.input.wait_for_falling_edge().await; // wait for release
                        dbg!(PressType::Double)
                    }
                }
            }
        }


    }
}

impl LedController {

    pub fn new(input: Input<'static>, output: Output<'static>) -> Self {
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
            let next_state = match &self.state {
                LedState::Off(state) => state.next_state(&mut self.input).await,
                LedState::On(state) => state.next_state(&mut self.input).await,
                LedState::Blinking(state) => state.next_state(&mut self.input).await,
            };
            let level = match &self.state {
                LedState::Off(state) => state.get_level(),
                LedState::On(state) => state.get_level(),
                LedState::Blinking(state) => state.get_level(),
            };
            self.set_level(level).await;
            self.state = next_state;
        }
    }
}