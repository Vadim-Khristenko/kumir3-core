//! Отрисовка экрана консоли.
//!
//! Раскладка: полоса заголовка, рабочая область, строка ввода, полоса клавиш.
//! Рабочая область — вывод и, если панель не скрыта, колонка справа, где
//! переменные и алгоритмы показаны одновременно, а не по очереди: чтобы
//! увидеть и то и другое, переключаться не нужно.
//!
//! Рамки скруглённые и приглушённые, а внимание уводится на содержимое —
//! в консоли смотрят на вывод, а не на её обрамление.

use ratatui::prelude::*;
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, Padding, Paragraph, Scrollbar,
    ScrollbarOrientation, ScrollbarState, Wrap,
};
use shared::types::Value;

use super::repl::{Panel, ReplApp};
use super::syntax;
use super::theme;

/// Ширина боковой колонки. Хватает имени переменной и короткого значения.
const PANEL_WIDTH: u16 = 32;
/// Ниже этой ширины боковая колонка съедала бы вывод — она прячется сама.
const MIN_WIDTH_FOR_PANEL: u16 = 76;

pub(crate) fn draw(frame: &mut Frame, app: &ReplApp) {
    let rows = Layout::vertical([
        Constraint::Length(1), // полоса заголовка
        Constraint::Min(3),    // вывод и панель
        Constraint::Length(3), // ввод
        Constraint::Length(1), // полоса клавиш
    ])
    .split(frame.area());

    header(frame, rows[0], app);

    let show_panel = app.panel() != Panel::Hidden && rows[1].width >= MIN_WIDTH_FOR_PANEL;
    let body = if show_panel {
        Layout::horizontal([Constraint::Min(30), Constraint::Length(PANEL_WIDTH)]).split(rows[1])
    } else {
        Layout::horizontal([Constraint::Percentage(100)]).split(rows[1])
    };

    output(frame, body[0], app);
    if show_panel {
        side_panel(frame, body[1], app);
    }

    input(frame, rows[2], app);
    keys(frame, rows[3], app);

    if app.help_visible() {
        help(frame);
    }
}

/// Скруглённая рамка с внутренним отступом и заголовком.
fn panel_block(title: &str, active: bool) -> Block<'_> {
    Block::default()
        .title(Span::styled(format!(" {title} "), theme::title(active)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border(active))
        .padding(Padding::horizontal(1))
}

// ---------------------------------------------------------------------------
//                            ПОЛОСА ЗАГОЛОВКА
// ---------------------------------------------------------------------------

fn header(frame: &mut Frame, area: Rect, app: &ReplApp) {
    let mut left = vec![
        Span::styled(
            " Кумир 3 ",
            Style::default()
                .fg(Color::Black)
                .bg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  консоль", Style::default().fg(theme::MUTED)),
    ];
    if app.debug() {
        left.push(Span::styled(
            "  ·  отладка",
            Style::default().fg(theme::WARN),
        ));
    }

    // Справа — то, что меняется по ходу работы: сколько всего определено и
    // ждём ли закрытия конструкции.
    let mut right = Vec::new();
    if app.depth() > 0 {
        right.push(Span::styled(
            format!("не закрыто: {}  ", app.depth()),
            Style::default().fg(theme::WARN),
        ));
    }
    let env = app.interpreter_env();
    right.push(Span::styled(
        format!(
            "перем.: {}   алг.: {} ",
            env.globals_snapshot().len(),
            env.algorithm_names().len()
        ),
        Style::default().fg(theme::MUTED),
    ));

    let columns =
        Layout::horizontal([Constraint::Min(10), Constraint::Length(width_of(&right))]).split(area);
    frame.render_widget(Line::from(left), columns[0]);
    frame.render_widget(Line::from(right).alignment(Alignment::Right), columns[1]);
}

fn width_of(spans: &[Span]) -> u16 {
    spans
        .iter()
        .map(|s| s.content.chars().count() as u16)
        .sum::<u16>()
}

// ---------------------------------------------------------------------------
//                                 ВЫВОД
// ---------------------------------------------------------------------------

fn output(frame: &mut Frame, area: Rect, app: &ReplApp) {
    let scrolled = app.scroll_back() > 0;
    let title = if scrolled {
        "Вывод (прокручен)"
    } else {
        "Вывод"
    };
    let block = panel_block(title, !scrolled);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = app.output();
    if lines.is_empty() {
        frame.render_widget(welcome(), inner);
        return;
    }

    let visible = inner.height as usize;
    let total = lines.len();
    // Показывается хвост: прокрутка отсчитывается от конца, поэтому новые
    // строки появляются сразу и «догонять» их не нужно.
    let end = total.saturating_sub(app.scroll_back());
    let start = end.saturating_sub(visible);

    let items: Vec<ListItem> = lines[start..end]
        .iter()
        .map(|line| ListItem::new(line.to_styled_line()))
        .collect();
    frame.render_widget(List::new(items), inner);

    if total > visible {
        let mut state = ScrollbarState::new(total).position(start);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme::MUTED))
                .track_symbol(None)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                horizontal: 0,
                vertical: 1,
            }),
            &mut state,
        );
    }
}

/// Пустой экран — не пустой: он показывает, с чего начать.
fn welcome<'a>() -> Paragraph<'a> {
    let dim = Style::default().fg(theme::MUTED);
    let code = Style::default().fg(theme::KEYWORD);
    Paragraph::new(vec![
        Line::raw(""),
        Line::from(Span::styled(
            "Наберите программу на Кумире и нажмите Enter.",
            Style::default().fg(Color::White),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  например   ", dim),
            Span::styled("вывод 2 + 3", code),
        ]),
        Line::from(vec![
            Span::styled("             ", dim),
            Span::styled("цел счётчик := 5", code),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "  F1 — помощь · Tab — дополнить имя · F2 — боковая панель",
            dim,
        )),
    ])
}

// ---------------------------------------------------------------------------
//                            БОКОВАЯ КОЛОНКА
// ---------------------------------------------------------------------------

fn side_panel(frame: &mut Frame, area: Rect, app: &ReplApp) {
    let env = app.interpreter_env();
    let variables = env.globals_snapshot();
    let algorithms = env.algorithm_names();

    // Переменные меняются чаще и заслуживают больше места, но алгоритмам
    // всегда остаётся видимый минимум.
    let rows = Layout::vertical([
        Constraint::Min(4),
        Constraint::Length(algorithms_height(algorithms.len(), area.height)),
    ])
    .split(area);

    let block = panel_block("Переменные", false);
    let inner = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);

    let lines: Vec<Line> = if variables.is_empty() {
        vec![Line::from(Span::styled(
            "пока ничего не объявлено",
            Style::default().fg(theme::MUTED),
        ))]
    } else {
        variables
            .iter()
            .map(|(name, value, is_const)| {
                let name_style = if *is_const {
                    Style::default()
                        .fg(theme::TYPE)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(vec![
                    Span::styled(name.clone(), name_style),
                    Span::styled(" = ", Style::default().fg(theme::MUTED)),
                    Span::styled(
                        short_value(value, inner.width),
                        Style::default().fg(theme::OK),
                    ),
                ])
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines), inner);

    let block = panel_block("Алгоритмы", false);
    let inner = block.inner(rows[1]);
    frame.render_widget(block, rows[1]);

    let lines: Vec<Line> = if algorithms.is_empty() {
        vec![Line::from(Span::styled(
            "не объявлены",
            Style::default().fg(theme::MUTED),
        ))]
    } else {
        algorithms
            .iter()
            .map(|name| {
                Line::from(Span::styled(
                    name.clone(),
                    Style::default().fg(theme::BUILTIN),
                ))
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Высота нижней части колонки: по содержимому, но не больше половины.
fn algorithms_height(count: usize, available: u16) -> u16 {
    let needed = count.max(1) as u16 + 2; // строки плюс рамка
    needed.min(available / 2).max(3)
}

/// Значение для колонки: длинное обрезается по её ширине.
fn short_value(value: &Value, width: u16) -> String {
    // Из ширины вычитается место под имя и знак равенства.
    let budget = width.saturating_sub(12).max(6) as usize;
    let text = value.to_string();
    if text.chars().count() > budget {
        let head: String = text.chars().take(budget.saturating_sub(1)).collect();
        format!("{head}…")
    } else {
        text
    }
}

// ---------------------------------------------------------------------------
//                                  ВВОД
// ---------------------------------------------------------------------------

fn input(frame: &mut Frame, area: Rect, app: &ReplApp) {
    let continued = app.depth() > 0;
    let (prompt, color, title) = if continued {
        ("   ⋮ ", theme::WARN, "Продолжение")
    } else {
        (" › ", theme::OK, "Ввод")
    };

    let block = Block::default()
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(color),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut spans = vec![Span::styled(
        prompt,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )];
    spans.extend(syntax::highlight(&app.input_text()));
    frame.render_widget(Paragraph::new(Line::from(spans)), inner);

    frame.set_cursor_position((
        inner.x + prompt.chars().count() as u16 + app.input_cursor() as u16,
        inner.y,
    ));
}

// ---------------------------------------------------------------------------
//                             ПОЛОСА КЛАВИШ
// ---------------------------------------------------------------------------

fn keys(frame: &mut Frame, area: Rect, app: &ReplApp) {
    let panel_label = if app.panel() == Panel::Hidden {
        "панель"
    } else {
        "скрыть"
    };
    let keys: [(&str, &str); 6] = [
        ("F1", "помощь"),
        ("F2", panel_label),
        ("Tab", "дополнить"),
        ("^L", "очистить"),
        ("^C", "прервать"),
        ("^D", "выход"),
    ];

    let mut spans = Vec::new();
    for (key, label) in keys {
        spans.push(Span::styled(format!(" {key} "), theme::key_hint()));
        spans.push(Span::styled(format!("{label} ",), theme::key_label()));
    }
    frame.render_widget(Line::from(spans), area);
}

// ---------------------------------------------------------------------------
//                                 ПОМОЩЬ
// ---------------------------------------------------------------------------

fn help(frame: &mut Frame) {
    let area = centered(70, 24, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(" Помощь ", theme::title(true)))
        .title_bottom(Span::styled(
            " любая клавиша — закрыть ",
            Style::default().fg(theme::MUTED),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border(true))
        .padding(Padding::horizontal(1));

    let section = |t: &str| {
        Line::from(Span::styled(
            t.to_string(),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ))
    };
    let row = |k: &str, v: &str| {
        Line::from(vec![
            Span::styled(format!("{k:<16}"), theme::key_hint()),
            Span::raw(v.to_string()),
        ])
    };

    let text = vec![
        section("Клавиши"),
        row("F1", "эта справка"),
        row("F2", "показать или скрыть боковую колонку"),
        row("Tab", "дополнить имя; ещё раз — следующий вариант"),
        row("↑ ↓", "история ввода"),
        row("Ctrl+← →", "перемещение по словам"),
        row("Ctrl+W U K", "удалить слово · до начала · до конца"),
        row("PgUp PgDn", "прокрутка вывода"),
        row("Ctrl+C", "прервать незакрытую конструкцию"),
        row("Ctrl+L", "очистить вывод"),
        row("Ctrl+D", "выход"),
        Line::raw(""),
        section("Команды"),
        row(".очистить", "убрать вывод; объявленное сохраняется"),
        row(".сброс", "забыть переменные и алгоритмы"),
        row(".загрузить ф", "выполнить файл .kum"),
        row(".отладка", "включить или выключить отладку"),
        row(".выход", "выход"),
    ];

    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

/// Прямоугольник заданного размера по центру экрана.
fn centered(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::types::Number;

    #[test]
    fn dlinnoe_znachenie_obrezaetsya_po_shirine_kolonki() {
        let long = Value::String("а".repeat(100));
        let shown = short_value(&long, PANEL_WIDTH);
        assert!(shown.chars().count() <= PANEL_WIDTH as usize);
        assert!(shown.ends_with('…'), "обрезка должна быть видна");
    }

    #[test]
    fn korotkoe_znachenie_ne_trogaetsya() {
        let value = Value::Number(Number::I64(42));
        assert_eq!(short_value(&value, PANEL_WIDTH), "42");
    }

    #[test]
    fn vysota_spiska_algoritmov_ostavlyaet_mesto_peremennym() {
        // Много алгоритмов не должны отобрать больше половины колонки.
        assert!(algorithms_height(100, 20) <= 10);
        // И даже без алгоритмов остаётся видимая рамка.
        assert!(algorithms_height(0, 20) >= 3);
    }
}
