use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, Padding, Paragraph, Scrollbar,
    ScrollbarOrientation, ScrollbarState, Wrap,
};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use super::app::{App, AppMode, ChatMessage, ModelSelectState, ProviderSelectState, SettingsState};
use super::markdown;

// ── Colors ───────────────────────────────────────────────────────────────────

const USER_COLOR: Color = Color::Cyan;
const TOOL_COLOR: Color = Color::Yellow;
const TOOL_SUCCESS: Color = Color::Green;
const TOOL_FAIL: Color = Color::Red;
const ERROR_COLOR: Color = Color::Red;
const SYSTEM_COLOR: Color = Color::DarkGray;
const HEADER_BG: Color = Color::Rgb(30, 30, 50);
const STATUS_BG: Color = Color::Rgb(30, 30, 50);
const BORDER_COLOR: Color = Color::Rgb(80, 80, 100);
const DIM: Color = Color::DarkGray;

// ── Main render ──────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Dynamic input height: grows with content up to 6 rows.
    let inner_width = size.width.saturating_sub(2) as usize;
    let input_rows = if app.input.is_empty() || inner_width == 0 {
        1u16
    } else {
        let display_width = UnicodeWidthStr::width(app.input.as_str());
        (((display_width.saturating_sub(1)) / inner_width) + 1).min(6) as u16
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
        AppMode::Settings(state) => render_settings_overlay(f, state, size),
        AppMode::Help => render_help_overlay(f, size),
        AppMode::ModelSelect(state) => render_model_select_overlay(f, state, size),
        AppMode::ProviderSelect(state) => render_provider_select_overlay(f, state, size),
        _ => {}
    }
}

// ── Header ───────────────────────────────────────────────────────────────────

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let provider_info = app.config.provider_display();
    let cwd_short = shorten_path(&app.cwd, (area.width as usize).saturating_sub(provider_info.len() + 20));

    let tycode_width = " ◈ TyCode ".len();
    let provider_width = format!(" {} ", provider_info).len();
    let cwd_width = format!(" {} ", cwd_short).len();
    let total_used = tycode_width + provider_width + cwd_width;
    let padding = (area.width as usize).saturating_sub(total_used);

    let header = Line::from(vec![
        Span::styled(
            " ◈ TyCode ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(100, 60, 180))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", provider_info),
            Style::default().fg(Color::Cyan).bg(HEADER_BG),
        ),
        Span::styled(
            " ".repeat(padding),
            Style::default().bg(HEADER_BG),
        ),
        Span::styled(
            format!(" {} ", cwd_short),
            Style::default().fg(DIM).bg(HEADER_BG),
        ),
    ]);

    let header_widget = Paragraph::new(header).style(Style::default().bg(HEADER_BG));
    f.render_widget(header_widget, area);
}

// ── Chat area ────────────────────────────────────────────────────────────────

fn render_chat(f: &mut Frame, app: &mut App, area: Rect) {
    let mut all_lines: Vec<Line<'static>> = Vec::new();

    for msg in &app.messages {
        match msg {
            ChatMessage::User(text) => {
                all_lines.push(Line::from(""));
                all_lines.push(Line::from(vec![
                    Span::styled(
                        "  You ",
                        Style::default()
                            .fg(Color::Black)
                            .bg(USER_COLOR)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                for line in text.lines() {
                    all_lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(line.to_string(), Style::default().fg(USER_COLOR)),
                    ]));
                }
            }
            ChatMessage::AssistantText(text) => {
                all_lines.push(Line::from(""));
                let md_lines = markdown::markdown_to_lines(text);
                for line in md_lines {
                    let mut prefixed: Vec<Span<'static>> = vec![Span::raw("  ")];
                    prefixed.extend(line.spans);
                    all_lines.push(Line::from(prefixed));
                }
            }
            ChatMessage::ToolCall {
                name,
                input_summary,
                success,
                output,
            } => {
                let bullet_color = match success {
                    Some(true) => TOOL_SUCCESS,
                    Some(false) => TOOL_FAIL,
                    None => TOOL_COLOR,
                };
                let bullet = match success {
                    Some(true) => "✓",
                    Some(false) => "✗",
                    None => "●",
                };
                all_lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {bullet} "),
                        Style::default().fg(bullet_color),
                    ),
                    Span::styled(
                        name.clone(),
                        Style::default()
                            .fg(TOOL_COLOR)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" {input_summary}"),
                        Style::default().fg(DIM),
                    ),
                ]));
                if let Some(out) = output {
                    let max_lines = 15;
                    let out_lines: Vec<&str> = out.lines().collect();
                    let mut i = 0;
                    while i < out_lines.len() && i < max_lines {
                        all_lines.push(Line::from(vec![
                            Span::styled(
                                format!("    {}", out_lines[i]),
                                Style::default().fg(DIM),
                            ),
                        ]));
                        i += 1;
                    }
                    if out_lines.len() > max_lines {
                        all_lines.push(Line::from(vec![
                            Span::styled(
                                format!("    ...(showing {max_lines} of {} lines)", out_lines.len()),
                                Style::default().fg(DIM),
                            ),
                        ]));
                    }
                }
            }
            ChatMessage::System(text) => {
                for line in text.lines() {
                    all_lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {line}"),
                            Style::default().fg(SYSTEM_COLOR).add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
            }
            ChatMessage::Error(text) => {
                all_lines.push(Line::from(vec![
                    Span::styled(
                        format!("  Error: {text}"),
                        Style::default()
                            .fg(ERROR_COLOR)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
        }
    }

    // Thinking indicator — stays visible the entire time the agent is running,
    // including while the response is being generated in the background.
    if matches!(app.mode, AppMode::Processing) {
        let dots = ".".repeat((app.thinking_dots % 4) + 1);
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(vec![
            Span::styled(
                format!("  Thinking{dots}"),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    }

    // Use content width minus 1 for the scrollbar column
    let content_width = area.width.saturating_sub(1);
    let total_lines = compute_wrapped_height(&all_lines, content_width);
    let visible_height = area.height;

    // Auto-scroll to bottom
    if app.scroll_offset == u16::MAX {
        app.scroll_offset = total_lines.saturating_sub(visible_height);
    }
    // Clamp scroll
    app.scroll_offset = app
        .scroll_offset
        .min(total_lines.saturating_sub(visible_height));

    let text = Text::from(all_lines);
    let chat_widget = Paragraph::new(text)
        .scroll((app.scroll_offset, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(chat_widget, area);

    // Scrollbar — show when content exceeds viewport
    if total_lines > visible_height {
        let mut scrollbar_state =
            ScrollbarState::new(total_lines as usize)
                .position(app.scroll_offset as usize)
                .viewport_content_length(visible_height as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(BORDER_COLOR));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

// ── Input area ───────────────────────────────────────────────────────────────

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let is_processing = matches!(app.mode, AppMode::Processing);

    let border_color = if is_processing {
        Color::DarkGray
    } else {
        BORDER_COLOR
    };

    let title = if is_processing {
        " Processing... ".to_string()
    } else {
        let history_pos = app.get_history_position_text();
        format!(" >{} ", history_pos)
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .title_style(Style::default().fg(if is_processing {
            Color::DarkGray
        } else {
            Color::Cyan
        }));

    let input_widget = if app.input.is_empty() && !is_processing {
        Paragraph::new(Line::from(Span::styled(
            "Type your prompt or /help for commands...",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
        )))
        .block(input_block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false })
    } else {
        let input_text = if let Some(hint) = get_command_hints(&app.input) {
            let typed = &app.input;
            let untyped = &hint[typed.len()..];
            Line::from(vec![
                Span::styled(typed.to_string(), Style::default().fg(Color::White)),
                Span::styled(untyped.to_string(), Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)),
            ])
        } else {
            Line::from(Span::raw(app.input.clone()))
        };
        Paragraph::new(input_text)
            .block(input_block)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false })
    };

    f.render_widget(input_widget, area);

    // Show cursor — account for line wrapping when calculating position.
    if !is_processing && !matches!(app.mode, AppMode::Settings(_) | AppMode::Help | AppMode::ModelSelect(_) | AppMode::ProviderSelect(_)) {
        let inner_width = area.width.saturating_sub(2) as usize;
        let (cursor_col, cursor_row) = if inner_width > 0 {
            (app.cursor_pos % inner_width, app.cursor_pos / inner_width)
        } else {
            (app.cursor_pos, 0)
        };
        let cursor_x = area.x + 1 + cursor_col as u16;
        let cursor_y = area.y + 1 + cursor_row as u16;
        if cursor_x < area.x + area.width - 1 && cursor_y < area.y + area.height - 1 {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

// ── Status bar ───────────────────────────────────────────────────────────────

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let commands = " ⌘help  model  settings  clear  import  exit  │  PgUp/Dn:scroll  C-c:quit ";
    let status = &app.status_message;

    let status_with_indicator = if let Some(ts) = app.status_timestamp {
        let elapsed = ts.elapsed().as_millis() as u64;
        let remaining = (3000u64).saturating_sub(elapsed);
        let dots = if remaining > 2000 {
            "●"
        } else if remaining > 1000 {
            "○"
        } else {
            "·"
        };
        format!("{} {}", status, dots)
    } else {
        status.clone()
    };

    let padding = (area.width as usize)
        .saturating_sub(commands.len() + status_with_indicator.len());

    let status_line = Line::from(vec![
        Span::styled(commands, Style::default().fg(DIM).bg(STATUS_BG)),
        Span::styled(
            " ".repeat(padding),
            Style::default().bg(STATUS_BG),
        ),
        Span::styled(
            format!("{} ", status_with_indicator),
            Style::default().fg(Color::Green).bg(STATUS_BG),
        ),
    ]);

    let status_widget = Paragraph::new(status_line).style(Style::default().bg(STATUS_BG));
    f.render_widget(status_widget, area);
}

// ── Settings overlay ─────────────────────────────────────────────────────────

const POPUP_BG: Color = Color::Rgb(18, 18, 32);
const POPUP_BORDER: Color = Color::Rgb(100, 100, 160);
const POPUP_TITLE: Color = Color::Rgb(130, 180, 255);

fn render_settings_overlay(f: &mut Frame, state: &SettingsState, area: Rect) {
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
    let width = 68u16.min(area.width.saturating_sub(4));
    let height = 28u16.min(area.height.saturating_sub(4));
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
        Line::from(vec![
            Span::styled("  /help        ", cmd),
            Span::styled("Show this help screen", desc),
        ]),
        Line::from(vec![
            Span::styled("  /model       ", cmd),
            Span::styled("Switch model (fetches available models)", desc),
        ]),
        Line::from(vec![
            Span::styled("  /provider    ", cmd),
            Span::styled("Pick provider from list (ollama/anthropic/openai/gemini/airllm)", desc),
        ]),
        Line::from(vec![
            Span::styled("  /settings    ", cmd),
            Span::styled("Open settings editor", desc),
        ]),
        Line::from(vec![
            Span::styled("  /clear       ", cmd),
            Span::styled("Clear chat history and agent context", desc),
        ]),
        Line::from(vec![
            Span::styled("  /system      ", cmd),
            Span::styled("Set custom system prompt", desc),
        ]),
        Line::from(vec![
            Span::styled("  /import      ", cmd),
            Span::styled("/import <path>  inject file into agent context", desc),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("  Keyboard & Navigation", section)]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter           ", cmd),
            Span::styled("Send message", desc),
        ]),
        Line::from(vec![
            Span::styled("  Up / Down       ", cmd),
            Span::styled("Navigate input history", desc),
        ]),
        Line::from(vec![
            Span::styled("  PgUp / PgDown   ", cmd),
            Span::styled("Scroll chat (10 lines)", desc),
        ]),
        Line::from(vec![
            Span::styled("  Scroll Wheel    ", cmd),
            Span::styled("Scroll chat (3 lines)", desc),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Home       ", cmd),
            Span::styled("Jump to top of chat", desc),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+End        ", cmd),
            Span::styled("Jump to bottom of chat", desc),
        ]),
        Line::from(vec![
            Span::styled("  Tab             ", cmd),
            Span::styled("Auto-complete slash commands", desc),
        ]),
        Line::from(vec![
            Span::styled("  Esc             ", cmd),
            Span::styled("Close overlay / clear input", desc),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+C          ", cmd),
            Span::styled("Quit application", desc),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("  Chat auto-scrolls to latest message when AI responds", dim)]),
    ];

    let help_widget = Paragraph::new(help_text)
        .block(block)
        .style(Style::default().bg(POPUP_BG));
    f.render_widget(help_widget, popup_area);
}

// ── Model select overlay ─────────────────────────────────────────────────────

fn render_model_select_overlay(f: &mut Frame, state: &ModelSelectState, area: Rect) {
    let width = 55u16.min(area.width.saturating_sub(4));
    let height = (state.models.len() as u16 + 6)
        .min(area.height.saturating_sub(4))
        .max(8);
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let popup_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup_area);

    let title = if state.loading {
        " ◌  Loading models... "
    } else {
        " ◈  Select Model "
    };

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
                Span::styled("↵esc", Style::default().fg(Color::Yellow)),
                Span::styled("=exit ", Style::default().fg(Color::DarkGray)),
            ])
        })
        .borders(Borders::ALL)
        .border_style(Style::default().fg(POPUP_BORDER))
        .padding(Padding::new(1, 1, 1, 1))
        .style(Style::default().bg(POPUP_BG));

    if state.loading {
        let loading = Paragraph::new(
            Line::from(vec![
                Span::styled("  ⏳ ", Style::default().fg(Color::Yellow)),
                Span::styled("Fetching available models...", Style::default().fg(Color::DarkGray)),
            ])
        )
        .block(block)
        .style(Style::default().bg(POPUP_BG));
        f.render_widget(loading, popup_area);
        return;
    }

    let items: Vec<ListItem> = state
        .models
        .iter()
        .enumerate()
        .map(|(i, model)| {
            if i == state.selected {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("► {} ", model),
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Rgb(80, 120, 220))
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {} ", model),
                        Style::default().fg(Color::Rgb(180, 180, 220)),
                    ),
                ]))
            }
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .style(Style::default().bg(POPUP_BG));
    f.render_widget(list, popup_area);
}

// ── Provider select overlay ──────────────────────────────────────────────────

fn render_provider_select_overlay(f: &mut Frame, state: &ProviderSelectState, area: Rect) {
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
            Span::styled("↵esc", Style::default().fg(Color::Yellow)),
            Span::styled("=exit ", Style::default().fg(Color::DarkGray)),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(POPUP_BORDER))
        .padding(Padding::new(1, 1, 1, 1))
        .style(Style::default().bg(POPUP_BG));

    let items: Vec<ListItem> = state
        .providers
        .iter()
        .enumerate()
        .map(|(i, provider)| {
            if i == state.selected {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("► {} ", provider),
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Rgb(80, 120, 220))
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {} ", provider),
                        Style::default().fg(Color::Rgb(180, 180, 220)),
                    ),
                ]))
            }
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .style(Style::default().bg(POPUP_BG));
    f.render_widget(list, popup_area);
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Get slash command hints for the current input
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

/// Calculate the actual rendered line count accounting for text wrapping.
fn compute_wrapped_height(lines: &[Line], width: u16) -> u16 {
    if width == 0 {
        return lines.len() as u16;
    }
    let mut total: u16 = 0;
    for line in lines {
        let line_width: usize = line
            .spans
            .iter()
            .map(|s| s.content.width())
            .sum();
        if line_width == 0 {
            total = total.saturating_add(1);
        } else {
            total = total.saturating_add(
                ((line_width as u16).saturating_sub(1)) / width + 1,
            );
        }
    }
    total
}

fn shorten_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    // Replace home dir with ~
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
