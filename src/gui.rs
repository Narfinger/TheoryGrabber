use chrono::{DateTime, Utc};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{BorderType, List, ListItem, Paragraph, Row, Table, TableState, Wrap},
    Frame, Terminal,
};

use crate::types::Paper;
use ratatui::widgets::{Block, Borders};
use std::io;
use std::time::Duration;

/// How should we style a paper in a table? Returns a row.
fn render_paper(p: &Paper) -> Row {
    Row::new(vec![
        Span::styled(p.title.clone(), Style::default().fg(Color::Green)),
        Span::styled(p.format_author(), Style::default().fg(Color::Blue)),
        Span::styled(p.source.to_string(), Style::default().fg(Color::DarkGray)),
        Span::styled(
            p.published.to_rfc2822(),
            Style::default().fg(Color::DarkGray),
        ),
    ])
}

/// Renders the right side details page
fn render_details<B: Backend>(
    state: &TableState,
    papers: &[Paper],
    p_abstract_layout: &[Rect],
    f: &mut Frame<B>,
) {
    let selected_paper = state.selected().and_then(|i| papers.get(i));
    if let Some(paper) = selected_paper {
        let details_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(5),
                    Constraint::Percentage(5),
                    Constraint::Percentage(5),
                    Constraint::Percentage(85),
                ]
                .as_ref(),
            )
            .split(p_abstract_layout[1]);

        {
            let display = Text::from(paper.title.clone());
            let p = Paragraph::new(display).wrap(Wrap { trim: false });
            f.render_widget(p, details_layout[0]);
        }
        {
            let display = Text::from(paper.format_author());
            let p = Paragraph::new(display).wrap(Wrap { trim: false });
            f.render_widget(p, details_layout[1]);
        }
        {
            let display = Text::from(paper.source.to_string());
            let p = Paragraph::new(display).wrap(Wrap { trim: false });
            f.render_widget(p, details_layout[2]);
        }

        let p_abstract_text = Text::from(paper.description.clone());

        let p_abstract_para = Paragraph::new(p_abstract_text)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title("Abstract")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double),
            );
        f.render_widget(p_abstract_para, details_layout[3])
    }
}

/// renders the help box
fn render_help<B: Backend>(main_layout: &[Rect], f: &mut Frame<B>, filter_date: DateTime<Utc>) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(main_layout[0]);

    let help = List::new(vec![
        ListItem::new("Use Up (or w) and Down to look at paper"),
        ListItem::new("Use d or Delete, to remove them from download"),
        ListItem::new("Press Enter to Download"),
        ListItem::new("Use Esc or q to quit and not save the current state"),
        ListItem::new("Use x to save the selected date up to current selected item"),
    ]);
    f.render_widget(help, layout[0]);

    let dates = List::new(vec![
        ListItem::new(format!(
            "Filter Date: {}",
            (filter_date - chrono::Duration::days(1)).format("%d-%m-%Y")
        )),
        ListItem::new(format!("Today: {}", Utc::now().format("%d-%m-%Y"))),
    ]);
    f.render_widget(dates, layout[1]);
}

/// Main Render
fn render<B: Backend>(
    papers: &[Paper],
    f: &mut Frame<B>,
    state: &mut TableState,
    filter_date: DateTime<Utc>,
) {
    // Create a layout into which to place our blocks.
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(10), Constraint::Percentage(90)].as_ref())
        .split(f.size());

    render_help(&main_layout, f, filter_date);

    let items = papers.iter().map(render_paper).collect::<Vec<Row>>();
    let table = Table::new(items)
        //.style(Style::default().fg(Color::Blue))
        .block(Block::default().title("Papers").borders(Borders::ALL))
        .header(
            Row::new(vec!["Title", "Authors", "Source", "Date"])
                .style(Style::default().add_modifier(Modifier::UNDERLINED)),
        )
        .widths(&[
            Constraint::Length(60),
            Constraint::Length(20),
            Constraint::Length(15),
            Constraint::Length(20),
        ])
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">>");

    let p_abstract_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
        .split(main_layout[1]);
    f.render_stateful_widget(table, p_abstract_layout[0], state);

    render_details(state, papers, &p_abstract_layout, f);
}

/// Will be used by the input handle to say which way we should save the papers
enum SavingType {
    /// Do not save the papers
    DoNotSave,
    /// Save all remaining papers
    Save,
    /// Save all remaining papers up to index
    SaveNewDate(usize),
}

/// Handles the input
fn input_handle(
    papers: &mut Vec<Paper>,
    run: &mut bool,
    saving_type: &mut SavingType,
    state: &mut TableState,
) {
    if event::poll(Duration::from_millis(100)).unwrap_or(false) {
        if let Event::Key(key) = event::read().unwrap() {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    *run = false;
                    *saving_type = SavingType::DoNotSave;
                }
                KeyCode::Enter => {
                    *run = false;
                    *saving_type = SavingType::Save;
                }
                KeyCode::Char('x') => {
                    *run = false;
                    if let Some(i) = state.selected() {
                        *saving_type = SavingType::SaveNewDate(i);
                    } else {
                        *saving_type = SavingType::DoNotSave;
                    }
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
                KeyCode::Up | KeyCode::Char('w') => {
                    let selected = state.selected().unwrap_or(0);
                    let new_selected = selected.saturating_sub(1);
                    state.select(Some(new_selected));
                }
                KeyCode::Delete | KeyCode::Char('d') => {
                    if let Some(i) = state.selected() {
                        if i < papers.len() {
                            // sometimes we could delete things despite them being already deleted
                            papers.remove(i);
                            let selected = state.selected().unwrap_or(0);
                            let new_selected = selected.saturating_sub(1);
                            state.select(Some(new_selected));
                        } else {
                            state.select(None);
                        }
                    }
                }
                KeyCode::PageUp => {
                    state.select(Some(0));
                }
                KeyCode::PageDown => state.select(Some(papers.len() - 1)),
                _ => {}
            }
        }
    }
}

/// Return type for papers that we selected in the gui
pub(crate) enum SelectedPapers {
    /// There are no new papers
    NoNew,
    /// We have some selected papers
    Selected(Vec<Paper>),
    /// We stopped looking at papers somewhere in the middle and want to save the last date
    SelectedAndSavedAt(Vec<Paper>),
    /// Do not save anything
    Abort,
}

/// Start point for the gui. Runs it in a loop and returns the paper we want to download
/// Return None if we should not save
pub(crate) fn get_selected_papers(
    papers: Vec<Paper>,
    filter_date: DateTime<Utc>,
) -> Result<SelectedPapers, io::Error> {
    if papers.is_empty() {
        Ok(SelectedPapers::NoNew)
    } else {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut papers = papers;
        papers.sort_unstable_by(|a, b| b.cmp(a));
        let mut state = TableState::default();
        state.select(Some(papers.len() - 1));
        let mut run = true;
        let mut saving_type = SavingType::DoNotSave;
        while run {
            terminal.draw(|f| {
                render(&papers, f, &mut state, filter_date);
                input_handle(&mut papers, &mut run, &mut saving_type, &mut state);
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

        match saving_type {
            SavingType::DoNotSave => Ok(SelectedPapers::Abort),
            SavingType::Save => Ok(SelectedPapers::Selected(papers)),
            SavingType::SaveNewDate(i) => {
                let (_, snd) = papers.split_at(i);
                Ok(SelectedPapers::SelectedAndSavedAt(snd.into()))
            }
        }
    }
}
