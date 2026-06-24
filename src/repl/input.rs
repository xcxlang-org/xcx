use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{self, ClearType},
    QueueableCommand,
};
use std::io::{stdout, Write};

pub struct InputReader;

impl InputReader {
    pub fn new() -> Self {
        Self
    }

    pub fn read_input(&self) -> Option<String> {
        let mut lines = vec![String::new()];
        let mut row = 0;
        let mut col = 0;

        if terminal::enable_raw_mode().is_err() {
            let mut line = String::new();
            if std::io::stdin().read_line(&mut line).is_err() {
                return None;
            }
            return Some(line);
        }

        let mut out = stdout();
        let prompt = "xcx> ";
        let cont_prompt = "...  ";

        let redraw = |out: &mut std::io::Stdout, lines: &[String], physical_r: usize, new_r: usize, new_c: usize| {
            if physical_r > 0 {
                let _ = out.queue(cursor::MoveUp(physical_r as u16));
            }
            let _ = out.queue(cursor::MoveToColumn(0));
            let _ = out.queue(terminal::Clear(ClearType::FromCursorDown));

            for (i, line) in lines.iter().enumerate() {
                let p = if i == 0 { prompt } else { cont_prompt };
                let _ = out.queue(crossterm::style::Print(p));
                let _ = crate::repl::highlighter::Highlighter::highlight(out, line);
                if i < lines.len() - 1 {
                    let _ = out.queue(crossterm::style::Print("\r\n"));
                }
            }

            let moves_up = (lines.len() - 1).saturating_sub(new_r);
            if moves_up > 0 {
                let _ = out.queue(cursor::MoveUp(moves_up as u16));
            }
            let p_len = if new_r == 0 { prompt.len() } else { cont_prompt.len() };
            let _ = out.queue(cursor::MoveToColumn((p_len + new_c) as u16));
            let _ = out.flush();
        };

        let mut init_draw = true;

        loop {
            if init_draw {
                redraw(&mut out, &lines, row, row, col);
                init_draw = false;
            }

            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == event::KeyEventKind::Release {
                    continue;
                }
                let old_row = row;
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let _ = terminal::disable_raw_mode();
                        let _ = out.queue(crossterm::style::Print("\r\n"));
                        let _ = out.flush();
                        std::process::exit(0);
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if lines.len() == 1 && lines[0].is_empty() {
                            let _ = terminal::disable_raw_mode();
                            let _ = out.queue(crossterm::style::Print("\r\n"));
                            let _ = out.flush();
                            return None;
                        }
                    }
                    KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        col = 0;
                    }
                    KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        col = lines[row].len();
                    }
                    KeyCode::Tab => {
                        lines[row].insert_str(col, "    ");
                        col += 4;
                    }
                    KeyCode::Char(ch) => {
                        lines[row].insert(col, ch);
                        col += 1;
                    }
                    KeyCode::Enter => {
                        let trimmed = lines[row].trim();
                        if trimmed.starts_with('!') {
                            if trimmed == "!exec" {
                                lines[row] = "".to_string(); 
                            }
                            break;
                        }
                        
                        let remaining = lines[row].split_off(col);
                        lines.insert(row + 1, remaining);
                        row += 1;
                        col = 0;
                    }
                    KeyCode::Backspace => {
                        if col > 0 {
                            col -= 1;
                            lines[row].remove(col);
                        } else if row > 0 {
                            let curr = lines.remove(row);
                            row -= 1;
                            col = lines[row].len();
                            lines[row].push_str(&curr);
                        }
                    }
                    KeyCode::Delete => {
                        if col < lines[row].len() {
                            lines[row].remove(col);
                        } else if row + 1 < lines.len() {
                            let next = lines.remove(row + 1);
                            lines[row].push_str(&next);
                        }
                    }
                    KeyCode::Left => {
                        if col > 0 {
                            col -= 1;
                        } else if row > 0 {
                            row -= 1;
                            col = lines[row].len();
                        }
                    }
                    KeyCode::Right => {
                        if col < lines[row].len() {
                            col += 1;
                        } else if row + 1 < lines.len() {
                            row += 1;
                            col = 0;
                        }
                    }
                    KeyCode::Up => {
                        if row > 0 {
                            row -= 1;
                            col = col.min(lines[row].len());
                        }
                    }
                    KeyCode::Down => {
                        if row + 1 < lines.len() {
                            row += 1;
                            col = col.min(lines[row].len());
                        }
                    }
                    KeyCode::Home => col = 0,
                    KeyCode::End => col = lines[row].len(),
                    _ => {}
                }
                redraw(&mut out, &lines, old_row, row, col);
            }
        }

        let _ = terminal::disable_raw_mode();
        let _ = out.queue(crossterm::style::Print("\r\n"));
        let _ = out.flush();

        if lines.len() == 1 {
            let t = lines[0].trim();
            if t.starts_with('!') { 
                return Some(lines[0].clone());
            }
        }
        
        while lines.last().map(|s| s.trim().is_empty()).unwrap_or(false) {
            lines.pop();
        }

        if lines.is_empty() {
             Some("".to_string())
        } else {
             Some(lines.join("\n"))
        }
    }
}
