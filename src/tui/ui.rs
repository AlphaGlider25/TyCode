use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, Padding, Paragraph, Scrollbar,
    ScrollbarOrientation, ScrollbarState, Wrap,
};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use super::app::{App, AppMode, ChatMessage, ConfirmState, ModelSelectState, ProviderSelectState, SettingsState};
use super::markdown;

// ── Color palette ────────────────────────────────────────────────────────────

const BG_BASE: Color     = Color::Rgb(13, 13, 17);
const BG_POPUP: Color    = Color::Rgb(18, 18, 28);

const BORDER_DIM: Color    = Color::Rgb(45, 45, 65);
const BORDER_ACCENT: Color = Color::Rgb(90, 60, 160);
const BORDER_BRIGHT: Color = Color::Rgb(120, 90, 200);

const TEXT_PRIMARY: Color   = Color::Rgb(220, 215, 235);
const TEXT_SECONDARY: Color = Color::Rgb(140, 130, 170);
const TEXT_MUTED: Color     = Color::Rgb(80, 75, 105);

const ACCENT_PRIMARY: Color = Color::Rgb(160, 100, 240);
const ACCENT_SOFT: Color    = Color::Rgb(100, 70, 180);
const ACCENT_DIM: Color     = Color::Rgb(60, 40, 120);

const SUCCESS: Color  = Color::Rgb(80, 200, 120);
const ERROR_COLOR: Color = Color::Rgb(240, 80, 80);
const INFO: Color     = Color::Rgb(80, 170, 240);

const USER_COLOR: Color   = Color::Rgb(100, 200, 255);
const TOOL_COLOR: Color   = Color::Rgb(240, 180, 60);
const TOOL_SUCCESS: Color = SUCCESS;
const TOOL_FAIL: Color    = ERROR_COLOR;

// Legacy aliases kept for overlays
const HEADER_BG: Color   = Color::Rgb(15, 13, 22);
const STATUS_BG: Color   = Color::Rgb(15, 13, 22);
const BORDER_COLOR: Color= BORDER_DIM;
const DIM: Color         = TEXT_MUTED;
const POPUP_BG: Color    = BG_POPUP;
const POPUP_BORDER: Color= BORDER_ACCENT;
const POPUP_TITLE: Color = Color::Rgb(180, 150, 255);

// ── Main render ──────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Dynamic input height: grows with content (and newlines) up to 8 rows.
    let inner_width = size.width.saturating_sub(2) as usize;
    let input_rows = if app.input.is_empty() || inner_width == 0 {
        1u16
    } else {
        // Count explicit newlines plus display-width wrapping.
        let mut rows: usize = 0;
        for segment in app.input.split('\n') {
            let w = UnicodeWidthStr::width(segment);
            rows += ((w.max(1) - 1) / inner_width) + 1;
        }
        rows.min(8) as u16
    };
    let input_height = (input_rows + 2).max(3); // +2 for borders

    // Layout: header (1), chat (flex), input (dynamic), status (1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),            // header
            Constraint::Min(5),               // chat
            Constraint::Length(input_height), // input
            Constraint::Length(1),            // status
        ])
        .split(size);

    render_header(f, app, chunks[0]);
    render_chat(f, app, chunks[1]);
    render_input(f, app, chunks[2]);
    render_status(f, app, chunks[3]);

    // Overlay screens
    match &app.mode {
        AppMode::Settings(state) => render_settings_overlay(f, state.clone(), size),
        AppMode::Help => render_help_overlay(f, size),
        AppMode::ModelSelect(state) => render_model_select_overlay(f, state.clone(), size),
        AppMode::ProviderSelect(state) => render_provider_select_overlay(f, state.clone(), size),
        AppMode::Confirm(state) => render_confirm_overlay(f, state.clone(), size),
        _ => {}
    }
}

// ── Header ───────────────────────────────────────────────────────────────────

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let model_info = app.config.provider_display();
    let branch_str = app.git_branch.as_deref()
        .map(|b| format!("  [{b}]"))
        .unwrap_or_default();
    let cwd_short = shorten_path(&app.cwd, (area.width as usize).saturating_sub(model_info.len() + branch_str.len() + 20));
    let right_part = format!("{}{}  ", cwd_short, branch_str);

    let badge = " ◈ TyCode ";
    let mid   = format!("  {}  ", model_info);
    let total = badge.len() + mid.len() + right_part.len();
    let gap   = (area.width as usize).saturating_sub(total);

    let header = Line::from(vec![
        Span::styled(badge, Style::default().fg(Color::White).bg(ACCENT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled(mid,   Style::default().fg(TEXT_SECONDARY).bg(HEADER_BG)),
        Span::styled(" ".repeat(gap), Style::default().bg(HEADER_BG)),
        Span::styled(cwd_short.to_string(), Style::default().fg(TEXT_MUTED).bg(HEADER_BG)),
        Span::styled(branch_str, Style::default().fg(ACCENT_SOFT).bg(HEADER_BG)),
        Span::styled("  ", Style::default().bg(HEADER_BG)),
    ]);

    f.render_widget(Paragraph::new(header).style(Style::default().bg(HEADER_BG)), area);
}

// ── Chat area ────────────────────────────────────────────────────────────────

fn render_chat(f: &mut Frame, app: &mut App, area: Rect) {
    let mut all_lines: Vec<Line<'static>> = Vec::new();

    for (msg_idx, msg) in app.messages.iter().enumerate() {
        let msg_start_line = all_lines.len();
        let is_selected = app.selected_message == Some(msg_idx);

        match msg {
            ChatMessage::User { text, timestamp } => {
                all_lines.push(Line::from(""));
                let ts = timestamp.format("%H:%M").to_string();
                let ts_pad = (area.width as usize).saturating_sub(8 + ts.len());
                all_lines.push(Line::from(vec![
                    Span::styled("  ❯ ", Style::default().fg(ACCENT_PRIMARY)),
                    Span::styled("You", Style::default().fg(USER_COLOR).add_modifier(Modifier::BOLD)),
                    Span::styled(" ".repeat(ts_pad), Style::default()),
                    Span::styled(ts, Style::default().fg(TEXT_MUTED)),
                ]));
                for line in text.lines() {
                    all_lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(line.to_string(), Style::default().fg(TEXT_PRIMARY)),
                    ]));
                }
            }
            ChatMessage::AssistantText { text, model, timestamp } => {
                all_lines.push(Line::from(""));
                let model_short = shorten_model(model);
                let ts = timestamp.format("%H:%M").to_string();
                let badge_len = 4 + model_short.len() + 2 + ts.len();
                let ts_pad = (area.width as usize).saturating_sub(badge_len);
                all_lines.push(Line::from(vec![
                    Span::styled("  ◈ ", Style::default().fg(ACCENT_PRIMARY)),
                    Span::styled(model_short, Style::default().fg(TEXT_SECONDARY).add_modifier(Modifier::BOLD)),
                    Span::styled(" ".repeat(ts_pad), Style::default()),
                    Span::styled(ts, Style::default().fg(TEXT_MUTED)),
                ]));
                let md_lines = markdown::markdown_to_lines(text, area.width);
                for line in md_lines {
                    let mut prefixed: Vec<Span<'static>> = vec![Span::raw("  ")];
                    prefixed.extend(line.spans);
                    all_lines.push(Line::from(prefixed));
                }
            }
            ChatMessage::AssistantLive(text) => {
                all_lines.push(Line::from(""));
                all_lines.push(Line::from(vec![
                    Span::styled("  ◈ ", Style::default().fg(ACCENT_PRIMARY)),
                    Span::styled("generating…", Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC)),
                ]));
                let md_lines = markdown::markdown_to_lines(text, area.width);
                for (i, line) in md_lines.iter().enumerate() {
                    let mut prefixed: Vec<Span<'static>> = vec![Span::raw("  ")];
                    prefixed.extend(line.spans.clone());
                    if i == md_lines.len() - 1 {
                        prefixed.push(Span::styled(
                            "▊",
                            Style::default().fg(ACCENT_PRIMARY).add_modifier(Modifier::SLOW_BLINK),
                        ));
                    }
                    all_lines.push(Line::from(prefixed));
                }
            }
            ChatMessage::ToolCall { name, input_summary, success, output, expanded, timestamp } => {
                let (status_color, status_icon) = match success {
                    Some(true)  => (TOOL_SUCCESS, "✓"),
                    Some(false) => (TOOL_FAIL,    "✗"),
                    None        => (TOOL_COLOR,   "●"),
                };
                let toggle = if *expanded { "▼" } else { "▶" };
                let ts = timestamp.format("%H:%M").to_string();
                let header_left_len = 6 + name.len() + 1 + input_summary.len() + 2 + 1;
                let ts_pad = (area.width as usize).saturating_sub(header_left_len + ts.len());
                all_lines.push(Line::from(vec![
                    Span::styled(format!("  {toggle}  "), Style::default().fg(TEXT_MUTED)),
                    Span::styled(name.clone(), Style::default().fg(TOOL_COLOR).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("  {input_summary}"), Style::default().fg(TEXT_MUTED)),
                    Span::styled(" ".repeat(ts_pad.max(2)), Style::default()),
                    Span::styled(status_icon.to_string(), Style::default().fg(status_color)),
                    Span::styled("  ", Style::default()),
                    Span::styled(ts, Style::default().fg(TEXT_MUTED)),
                ]));
                if *expanded {
                    if let Some(out) = output {
                        all_lines.push(Line::from(vec![
                            Span::styled(
                                format!("     {}", "─".repeat((area.width as usize).saturating_sub(7))),
                                Style::default().fg(BORDER_DIM),
                            ),
                        ]));
                        for line in out.lines() {
                            all_lines.push(Line::from(vec![
                                Span::styled(
                                    format!("     {}", line),
                                    Style::default().fg(TEXT_SECONDARY),
                                ),
                            ]));
                        }
                    }
                }
            }
            ChatMessage::System(text) => {
                for line in text.lines() {
                    all_lines.push(Line::from(vec![
                        Span::styled("  ℹ ", Style::default().fg(INFO)),
                        Span::styled(line.to_string(), Style::default().fg(TEXT_SECONDARY)),
                    ]));
                }
            }
            ChatMessage::Error(text) => {
                all_lines.push(Line::from(vec![
                    Span::styled("  ✖ ", Style::default().fg(ERROR_COLOR)),
                    Span::styled(text.clone(), Style::default().fg(ERROR_COLOR).add_modifier(Modifier::BOLD)),
                ]));
            }
        }

        // Add selection highlight gutter to first line of selected message
        if is_selected && msg_start_line < all_lines.len() {
            let line = all_lines[msg_start_line].clone();
            let gutter = Span::styled("▌ ", Style::default().fg(Color::Rgb(100, 100, 160)));
            let mut new_spans = vec![gutter];
            new_spans.extend(line.spans);
            all_lines[msg_start_line] = Line::from(new_spans);
        }
    }

    // Thinking indicator (only when not streaming).
    let is_streaming = app.messages.last().map(|m| matches!(m, ChatMessage::AssistantLive(_))).unwrap_or(false);
    if matches!(app.mode, AppMode::Processing | AppMode::Confirm(_)) && !is_streaming {
        let dots = ".".repeat((app.thinking_dots % 4) + 1);
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(vec![
            Span::styled("  ◈ ", Style::default().fg(ACCENT_PRIMARY)),
            Span::styled(
                format!("thinking{dots}"),
                Style::default().fg(TEXT_MUTED).add_modifier(Modifier::ITALIC),
            ),
        ]));
    }

    let content_width = area.width.saturating_sub(1);
    let total_lines = compute_wrapped_height(&all_lines, content_width);
    let visible_height = area.height;

    // Auto-scroll: sentinel u16::MAX → scroll to actual bottom.
    if app.scroll_offset == u16::MAX {
        app.scroll_offset = total_lines.saturating_sub(visible_height);
    }
    app.scroll_offset = app
        .scroll_offset
        .min(total_lines.saturating_sub(visible_height));

    let text = Text::from(all_lines);
    let chat_widget = Paragraph::new(text)
        .scroll((app.scroll_offset, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(chat_widget, area);

    if total_lines > visible_height {
        let mut scrollbar_state =
            ScrollbarState::new(total_lines as usize)
                .position(app.scroll_offset as usize)
                .viewport_content_length(visible_height as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(BORDER_DIM));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

// ── Input area ───────────────────────────────────────────────────────────────

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let is_processing = matches!(app.mode, AppMode::Processing | AppMode::Confirm(_));

    let border_color = if is_processing { ACCENT_DIM } else { BORDER_ACCENT };

    // Count newlines for the line badge.
    let line_count = app.input.chars().filter(|&c| c == '\n').count() + 1;
    let line_badge = if line_count > 1 {
        format!(" · {} lines", line_count)
    } else {
        String::new()
    };

    let title = if is_processing {
        if app.input_queue.is_empty() {
            " Processing… (ESC to clear queue) ".to_string()
        } else {
            format!(" Processing… {} queued ", app.input_queue.len())
        }
    } else {
        let history_pos = app.get_history_position_text();
        format!(" >{}{} ", history_pos, line_badge)
    };

    let title_color = if is_processing { TEXT_MUTED } else { TEXT_SECONDARY };
    let hint = " ⌥↵ newline · ↑↓ history · ?=help ";

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .title_style(Style::default().fg(title_color))
        .title_bottom(Line::from(Span::styled(hint, Style::default().fg(TEXT_MUTED))));

    let input_widget = if app.input.is_empty() && !is_processing {
        Paragraph::new(Line::from(Span::styled(
            "Type your prompt…",
            Style::default().fg(TEXT_MUTED),
        )))
        .block(input_block)
        .style(Style::default().fg(TEXT_PRIMARY))
        .wrap(Wrap { trim: false })
    } else {
        let input_text = if app.input.contains('\n') {
            Line::from(Span::raw(app.input.clone()))
        } else if let Some(hint) = get_command_hints(&app.input) {
            let typed = &app.input;
            let untyped = &hint[typed.len()..];
            Line::from(vec![
                Span::styled(typed.to_string(), Style::default().fg(TEXT_PRIMARY)),
                Span::styled(untyped.to_string(), Style::default().fg(TEXT_MUTED)),
            ])
        } else {
            Line::from(Span::raw(app.input.clone()))
        };
        Paragraph::new(input_text)
            .block(input_block)
            .style(Style::default().fg(TEXT_PRIMARY))
            .wrap(Wrap { trim: false })
    };

    f.render_widget(input_widget, area);

    // Cursor position — account for physical newlines and display wrapping.
    if !matches!(app.mode, AppMode::Settings(_) | AppMode::Help | AppMode::ModelSelect(_) | AppMode::ProviderSelect(_)) {
        let inner_width = area.width.saturating_sub(2) as usize;
        if inner_width > 0 {
            let text_before = &app.input[..app.cursor_pos.min(app.input.len())];
            let segments: Vec<&str> = text_before.split('\n').collect();
            let mut cursor_row: usize = 0;
            let mut cursor_col: usize = 0;
            for (i, segment) in segments.iter().enumerate() {
                let w = UnicodeWidthStr::width(*segment);
                if i < segments.len() - 1 {
                    cursor_row += w / inner_width + 1;
                } else {
                    cursor_row += w / inner_width;
                    cursor_col = w % inner_width;
                }
            }

            let cursor_x = area.x + 1 + cursor_col as u16;
            let cursor_y = area.y + 1 + cursor_row as u16;
            if cursor_x < area.x + area.width - 1 && cursor_y < area.y + area.height - 1 {
                f.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}

// ── Status bar ───────────────────────────────────────────────────────────────

fn fmt_k(n: u32) -> String {
    if n >= 1000 {
        format!("{:.1}K", n as f32 / 1000.0)
    } else {
        n.to_string()
    }
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    // Context token estimate with color.
    let ctx_estimate = app.context_token_estimate();
    let ctx_color = if ctx_estimate > 90_000 {
        ERROR_COLOR
    } else if ctx_estimate > 60_000 {
        Color::Rgb(220, 170, 50)
    } else {
        SUCCESS
    };

    let ctx_str = fmt_k(ctx_estimate as u32);
    let turn_str = if app.last_turn_in > 0 || app.last_turn_out > 0 {
        format!("  ↑{}  ↓{}", fmt_k(app.last_turn_in), fmt_k(app.last_turn_out))
    } else {
        String::new()
    };
    let ctx_display = format!(" ctx {}K{}", ctx_str, turn_str);

    let status = &app.status_message;
    let (status_str, status_color) = if let Some(ts) = app.status_timestamp {
        let elapsed = ts.elapsed().as_millis() as u64;
        let remaining = (3000u64).saturating_sub(elapsed);
        let dot = if remaining > 2000 { "●" } else if remaining > 1000 { "○" } else { "·" };
        (format!("{}  {}", dot, status), SUCCESS)
    } else if status.is_empty() || status == "Ready" {
        ("● Ready".to_string(), SUCCESS)
    } else {
        (format!("● {}", status), TEXT_SECONDARY)
    };

    let sep = Span::styled("  │  ", Style::default().fg(BORDER_DIM).bg(STATUS_BG));
    let help_hint = Span::styled("?=help ", Style::default().fg(TEXT_MUTED).bg(STATUS_BG));

    // Build left side: ctx info
    let left = Span::styled(ctx_display.clone(), Style::default().fg(ctx_color).bg(STATUS_BG));
    // Build right side: status + help
    let right_text = format!("  {}  ", status_str);
    let right_len = right_text.len() + 7; // "?=help " = 7
    let left_len = ctx_display.len() + 3; // sep = 3
    let gap = (area.width as usize).saturating_sub(left_len + right_len);

    let status_line = Line::from(vec![
        left,
        sep,
        Span::styled(" ".repeat(gap), Style::default().bg(STATUS_BG)),
        Span::styled(right_text, Style::default().fg(status_color).bg(STATUS_BG)),
        help_hint,
    ]);

    f.render_widget(Paragraph::new(status_line).style(Style::default().bg(STATUS_BG)), area);
}

// ── Confirm overlay ──────────────────────────────────────────────────────────

fn render_confirm_overlay(f: &mut Frame, state: ConfirmState, area: Rect) {
    let width = 70u16.min(area.width.saturating_sub(4));
    let height = 10u16.min(area.height.saturating_sub(4));
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let popup_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" ⚠  Dangerous Command ")
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .title_bottom(Line::from(vec![
            Span::styled(" Y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("/Enter=Allow  ", Style::default().fg(Color::DarkGray)),
            Span::styled("N", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled("/Esc=Deny ", Style::default().fg(Color::DarkGray)),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .padding(Padding::new(1, 1, 1, 1))
        .style(Style::default().bg(Color::Rgb(30, 10, 10)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let lines = vec![
        Line::from(vec![
            Span::styled("Reason: ", Style::default().fg(Color::DarkGray)),
            Span::styled(state.reason.clone(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Command: ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  {}", state.command),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Allow this command to run?",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
        ]),
    ];

    let widget = Paragraph::new(lines)
        .style(Style::default().bg(Color::Rgb(30, 10, 10)))
        .wrap(Wrap { trim: false });
    f.render_widget(widget, inner);
}

// ── Settings overlay ─────────────────────────────────────────────────────────

fn render_settings_overlay(f: &mut Frame, state: SettingsState, area: Rect) {
    let width = 72u16.min(area.width.saturating_sub(4));
    let height = (state.fields.len() as u16 + 8).min(area.height.saturating_sub(4));
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let popup_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" ⚙  Settings ")
        .title_style(Style::default().fg(POPUP_TITLE).add_modifier(Modifier::BOLD))
        .title_bottom(Line::from(vec![
            Span::styled(" Enter", Style::default().fg(Color::Cyan)),
            Span::styled("=edit/pick  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab", Style::default().fg(Color::Cyan)),
            Span::styled("=next  ", Style::default().fg(Color::DarkGray)),
            Span::styled("S", Style::default().fg(Color::Cyan)),
            Span::styled("=save  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::styled("=close ", Style::default().fg(Color::DarkGray)),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(POPUP_BORDER))
        .padding(Padding::new(1, 1, 1, 1))
        .style(Style::default().bg(POPUP_BG));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (i, field) in state.fields.iter().enumerate() {
        let is_selected = i == state.selected_field;
        let cursor = if is_selected { "▶ " } else { "  " };

        let label_style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(60, 100, 200))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(180, 180, 220))
        };

        let value_display = if field.key == "provider" {
            format!("{}  ◀▶", field.value)
        } else if field.key.contains("key") && !field.value.is_empty() && !(state.editing && is_selected) {
            let visible = field.value.len().min(4);
            format!("{}...", &field.value[..visible])
        } else {
            field.value.clone()
        };

        let value_style = if is_selected && state.editing {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::UNDERLINED)
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(Color::White).bg(Color::Rgb(60, 100, 200))
        } else {
            Style::default().fg(Color::Rgb(100, 220, 130))
        };

        lines.push(Line::from(vec![
            Span::styled(cursor.to_string(), label_style),
            Span::styled(format!("{:<22}", field.label), label_style),
            Span::styled(value_display, value_style),
        ]));
    }

    let settings_widget = Paragraph::new(lines).style(Style::default().bg(POPUP_BG));
    f.render_widget(settings_widget, inner);
}

// ── Help overlay ─────────────────────────────────────────────────────────────

fn render_help_overlay(f: &mut Frame, area: Rect) {
    let width = 70u16.min(area.width.saturating_sub(4));
    let height = 32u16.min(area.height.saturating_sub(4));
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let popup_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" ?  Help ")
        .title_style(Style::default().fg(POPUP_TITLE).add_modifier(Modifier::BOLD))
        .title_bottom(Line::from(vec![
            Span::styled(" Esc", Style::default().fg(Color::Cyan)),
            Span::styled("=close ", Style::default().fg(Color::DarkGray)),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(POPUP_BORDER))
        .padding(Padding::new(1, 1, 1, 1))
        .style(Style::default().bg(POPUP_BG));

    let cmd = Style::default().fg(Color::Rgb(130, 200, 255));
    let desc = Style::default().fg(Color::Rgb(200, 200, 220));
    let section = Style::default().fg(Color::Rgb(255, 200, 80)).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let help_text = vec![
        Line::from(vec![Span::styled("  Commands", section)]),
        Line::from(""),
        Line::from(vec![Span::styled("  /help        ", cmd), Span::styled("Show this help screen", desc)]),
        Line::from(vec![Span::styled("  /model       ", cmd), Span::styled("Switch model (fetches available models)", desc)]),
        Line::from(vec![Span::styled("  /provider    ", cmd), Span::styled("Pick provider from list", desc)]),
        Line::from(vec![Span::styled("  /settings    ", cmd), Span::styled("Open settings editor", desc)]),
        Line::from(vec![Span::styled("  /clear       ", cmd), Span::styled("Clear chat + agent context (re-injects project files)", desc)]),
        Line::from(vec![Span::styled("  /cache       ", cmd), Span::styled("Reset agent memory only (keep chat display)", desc)]),
        Line::from(vec![Span::styled("  /copy        ", cmd), Span::styled("Copy last 50 messages to /tmp/tycode_copy.txt", desc)]),
        Line::from(vec![Span::styled("  /system      ", cmd), Span::styled("Set custom system prompt", desc)]),
        Line::from(vec![Span::styled("  /import      ", cmd), Span::styled("/import <path>  inject file into agent context", desc)]),
        Line::from(""),
        Line::from(vec![Span::styled("  Keyboard & Navigation", section)]),
        Line::from(""),
        Line::from(vec![Span::styled("  Enter             ", cmd), Span::styled("Send message", desc)]),
        Line::from(vec![Span::styled("  Shift/Alt+Enter   ", cmd), Span::styled("Insert newline (multiline input)", desc)]),
        Line::from(vec![Span::styled("  Ctrl+Backspace    ", cmd), Span::styled("Delete previous word", desc)]),
        Line::from(vec![Span::styled("  Ctrl+C            ", cmd), Span::styled("Cancel/clear (×2 within 2s to quit)", desc)]),
        Line::from(vec![Span::styled("  Up / Down         ", cmd), Span::styled("Navigate input history", desc)]),
        Line::from(vec![Span::styled("  PgUp / PgDown     ", cmd), Span::styled("Scroll chat (10 lines)", desc)]),
        Line::from(vec![Span::styled("  Shift+Drag        ", cmd), Span::styled("Select text (terminal native)", desc)]),
        Line::from(vec![Span::styled("  Ctrl+Shift+C      ", cmd), Span::styled("Copy selected text (terminal native)", desc)]),
        Line::from(vec![Span::styled("  Ctrl+Home         ", cmd), Span::styled("Jump to top of chat", desc)]),
        Line::from(vec![Span::styled("  Ctrl+End          ", cmd), Span::styled("Jump to bottom of chat", desc)]),
        Line::from(vec![Span::styled("  Tab               ", cmd), Span::styled("Auto-complete slash commands", desc)]),
        Line::from(vec![Span::styled("  Esc               ", cmd), Span::styled("Close overlay / clear input", desc)]),
        Line::from(""),
        Line::from(vec![Span::styled("  While Processing", section)]),
        Line::from(""),
        Line::from(vec![Span::styled("  Enter             ", cmd), Span::styled("Queue additional instructions", desc)]),
        Line::from(vec![Span::styled("  Shift/Alt+Enter   ", cmd), Span::styled("Insert newline in queued message", desc)]),
        Line::from(vec![Span::styled("  Esc               ", cmd), Span::styled("Clear queue (agent task continues)", desc)]),
        Line::from(""),
        Line::from(vec![Span::styled("  Dangerous commands prompt for Y/N confirmation before executing.", dim)]),
    ];

    let help_widget = Paragraph::new(help_text)
        .block(block)
        .style(Style::default().bg(POPUP_BG));
    f.render_widget(help_widget, popup_area);
}

// ── Model select overlay ─────────────────────────────────────────────────────

fn render_model_select_overlay(f: &mut Frame, state: ModelSelectState, area: Rect) {
    let width = 55u16.min(area.width.saturating_sub(4));
    let height = (state.models.len() as u16 + 6)
        .min(area.height.saturating_sub(4))
        .max(8);
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let popup_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup_area);

    let title = if state.loading { " ◌  Loading models... " } else { " ◈  Select Model " };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(POPUP_TITLE).add_modifier(Modifier::BOLD))
        .title_bottom(if state.loading {
            Line::from(vec![
                Span::styled(" ↵", Style::default().fg(Color::Yellow)),
                Span::styled("cancel ", Style::default().fg(Color::DarkGray)),
            ])
        } else {
            Line::from(vec![
                Span::styled(" ↑↓", Style::default().fg(Color::Yellow)),
                Span::styled("navigate  ", Style::default().fg(Color::DarkGray)),
                Span::styled("↵", Style::default().fg(Color::Yellow)),
                Span::styled("select  ", Style::default().fg(Color::DarkGray)),
                Span::styled("esc", Style::default().fg(Color::Yellow)),
                Span::styled("=exit ", Style::default().fg(Color::DarkGray)),
            ])
        })
        .borders(Borders::ALL)
        .border_style(Style::default().fg(POPUP_BORDER))
        .padding(Padding::new(1, 1, 1, 1))
        .style(Style::default().bg(POPUP_BG));

    if state.loading {
        let loading = Paragraph::new(Line::from(vec![
            Span::styled("  ⏳ ", Style::default().fg(Color::Yellow)),
            Span::styled("Fetching available models...", Style::default().fg(Color::DarkGray)),
        ]))
        .block(block)
        .style(Style::default().bg(POPUP_BG));
        f.render_widget(loading, popup_area);
        return;
    }

    let items: Vec<ListItem> = state.models.iter().enumerate().map(|(i, model)| {
        if i == state.selected {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("► {} ", model),
                    Style::default().fg(Color::White).bg(Color::Rgb(80, 120, 220)).add_modifier(Modifier::BOLD),
                ),
            ]))
        } else {
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {} ", model), Style::default().fg(Color::Rgb(180, 180, 220))),
            ]))
        }
    }).collect();

    let list = List::new(items).block(block).style(Style::default().bg(POPUP_BG));
    f.render_widget(list, popup_area);
}

// ── Provider select overlay ──────────────────────────────────────────────────

fn render_provider_select_overlay(f: &mut Frame, state: ProviderSelectState, area: Rect) {
    let width = 42u16.min(area.width.saturating_sub(4));
    let height = (state.providers.len() as u16 + 6)
        .min(area.height.saturating_sub(4))
        .max(8);
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let popup_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" ◈  Select Provider ")
        .title_style(Style::default().fg(POPUP_TITLE).add_modifier(Modifier::BOLD))
        .title_bottom(Line::from(vec![
            Span::styled(" ↑↓", Style::default().fg(Color::Yellow)),
            Span::styled("navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("↵", Style::default().fg(Color::Yellow)),
            Span::styled("select  ", Style::default().fg(Color::DarkGray)),
            Span::styled("esc", Style::default().fg(Color::Yellow)),
            Span::styled("=exit ", Style::default().fg(Color::DarkGray)),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(POPUP_BORDER))
        .padding(Padding::new(1, 1, 1, 1))
        .style(Style::default().bg(POPUP_BG));

    let items: Vec<ListItem> = state.providers.iter().enumerate().map(|(i, provider)| {
        if i == state.selected {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("► {} ", provider),
                    Style::default().fg(Color::White).bg(Color::Rgb(80, 120, 220)).add_modifier(Modifier::BOLD),
                ),
            ]))
        } else {
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {} ", provider), Style::default().fg(Color::Rgb(180, 180, 220))),
            ]))
        }
    }).collect();

    let list = List::new(items).block(block).style(Style::default().bg(POPUP_BG));
    f.render_widget(list, popup_area);
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn get_command_hints(input: &str) -> Option<&'static str> {
    let commands = [
        "/help", "/model", "/settings", "/clear", "/import", "/system", "/provider", "/exit",
    ];
    if input.starts_with('/') {
        for cmd in &commands {
            if cmd.starts_with(input) {
                return Some(cmd);
            }
        }
    }
    None
}

fn compute_wrapped_height(lines: &[Line], width: u16) -> u16 {
    if width == 0 {
        return lines.len() as u16;
    }
    let mut total: u16 = 0;
    for line in lines {
        let line_width: usize = line.spans.iter().map(|s| s.content.width()).sum();
        if line_width == 0 {
            total = total.saturating_add(1);
        } else {
            total = total.saturating_add(((line_width as u16).saturating_sub(1)) / width + 1);
        }
    }
    total
}

fn shorten_model(model: &str) -> String {
    // Strip common prefixes for compact badge display
    let s = model
        .trim_start_matches("claude-")
        .trim_start_matches("anthropic/claude-")
        .trim_start_matches("openai/")
        .trim_start_matches("gpt-")
        .trim_start_matches("gemini-");
    // Replace long version suffixes like "-20251001"
    let s = if let Some(pos) = s.rfind("-202") { &s[..pos] } else { s };
    s.to_string()
}

fn shorten_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    let home = dirs::home_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let shortened = if !home.is_empty() && path.starts_with(&home) {
        format!("~{}", &path[home.len()..])
    } else {
        path.to_string()
    };
    if shortened.len() <= max_len {
        shortened
    } else {
        format!("...{}", &shortened[shortened.len().saturating_sub(max_len - 3)..])
    }
}
