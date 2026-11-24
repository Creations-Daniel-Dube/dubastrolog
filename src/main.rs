#![cfg_attr(windows, windows_subsystem = "windows")]

mod attachments;
mod model;
mod query;
mod store;
mod ui;

fn main() -> eframe::Result {
    ui::run()
}
