pub mod app;
pub mod device;
pub mod event;
pub mod jobs;
pub mod tools;
pub mod ui;

use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self as term_event, Event};
use ratatui::DefaultTerminal;

use app::App;
use event::AppEvent;

pub fn run() -> io::Result<()> {
    ratatui::run(run_with)
}

fn run_with(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let (tx, rx) = mpsc::channel::<AppEvent>();
    {
        let tx = tx.clone();
        thread::spawn(move || {
            loop {
                match term_event::poll(Duration::from_millis(200)) {
                    Ok(true) => match term_event::read() {
                        Ok(Event::Resize(_, _)) => {
                            if tx.send(AppEvent::Tick).is_err() {
                                break;
                            }
                        }
                        Ok(ev) => {
                            if tx.send(AppEvent::Input(ev)).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    },
                    Ok(false) => {
                        if tx.send(AppEvent::Tick).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    let mut app = App::new(tx);
    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;
        match rx.recv() {
            Ok(ev) => app.handle(ev),
            Err(_) => break,
        }
        if app.should_quit {
            break;
        }
    }
    Ok(())
}
