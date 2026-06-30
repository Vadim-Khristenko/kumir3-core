use crate::terminal::{init_terminal, restore_terminal};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use std::io::Stdout;
use std::time::Duration;

pub(crate) fn run_info_tui() -> Result<(), String> {
    let mut terminal = init_terminal()?;
    let result = run_info_loop(&mut terminal);
    restore_terminal()?;
    result
}

fn run_info_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), String> {
    loop {
        terminal.draw(draw_info).map_err(|e| e.to_string())?;

        if event::poll(Duration::from_millis(100)).map_err(|e| e.to_string())?
            && let Event::Key(key) = event::read().map_err(|e| e.to_string())?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => break,
                _ => {}
            }
        }
    }
    Ok(())
}

fn draw_info(frame: &mut Frame) {
    let area = frame.area();

    let block = Block::default()
        .title(" KUMIR 3 INTERPRETER ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Min(1),
        ])
        .split(inner);

    // Заголовок
    let title = Paragraph::new(vec![
        Line::from("╔══════════════════════════════════════════════════════════╗")
            .style(Style::default().fg(Color::Cyan)),
        Line::from("║              KUMIR 3 - Интерпретатор                     ║")
            .style(Style::default().fg(Color::Cyan).bold()),
        Line::from("╚══════════════════════════════════════════════════════════╝")
            .style(Style::default().fg(Color::Cyan)),
    ]);
    frame.render_widget(title, chunks[0]);

    // Версия
    let version = Paragraph::new(vec![Line::from(vec![
        Span::styled("  Версия: ", Style::default().fg(Color::White).bold()),
        Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(Color::Green)),
    ])]);
    frame.render_widget(version, chunks[1]);

    // Возможности
    let features = Paragraph::new(vec![
        Line::from(Span::styled(
            "  Возможности:",
            Style::default().fg(Color::White).bold(),
        )),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Базовые типы: цел, вещ, лит, лог, сим"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Массивы и таблицы"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Алгоритмы с параметрами"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("ООП: классы, объекты, методы"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Перечисления и сопоставление с образцом"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Обработка исключений (попытка/перехват)"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Импорт модулей (.kum файлы)"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Стандартные библиотеки"),
        ]),
    ]);
    frame.render_widget(features, chunks[2]);

    // Использование
    let usage = Paragraph::new(vec![
        Line::from(Span::styled(
            "  Использование:",
            Style::default().fg(Color::White).bold(),
        )),
        Line::from(vec![
            Span::styled(
                "    kumir3-interpreter ",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("<файл.kum>", Style::default().fg(Color::Cyan)),
            Span::raw("     Выполнить файл"),
        ]),
        Line::from(vec![
            Span::styled(
                "    kumir3-interpreter ",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("repl", Style::default().fg(Color::Cyan)),
            Span::raw("             Интерактивный режим"),
        ]),
        Line::from(vec![
            Span::styled(
                "    kumir3-interpreter ",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("check <файл>", Style::default().fg(Color::Cyan)),
            Span::raw("   Проверить синтаксис"),
        ]),
        Line::from(vec![
            Span::styled(
                "    kumir3-interpreter ",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("--help", Style::default().fg(Color::Cyan)),
            Span::raw("           Помощь"),
        ]),
    ]);
    frame.render_widget(usage, chunks[3]);

    // Подсказка
    let hint = Paragraph::new(Line::from(Span::styled(
        "  Нажмите Enter или Q для выхода",
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(hint, chunks[4]);
}
