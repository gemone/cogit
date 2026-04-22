use crossterm::event::KeyCode;
use ratatui::widgets::ListState;

/// Handle common list navigation keys (j/k/G/g/PageUp/PageDown).
/// Returns true if the key was handled, false otherwise.
pub fn handle_list_navigation(
    state: &mut ListState,
    filtered_len: usize,
    key: KeyCode,
) -> bool {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if filtered_len > 0 {
                let i = state.selected().unwrap_or(0);
                state.select(Some((i + 1).min(filtered_len - 1)));
            }
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let i = state.selected().unwrap_or(0);
            state.select(Some(i.saturating_sub(1)));
            true
        }
        KeyCode::Char('G') => {
            if filtered_len > 0 {
                state.select(Some(filtered_len - 1));
            }
            true
        }
        KeyCode::Char('g') => {
            state.select(Some(0));
            true
        }
        KeyCode::PageDown | KeyCode::Char('J') => {
            if filtered_len > 0 {
                let i = state.selected().unwrap_or(0);
                state.select(Some((i + 15).min(filtered_len - 1)));
            }
            true
        }
        KeyCode::PageUp | KeyCode::Char('K') => {
            let i = state.selected().unwrap_or(0);
            state.select(Some(i.saturating_sub(15)));
            true
        }
        _ => false,
    }
}
