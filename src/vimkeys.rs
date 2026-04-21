use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::panels::Mode;

#[derive(Debug, Clone, PartialEq)]
pub enum Motion {
    Up(usize),
    Down(usize),
    Top,
    Bottom,
    PageUp,
    PageDown,
    Left,
    Right,
    NextWord,
    PrevWord,
    LineStart,
    LineEnd,
    Search(String),
    NextMatch,
    PrevMatch,
    NoOp,
}

pub fn parse_key_event(key: KeyEvent, mode: Mode) -> Option<Motion> {
    match mode {
        Mode::Normal | Mode::Visual => parse_normal(key),
        Mode::Command | Mode::Insert => None,
    }
}

fn parse_normal(key: KeyEvent) -> Option<Motion> {
    match key.code {
        KeyCode::Char('h') | KeyCode::Left => Some(Motion::Left),
        KeyCode::Char('j') | KeyCode::Down => Some(Motion::Down(1)),
        KeyCode::Char('k') | KeyCode::Up => Some(Motion::Up(1)),
        KeyCode::Char('l') | KeyCode::Right => Some(Motion::Right),
        KeyCode::Char('g') => {
            // gg handled separately via pending; here just Top for single g if we get it
            Some(Motion::Top)
        }
        KeyCode::Char('G') => Some(Motion::Bottom),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Motion::PageUp)
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Motion::PageDown)
        }
        KeyCode::Char('0') => Some(Motion::LineStart),
        KeyCode::Char('w') => Some(Motion::NextWord),
        KeyCode::Char('b') => Some(Motion::PrevWord),
        KeyCode::Char('n') => Some(Motion::NextMatch),
        KeyCode::Char('N') => Some(Motion::PrevMatch),
        KeyCode::Char('$') => Some(Motion::LineEnd),
        _ => None,
    }
}

/// Parse a key with an optional count prefix. Returns the motion and remaining key if count consumed part.
/// For MVP we handle single-digit counts directly in panels.
pub fn parse_with_count(key: KeyEvent, _count: Option<usize>) -> Option<Motion> {
    parse_normal(key)
}
