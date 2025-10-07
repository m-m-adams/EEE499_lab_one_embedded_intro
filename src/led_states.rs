use crate::pending;
use core::future::Future;
use defmt::Format;
use embassy_rp::gpio::Level;
use embassy_time::{Duration, Timer};

#[derive(Debug, Format)]
pub struct Off;
#[derive(Debug, Format)]
pub struct On;
#[derive(Debug, Format)]
pub struct Blinking(bool);

#[derive(Debug, Format)]
pub enum PressType {
    Short,
    Long,
    Double,
}

pub trait LedStateTransition {
    async fn time_transition(&self) -> LedState;
    fn press_transition(&self, b: PressType) -> LedState;
    fn get_level(&self) -> Level;
}

impl LedStateTransition for Off {
    async fn time_transition(&self) -> LedState {
        pending::pending::<LedState>().await
    }
    fn press_transition(&self, b: PressType) -> LedState {
        LedState::On(On)
    }
    fn get_level(&self) -> Level {
        Level::Low
    }
}

impl LedStateTransition for On {
    async fn time_transition(&self) -> LedState {
        pending::pending::<LedState>().await
    }
    fn press_transition(&self, b: PressType) -> LedState {
        match b {
            PressType::Long => LedState::Blinking(Blinking(true)),
            _ => LedState::Off(Off),
        }
    }

    fn get_level(&self) -> Level {
        Level::High
    }
}

impl LedStateTransition for Blinking {
    async fn time_transition(&self) -> LedState {
        let time = Timer::after(Duration::from_millis(1000));
        time.await;
        LedState::Blinking(Blinking(!self.0))
    }
    fn press_transition(&self, b: PressType) -> LedState {
        match b {
            PressType::Long => LedState::On(On),
            _ => LedState::Off(Off),
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
pub(crate) enum LedState {
    Off(Off),
    On(On),
    Blinking(Blinking),
}

impl LedStateTransition for LedState {
    async fn time_transition(&self) -> LedState {
        match self {
            LedState::Off(state) => state.time_transition().await,
            LedState::On(state) => state.time_transition().await,
            LedState::Blinking(state) => state.time_transition().await,
        }
    }
    fn press_transition(&self, b: PressType) -> LedState {
        match self {
            LedState::Off(state) => state.press_transition(b),
            LedState::On(state) => state.press_transition(b),
            LedState::Blinking(state) => state.press_transition(b),
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

