use std::io;
use std::io::Read;

use crate::Event;
use crate::Navigation;
use crate::Direction;

const BACKSPACE: u8 = 0x08;
const DEL: u8 = 0x7f;
const CTRL_U: u8 = 0x15;
const CTRL_D: u8 = 0x04;
const ENTER: u8 = b'\r';
const ESCAPE: u8 = 0x1b;

fn consume_7e(char_value: char, event: Navigation) -> Event {
    let mut buffer = [0; 1];
    match io::stdin().read_exact(&mut buffer) {
        Ok(()) => {
            let char_value = buffer[0] as char;
            match char_value as u8 {
                0x7e => Event::Navigation(event),
                _ => Event::Key(char_value),
            }
        }
        _ => Event::Key(char_value),
    }
}

fn handle_control_keys(char_value: char) -> Event {
    let mut buffer = [0; 1];
    match io::stdin().read_exact(&mut buffer) {
        Ok(()) => {
            let char_value = buffer[0] as char;
            match char_value as u8 {
                0x5b => {}
                _ => return Event::Key(char_value),
            }
        }
        Err(_) => return Event::Key(char_value),
    };

    let mut buffer = [0; 1];
    match io::stdin().read_exact(&mut buffer) {
        Ok(()) => {
            let char_value = buffer[0] as char;
            match char_value as u8 {
                0x41 => Event::Direction(Direction::Up),
                0x42 => Event::Direction(Direction::Down),
                0x43 => Event::Direction(Direction::Right),
                0x44 => Event::Direction(Direction::Left),
                0x35 => consume_7e(char_value, Navigation::PageUp),
                0x36 => consume_7e(char_value, Navigation::PageDown),
                0x31 => consume_7e(char_value, Navigation::Home),
                0x34 => consume_7e(char_value, Navigation::End),
                _ => Event::Key(char_value),
            }
        }
        Err(_) => Event::Key(char_value),
    }
}

pub fn get_input() -> Event {
    let mut buffer = [0; 1];
    match io::stdin().read_exact(&mut buffer) {
        Ok(()) => {
            let char_value = buffer[0] as char;
            match char_value as u8 {
                BACKSPACE | DEL => Event::Backspace,
                CTRL_U => Event::Clear,
                CTRL_D => Event::Exit,
                ENTER => Event::Enter,
                ESCAPE => handle_control_keys(char_value),
                _ => Event::Key(char_value),
            }
        }
        Err(_) => Event::Exit,
    }
}
