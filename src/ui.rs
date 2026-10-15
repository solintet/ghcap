use crate::i18n::text;
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::Text,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};
use rustyline::{error::ReadlineError, DefaultEditor};
use std::io::{self, Stdout, Write};

/// Ratatui owns terminal rendering; Crossterm remains the cross-platform backend.
pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

pub fn terminal() -> io::Result<Tui> {
    let backend = CrosstermBackend::new(io::stdout());
    Ok(Tui {
        terminal: Terminal::new(backend)?,
    })
}

pub fn draw_menu(
    t: &mut Tui,
    lang: &str,
    title: &str,
    items: &[String],
    selected: usize,
    info: Option<&str>,
) -> io::Result<()> {
    let footer_default = format!(
        "{}    {}",
        text(lang, "key.navigate"),
        text(lang, "key.select_back")
    );
    let footer = info.unwrap_or(&footer_default).to_string();
    t.terminal.clear()?;
    t.terminal.draw(|frame| {
        let area = frame.area();
        let outer = Block::default()
            .title(format!(" ghcap 0.0.29.8 · {title} "))
            .borders(Borders::ALL);
        let inner = outer.inner(area);
        frame.render_widget(outer, area);
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        let rows = items
            .iter()
            .map(|item| ListItem::new(item.as_str()))
            .collect::<Vec<_>>();
        let list = List::new(rows)
            .highlight_symbol("› ")
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        let mut state = ListState::default();
        state.select(if items.is_empty() {
            None
        } else {
            Some(selected.min(items.len() - 1))
        });
        frame.render_stateful_widget(list, layout[0], &mut state);
        frame.render_widget(
            Paragraph::new(footer).alignment(Alignment::Center),
            layout[1],
        );
    })?;
    Ok(())
}

pub fn read_key() -> io::Result<KeyEvent> {
    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                return Ok(key);
            }
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

pub fn manage_select(
    t: &mut Tui,
    lang: &str,
    title: &str,
    items: &[String],
    footer: &str,
    selected: &mut usize,
) -> io::Result<Option<(usize, ListAction)>> {
    if items.is_empty() {
        return Ok(None);
    }
    *selected = (*selected).min(items.len() - 1);
    loop {
        draw_menu(t, lang, title, items, *selected, Some(footer))?;
        let key = read_key()?;
        match key.code {
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                return Ok(Some((*selected, ListAction::MoveUp)))
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                return Ok(Some((*selected, ListAction::MoveDown)))
            }
            KeyCode::Up | KeyCode::Char('k') => *selected = (*selected).saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => *selected = (*selected + 1).min(items.len() - 1),
            KeyCode::Enter => return Ok(Some((*selected, ListAction::Open))),
            KeyCode::Char('a') | KeyCode::Char('A') => {
                return Ok(Some((*selected, ListAction::Add)))
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                return Ok(Some((*selected, ListAction::Edit)))
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                return Ok(Some((*selected, ListAction::Delete)))
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                return Ok(Some((*selected, ListAction::Back)))
            }
            _ => {}
        }
    }
}

pub fn select(t: &mut Tui, lang: &str, title: &str, items: &[String]) -> io::Result<Option<usize>> {
    if items.is_empty() {
        return Ok(None);
    }
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
    let items = vec![
        format!("{} [y]", text(lang, "common.yes")),
        format!("{} [n]", text(lang, "common.no")),
    ];
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
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;
    terminal.draw(|frame| {
        let area = frame.area();
        let block = Block::default()
            .title(format!(" {title} "))
            .borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        frame.render_widget(
            Paragraph::new(Text::from(message)).wrap(Wrap { trim: false }),
            chunks[0],
        );
        frame.render_widget(
            Paragraph::new(text(lang, "key.message_close")).alignment(Alignment::Center),
            chunks[1],
        );
    })?;
    loop {
        if matches!(
            read_key()?.code,
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')
        ) {
            return Ok(());
        }
    }
}

#[derive(Debug)]
pub enum CommitMessageDecision {
    Commit(String),
    BackToPresets,
    BackToRepo,
}

pub fn edit_commit_message(
    tui: &mut Tui,
    lang: &str,
    initial: &str,
) -> io::Result<CommitMessageDecision> {
    let mut current = initial.to_string();
    loop {
        leave_tui()?;
        println!("\nghcap 0.0.29.8 - Commit Message\n=== コミットメッセージ ===\n");
        println!("プリセット内容は入力欄に事前入力されています。\\n は改行として扱われます。空欄のままEnterを押すと初期値を維持します。\n:back と入力するとプリセット選択へ戻ります。\n");
        let edited_result = readline_initial("> ", &current);
        let edited = match edited_result {
            Ok(value) => value,
            Err(error) => {
                let _ = enter_tui();
                return Err(error);
            }
        };
        let Some(edited) = edited else {
            enter_tui()?;
            return Ok(CommitMessageDecision::BackToPresets);
        };
        if edited == ":back" {
            enter_tui()?;
            return Ok(CommitMessageDecision::BackToPresets);
        }
        if !edited.is_empty() {
            current = edited;
        }
        enter_tui()?;
        match commit_confirm(tui, lang)? {
            CommitConfirm::Yes => return Ok(CommitMessageDecision::Commit(current)),
            CommitConfirm::No => return Ok(CommitMessageDecision::BackToRepo),
            CommitConfirm::Edit => continue,
        }
    }
}

fn readline_initial(prompt: &str, initial: &str) -> io::Result<Option<String>> {
    let mut editor = DefaultEditor::new().map_err(io::Error::other)?;
    match editor.readline_with_initial(prompt, (initial, "")) {
        Ok(value) => Ok(Some(value)),
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => Ok(None),
        Err(error) => Err(io::Error::other(error)),
    }
}

enum CommitConfirm {
    Yes,
    No,
    Edit,
}

fn commit_confirm(tui: &mut Tui, lang: &str) -> io::Result<CommitConfirm> {
    let items = vec![
        "Yes — この内容でCommitする".to_string(),
        "No — Commitせずリポジトリ画面へ戻る".to_string(),
        "Edit — メッセージを編集する".to_string(),
    ];
    let mut selected = 0usize;
    loop {
        draw_menu(
            tui,
            lang,
            "Commit確認",
            &items,
            selected,
            Some("Y: Commit / N: リポジトリ画面へ / E: 編集 / Esc: 編集画面へ"),
        )?;
        match read_key()?.code {
            KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => selected = (selected + 1).min(items.len() - 1),
            KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(CommitConfirm::Yes),
            KeyCode::Char('n') | KeyCode::Char('N') => return Ok(CommitConfirm::No),
            KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Esc => {
                return Ok(CommitConfirm::Edit)
            }
            KeyCode::Enter => {
                return Ok(match selected {
                    0 => CommitConfirm::Yes,
                    1 => CommitConfirm::No,
                    _ => CommitConfirm::Edit,
                })
            }
            _ => {}
        }
    }
}

pub fn cui_prompt(title: &str, default: &str) -> io::Result<Option<String>> {
    leave_tui()?;
    println!("\n{title}");
    println!("Current/default: {default}");
    let input = readline_initial("Input (Ctrl-D cancels): ", default);
    let restore = enter_tui();
    let input = input?;
    restore?;
    Ok(input.map(|value| {
        if value.is_empty() {
            default.to_string()
        } else {
            value
        }
    }))
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
    read_key_or_line()?;
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

pub fn cui_operation_result<F>(
    lang: &str,
    title: &str,
    operation: F,
) -> io::Result<Result<String, String>>
where
    F: FnOnce() -> Result<String, String>,
{
    leave_tui()?;
    println!("\n=== {title} ===\n");
    let result = operation();
    match &result {
        Ok(output) => {
            if !output.trim().is_empty() {
                println!("{output}");
            }
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
    println!("\n=== {title} ===\n{message}");
    let input = readline_initial("[y/n]: ", "n");
    let restore = enter_tui();
    let input = input?;
    restore?;
    Ok(matches!(
        input
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "y" | "yes"
    ))
}

/// Show a confirmation in the normal terminal. Enter continues; Ctrl-D cancels.
pub fn cui_confirm_enter(_lang: &str, title: &str, message: &str) -> io::Result<bool> {
    leave_tui()?;
    println!("\n=== {title} ===\n{message}\n");
    let input = readline_initial("Press Enter to continue (Ctrl-D to cancel): ", "");
    let restore = enter_tui();
    let input = input?;
    restore?;
    Ok(input.is_some())
}

pub fn cui_operation<F>(lang: &str, title: &str, operation: F) -> io::Result<bool>
where
    F: FnOnce() -> Result<String, String>,
{
    leave_tui()?;
    println!("\n=== {title} ===\n");
    let success = match operation() {
        Ok(output) => {
            if !output.trim().is_empty() {
                println!("{output}");
            }
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
    error
        .split(marker)
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn read_key_or_line() -> io::Result<()> {
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line)?;
    Ok(())
}

pub fn prompt(_t: &mut Tui, title: &str, default: &str) -> io::Result<Option<String>> {
    cui_prompt(title, default)
}
