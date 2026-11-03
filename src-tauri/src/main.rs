#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod data;
mod di;
mod domain;
mod infrastructure;
mod presentation;

fn main() {
    di::run();
}
