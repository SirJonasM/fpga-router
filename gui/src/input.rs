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
pub enum Command {
    Next(usize),
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
