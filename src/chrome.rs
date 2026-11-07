use crate::icons;
use crate::theme::Theme;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

pub fn colorize_icons(buffer: &mut ratatui::buffer::Buffer, theme: &Theme) {
    let area = *buffer.area();
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            let is_icon = {
                let Some(cell) = buffer.cell((x, y)) else {
                    continue;
                };
                let mut chars = cell.symbol().chars();
                matches!(
                    (chars.next(), chars.next()),
                    (Some(ch), None) if (0xE000..=0xF8FF).contains(&(ch as u32))
                )
            };
            if !is_icon {
                continue;
            }
            let symbol = buffer
                .cell((x, y))
                .map(|cell| cell.symbol().to_string())
                .unwrap_or_default();
            if let Some(cell) = buffer.cell_mut((x, y)) {
                cell.set_fg(icon_tint(&symbol, theme));
            }
        }
    }
}

fn icon_tint(symbol: &str, theme: &Theme) -> Color {
    if symbol == icons::STAR || symbol == icons::STAR_EMPTY || symbol == icons::SPARKLE {
        theme.color("yellow")
    } else if symbol == icons::FLAME || symbol == icons::TRASH || symbol == icons::WARNING {
        theme.color("red")
    } else if symbol == icons::REPO
        || symbol == icons::FOLDER_OPEN
        || symbol == icons::DOWNLOAD_DIR
        || symbol == icons::INFO
        || symbol == icons::GITHUB
    {
        theme.color("blue")
    } else if symbol == icons::USER || symbol == icons::PLAY {
        theme.color("green")
    } else if symbol == icons::BELL
        || symbol == icons::TAG
        || symbol == icons::DOWNLOAD
        || symbol == icons::ARCHIVE
        || symbol == icons::RANDOM
    {
        theme.color("peach")
    } else if symbol == icons::SEARCH
        || symbol == icons::EXTERNAL
        || symbol == icons::EYE
        || symbol == icons::EXPAND
        || symbol == icons::CODE
        || symbol == icons::CODE_FILE
    {
        theme.color("sky")
    } else if symbol == icons::LOCK || symbol == icons::PIN || symbol == icons::CHART {
        theme.color("mauve")
    } else if symbol == icons::GLOBE || symbol == icons::NETWORK {
        theme.color("teal")
    } else if symbol == icons::ISSUE
        || symbol == icons::CHECK
        || symbol == icons::CHECK_CIRCLE
        || symbol == icons::CHECK_SQUARE
        || symbol == icons::SAVE
        || symbol == icons::REFRESH
        || symbol == icons::TERMINAL
        || symbol == icons::COMMAND
    {
        theme.color("green")
    } else if symbol == icons::CLOCK || symbol == icons::COMMIT {
        theme.color("yellow")
    } else if symbol == icons::COPY
        || symbol == icons::LICENSE
        || symbol == icons::COMMENT
        || symbol == icons::BOOK
        || symbol == icons::BRANCH
        || symbol == icons::PULL
        || symbol == icons::COGS
    {
        theme.color("lavender")
    } else if symbol == icons::DATABASE {
        theme.color("peach")
    } else if symbol == icons::PALETTE
        || symbol == icons::THEME
        || symbol == icons::IMAGE
        || symbol == icons::HOME
    {
        theme.color("pink")
    } else if symbol == icons::BACK
        || symbol == icons::CHEVRON_LEFT
        || symbol == icons::CHEVRON_RIGHT
    {
        theme.color("subtext0")
    } else if symbol == icons::X_CIRCLE {
        theme.color("red")
    } else if symbol == icons::SQUARE {
        theme.color("overlay1")
    } else {
        let palette = [
            theme.color("blue"),
            theme.color("mauve"),
            theme.color("green"),
            theme.color("peach"),
            theme.color("sky"),
            theme.color("pink"),
            theme.color("yellow"),
            theme.color("teal"),
        ];
        let code = symbol.chars().next().unwrap_or('?') as usize;
        palette[code % palette.len()]
    }
}

pub fn color_mix(start: Color, end: Color, t: f32) -> Color {
    let (Color::Rgb(sr, sg, sb), Color::Rgb(er, eg, eb)) = (start, end) else {
        return start;
    };
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Color::Rgb(mix(sr, er), mix(sg, eg), mix(sb, eb))
}

pub fn gradient_chars(count: usize, glyph: &str, start: Color, end: Color) -> Vec<Span<'static>> {
    if count == 0 {
        return Vec::new();
    }
    let denominator = count.saturating_sub(1).max(1) as f32;
    (0..count)
        .map(|index| {
            let t = index as f32 / denominator;
            Span::styled(
                glyph.to_string(),
                Style::default().fg(color_mix(start, end, t)),
            )
        })
        .collect()
}

pub fn gradient_line(count: usize, glyph: &str, start: Color, end: Color) -> Line<'static> {
    Line::from(gradient_chars(count.max(1), glyph, start, end))
}
pub fn gradient_text(text: &str, start: Color, end: Color, bold: bool) -> Vec<Span<'static>> {
    let count = text.chars().count().max(1);
    text.chars()
        .enumerate()
        .map(|(index, ch)| {
            let t = index as f32 / count.saturating_sub(1).max(1) as f32;
            let style = Style::default().fg(color_mix(start, end, t));
            let style = if bold {
                style.add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                style
            };
            Span::styled(ch.to_string(), style)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Paths;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn colorizes_nerd_font_cells() {
        let base = std::env::temp_dir().join(format!("gxt-chrome-{}", std::process::id()));
        let paths = Paths::for_test(base.clone(), base.join("cache"));
        let theme = Theme::load(&paths, "mocha");
        let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
        buffer.cell_mut((0, 0)).unwrap().set_symbol(icons::STAR);
        buffer.cell_mut((1, 0)).unwrap().set_symbol(icons::BRANCH);
        colorize_icons(&mut buffer, &theme);
        assert_eq!(buffer.cell((0, 0)).unwrap().fg, theme.color("yellow"));
        assert_eq!(buffer.cell((1, 0)).unwrap().fg, theme.color("lavender"));
    }
}
