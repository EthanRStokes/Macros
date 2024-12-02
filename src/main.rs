// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

//! Application API example

mod macros;
mod util;
mod app;

use cosmic::app::Settings;
use cosmic::iced_core::Size;
use std::thread::JoinHandle;
use app::{App, Page};

/// Runs application with these settings
#[rustfmt::skip]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let _ = tracing_log::LogTracer::init();

    let input = vec![
        (Page::Page1, "🌟 Create and manage macros.".into()),
    ];

    let settings = Settings::default()
        .size(Size::new(1024., 768.));

    cosmic::app::run::<App>(settings, input)?;

    Ok(())
}

struct ThreadPool {
    workers: Vec<JoinHandle<()>>,
}

impl ThreadPool {
    fn new() -> Self {
        ThreadPool { workers: Vec::new() }
    }

    fn add_worker(&mut self, worker: JoinHandle<()>) {
        self.workers.push(worker);
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in self.workers.drain(..) {
            worker.join().expect("TODO: panic message");
        }
    }
}

