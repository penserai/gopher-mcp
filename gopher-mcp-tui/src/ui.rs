use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Mode, Pane};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[0]);

    draw_menu(frame, app, body[0]);
    draw_content(frame, app, body[1]);
    draw_status(frame, app, chunks[1]);

    // GoTo popup overlay
    if app.mode == Mode::GoTo {
        draw_goto_popup(frame, app, frame.area());
    }
}

fn draw_menu(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .items
        .iter()
        .map(|item| {
            let (indicator, color) = match item.item_type.as_str() {
                "1" => ("[+]", Color::Yellow),
                "0" => ("[T]", Color::White),
                "7" => ("[?]", Color::Green),
                "h" => ("[H]", Color::Magenta),
                "i" => ("   ", Color::DarkGray),
                _ => ("[.]", Color::DarkGray),
            };
            let line = Line::from(vec![
                Span::styled(format!("{} ", indicator), Style::default().fg(color)),
                Span::styled(&item.display, Style::default().fg(color)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let border_color = if app.active_pane == Pane::Menu {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let title = if app.current_path.is_empty() {
        " >(•.•)> / ".to_string()
    } else {
        format!(" >(•.•)> {} ", app.current_path)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(title),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    state.select(Some(app.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_content(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.active_pane == Pane::Content {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let text = if app.content.is_empty() {
        concat!(
            "\n",
            "        >(•.•)>\n",
            "\n",
            "      gopher-mcp\n",
            "\n",
            "  Select an item to view\n",
            "  its content.\n",
        )
        .to_string()
    } else {
        app.content.clone()
    };

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(" Content "),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.content_scroll, 0));

    frame.render_widget(paragraph, area);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let status = match app.mode {
        Mode::Search => Line::from(vec![
            Span::styled("Search: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.search_input),
            Span::styled("_", Style::default().fg(Color::Yellow)),
        ]),
        Mode::GoTo | Mode::Normal => {
            let path = if app.current_path.is_empty() {
                "/".to_string()
            } else {
                app.current_path.clone()
            };

            let mut spans = vec![Span::styled(
                format!(" {} ", path),
                Style::default().fg(Color::Cyan),
            )];

            if app.loading {
                spans.push(Span::styled(
                    " loading... ",
                    Style::default().fg(Color::Yellow),
                ));
            }

            if !app.status_message.is_empty() {
                spans.push(Span::styled(
                    format!(" {} ", app.status_message),
                    Style::default().fg(Color::Red),
                ));
            }

            spans.push(Span::styled(
                " q:quit b:back /:search ::goto Tab:pane Enter:open PgUp/Dn:scroll ",
                Style::default().fg(Color::DarkGray),
            ));

            Line::from(spans)
        }
    };

    let bar = Paragraph::new(status).style(Style::default().bg(Color::Black));
    frame.render_widget(bar, area);
}

fn draw_goto_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup_w = (area.width as u32 * 60 / 100).max(30) as u16;
    let popup_h = (area.height as u32 * 60 / 100).max(6) as u16;
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w.min(area.width), popup_h.min(area.height));

    frame.render_widget(Clear, popup_area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(popup_area);

    // Input box
    let input_text = Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Green)),
        Span::raw(&app.search_input),
        Span::styled("_", Style::default().fg(Color::Green)),
    ]);
    let count = app.goto_filtered.len();
    let input_block = Paragraph::new(input_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title(format!(" Go to ({}) Tab:expand ", count)),
    );
    frame.render_widget(input_block, inner[0]);

    // Tree list
    let items: Vec<ListItem> = app
        .goto_filtered
        .iter()
        .map(|&idx| {
            let entry = &app.goto_items[idx];
            let indent = "  ".repeat(entry.depth as usize);
            let (marker, color) = if entry.is_dir {
                if entry.expanded {
                    ("[-] ", Color::Yellow)
                } else {
                    ("[+] ", Color::Yellow)
                }
            } else {
                ("    ", Color::White)
            };
            let line = Line::from(vec![
                Span::raw(indent),
                Span::styled(marker, Style::default().fg(color)),
                Span::styled(entry.display.as_str(), Style::default().fg(color)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_style(Style::default().fg(Color::Green)),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Black)
                .bg(Color::Green),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    if !app.goto_filtered.is_empty() {
        state.select(Some(app.goto_selected));
    }
    frame.render_stateful_widget(list, inner[1], &mut state);
}
