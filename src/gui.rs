use cursive::traits::*;
use cursive::views::Dialog;
use cursive::Cursive;
use cursive_table_view::{TableView, TableViewItem};
//use errors::*;
use crate::paper_dialog;
use crate::types::{BasicColumn, Paper};
use std::sync::atomic::{AtomicBool, Ordering};

static SHOULD_WE_SAVE: AtomicBool = AtomicBool::new(false);

impl TableViewItem<BasicColumn> for Paper {
    fn to_column(&self, column: BasicColumn) -> String {
        match column {
            BasicColumn::Title => {
                let mut res = self.title.replace("\n", "");
                res.truncate(120);
                res
            }
            BasicColumn::Source => format!("{}", self.source),
            BasicColumn::Published => format!("{}", self.published),
        }
    }

    fn cmp(&self, other: &Self, column: BasicColumn) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            BasicColumn::Title => self.title.cmp(&other.title),
            BasicColumn::Source => self.source.cmp(&other.source),
            BasicColumn::Published => self.published.cmp(&other.published),
        }
    }
}

fn table_on_submit(siv: &mut Cursive, row: usize, index: usize) {
    let value: Paper = siv
        .call_on_name("table", move |table: &mut TableView<Paper, BasicColumn>| {
            table.borrow_item(index).unwrap().clone()
        })
        .unwrap();
    siv.add_layer(paper_dialog::new(&value, row, index));
}

/// Clears the table and quits the gui.
fn button_quit(siv: &mut Cursive) {
    siv.call_on_name("table", move |table: &mut TableView<Paper, BasicColumn>| {
        table.clear();
    });
    siv.clear();
    siv.quit();
}

/// Set that we should save the date and quits the gui.
fn button_download_all(siv: &mut Cursive) {
    SHOULD_WE_SAVE.store(true, Ordering::Relaxed); //I have no idea if this is the correct ordering
    siv.clear();
    siv.quit();
}

/// Returns papers that are where selected for download. We return None if we do not want to save the date.
pub fn get_selected_papers(papers: Vec<Paper>) -> Option<Vec<Paper>> {
    let val = {
        let mut siv = Cursive::default();
        let mut table = TableView::<Paper, BasicColumn>::new()
            .column(BasicColumn::Title, "Title", |c| {
                c.ordering(std::cmp::Ordering::Greater).width_percent(75)
            })
            .column(BasicColumn::Source, "Source", |c| {
                c.ordering(std::cmp::Ordering::Greater)
            })
            .column(BasicColumn::Published, "Published", |c| {
                c.ordering(std::cmp::Ordering::Greater)
            })
            .default_column(BasicColumn::Published);

        table.set_items(papers);
        let length = table.len();
        table.set_selected_row(length - 1);
        table.set_on_submit(table_on_submit);

        siv.add_layer(
            Dialog::around(table.with_name("table").min_size((500, 80)))
                .title("Table View")
                .button("Download all and save", button_download_all)
                .button("Quit", button_quit),
        );

        siv.run();

        siv.call_on_name("table", |table: &mut TableView<Paper, BasicColumn>| {
            table.take_items()
        })
        .unwrap()
    };

    if SHOULD_WE_SAVE.load(Ordering::Relaxed) {
        Some(val)
    } else {
        None
    }
}
