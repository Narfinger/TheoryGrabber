use std::any::Any;
use cursive::Printer;
use cursive::direction::Direction;
use cursive::event::{Event, EventResult};
use cursive::view::{Selector};
use cursive::views::{Dialog, DummyView, LinearLayout, TextView};
use cursive_table_view::TableView;
use cursive::vec::Vec2;
use cursive::traits::View;
use crate::types::{BasicColumn, Paper, print_authors};


///This is just taking some keyboard shortcuts and make them something
///if you want to understand what I am doing look at View `on_event` and
///we impl Dialog because we need some setup procedure.

pub struct PaperDialog {
    d: Dialog,
}

impl View for PaperDialog {
    fn draw(&self, printer: &Printer)              { self.d.draw(printer); }
    fn required_size(&mut self, req: Vec2) -> Vec2 { self.d.required_size(req) }
    fn layout(&mut self, size: Vec2)               { self.d.layout(size); }
    fn on_event(&mut self, event: Event) -> EventResult { custom_event_handler(self, event) }
    fn take_focus(&mut self, source: Direction) -> bool { self.d.take_focus(source) }
    fn call_on_any<'a>(&mut self, selector: &Selector, callback: Box<FnMut(&mut Any) + 'a>) { self.d.call_on_any(selector, callback); }
    fn focus_view(&mut self, selector: &Selector) -> Result<(), ()> { self.d.focus_view(selector) }
    fn needs_relayout(&self) -> bool               { self.d.needs_relayout() }
}

fn custom_event_handler(pd: &mut PaperDialog, event:Event) -> EventResult {
    //pd.call_on_id(button delete)
    pd.d.on_event(event)
}

pub fn new(value: &Paper, row: usize, index: usize) -> PaperDialog {
    let nd = LinearLayout::vertical()
        .child(TextView::new(value.title.clone()))
        .child(DummyView)
        .child(TextView::new(print_authors(value)))
        .child(DummyView)
        .child(TextView::new(value.link.clone().to_string()))
        .child(DummyView)
        .child(TextView::new(value.description.clone()));
    let dialog = Dialog::around(nd)
        .title(format!("Details row # {}", row))
        /*.button("Next", move |s| {
            //this is kind of hacky
            s.call_on_id("table", |table: &mut TableView<Paper, BasicColumn>| {
                //this is technically not correct as index+1 is in the unsorted view, while we look in the sorted one
                table.set_selected_row(index+1);
            });
            s.pop_layer();
            //s.on_event(cursive::event::Event::Key(cursive::event::Key::Enter));
        })*/
        .button("Delete", move |s| {
            s.call_on_id("table", |table: &mut TableView<Paper, BasicColumn>| {
                table.remove_item(index);
            });
            s.pop_layer()
        })
        .button("Close", move |s| s.pop_layer());
        
    PaperDialog { d: dialog }
}
