//! Key specification: a parseable representation of a crossterm key
//! (KeyCode + KeyModifiers), used both for default keymap tables and for
//! user TOML configuration.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

/// A key press specification, e.g. `j`, `G`, `Ctrl+d`, `Space`, `Enter`.
///
/// Parsing rules:
/// - Single printable character: `j`, `G`, `?`, `:` (case sensitive)
/// - Named keys: `Space`, `Enter`, `Esc`, `Tab`, `Backspace`, `Left`,
///   `Right`, `Up`, `Down`, `PageUp`, `PageDown`, `Home`, `End`, `Delete`
/// - Modifiers joined with `+`: `Ctrl+d`, `Shift+G`, `Ctrl+Shift+Q`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeySpec {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeySpec {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Simple-char constructor: `KeySpec::char('j')`.
    pub fn char(c: char) -> Self {
        Self::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    /// Parse from a TOML key string like `"j"`, `"Ctrl+d"`, `"Space"`.
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("empty key spec".to_string());
        }

        let mut modifiers = KeyModifiers::NONE;
        let mut parts = s.split('+').map(str::trim).peekable();
        let last = parts.clone().last().unwrap_or("").to_ascii_lowercase();

        // Split off modifier prefixes; the final part is the key itself.
        let key_part = loop {
            let Some(part) = parts.next() else {
                return Err(format!("no key in key spec: {s:?}"));
            };
            if parts.peek().is_none() {
                break part;
            }
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                "alt" | "opt" | "option" => modifiers |= KeyModifiers::ALT,
                "super" | "cmd" | "meta" => modifiers |= KeyModifiers::SUPER,
                other => return Err(format!("unknown modifier {other:?} in key spec: {s:?}")),
            }
        };

        // Named keys (case-insensitive)
        let named = match last.as_str() {
            "space" => Some(KeyCode::Char(' ')),
            "enter" | "return" => Some(KeyCode::Enter),
            "esc" | "escape" => Some(KeyCode::Esc),
            "tab" => Some(KeyCode::Tab),
            "backspace" => Some(KeyCode::Backspace),
            "left" => Some(KeyCode::Left),
            "right" => Some(KeyCode::Right),
            "up" => Some(KeyCode::Up),
            "down" => Some(KeyCode::Down),
            "pageup" => Some(KeyCode::PageUp),
            "pagedown" => Some(KeyCode::PageDown),
            "home" => Some(KeyCode::Home),
            "end" => Some(KeyCode::End),
            "delete" => Some(KeyCode::Delete),
            _ => None,
        };
        if let Some(code) = named {
            return Ok(Self::new(code, modifiers));
        }
        // Character keys: exactly one character, matched exactly
        // (case sensitive: "G" != "g"). If a Shift modifier was given,
        // uppercase the char.
        let mut chars = key_part.chars();
        let c = match (chars.next(), chars.next()) {
            (Some(c), None) => c,
            _ => return Err(format!("invalid key {key_part:?} in key spec: {s:?}")),
        };
        let c = if modifiers.contains(KeyModifiers::SHIFT) {
            c.to_ascii_uppercase()
        } else {
            c
        };
        // Canonical form: uppercase chars carry SHIFT, lowercase do not.
        let modifiers = if c.is_ascii_uppercase() {
            modifiers | KeyModifiers::SHIFT
        } else {
            modifiers & !KeyModifiers::SHIFT
        };
        Ok(Self::new(KeyCode::Char(c), modifiers))
    }

    /// Convert to a canonical display string (round-trips through `parse`).
    pub fn to_spec_string(self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("Ctrl");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt");
        }
        if self.modifiers.contains(KeyModifiers::SUPER) {
            parts.push("Super");
        }
        // SHIFT is implied by uppercase chars; only list it for named keys.
        let is_char = matches!(self.code, KeyCode::Char(_));
        if self.modifiers.contains(KeyModifiers::SHIFT) && !is_char {
            parts.push("Shift");
        }
        let key_str: String;
        match self.code {
            KeyCode::Char(' ') => parts.push("Space"),
            KeyCode::Char(c) => {
                key_str = c.to_string();
                parts.push(key_str.as_str());
            }
            KeyCode::Enter => parts.push("Enter"),
            KeyCode::Esc => parts.push("Esc"),
            KeyCode::Tab => parts.push("Tab"),
            KeyCode::Backspace => parts.push("Backspace"),
            KeyCode::Left => parts.push("Left"),
            KeyCode::Right => parts.push("Right"),
            KeyCode::Up => parts.push("Up"),
            KeyCode::Down => parts.push("Down"),
            KeyCode::PageUp => parts.push("PageUp"),
            KeyCode::PageDown => parts.push("PageDown"),
            KeyCode::Home => parts.push("Home"),
            KeyCode::End => parts.push("End"),
            KeyCode::Delete => parts.push("Delete"),
            _ => parts.push("Unknown"),
        }
        parts.join("+")
    }

    /// True if this key spec matches a crossterm KeyEvent.
    pub fn matches(&self, key: &KeyEvent) -> bool {
        self.code == key.code && self.modifiers == key.modifiers
    }
}

impl std::fmt::Display for KeySpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_spec_string())
    }
}

/// Serde helper so TOML maps keyed by `"Ctrl+d"` etc. parse into `KeySpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySpecDe(pub KeySpec);

impl<'de> Deserialize<'de> for KeySpecDe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        KeySpec::parse(&s)
            .map(KeySpecDe)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn parse_plain_char() {
        let k = KeySpec::parse("j").unwrap();
        assert_eq!(k, KeySpec::char('j'));
        assert!(k.matches(&key(KeyCode::Char('j'), KeyModifiers::NONE)));
        assert!(!k.matches(&key(KeyCode::Char('k'), KeyModifiers::NONE)));
    }

    #[test]
    fn parse_ctrl() {
        let k = KeySpec::parse("Ctrl+d").unwrap();
        assert_eq!(k.code, KeyCode::Char('d'));
        assert_eq!(k.modifiers, KeyModifiers::CONTROL);
        assert!(k.matches(&key(KeyCode::Char('d'), KeyModifiers::CONTROL)));
    }

    #[test]
    fn parse_named_keys() {
        assert_eq!(
            KeySpec::parse("Space").unwrap(),
            KeySpec::new(KeyCode::Char(' '), KeyModifiers::NONE)
        );
        assert_eq!(
            KeySpec::parse("Enter").unwrap(),
            KeySpec::new(KeyCode::Enter, KeyModifiers::NONE)
        );
        assert_eq!(
            KeySpec::parse("Esc").unwrap(),
            KeySpec::new(KeyCode::Esc, KeyModifiers::NONE)
        );
        assert_eq!(
            KeySpec::parse("PageDown").unwrap(),
            KeySpec::new(KeyCode::PageDown, KeyModifiers::NONE)
        );
    }

    #[test]
    fn parse_shift_uppercase() {
        // "G" is an uppercase char: SHIFT is implied.
        let g = KeySpec::parse("G").unwrap();
        assert_eq!(g.code, KeyCode::Char('G'));
        assert_eq!(g.modifiers, KeyModifiers::SHIFT);

        // "Shift+G" is equivalent.
        assert_eq!(KeySpec::parse("Shift+G").unwrap(), g);

        // Both match a real crossterm event for capital G.
        let ev = key(KeyCode::Char('G'), KeyModifiers::SHIFT);
        assert!(g.matches(&ev));
    }

    #[test]
    fn parse_errors() {
        assert!(KeySpec::parse("").is_err());
        assert!(KeySpec::parse("Ctrl+").is_err());
        assert!(KeySpec::parse("Bogus+X").is_err());
        assert!(KeySpec::parse("abc").is_err());
    }

    #[test]
    fn roundtrip_display() {
        for s in ["j", "G", "Ctrl+d", "Space", "Enter", "Esc", "Alt+x"] {
            let k = KeySpec::parse(s).unwrap();
            assert_eq!(k.to_spec_string(), s, "roundtrip failed for {s:?}");
        }
    }
}
