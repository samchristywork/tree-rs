use std::io;
use std::io::Read;

use crate::Direction;
use crate::Event;
use crate::Navigation;

const BACKSPACE: u8 = 0x08;
const DEL: u8 = 0x7f;
const CTRL_U: u8 = 0x15;
const CTRL_D: u8 = 0x04;
const ENTER: u8 = b'\r';
const ESCAPE: u8 = 0x1b;

fn handle_nav_key(char_value: char, event: Navigation) -> Event {
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
                0x43 => Event::Direction(Direction::Right),
                0x44 => Event::Direction(Direction::Left),
                0x35 => handle_nav_key(char_value, Navigation::PageUp),
                0x36 => handle_nav_key(char_value, Navigation::PageDown),
                0x31 => handle_nav_key(char_value, Navigation::Home),
                0x34 => handle_nav_key(char_value, Navigation::End),
                _ => Event::Key(char_value),
            }
        }
        Err(_) => Event::Key(char_value),
    }
}

fn get_input_chars() -> Event {
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

pub fn handle_input(
    pattern: &mut String,
    cursor_pos: &mut usize,
    scroll: &mut usize,
) -> Option<String> {
    match get_input_chars() {
        Event::Key(c) => {
            if *cursor_pos < pattern.len() {
                pattern.insert(*cursor_pos, c);
            } else {
                pattern.push(c);
            }
            *cursor_pos += 1;
        }
        Event::Direction(d) => {
            match d {
                Direction::Left => {
                    *cursor_pos = cursor_pos.saturating_sub(1);
                }
                Direction::Right => {
                    *cursor_pos += 1;
                    if *cursor_pos > pattern.len() {
                        *cursor_pos = pattern.len();
                    }
                }
            };
        }
        Event::Navigation(n) => {
            match n {
                Navigation::PageUp => {
                    *scroll = scroll.saturating_sub(5);
                }
                Navigation::PageDown => {
                    *scroll += 5;
                }
                Navigation::Home => {
                    *cursor_pos = 0;
                }
                Navigation::End => {
                    *cursor_pos = pattern.len();
                }
            };
        }
        Event::Backspace => {
            let one_before = cursor_pos.saturating_sub(1);
            if one_before < pattern.len() {
                pattern.remove(one_before);
            }
            *cursor_pos = cursor_pos.saturating_sub(1);
        }
        Event::Clear => {
            pattern.clear();
            *cursor_pos = 0;
        }
        Event::Enter => {
            return Some(pattern.clone());
        }
        Event::Exit => {
            return Some(String::new());
        }
    }

    None
}
