use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};
use crate::i18n::text;

/// Terminal handle used by the application. Rendering is deliberately kept
/// on top of crossterm so ghcap has no heavyweight widget/layout dependency.
pub struct Tui;

pub fn terminal() -> io::Result<Tui> {
    Ok(Tui)
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 { return String::new(); }
    if s.chars().count() <= max { return s.to_string(); }
    if max == 1 { return "…".into(); }
    s.chars().take(max - 1).collect::<String>() + "…"
}

fn center_text(text: &str, width: u16) -> String {
    let len = text.chars().count() as u16;
    if len >= width { return truncate(text, width as usize); }
    let left = (width - len) / 2;
    format!("{}{}", " ".repeat(left as usize), text)
}

fn draw_box(stdout: &mut io::Stdout, x: u16, y: u16, width: u16, height: u16) -> io::Result<()> {
    if width < 2 || height < 2 { return Ok(()); }
    execute!(stdout,
        MoveTo(x, y), Print("┌"),
        MoveTo(x + width - 1, y), Print("┐"),
        MoveTo(x, y + height - 1), Print("└"),
        MoveTo(x + width - 1, y + height - 1), Print("┘")
    )?;
    for col in x + 1..x + width - 1 {
        execute!(stdout, MoveTo(col, y), Print("─"), MoveTo(col, y + height - 1), Print("─"))?;
    }
    for row in y + 1..y + height - 1 {
        execute!(stdout, MoveTo(x, row), Print("│"), MoveTo(x + width - 1, row), Print("│"))?;
    }
    Ok(())
}

pub fn draw_menu(_t: &mut Tui, lang: &str, title: &str, items: &[String], selected: usize, info: Option<&str>) -> io::Result<()> {
    let mut stdout = io::stdout();
    let (width, height) = terminal::size()?;
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0), SetAttribute(Attribute::Reset))?;

    let outer_w = width.saturating_sub(2).max(2);
    let outer_h = height.saturating_sub(2).max(4);
    draw_box(&mut stdout, 0, 0, outer_w, outer_h)?;

    let header = format!(" ghcap 0.21.0 · {title} ");
    execute!(stdout, MoveTo(2, 0), SetAttribute(Attribute::Bold), Print(truncate(&header, outer_w.saturating_sub(4) as usize)), SetAttribute(Attribute::Reset))?;

    let list_top = 2u16;
    let footer_y = outer_h.saturating_sub(2);
    let available = footer_y.saturating_sub(list_top).max(1);
    let content_width = outer_w.saturating_sub(6).max(1) as usize;
    let first = if selected >= available as usize { selected + 1 - available as usize } else { 0 };

    for row in 0..available {
        let index = first + row as usize;
        if index >= items.len() { break; }
        let marker = if index == selected { "› " } else { "  " };
        let item = truncate(&items[index], content_width.saturating_sub(2));
        execute!(stdout, MoveTo(2, list_top + row),
            SetAttribute(if index == selected { Attribute::Bold } else { Attribute::Reset }),
            Print(marker), Print(item), SetAttribute(Attribute::Reset))?;
    }

    let footer_default = format!("{}    {}", text(lang, "key.navigate"), text(lang, "key.select_back"));
    let footer = info.unwrap_or(&footer_default);
    execute!(stdout, MoveTo(2, footer_y), Print(center_text(&truncate(footer, outer_w.saturating_sub(4) as usize), outer_w.saturating_sub(4))))?;
    stdout.flush()?;
    Ok(())
}

pub fn read_key() -> io::Result<KeyEvent> {
    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press { return Ok(key); }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListAction {
    Open,
    Add,
    Edit,
    Delete,
    MoveUp,
    MoveDown,
    Back,
}

pub fn manage_select(t: &mut Tui, lang: &str, title: &str, items: &[String], footer: &str, selected: &mut usize) -> io::Result<Option<(usize, ListAction)>> {
    if items.is_empty() { return Ok(None); }
    *selected = (*selected).min(items.len() - 1);
    loop {
        draw_menu(t, lang, title, items, *selected, Some(footer))?;
        let key = read_key()?;
        match key.code {
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => return Ok(Some((*selected, ListAction::MoveUp))),
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => return Ok(Some((*selected, ListAction::MoveDown))),
            KeyCode::Up | KeyCode::Char('k') => *selected = (*selected).saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => *selected = (*selected + 1).min(items.len() - 1),
            KeyCode::Enter => return Ok(Some((*selected, ListAction::Open))),
            KeyCode::Char('a') | KeyCode::Char('A') => return Ok(Some((*selected, ListAction::Add))),
            KeyCode::Char('e') | KeyCode::Char('E') => return Ok(Some((*selected, ListAction::Edit))),
            KeyCode::Char('d') | KeyCode::Char('D') => return Ok(Some((*selected, ListAction::Delete))),
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(Some((*selected, ListAction::Back))),
            _ => {}
        }
    }
}

pub fn select(t: &mut Tui, lang: &str, title: &str, items: &[String]) -> io::Result<Option<usize>> {
    if items.is_empty() { return Ok(None); }
    let mut selected = 0usize;
    loop {
        draw_menu(t, lang, title, items, selected, None)?;
        match read_key()?.code {
            KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => selected = (selected + 1).min(items.len() - 1),
            KeyCode::Enter => return Ok(Some(selected)),
            KeyCode::Esc => return Ok(None),
            _ => {}
        }
    }
}

pub fn confirm(t: &mut Tui, lang: &str, title: &str, message: &str) -> io::Result<bool> {
    let items = vec![format!("{} [y]", text(lang, "common.yes")), format!("{} [n]", text(lang, "common.no"))];
    let mut selected = 1usize;
    loop {
        draw_menu(t, lang, title, &items, selected, Some(message))?;
        match read_key()?.code {
            KeyCode::Left | KeyCode::Up => selected = 0,
            KeyCode::Right | KeyCode::Down => selected = 1,
            KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => return Ok(false),
            KeyCode::Enter => return Ok(selected == 0),
            _ => {}
        }
    }
}

pub fn message(lang: &str, title: &str, message: &str) -> io::Result<()> {
    let mut stdout = io::stdout();
    let (width, height) = terminal::size()?;
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0), SetAttribute(Attribute::Reset))?;
    let box_w = width.saturating_mul(4).saturating_div(5).max(20).min(width.saturating_sub(2));
    let box_h = height.saturating_mul(3).saturating_div(5).max(7).min(height.saturating_sub(2));
    let x = width.saturating_sub(box_w) / 2;
    let y = height.saturating_sub(box_h) / 2;
    draw_box(&mut stdout, x, y, box_w, box_h)?;
    execute!(stdout, MoveTo(x + 2, y), SetAttribute(Attribute::Bold), Print(format!(" {title} ")), SetAttribute(Attribute::Reset))?;

    let max_width = box_w.saturating_sub(4) as usize;
    let mut row = y + 2;
    for paragraph in message.lines() {
        if row >= y + box_h.saturating_sub(3) { break; }
        let chars: Vec<char> = paragraph.chars().collect();
        if chars.is_empty() {
            row += 1;
            continue;
        }
        let mut offset = 0usize;
        while offset < chars.len() && row < y + box_h.saturating_sub(3) {
            let end = (offset + max_width).min(chars.len());
            let part: String = chars[offset..end].iter().collect();
            execute!(stdout, MoveTo(x + 2, row), Print(part))?;
            row += 1;
            offset = end;
        }
    }
    let hint = text(lang, "key.message_close");
    execute!(stdout, MoveTo(x + 2, y + box_h - 2), Print(center_text(&hint, box_w.saturating_sub(4))))?;
    stdout.flush()?;
    loop {
        if matches!(read_key()?.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) { return Ok(()); }
    }
}

#[derive(Debug)]
pub enum CommitMessageDecision {
    Commit(String),
    BackToPresets,
    BackToRepo,
}

fn draw_commit_editor_cui(text_value: &[char], cursor: usize) -> io::Result<()> {
    let mut stdout = io::stdout();
    let (width, height) = terminal::size()?;
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0), SetAttribute(Attribute::Reset))?;
    println!("ghcap 0.21.0 - Commit Message");
    println!("=== コミットメッセージ ===");
    println!();

    let mut lines = vec![String::new()];
    let mut cursor_line = 0usize;
    let mut cursor_col = 0usize;
    for (i, ch) in text_value.iter().enumerate() {
        if i == cursor {
            cursor_line = lines.len() - 1;
            cursor_col = lines.last().map(|x| x.chars().count()).unwrap_or(0);
        }
        if *ch == '\n' { lines.push(String::new()); } else { lines.last_mut().unwrap().push(*ch); }
    }
    if cursor == text_value.len() {
        cursor_line = lines.len() - 1;
        cursor_col = lines.last().map(|x| x.chars().count()).unwrap_or(0);
    }

    let max_width = width.saturating_sub(1) as usize;
    let max_lines = height.saturating_sub(6).max(1) as usize;
    let first_line = if cursor_line >= max_lines { cursor_line + 1 - max_lines } else { 0 };
    for line in lines.iter().skip(first_line).take(max_lines) {
        println!("{}", truncate(line, max_width));
    }
    println!();
    println!();
    println!("Shift+Enter: 改行    Enter: Commit確認    Esc: 戻る");

    let visible_line = cursor_line.saturating_sub(first_line) as u16;
    let visible_col = cursor_col.min(max_width.saturating_sub(1)) as u16;
    let y = 2u16.saturating_add(visible_line);
    execute!(stdout, MoveTo(visible_col, y), Show)?;
    stdout.flush()?;
    Ok(())
}

pub fn edit_commit_message(_t: &mut Tui, lang: &str, initial: &str) -> io::Result<CommitMessageDecision> {
    let mut text_value: Vec<char> = initial.chars().collect();
    let mut cursor = text_value.len();
    leave_tui()?;
    terminal::enable_raw_mode()?;

    loop {
        draw_commit_editor_cui(&text_value, cursor)?;
        let key = read_key()?;
        match key.code {
            KeyCode::Esc => {
                terminal::disable_raw_mode()?;
                enter_tui()?;
                return Ok(CommitMessageDecision::BackToPresets);
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                text_value.insert(cursor, '\n');
                cursor += 1;
            }
            KeyCode::Enter => {
                terminal::disable_raw_mode()?;
                enter_tui()?;
                match commit_confirm(lang)? {
                    CommitConfirm::Yes => return Ok(CommitMessageDecision::Commit(text_value.iter().collect())),
                    CommitConfirm::No => return Ok(CommitMessageDecision::BackToRepo),
                    CommitConfirm::Edit => {
                        leave_tui()?;
                        terminal::enable_raw_mode()?;
                    }
                }
            }
            KeyCode::Char(c) => {
                text_value.insert(cursor, c);
                cursor += 1;
            }
            KeyCode::Backspace => {
                if cursor > 0 { cursor -= 1; text_value.remove(cursor); }
            }
            KeyCode::Delete => {
                if cursor < text_value.len() { text_value.remove(cursor); }
            }
            KeyCode::Left => cursor = cursor.saturating_sub(1),
            KeyCode::Right => cursor = (cursor + 1).min(text_value.len()),
            KeyCode::Home => {
                while cursor > 0 && text_value[cursor - 1] != '\n' { cursor -= 1; }
            }
            KeyCode::End => {
                while cursor < text_value.len() && text_value[cursor] != '\n' { cursor += 1; }
            }
            KeyCode::Up => {
                let line_start = text_value[..cursor].iter().rposition(|c| *c == '\n').map(|i| i + 1).unwrap_or(0);
                let col = cursor - line_start;
                if line_start > 0 {
                    let prev_end = line_start - 1;
                    let prev_start = text_value[..prev_end].iter().rposition(|c| *c == '\n').map(|i| i + 1).unwrap_or(0);
                    cursor = prev_start + col.min(prev_end - prev_start);
                }
            }
            KeyCode::Down => {
                let line_start = text_value[..cursor].iter().rposition(|c| *c == '\n').map(|i| i + 1).unwrap_or(0);
                let col = cursor - line_start;
                let line_end = text_value[cursor..].iter().position(|c| *c == '\n').map(|i| cursor + i).unwrap_or(text_value.len());
                if line_end < text_value.len() {
                    let next_start = line_end + 1;
                    let next_end = text_value[next_start..].iter().position(|c| *c == '\n').map(|i| next_start + i).unwrap_or(text_value.len());
                    cursor = next_start + col.min(next_end - next_start);
                }
            }
            _ => {}
        }
    }
}

enum CommitConfirm { Yes, No, Edit }

fn commit_confirm(lang: &str) -> io::Result<CommitConfirm> {
    let items = vec![
        "Yes — この内容でCommitする".to_string(),
        "No — Commitせずリポジトリ画面へ戻る".to_string(),
        "Edit — メッセージを編集する".to_string(),
    ];
    let mut selected = 0usize;
    let mut tui = Tui;
    loop {
        draw_menu(&mut tui, lang, "Commit確認", &items, selected, Some("Y: Commit / N: リポジトリ画面へ / E: 編集 / Esc: 編集画面へ"))?;
        match read_key()?.code {
            KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => selected = (selected + 1).min(items.len() - 1),
            KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(CommitConfirm::Yes),
            KeyCode::Char('n') | KeyCode::Char('N') => return Ok(CommitConfirm::No),
            KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Esc => return Ok(CommitConfirm::Edit),
            KeyCode::Enter => return Ok(match selected { 0 => CommitConfirm::Yes, 1 => CommitConfirm::No, _ => CommitConfirm::Edit }),
            _ => {}
        }
    }
}

pub fn cui_prompt(title: &str, default: &str) -> io::Result<Option<String>> {
    leave_tui()?;
    println!("\n{title}");
    println!("Current/default: {default}");
    print!("Input (empty keeps default, Ctrl-D cancels): ");
    io::stdout().flush()?;
    let mut input = String::new();
    let read = io::stdin().read_line(&mut input)?;
    let result = if read == 0 {
        None
    } else {
        let value = input.trim_end_matches(['\r', '\n']);
        Some(if value.is_empty() { default.to_string() } else { value.to_string() })
    };
    enter_tui()?;
    Ok(result)
}

pub fn leave_tui() -> io::Result<()> {
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, Show)?;
    Ok(())
}

pub fn enter_tui() -> io::Result<()> {
    execute!(io::stdout(), EnterAlternateScreen, Hide)?;
    terminal::enable_raw_mode()?;
    Ok(())
}

pub fn cui_login(lang: &str, command: &mut std::process::Command, url: &str) -> io::Result<bool> {
    leave_tui()?;
    println!("{}", text(lang, "login.url_notice"));
    println!("{url}");
    println!("{}", text(lang, "login.browser_ready"));
    println!("{}", text(lang, "login.browser_open_key"));
    io::stdout().flush()?;
    let _ = read_key_or_line()?;
    println!("\n{}\n", text(lang, "login.start"));
    io::stdout().flush()?;

    let status = match command.status() {
        Ok(status) => status,
        Err(error) => {
            println!("\n{}: 1", text(lang, "common.exit_code"));
            println!("{error}");
            println!("\n{}", text(lang, "login.failed"));
            println!("\n{}", text(lang, "key.cui_return"));
            io::stdout().flush()?;
            let _ = read_key_or_line();
            enter_tui()?;
            return Ok(false);
        }
    };

    if !status.success() {
        match status.code() {
            Some(code) => println!("\n{}: {code}", text(lang, "common.exit_code")),
            None => println!("\n{}", text(lang, "common.terminated")),
        }
        println!("{}", text(lang, "login.failed"));
        println!("\n{}", text(lang, "key.cui_return"));
        io::stdout().flush()?;
        let _ = read_key_or_line();
    }

    enter_tui()?;
    Ok(status.success())
}

pub fn cui_operation_result<F>(lang: &str, title: &str, operation: F) -> io::Result<Result<String, String>>
where F: FnOnce() -> Result<String, String> {
    leave_tui()?;
    println!("\n=== {title} ===\n");
    let result = operation();
    match &result {
        Ok(output) => {
            if !output.trim().is_empty() { println!("{output}"); }
            println!("\n{}: 0", text(lang, "common.exit_code"));
        }
        Err(error) => {
            println!("{error}");
            let code = extract_exit_code(error).unwrap_or(1);
            println!("\n{}: {code}", text(lang, "common.exit_code"));
        }
    }
    println!("\n{}", text(lang, "key.cui_return"));
    io::stdout().flush()?;
    let _ = read_key_or_line();
    enter_tui()?;
    Ok(result)
}

pub fn cui_yes_no(_lang: &str, title: &str, message: &str) -> io::Result<bool> {
    leave_tui()?;
    println!("\n=== {title} ===\n");
    println!("{message}");
    print!("[y/n]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    enter_tui()?;
    Ok(matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}

pub fn cui_operation<F>(lang: &str, title: &str, operation: F) -> io::Result<bool>
where F: FnOnce() -> Result<String, String> {
    leave_tui()?;
    println!("\n=== {title} ===\n");
    let success = match operation() {
        Ok(output) => {
            if !output.trim().is_empty() { println!("{output}"); }
            println!("\n{}: 0", text(lang, "common.exit_code"));
            true
        }
        Err(error) => {
            println!("{error}");
            let code = extract_exit_code(&error).unwrap_or(1);
            println!("\n{}: {code}", text(lang, "common.exit_code"));
            false
        }
    };
    println!("\n{}", text(lang, "key.cui_return"));
    io::stdout().flush()?;
    let _ = read_key_or_line();
    enter_tui()?;
    Ok(success)
}

fn extract_exit_code(error: &str) -> Option<i32> {
    let marker = "exit code ";
    error.split(marker).nth(1)?.split_whitespace().next()?.parse().ok()
}

fn read_key_or_line() -> io::Result<()> {
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line)?;
    Ok(())
}

pub fn prompt(_t: &mut Tui, title: &str, default: &str) -> io::Result<Option<String>> {
    cui_prompt(title, default)
}
