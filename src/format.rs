use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn human_count(value: u64) -> String {
    match value {
        0..=999 => value.to_string(),
        1_000..=999_999 => {
            let number = value as f64 / 1000.0;
            if number >= 100.0 {
                format!("{number:.0}k")
            } else {
                format!("{number:.1}k")
            }
        }
        1_000_000..=999_999_999 => {
            let number = value as f64 / 1_000_000.0;
            if number >= 100.0 {
                format!("{number:.0}m")
            } else {
                format!("{number:.1}m")
            }
        }
        _ => format!("{:.1}b", value as f64 / 1_000_000_000.0),
    }
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

pub fn ttl_label(seconds: u64) -> String {
    if seconds < 3600 {
        format!("{} 分钟", (seconds / 60).max(1))
    } else if seconds < 86_400 {
        let hours = seconds as f64 / 3600.0;
        if (hours - hours.round()).abs() < 0.01 {
            format!("{} 小时", hours.round() as u64)
        } else {
            format!("{hours:.1} 小时")
        }
    } else {
        format!("{} 小时", seconds / 3600)
    }
}

pub fn relative_time(raw: &str) -> String {
    if raw.trim().is_empty() {
        return "未知".to_string();
    }
    let Ok(time) = OffsetDateTime::parse(raw, &Rfc3339) else {
        return raw.to_string();
    };
    relative_epoch(
        time.unix_timestamp(),
        OffsetDateTime::now_utc().unix_timestamp(),
    )
}

pub fn duration_label(start: &str, end: Option<&str>) -> Option<String> {
    let start = OffsetDateTime::parse(start, &Rfc3339).ok()?;
    let end = match end {
        Some(raw) => OffsetDateTime::parse(raw, &Rfc3339).ok()?,
        None => OffsetDateTime::now_utc(),
    };
    let seconds = (end - start).whole_seconds().max(0);
    Some(short_duration(seconds))
}

pub fn short_duration(seconds: i64) -> String {
    let seconds = seconds.max(0);
    if seconds < 60 {
        format!("{seconds} 秒")
    } else if seconds < 3600 {
        format!("{} 分 {} 秒", seconds / 60, seconds % 60)
    } else if seconds < 86_400 {
        format!("{} 小时 {} 分", seconds / 3600, seconds % 3600 / 60)
    } else {
        format!("{} 天 {} 小时", seconds / 86_400, seconds % 86_400 / 3600)
    }
}

pub fn relative_epoch(then: i64, now: i64) -> String {
    let seconds = (now - then).max(0);
    if seconds < 60 {
        "刚刚".to_string()
    } else if seconds < 3600 {
        format!("{} 分钟前", seconds / 60)
    } else if seconds < 86_400 {
        format!("{} 小时前", seconds / 3600)
    } else if seconds < 2_592_000 {
        format!("{} 天前", seconds / 86_400)
    } else if seconds < 31_536_000 {
        format!("{} 个月前", seconds / 2_592_000)
    } else {
        format!("{} 年前", seconds / 31_536_000)
    }
}

pub fn countdown_epoch(then: i64, now: i64) -> String {
    if then <= now {
        return "已经恢复".to_string();
    }
    let seconds = then - now;
    if seconds < 60 {
        "不到 1 分钟".to_string()
    } else if seconds < 3600 {
        format!("{} 分钟后", seconds / 60)
    } else if seconds < 86_400 {
        format!("{} 小时后", seconds / 3600)
    } else {
        format!("{} 天后", seconds / 86_400)
    }
}

pub fn reset_label(then: i64, now: i64) -> String {
    let left = countdown_epoch(then, now);
    if left == "已经恢复" {
        left
    } else {
        format!("{left}恢复")
    }
}

pub fn text_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

pub fn visible_text(value: &str, cursor: usize, max_width: usize) -> (String, u16) {
    let max_width = max_width.max(1);
    let prefix_width = text_width(&value[..cursor]);
    let start_column = prefix_width.saturating_sub(max_width.saturating_sub(1));
    let mut visible = String::new();
    let mut total = 0;
    let mut cursor_column = 0;
    for (index, ch) in value.char_indices() {
        let width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if total + width <= start_column {
            total += width;
            continue;
        }
        if index < cursor {
            cursor_column += width as u16;
        }
        if text_width(&visible) + width > max_width {
            break;
        }
        visible.push(ch);
    }
    (visible, cursor_column.min(max_width as u16))
}

pub fn truncate(text: &str, width: usize) -> String {
    let text = text.replace(['\n', '\r'], " ");
    if width == 0 {
        return String::new();
    }
    if text_width(&text) <= width {
        return text;
    }
    if width <= 3 {
        return ".".repeat(width);
    }
    let budget = width - 3;
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width > budget {
            break;
        }
        out.push(ch);
        used += ch_width;
    }
    out.push_str("...");
    out
}

pub fn wrap_text(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    if width == 0 || max_lines == 0 {
        return Vec::new();
    }
    let text = text.replace(['\r', '\n'], " ");
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut line_width = 0;
    for word in text.split_whitespace() {
        let word_width = text_width(word);
        let separator = usize::from(!line.is_empty());
        if line_width + separator + word_width <= width {
            if !line.is_empty() {
                line.push(' ');
                line_width += 1;
            }
            line.push_str(word);
            line_width += word_width;
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            line_width = 0;
        }
        if word_width <= width {
            line.push_str(word);
            line_width = word_width;
            continue;
        }
        for ch in word.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if line_width + ch_width > width && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                line_width = 0;
            }
            line.push(ch);
            line_width += ch_width;
            if lines.len() >= max_lines {
                break;
            }
        }
    }
    if !line.is_empty() && lines.len() < max_lines {
        lines.push(line);
    }
    if lines.len() > max_lines {
        lines.truncate(max_lines);
    }
    if lines.len() == max_lines {
        let consumed: usize =
            lines.iter().map(|line| text_width(line)).sum::<usize>() + lines.len();
        if consumed < text_width(&text)
            && let Some(last) = lines.last_mut()
        {
            *last = truncate(last, width);
        }
    }
    lines
}

pub fn clean_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if is_emoji(ch) {
            continue;
        }
        out.push(ch);
    }
    let mut collapsed = String::with_capacity(out.len());
    let mut last_space = false;
    for ch in out.chars() {
        let is_space = ch == ' ' || ch == '\t';
        if is_space && last_space {
            continue;
        }
        collapsed.push(ch);
        last_space = is_space;
    }
    collapsed.trim().to_string()
}

fn is_emoji(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1F000..=0x1FAFF
            | 0x2600..=0x27BF
            | 0xFE0F
            | 0xFE0E
            | 0x200D
            | 0x20E3
            | 0xE0020..=0xE007F
    )
}

pub fn spinner(frame: u64) -> &'static str {
    const FRAMES: [&str; 4] = ["-", "\\", "|", "/"];
    FRAMES[(frame / 2) as usize % FRAMES.len()]
}

pub fn base64(input: &str) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((triple >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(triple & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub fn display_path(path: &std::path::Path) -> String {
    let display = path.display().to_string();
    if let Some(home) = std::env::var_os("HOME") {
        let home = home.to_string_lossy();
        if let Some(rest) = display.strip_prefix(home.as_ref()) {
            return format!("~{rest}");
        }
    }
    display
}

pub fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let mut values = Vec::new();
    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\n' | b'\r' | b' ' | b'\t' => continue,
            _ => return None,
        };
        values.push(value);
    }
    let mut out = Vec::with_capacity(values.len() * 3 / 4);
    for chunk in values.chunks(4) {
        if chunk.len() < 2 {
            return None;
        }
        out.push((chunk[0] << 2) | (chunk[1] >> 4));
        if chunk.len() > 2 {
            out.push((chunk[1] << 4) | (chunk[2] >> 2));
        }
        if chunk.len() > 3 {
            out.push((chunk[2] << 6) | chunk[3]);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_are_compact() {
        assert_eq!(human_count(999), "999");
        assert_eq!(human_count(1_200), "1.2k");
        assert_eq!(human_count(12_400), "12.4k");
        assert_eq!(human_count(1_230_000), "1.2m");
    }

    #[test]
    fn truncates_by_display_width() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 8), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(text_width(&truncate("中文仓库", 5)), 5);
    }

    #[test]
    fn wraps_chinese_text() {
        let lines = wrap_text("这是一个没有空格的仓库简介，需要按照显示宽度换行喵~", 10, 3);
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|line| text_width(line) <= 10));
    }

    #[test]
    fn formats_relative_dates() {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        assert_eq!(relative_epoch(now - 10, now), "刚刚");
        assert_eq!(relative_epoch(now - 120, now), "2 分钟前");
        assert_eq!(relative_epoch(now - 7200, now), "2 小时前");
    }

    #[test]
    fn counts_down_rate_limit_reset() {
        let now = 1_000_000;
        assert_eq!(countdown_epoch(now - 10, now), "已经恢复");
        assert_eq!(countdown_epoch(now + 30, now), "不到 1 分钟");
        assert_eq!(countdown_epoch(now + 125, now), "2 分钟后");
        assert_eq!(countdown_epoch(now + 7200, now), "2 小时后");
        assert_eq!(reset_label(now + 125, now), "2 分钟后恢复");
        assert_eq!(reset_label(now - 1, now), "已经恢复");
    }

    #[test]
    fn visible_text_tracks_chinese_cursor() {
        let (text, cursor) = visible_text("中文abc", 6, 10);
        assert_eq!(text, "中文abc");
        assert_eq!(cursor, 4);
        let (text, cursor) = visible_text("啊啊啊", 9, 4);
        assert_eq!(text, "啊啊");
        assert_eq!(cursor, 4);
    }

    #[test]
    fn formats_workflow_durations() {
        assert_eq!(short_duration(9), "9 秒");
        assert_eq!(short_duration(125), "2 分 5 秒");
        assert_eq!(short_duration(7_320), "2 小时 2 分");
        assert_eq!(short_duration(93_600), "1 天 2 小时");
        assert_eq!(
            duration_label("2026-01-01T00:00:00Z", Some("2026-01-01T00:02:05Z")).as_deref(),
            Some("2 分 5 秒")
        );
        assert_eq!(duration_label("not-a-date", None), None);
    }

    #[test]
    fn strips_emoji_from_external_text() {
        let cleaned = clean_text("A terminal  terminal \u{1F468}\u{200D}\u{1F407} for Rust");
        assert_eq!(cleaned, "A terminal terminal for Rust");
        assert!(!clean_text("a \u{1F600} b").contains('\u{1F600}'));
    }

    #[test]
    fn base64_matches_known_value() {
        assert_eq!(base64("gitx-tui"), "Z2l0eC10dWk=");
        assert_eq!(base64("a"), "YQ==");
        assert_eq!(base64("ab"), "YWI=");
        assert_eq!(base64_decode("Z2l0eC10dWk=").unwrap(), b"gitx-tui");
        assert_eq!(base64_decode("5Lit5paH").unwrap(), "中文".as_bytes());
    }

    #[test]
    fn ttl_is_readable() {
        assert_eq!(ttl_label(300), "5 分钟");
        assert_eq!(ttl_label(900), "15 分钟");
        assert_eq!(ttl_label(3600), "1 小时");
    }
}
