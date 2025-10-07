use crate::pending;
use core::ops::{Add, Mul};
use defmt::Format;
use embassy_time::{Duration, Timer};
use enum_dispatch::enum_dispatch;

#[enum_dispatch(LedStateTransition)]
#[derive(Debug, Format)]
pub(crate) enum LedState {
    Off,
    On,
    Blinking,
    Fading,
}
#[derive(Debug, Format)]
pub struct Off;
#[derive(Debug, Format)]
pub struct On;
#[derive(Debug, Format)]
pub struct Blinking(bool);

#[derive(Debug, Format)]
pub struct Fading {
    level: LedLevel,
    direction: FadeDirection,
}

#[derive(Debug, Format)]
pub enum PressType {
    Short,
    Long,
    Double,
}
/// an i8 limited to the range 0-100 that can only be created at 0 or 100
#[derive(Debug, Format, PartialOrd, PartialEq, Default, Copy, Clone)]
pub struct LedLevel {
    level: i8,
}


#[derive(Debug, Format, Copy, Clone)]
enum FadeDirection {
    Up,
    Down,
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
        On.into()
    }
    fn get_level(&self) -> LedLevel {
        LedLevel::MIN
    }
}

impl LedStateTransition for On {
    async fn time_transition(&self) -> LedState {
        pending::pending::<LedState>().await
    }
    fn press_transition(&self, b: PressType) -> LedState {
        match b {
            PressType::Long => Blinking(true).into(),
            PressType::Double => Fading {
                level: LedLevel::MAX,
                direction: FadeDirection::Up,
            }
            .into(),

            _ => Off.into(),
        }
    }

    fn get_level(&self) -> LedLevel {
        LedLevel::MAX
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
            PressType::Long => On.into(),
            _ => Off.into(),
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
        let dir = {
            match self.level {
                LedLevel::MIN => FadeDirection::Up,
                LedLevel::MAX => FadeDirection::Down,
                _ => self.direction,
            }
        };
        Fading {
            level: self.level + self.direction * 10,
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
        self.level
    }
}



impl LedLevel {
    pub const MAX: LedLevel = LedLevel { level: 100 };
    pub const MIN: LedLevel = LedLevel { level: 0 };
}
impl Add<i8> for LedLevel {
    type Output = Self;
    fn add(self, rhs: i8) -> Self::Output {
        let level = (self.level + rhs).clamp(0, 100);
        LedLevel { level }
    }
}
impl From<bool> for LedLevel {
    fn from(l: bool) -> Self {
        if l {
            LedLevel::MAX
        } else {
            LedLevel::MIN
        }
    }
}

impl From<LedLevel> for u8 {
    fn from(l: LedLevel) -> Self {
        l.level as u8
    }
}

impl<T: core::ops::Neg<Output = T>> Mul<T> for FadeDirection {
    type Output = T;
    fn mul(self, rhs: T) -> Self::Output {
        match self {
            FadeDirection::Up => rhs,
            FadeDirection::Down => -rhs,
        }
    }
}