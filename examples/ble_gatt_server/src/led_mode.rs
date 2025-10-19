#[repr(u8)]
#[derive(Debug, defmt::Format)]
pub enum LedMode {
    Off = 0,
    Red = 1,
    Green = 2,
    Blue = 3,
}

impl TryFrom<u8> for LedMode {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => LedMode::Off,
            1 => LedMode::Red,
            2 => LedMode::Green,
            3 => LedMode::Blue,
            _ => return Err("invalid LED mode"),
        })
    }
}
