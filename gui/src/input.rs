use std::{collections::VecDeque, str::SplitWhitespace};

use winit::keyboard::{NamedKey, SmolStr};

#[derive(Default)]
pub struct InputHandler {
    pub buffer: String,
    pub state: InputHandlerState,
    pub command_queue: VecDeque<Command>,
}

#[derive(Default, Eq, PartialEq)]
pub enum InputHandlerState {
    Command,
    #[default]
    Idle,
}

#[derive(Debug)]
pub enum Goto {
    Tile {
        x: usize,
        y: usize,
    },
    Lut {
        x: usize,
        y: usize,
        bel: char,
    },
    Edge {
        x1: usize,
        y1: usize,
        label1: String,
        x2: usize,
        y2: usize,
        label2: String,
    },
    Node {
        x: usize,
        y: usize,
        label: String,
    },
}
#[derive(Debug)]
pub enum Command {
    Next(usize),
    Goto(Goto),
    LoadBel(String),
    LoadPips(String),
    LoadGraph,
    Pause,
}
impl Command {
    pub fn parse_command(command: &str) -> Option<Self> {
        println!("parsing command: {command}");
        let mut full_command = command.split_whitespace();
        if let Some(m) = full_command.next() {
            return match m {
                "next" => Self::parse_next(full_command),
                "load-graph" => Some(Self::LoadGraph),
                "pause" => Some(Self::Pause),
                "load-bel" => Self::parse_load_bel(full_command),
                "load-pips" => Self::parse_load_pips(full_command),
                "goto" => Self::parse_goto(full_command),
                _ => None,
            };
        }
        None
    }
    pub fn parse_next(mut arguments: SplitWhitespace) -> Option<Self> {
        arguments
            .next()
            .and_then(|amount| amount.parse::<usize>().ok())
            .map(Self::Next)
    }
    pub fn parse_load_bel(mut arguments: SplitWhitespace) -> Option<Self> {
        arguments.next().map(|file| Self::LoadBel(file.to_string()))
    }
    pub fn parse_load_pips(mut arguments: SplitWhitespace) -> Option<Self> {
        arguments.next().map(|file| Self::LoadPips(file.to_string()))
    }
    pub fn parse_goto(mut arguments: SplitWhitespace) -> Option<Self> {
        let parse_int = |x: Option<&str>| -> Option<usize> { x?.parse::<usize>().ok() };
        match arguments.next() {
            Some("tile") => {
                let x = parse_int(arguments.next())?;
                let y = parse_int(arguments.next())?;
                Some(Command::Goto(Goto::Tile { x, y }))
            }
            Some("lut") => {
                let x = parse_int(arguments.next())?;
                let y = parse_int(arguments.next())?;
                let bel = arguments.next()?;
                let bel = if bel.len() == 1 {
                    bel.chars().next().unwrap()
                } else {
                    return None;
                };
                Some(Command::Goto(Goto::Lut { x, y, bel }))
            }
            Some("node") => {
                let x = parse_int(arguments.next())?;
                let y = parse_int(arguments.next())?;
                let id = arguments.next()?;
                let id = id.to_string();
                Some(Command::Goto(Goto::Node { x, y, label: id }))
            }
            Some("edge") => {
                let x1 = parse_int(arguments.next())?;
                let y1 = parse_int(arguments.next())?;
                let id1 = arguments.next()?;
                let id1 = id1.to_string();
                let (x2, y2, id2) = {
                    let next = arguments.next()?;
                    println!("Next: {next:?}");
                    if let Ok(x2) = next.parse::<usize>() {
                        let y2 = parse_int(arguments.next())?;
                        let id2 = arguments.next()?;
                        let id2 = id2.to_string();
                        (x2, y2, id2)
                    } else {
                        (x1, y1, next.to_string())
                    }
                };

                Some(Command::Goto(Goto::Edge {
                    x1,
                    y1,
                    label1: id1,
                    x2,
                    y2,
                    label2: id2,
                }))
            }
            _ => None,
        }
    }
}

impl InputHandler {
    pub fn pop_command(&mut self) -> Option<Command> {
        self.command_queue.pop_front()
    }
    pub fn handle_named_key(&mut self, key: NamedKey) {
        match key {
            NamedKey::Escape if self.state == InputHandlerState::Command => {
                self.buffer.clear();
                self.state = InputHandlerState::Idle;
            }
            NamedKey::Space => self.handle_char(SmolStr::new(" ")),
            NamedKey::Backspace => {
                self.buffer.pop();
            }
            NamedKey::Enter if self.state == InputHandlerState::Command => {
                if let Some(command) = Command::parse_command(&self.buffer) {
                    self.command_queue.push_back(command);
                }
                self.state = InputHandlerState::Idle;
                self.buffer.clear();
            }
            _ => {}
        }
    }
    pub fn handle_char(&mut self, input: SmolStr) {
        match self.state {
            InputHandlerState::Command => {
                self.buffer += &input;
            }
            InputHandlerState::Idle => {
                if input == ":" {
                    self.state = InputHandlerState::Command;
                }
            }
        }
    }
}
