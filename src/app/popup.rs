use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear},
};

pub(crate) fn popup_style() -> Style {
    Style::default().bg(Color::Black).fg(Color::White)
}

pub(crate) fn centered_popup_area(
    area: Rect,
    width_numerator: u16,
    width_denominator: u16,
    min_width: u16,
    height_numerator: u16,
    height_denominator: u16,
    min_height: u16,
) -> Rect {
    let popup_w = (area.width.saturating_mul(width_numerator) / width_denominator).max(min_width);
    let popup_h =
        (area.height.saturating_mul(height_numerator) / height_denominator).max(min_height);
    let popup_x = (area.width.saturating_sub(popup_w)) / 2;
    let popup_y = (area.height.saturating_sub(popup_h)) / 2;
    Rect::new(popup_x, popup_y, popup_w, popup_h)
}

pub(crate) fn popup_inner_area(popup_area: Rect) -> Rect {
    Rect {
        x: popup_area.x + 1,
        y: popup_area.y + 1,
        width: popup_area.width.saturating_sub(2),
        height: popup_area.height.saturating_sub(3),
    }
}

pub(crate) fn popup_inner_area_without_footer(popup_area: Rect) -> Rect {
    Rect {
        x: popup_area.x + 1,
        y: popup_area.y + 1,
        width: popup_area.width.saturating_sub(2),
        height: popup_area.height.saturating_sub(2),
    }
}

pub(crate) fn render_popup_background(f: &mut Frame, popup_area: Rect) {
    f.render_widget(Clear, popup_area);
    f.render_widget(Block::default().style(popup_style()), popup_area);
}

pub(crate) fn popup_block<'a>(title: &'a str, border_style: Style) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style)
        .style(popup_style())
}
