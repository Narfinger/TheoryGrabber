use cursive::Cursive;
use cursive::views::Dialog;
use cursive_table_view::{TableView, TableViewItem};
use cursive::traits::*;
//use errors::*;
use std;
use std::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};
use paper_dialog;
use types::{BasicColumn, Paper};

static SHOULD_WE_SAVE: AtomicBool = ATOMIC_BOOL_INIT;


impl TableViewItem<BasicColumn> for Paper {
    fn to_column(&self, column: BasicColumn) -> String {
        match column {
            BasicColumn::Title => {
                let mut res = self.title.replace("\n", "").to_owned();
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
    let value: Paper =
        siv.call_on_id("table", move |table: &mut TableView<Paper, BasicColumn>| {
            table.borrow_item(index).unwrap().clone()
        }).unwrap();
    siv.add_layer(paper_dialog::new(&value, row, index));
}

fn button_quit(siv: &mut Cursive) {
    siv.call_on_id("table", move |table: &mut TableView<Paper, BasicColumn>| {
        table.clear();
    });
    siv.quit();
}

fn button_download_all(siv: &mut Cursive) {
    SHOULD_WE_SAVE.store(true, Ordering::Relaxed); //I have no idea if this is the correct ordering
    siv.quit();
}

/// return None if we do not want to save the date
pub fn get_selected_papers(papers: Vec<Paper>) -> Option<Vec<Paper>> {
    let mut siv = Cursive::new();
    let mut table = TableView::<Paper, BasicColumn>::new()
        .column(BasicColumn::Title, "Title", |c| {
            c.ordering(std::cmp::Ordering::Greater).width_percent(75)
        })
        .column(BasicColumn::Source, "Source", |c| {
            c.ordering(std::cmp::Ordering::Greater)
        })
        .column(BasicColumn::Published, "Published", |c| {
            c.ordering(std::cmp::Ordering::Greater)
        }).default_column(BasicColumn::Published);

    table.set_items(papers);
    table.set_on_submit(table_on_submit);

    siv.add_layer(
        Dialog::around(table.with_id("table").min_size((500, 80)))
            .title("Table View")
            .button("Download all and save", button_download_all)
            .button("Quit", button_quit)
    );

    //pb.finish_and_clear();
    
    siv.run();

    //returning elements that are left in the table
    let val = siv.call_on_id("table", |table: &mut TableView<Paper, BasicColumn>| {
        table.take_items()
    }).unwrap();

    if SHOULD_WE_SAVE.load(Ordering::Relaxed) {
        Some(val)
    } else {
        None
    }
}
