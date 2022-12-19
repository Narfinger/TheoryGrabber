use chrono::Utc;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen},
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Cell, List, ListItem, Paragraph, Row, Table, TableState},
    Terminal,
};

use crate::types::Paper;
use std::{
    io,
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
};
use std::{thread, time::Duration};
use tui::widgets::{Block, Borders, Widget};

pub(crate) fn get_selected_papers(papers: Vec<Paper>) -> Result<Vec<Paper>, io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut papers = papers;
    papers.push(Paper {
        title: "Test".to_string(),
        description: "Desc Test".to_string(),
        link: reqwest::Url::from_str("http://example.com").unwrap(),
        source: crate::types::Source::ECCC,
        authors: vec![],
        published: Utc::now(),
    });
    let mut state = TableState::default();
    let mut run = true;
    let mut should_we_save = false;
    while run {
        terminal.draw(|f| {
            // Create a layout into which to place our blocks.
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
                .split(f.size());

            let help = List::new(vec![
                ListItem::new("Use Up and Down to look at paper"),
                ListItem::new("Use d or Delete, to remove them from download"),
                ListItem::new("Press Enter to Download"),
            ]);
            f.render_widget(help, chunks[0]);

            let items = papers
                .iter()
                .map(|p| Row::new(vec![p.title.clone(), p.author_string(), "tttt".to_string()]))
                .collect::<Vec<Row>>();
            let table = Table::new(items)
                //.style(Style::default().fg(Color::Blue))
                .block(Block::default().title("Papers").borders(Borders::ALL))
                .header(Row::new(vec!["Col1", "Col2", "Col3"]).style(Style::default()))
                .widths(&[
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(10),
                ])
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol(">>");

            f.render_stateful_widget(table, chunks[1], &mut state);

            if let Event::Key(key) = event::read().unwrap() {
                match key.code {
                    KeyCode::Char('q') => {
                        run = false;
                        should_we_save = false;
                    }
                    KeyCode::Enter => {
                        run = false;
                        should_we_save = true;
                    }
                    KeyCode::Down => {
                        let selected = state.selected().unwrap_or(0);
                        let new_selected = if selected + 1 >= papers.len() {
                            selected
                        } else {
                            selected + 1
                        };

                        state.select(Some(new_selected));
                    }
                    KeyCode::Up => {
                        let selected = state.selected().unwrap_or(0);
                        let new_selected = selected.checked_sub(1).unwrap_or(0);
                        state.select(Some(new_selected));
                    }
                    KeyCode::Delete | KeyCode::Char('D') => {
                        if let Some(i) = state.selected() {
                            papers.remove(i);
                        }
                    }
                    _ => {}
                }
            }

            //let block = Block::default().title("Block").borders(Borders::ALL);
            //f.render_widget(block, size);
        })?;
    }
    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if should_we_save {
        Ok(papers)
    } else {
        Ok(vec![])
    }
}
