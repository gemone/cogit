use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Visual,
    Command,
    Insert,
}

#[derive(Debug, Clone)]
pub enum Motion {
    Up,
    Down,
    PageUp,
    PageDown,
    Top,
    Bottom,
    Enter,
    Tab,
    Escape,
    Char(char),
    Delete,
    Search,
}

pub fn parse_key_event(key: KeyEvent, mode: Mode) -> Option<Motion> {
    match mode {
        Mode::Normal => parse_normal(key),
        Mode::Command | Mode::Insert => parse_insert(key),
        Mode::Visual => parse_normal(key),
    }
}

fn parse_normal(key: KeyEvent) -> Option<Motion> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Motion::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Motion::Up),
        KeyCode::Char('G') => Some(Motion::Bottom),
        KeyCode::Char('g') => {
            // gg for top - simplified: single g goes to top
            Some(Motion::Top)
        }
        KeyCode::PageUp => Some(Motion::PageUp),
        KeyCode::PageDown => Some(Motion::PageDown),
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Motion::PageDown)
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Motion::PageUp)
        }
        KeyCode::Enter => Some(Motion::Enter),
        KeyCode::Tab => Some(Motion::Tab),
        KeyCode::Esc => Some(Motion::Escape),
        KeyCode::Char(c) => Some(Motion::Char(c)),
        _ => None,
    }
}

fn parse_insert(key: KeyEvent) -> Option<Motion> {
    match key.code {
        KeyCode::Esc => Some(Motion::Escape),
        KeyCode::Enter => Some(Motion::Enter),
        KeyCode::Tab => Some(Motion::Tab),
        KeyCode::Char(c) => Some(Motion::Char(c)),
        KeyCode::Backspace => Some(Motion::Delete),
        _ => None,
    }
}
