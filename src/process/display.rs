//! EN: Escape unsafe terminal controls and redact sensitive diagnostics.
//! JA: 端末制御文字の無害化と機密情報のマスクを担当します。
use super::Output;

/// Visible escapes, never executable terminal control sequences. Redact common
/// credential formats and URL userinfo before displaying external diagnostics.
pub fn safe_text(text: &str) -> String {
    let mut s = String::with_capacity(text.len());
    for c in text.chars() {
        if (c == '\n' || c == '\t' || !c.is_control())
            && !matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200e}' | '\u{200f}')
        {
            s.push(c);
        } else {
            s.push_str(&format!("\\u{{{:x}}}", c as u32));
        }
    }
    for prefix in ["github_pat_", "ghp_", "gho_", "ghu_", "ghs_", "ghr_"] {
        let mut from = 0;
        while let Some(i) = s[from..].find(prefix) {
            let start = from + i;
            let end = s[start..]
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .map_or(s.len(), |n| start + n);
            s.replace_range(start..end, "[REDACTED]");
            from = start + 10;
        }
    }
    let mut from = 0;
    while let Some(i) = s[from..].find("://") {
        let start = from + i + 3;
        let end = s[start..]
            .find(['/', '\n', ' ', '\t'])
            .map_or(s.len(), |n| start + n);
        if let Some(at) = s[start..end].rfind('@') {
            s.replace_range(start..start + at, "[REDACTED]");
            from = start + 11;
        } else {
            from = end;
        }
        if from >= s.len() {
            break;
        }
    }
    s
}

/// EN: Preserve BOTH stdout and stderr when a command fails; the diagnosis is
/// appended after the external output rather than replacing it. Redact secrets
/// and terminal control sequences before writing to the user's screen.
/// JA: コマンド失敗時にstdoutとstderrの両方を保持し、外部出力の下に
/// ghcapの診断を追加します。表示前に機密情報と危険な制御文字を除去します。
pub fn failure(program: &str, output: &Output) -> String {
    fn tail(text: &str, limit: usize) -> &str {
        let index = text.char_indices().rev().nth(limit).map_or(0, |(i, _)| i);
        &text[index..]
    }
    let stdout = safe_text(tail(&output.stdout, 8192));
    let stderr = safe_text(tail(&output.stderr, 8192));
    format!(
        "[COMMAND / 実行コマンド] {program}\n[STDOUT / 標準出力]\n{stdout}\n[STDERR / 標準エラー]\n{stderr}\n[EXIT STATUS / 終了状態] {:?} (code {})\n[GHCAP DIAGNOSIS / ghcap診断]\nThe external command did not complete successfully. Check the original output above. / 外部コマンドが正常終了しませんでした。上記の出力を確認してください。",
        output.termination,
        output.code,
        program = safe_text(program)
    )
}
