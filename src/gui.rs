use chrono::Utc;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen},
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Cell, Row, Table, TableState},
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

static SHOULD_WE_SAVE: AtomicBool = AtomicBool::new(false);

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
    println!("leng {}", papers.len());

    let mut still_running = true;
    //while still_running {
    terminal.draw(|f| {
        // Create a layout into which to place our blocks.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
            .split(f.size());

        let block = Block::default()
            // With a given title...
            .title("Color Changer")
            // Borders on every side...
            .borders(Borders::ALL)
            // The background of the current color...
            .style(Style::default().bg(Color::Red));
        f.render_widget(block, chunks[0]);

        let items = papers
            .iter()
            .map(|p| Row::new(vec![p.title.clone(), p.author_string(), "tttt".to_string()]))
            .collect::<Vec<Row>>();
        let table = Table::new(vec![
            Row::new(vec!["aaa", "bbb", "ccc"]).style(Style::default().fg(Color::Green)),
            Row::new(vec!["aaa", "bb", "cc"]).style(Style::default()),
        ])
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
        //let block = Block::default().title("Block").borders(Borders::ALL);
        //f.render_widget(block, size);
    })?;
    //}
    thread::sleep(Duration::from_secs(5));

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    return Ok(vec![]);

    let mut not_finished = true;
    while not_finished {
        terminal.draw(|f| {})?;
    }

    Ok(papers)
}
