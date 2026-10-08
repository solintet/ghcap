use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
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

    let header = format!(" ghcap 0.13.0 · {title} ");
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

pub fn message(t: &mut Tui, lang: &str, title: &str, message: &str) -> io::Result<()> {
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
    println!("{}", text(lang, "login.browser_optional"));
    println!("{}\n", text(lang, "login.start"));
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
    match status.code() {
        Some(code) => println!("\n{}: {code}", text(lang, "common.exit_code")),
        None => println!("\n{}", text(lang, "common.terminated")),
    }
    if !status.success() {
        println!("{}", text(lang, "login.failed"));
    }
    println!("\n{}", text(lang, "key.cui_return"));
    io::stdout().flush()?;
    let _ = read_key_or_line();
    enter_tui()?;
    Ok(status.success())
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
