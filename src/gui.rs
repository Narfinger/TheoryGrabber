use cursive;
use cursive::Cursive;
use cursive::views::{Dialog, LinearLayout, TextView, DummyView};
use cursive_table_view::{TableView, TableViewItem};
use cursive::traits::*;
//use errors::*;
use std;
use types::{Paper, print_authors};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum BasicColumn {
    Title,
    Source,
    Published,
}

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

pub fn get_selected_papers(papers: Vec<Paper>) -> Vec<Paper> {
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
    table.set_on_submit(|siv: &mut Cursive, row: usize, index: usize| {
        
        let value: Paper =
            siv.call_on_id("table", move |table: &mut TableView<Paper, BasicColumn>| {
                table.borrow_item(index).unwrap().clone()
            }).unwrap();

        let d = LinearLayout::vertical()
            .child(TextView::new(value.title.clone()))
            .child(DummyView)
            .child(TextView::new(print_authors(&value)))
            .child(DummyView)
            .child(TextView::new(value.link.clone().to_string()))
            .child(DummyView)
            .child(TextView::new(value.description.clone()));

        siv.add_layer(
            Dialog::around(d)
                .title(format!("Details row # {}", row))
                .button("Next", move |s| {
                    //this is kind of hacky
                    s.call_on_id("table", |table: &mut TableView<Paper, BasicColumn>| {
                        //this is technically not correct as index+1 is in the unsorted view, while we look in the sorted one
                        table.set_selected_row(index+1);
                    });
                    s.pop_layer();
                    //s.on_event(cursive::event::Event::Key(cursive::event::Key::Enter));
                })
                .button("Delete", move |s| {
                    s.call_on_id("table", |table: &mut TableView<Paper, BasicColumn>| {
                        table.remove_item(index);
                    });
                    s.pop_layer()
                })
                .button("Close", move |s| s.pop_layer()),
        );
    });

    siv.add_layer(
        Dialog::around(table.with_id("table").min_size((500, 80)))
            .title("Table View")
            .button("Download all", move |s| s.quit())
            .button("Quit", move |s| {
                s.call_on_id("table", move |table: &mut TableView<Paper, BasicColumn>| {
                    table.clear();
                });
                s.quit();
            }),
    );

    //pb.finish_and_clear();
    
    siv.run();
    siv.call_on_id("table", |table: &mut TableView<Paper, BasicColumn>| {
        table.take_items()
    }).unwrap()
}
