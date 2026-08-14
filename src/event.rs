use crossterm::event::Event as TermEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStream {
    Stdout,
    Stderr,
}

#[derive(Debug)]
pub enum AppEvent {
    Input(TermEvent),
    Tick,
    JobLine {
        serial: String,
        stream: LineStream,
        text: String,
    },
}
