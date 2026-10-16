use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

/// A global shortcut represented as a normalized set of simultaneously held keys.
#[derive(Debug, Clone, Default)]
pub struct Hotkey {
    keys: Vec<u16>,
}

impl Hotkey {
    pub fn from_keys(keys: impl IntoIterator<Item = u16>) -> Self {
        let mut keys = keys
            .into_iter()
            .map(normalize_key)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        keys.sort_by_key(|key| key_order(*key));
        Self { keys }
    }

    pub fn matches_keys(&self, keys: &[u16]) -> bool {
        self.is_configured() && self.keys == Self::from_keys(keys.iter().copied()).keys
    }

    pub fn conflicts_with(&self, other: &Self) -> bool {
        if !self.is_configured() || !other.is_configured() {
            return false;
        }
        is_subset(&self.keys, &other.keys) || is_subset(&other.keys, &self.keys)
    }

    pub fn label(&self) -> String {
        if self.keys.is_empty() {
            return "Não configurado".to_owned();
        }
        self.keys
            .iter()
            .map(|key| key_label(*key))
            .collect::<Vec<_>>()
            .join(" + ")
    }

    pub fn is_configured(&self) -> bool {
        !self.keys.is_empty()
    }
}

impl PartialEq for Hotkey {
    fn eq(&self, other: &Self) -> bool {
        self.keys == other.keys
    }
}

impl Eq for Hotkey {}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct HotkeyWire {
    keys: Vec<u16>,
    // Kept so profiles written by the previous release still load.
    key_code: u16,
    modifiers: Modifiers,
}

impl Serialize for Hotkey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let modifiers = Modifiers {
            control: self.keys.contains(&0x11),
            shift: self.keys.contains(&0x10),
            alt: self.keys.contains(&0x12),
            windows: self.keys.contains(&0x5b),
        };
        let key_code = self
            .keys
            .iter()
            .rev()
            .find(|key| !is_modifier(**key))
            .copied()
            .unwrap_or(0);
        HotkeyWire {
            keys: self.keys.clone(),
            key_code,
            modifiers,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Hotkey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = HotkeyWire::deserialize(deserializer)?;
        if !wire.keys.is_empty() {
            return Ok(Self::from_keys(wire.keys));
        }
        if wire.key_code == 0 {
            return Ok(Self::default());
        }
        let mut keys = Vec::new();
        if wire.modifiers.control {
            keys.push(0x11);
        }
        if wire.modifiers.shift {
            keys.push(0x10);
        }
        if wire.modifiers.alt {
            keys.push(0x12);
        }
        if wire.modifiers.windows {
            keys.push(0x5b);
        }
        keys.push(wire.key_code);
        Ok(Self::from_keys(keys))
    }
}

pub fn normalize_key(code: u16) -> u16 {
    match code {
        0xa2 | 0xa3 => 0x11,
        0xa0 | 0xa1 => 0x10,
        0xa4 | 0xa5 => 0x12,
        0x5c => 0x5b,
        _ => code,
    }
}

fn is_modifier(code: u16) -> bool {
    matches!(normalize_key(code), 0x10..=0x12 | 0x5b)
}

fn key_order(code: u16) -> (u8, u16) {
    let priority = match code {
        0x11 => 0,
        0x10 => 1,
        0x12 => 2,
        0x5b => 3,
        _ => 4,
    };
    (priority, code)
}

pub fn key_label(code: u16) -> String {
    match normalize_key(code) {
        0x10 => "Shift".to_owned(),
        0x11 => "Ctrl".to_owned(),
        0x12 => "Alt".to_owned(),
        0x5b => "Win".to_owned(),
        0x30..=0x39 | 0x41..=0x5a => char::from_u32(code as u32).unwrap().to_string(),
        0x70..=0x87 => format!("F{}", code - 0x6f),
        0x20 => "Espaço".to_owned(),
        0x1b => "Esc".to_owned(),
        0x09 => "Tab".to_owned(),
        0x2d => "Insert".to_owned(),
        0x2e => "Delete".to_owned(),
        0x25 => "←".to_owned(),
        0x26 => "↑".to_owned(),
        0x27 => "→".to_owned(),
        0x28 => "↓".to_owned(),
        _ => format!("VK {code:02X}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
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
    pub toggle_overlay_hotkey: Hotkey,
    pub add_coordinate_hotkey: Hotkey,
    pub clear_positions_hotkey: Hotkey,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
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
            start_hotkey: Hotkey::from_keys([0x75]),
            add_position_hotkey: Hotkey::from_keys([0x76]),
            toggle_overlay_hotkey: Hotkey::from_keys([0x77]),
            add_coordinate_hotkey: Hotkey::from_keys([0x11, 0x78]),
            clear_positions_hotkey: Hotkey::from_keys([0x12, 0x10, 0x78]),
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

        let shortcuts = [
            ("iniciar/parar", &self.start_hotkey),
            ("capturar posição", &self.add_position_hotkey),
            ("mostrar posições", &self.toggle_overlay_hotkey),
            ("adicionar coordenada", &self.add_coordinate_hotkey),
            ("limpar posições", &self.clear_positions_hotkey),
        ];
        for (index, (name, shortcut)) in shortcuts.iter().enumerate() {
            if !shortcut.is_configured() {
                continue;
            }
            if shortcuts[index + 1..]
                .iter()
                .any(|(_, other)| shortcut.conflicts_with(other))
            {
                return Err(format!(
                    "O atalho de {name} está associado a mais de uma ação."
                ));
            }
        }
        Ok(())
    }

    pub fn scheduler_warning(&self) -> Option<&'static str> {
        if self.press_duration_ms > self.interval_ms {
            Some(
                "A duração do pressionamento supera o intervalo; o CPS efetivo ficará abaixo do configurado.",
            )
        } else if self.interval_ms < 15.0 {
            Some(
                "Abaixo de 15 ms, a agenda do Windows pode alongar os intervalos; a taxa solicitada não é garantida.",
            )
        } else {
            None
        }
    }
}

fn is_subset(left: &[u16], right: &[u16]) -> bool {
    left.iter().all(|key| right.contains(key))
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
    fn supports_arbitrary_chords_and_normalizes_modifier_sides() {
        let hotkey = Hotkey::from_keys([0xa2, 0x10, 0x75]);
        assert_eq!(hotkey.label(), "Ctrl + Shift + F6");
        assert!(hotkey.matches_keys(&[0x11, 0x10, 0x75]));
    }

    #[test]
    fn reads_legacy_shortcuts_and_round_trips_new_chords() {
        let old: Hotkey = serde_json::from_str(
            r#"{"key_code":117,"modifiers":{"control":true,"shift":false,"alt":false,"windows":false}}"#,
        )
        .unwrap();
        assert_eq!(old.label(), "Ctrl + F6");

        let new = Hotkey::from_keys([0x41, 0x42, 0x43]);
        let restored: Hotkey = serde_json::from_str(&serde_json::to_string(&new).unwrap()).unwrap();
        assert_eq!(restored.label(), "A + B + C");
    }

    #[test]
    fn rejects_conflicting_shortcuts_and_empty_sequence() {
        let mut settings = Settings::default();
        settings.add_position_hotkey = settings.start_hotkey.clone();
        assert!(settings.validate().is_err());
        settings.add_position_hotkey = Hotkey::from_keys([0x79]);
        settings.destination_mode = DestinationMode::PositionSequence;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn rejects_ambiguous_subset_chords() {
        let settings = Settings {
            add_position_hotkey: Hotkey::from_keys([0x11, 0x75]),
            ..Settings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn recognizes_when_scheduler_cannot_keep_up() {
        let mut settings = Settings {
            interval_ms: 0.5,
            press_duration_ms: 0.0,
            ..Settings::default()
        };
        assert!(settings.scheduler_warning().is_some());
        settings.interval_ms = 50.0;
        settings.press_duration_ms = 100.0;
        assert!(settings.scheduler_warning().is_some());
    }
}
