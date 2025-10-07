use crate::pending;
use core::future::Future;
use defmt::Format;
use embassy_rp::gpio::Level;
use embassy_rp::pio::Direction;
use embassy_time::{Duration, Timer};
use enum_dispatch::enum_dispatch;

#[derive(Debug, Format)]
pub struct Off;
#[derive(Debug, Format)]
pub struct On;
#[derive(Debug, Format)]
pub struct Blinking(bool);

#[derive(Debug, Format)]
pub struct Fading {
    level: u8,
    direction: bool,
}

#[derive(Debug, Format)]
pub enum PressType {
    Short,
    Long,
    Double,
}

#[derive(Debug, Format)]
pub struct LedLevel {
    level: u8,
}

impl From<u8> for LedLevel {
    fn from(l: u8) -> Self {
        let level = l.clamp(0, 100);
        LedLevel { level }
    }
}
impl From<bool> for LedLevel {
    fn from(l: bool) -> Self {
        match l {
            false => LedLevel { level: 0 },
            true => LedLevel { level: 100 },
        }
    }
}

impl From<LedLevel> for u8 {
    fn from(l: LedLevel) -> Self {
        l.level
    }
}

#[enum_dispatch]
pub trait LedStateTransition {
    async fn time_transition(&self) -> LedState;
    fn press_transition(&self, b: PressType) -> LedState;
    fn get_level(&self) -> LedLevel;
}

impl LedStateTransition for Off {
    async fn time_transition(&self) -> LedState {
        pending::pending::<LedState>().await
    }
    fn press_transition(&self, _b: PressType) -> LedState {
        LedState::On(On)
    }
    fn get_level(&self) -> LedLevel {
        LedLevel { level: 0 }
    }
}

impl LedStateTransition for On {
    async fn time_transition(&self) -> LedState {
        pending::pending::<LedState>().await
    }
    fn press_transition(&self, b: PressType) -> LedState {
        match b {
            PressType::Long => LedState::Blinking(Blinking(true)),
            PressType::Double => LedState::Fading(Fading {
                level: 100,
                direction: false,
            }),
            _ => LedState::Off(Off),
        }
    }

    fn get_level(&self) -> LedLevel {
        LedLevel { level: 100 }
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
    fn get_level(&self) -> LedLevel {
        self.0.into()
    }
}

impl LedStateTransition for Fading {
    async fn time_transition(&self) -> LedState {
        let time = Timer::after(Duration::from_millis(50));
        time.await;
        let l = self.level;
        let dir = {
            if self.level == 0 {
                true
            } else if self.level == 100 {
                false
            } else {
                self.direction
            }
        };
        let mult: i8 = if dir { 1 } else { -1 };
        Fading {
            level: (l as i8 + 10 * mult).clamp(0, 100) as u8,
            direction: dir,
        }
        .into()
    }

    fn press_transition(&self, b: PressType) -> LedState {
        match b {
            PressType::Long => LedState::On(On),
            _ => LedState::Off(Off),
        }
    }

    fn get_level(&self) -> LedLevel {
        self.level.into()
    }
}

#[enum_dispatch(LedStateTransition)]
#[derive(Debug, Format)]
pub(crate) enum LedState {
    Off,
    On,
    Blinking,
    Fading,
}
