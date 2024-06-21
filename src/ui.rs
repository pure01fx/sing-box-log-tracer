use ratatui::{prelude::*, widgets::*};

use crate::app::App;

pub fn ui(f: &mut Frame, app: &App) {
    let area = f.size();
    f.render_widget(
        Paragraph::new(format!(
            "Press j or k to increment or decrement.\n\nCounter: {}",
            1,
        ))
        .block(
            Block::default()
                .title("ratatui async counter app")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center),
        area,
    );
}
