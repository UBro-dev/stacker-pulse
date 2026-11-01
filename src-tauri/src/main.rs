//! Author: UBro-dev | License: MIT (Copyright (c) 2026 UBro-dev)
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    stacker_pulse_lib::run();
}
