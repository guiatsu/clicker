use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SpeedMode {
    #[default]
    Normal,
    HighPrecision,
    Unlimited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ActivationMode {
    #[default]
    Toggle,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DestinationMode {
    #[default]
    CurrentCursor,
    FixedPosition,
    PositionSequence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ClickKind {
    #[default]
    Single,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Modifiers {
    pub control: bool,
    pub shift: bool,
    pub alt: bool,
    pub windows: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Hotkey {
    pub key_code: u16,
    pub modifiers: Modifiers,
}

impl Hotkey {
    pub fn label(self) -> String {
        if self.key_code == 0 {
            return "Não configurado".to_owned();
        }

        let mut parts = Vec::new();
        if self.modifiers.control {
            parts.push("Ctrl".to_owned());
        }
        if self.modifiers.shift {
            parts.push("Shift".to_owned());
        }
        if self.modifiers.alt {
            parts.push("Alt".to_owned());
        }
        if self.modifiers.windows {
            parts.push("Win".to_owned());
        }
        parts.push(key_label(self.key_code));
        parts.join("+")
    }

    pub fn is_configured(self) -> bool {
        self.key_code != 0
    }
}

pub fn key_label(code: u16) -> String {
    match code {
        0x30..=0x39 | 0x41..=0x5a => char::from_u32(code as u32).unwrap().to_string(),
        0x70..=0x87 => format!("F{}", code - 0x6f),
        0x20 => "Espaço".to_owned(),
        0x1b => "Esc".to_owned(),
        0x09 => "Tab".to_owned(),
        0x2d => "Insert".to_owned(),
        0x2e => "Delete".to_owned(),
        _ => format!("VK {code:02X}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub speed_mode: SpeedMode,
    pub activation_mode: ActivationMode,
    pub destination_mode: DestinationMode,
    pub mouse_button: MouseButton,
    pub click_kind: ClickKind,
    pub cps: f64,
    pub interval_ms: f64,
    pub press_duration_ms: f64,
    pub max_clicks: Option<u64>,
    pub fixed_position: Point,
    pub positions: Vec<Point>,
    pub clicks_per_position: u32,
    pub interval_between_positions_ms: f64,
    pub cycles: u64,
    pub repeat_continuously: bool,
    pub start_hotkey: Hotkey,
    pub add_position_hotkey: Hotkey,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            speed_mode: SpeedMode::Normal,
            activation_mode: ActivationMode::Toggle,
            destination_mode: DestinationMode::CurrentCursor,
            mouse_button: MouseButton::Left,
            click_kind: ClickKind::Single,
            cps: 10.0,
            interval_ms: 100.0,
            press_duration_ms: 1.0,
            max_clicks: None,
            fixed_position: Point::default(),
            positions: Vec::new(),
            clicks_per_position: 1,
            interval_between_positions_ms: 100.0,
            cycles: 1,
            repeat_continuously: true,
            start_hotkey: Hotkey {
                key_code: 0x75,
                modifiers: Modifiers::default(),
            },
            add_position_hotkey: Hotkey {
                key_code: 0x76,
                modifiers: Modifiers::default(),
            },
        }
    }
}

impl Settings {
    pub fn interval_from_cps(cps: f64) -> Option<f64> {
        (cps.is_finite() && cps > 0.0).then(|| 1000.0 / cps)
    }

    pub fn cps_from_interval(interval_ms: f64) -> Option<f64> {
        (interval_ms.is_finite() && interval_ms > 0.0).then(|| 1000.0 / interval_ms)
    }

    pub fn set_cps(&mut self, cps: f64) {
        if let Some(interval) = Self::interval_from_cps(cps) {
            self.cps = cps;
            self.interval_ms = interval;
        }
    }

    pub fn set_interval_ms(&mut self, interval_ms: f64) {
        if let Some(cps) = Self::cps_from_interval(interval_ms) {
            self.interval_ms = interval_ms;
            self.cps = cps;
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.cps.is_finite() || !(0.1..=100_000.0).contains(&self.cps) {
            return Err("CPS deve estar entre 0,1 e 100.000.".to_owned());
        }
        if !self.interval_ms.is_finite() || !(0.01..=600_000.0).contains(&self.interval_ms) {
            return Err("O intervalo deve estar entre 0,01 e 600.000 ms.".to_owned());
        }
        if !self.press_duration_ms.is_finite()
            || !(0.0..=60_000.0).contains(&self.press_duration_ms)
        {
            return Err("A duração do pressionamento deve estar entre 0 e 60.000 ms.".to_owned());
        }
        if !self.interval_between_positions_ms.is_finite()
            || !(0.0..=600_000.0).contains(&self.interval_between_positions_ms)
        {
            return Err("O intervalo entre posições deve estar entre 0 e 600.000 ms.".to_owned());
        }
        if self.clicks_per_position == 0 {
            return Err("Configure ao menos um clique por posição.".to_owned());
        }
        if self.cycles == 0 {
            return Err("Configure ao menos um ciclo.".to_owned());
        }
        if self.destination_mode == DestinationMode::PositionSequence && self.positions.is_empty() {
            return Err("Adicione ao menos uma posição para usar a sequência.".to_owned());
        }
        if self.start_hotkey.is_configured()
            && self.add_position_hotkey.is_configured()
            && self.start_hotkey == self.add_position_hotkey
        {
            return Err("O atalho de execução e o de captura precisam ser diferentes.".to_owned());
        }
        Ok(())
    }

    pub fn scheduler_warning(&self) -> Option<&'static str> {
        if self.speed_mode != SpeedMode::Unlimited && self.press_duration_ms > self.interval_ms {
            return Some(
                "A duração do pressionamento supera o intervalo; o CPS efetivo ficará abaixo do configurado.",
            );
        }
        match self.speed_mode {
            SpeedMode::Normal if self.interval_ms < 15.0 => Some(
                "Abaixo de 15 ms o scheduler normal pode alongar os intervalos; a taxa solicitada não é garantida.",
            ),
            SpeedMode::HighPrecision if self.interval_ms < 15.0 => Some(
                "A espera ativa melhora o trecho final do intervalo, mas o scheduler e o destino ainda limitam a precisão.",
            ),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cps_and_interval_are_reciprocal() {
        let mut settings = Settings::default();
        settings.set_cps(25.0);
        assert_eq!(settings.interval_ms, 40.0);
        settings.set_interval_ms(20.0);
        assert_eq!(settings.cps, 50.0);
    }

    #[test]
    fn rejects_conflicting_shortcuts_and_empty_sequence() {
        let mut settings = Settings::default();
        settings.add_position_hotkey = settings.start_hotkey;
        assert!(settings.validate().is_err());
        settings.add_position_hotkey.key_code = 0x77;
        settings.destination_mode = DestinationMode::PositionSequence;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn recognizes_when_scheduler_cannot_guarantee_requested_rate() {
        let mut settings = Settings {
            interval_ms: 0.5,
            press_duration_ms: 0.0,
            ..Settings::default()
        };
        assert!(settings.scheduler_warning().is_some());
        settings.speed_mode = SpeedMode::HighPrecision;
        assert!(settings.scheduler_warning().is_some());
    }
}
