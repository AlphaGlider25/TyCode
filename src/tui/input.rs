use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use tokio::sync::mpsc;

use super::app::{App, AppMode, ChatMessage, ModelSelectState, ProviderSelectState, SettingsState};

/// Handle a mouse event (scroll wheel for chat).
pub fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.scroll_offset = app.scroll_offset.saturating_sub(3);
        }
        MouseEventKind::ScrollDown => {
            app.scroll_offset = app.scroll_offset.saturating_add(3);
        }
        _ => {}
    }
}

/// Handle a key event. Returns true if the event was consumed.
pub async fn handle_key(
    app: &mut App,
    key: KeyEvent,
    agent_tx: &mpsc::UnboundedSender<String>,
) -> bool {
    // Global keys
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return true;
    }

    match &app.mode {
        AppMode::Processing => {
            // Ignore keys while processing (Esc could cancel in future)
            true
        }
        AppMode::Settings(_) => {
            handle_settings_key(app, key);
            true
        }
        AppMode::Help => {
            if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
                app.mode = AppMode::Normal;
            }
            true
        }
        AppMode::ModelSelect(_) => {
            handle_model_select_key(app, key);
            true
        }
        AppMode::ProviderSelect(_) => {
            handle_provider_select_key(app, key);
            true
        }
        AppMode::Normal => handle_normal_key(app, key, agent_tx).await,
    }
}

async fn handle_normal_key(
    app: &mut App,
    key: KeyEvent,
    agent_tx: &mpsc::UnboundedSender<String>,
) -> bool {
    match key.code {
        KeyCode::Enter => {
            let input = app.input.trim().to_string();
            if input.is_empty() {
                return true;
            }

            app.add_to_history(input.clone());
            app.input.clear();
            app.cursor_pos = 0;

            // Handle slash commands
            if input.starts_with('/') {
                handle_command(app, &input).await;
            } else {
                // Send to agent
                app.messages.push(ChatMessage::User(input.clone()));
                app.mode = AppMode::Processing;
                app.scroll_to_bottom();
                let _ = agent_tx.send(input);
            }
            true
        }
        KeyCode::Char(c) => {
            app.input.insert(app.cursor_pos, c);
            app.cursor_pos += 1;
            true
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
                app.input.remove(app.cursor_pos);
            }
            true
        }
        KeyCode::Delete => {
            if app.cursor_pos < app.input.len() {
                app.input.remove(app.cursor_pos);
            }
            true
        }
        KeyCode::Left => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
            }
            true
        }
        KeyCode::Right => {
            if app.cursor_pos < app.input.len() {
                app.cursor_pos += 1;
            }
            true
        }
        KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_offset = 0;
            true
        }
        KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_to_bottom();
            true
        }
        KeyCode::Home => {
            app.cursor_pos = 0;
            true
        }
        KeyCode::End => {
            app.cursor_pos = app.input.len();
            true
        }
        KeyCode::Up => {
            app.history_up();
            true
        }
        KeyCode::Down => {
            app.history_down();
            true
        }
        KeyCode::PageUp => {
            app.scroll_offset = app.scroll_offset.saturating_sub(10);
            true
        }
        KeyCode::PageDown => {
            app.scroll_offset = app.scroll_offset.saturating_add(10);
            true
        }
        KeyCode::Esc => {
            app.input.clear();
            app.cursor_pos = 0;
            true
        }
        KeyCode::Tab => {
            // Auto-complete slash commands
            if app.input.starts_with('/') {
                let commands = [
                    "/help",
                    "/import",
                    "/model",
                    "/provider",
                    "/settings",
                    "/clear",
                    "/system",
                    "/exit",
                ];
                let matches: Vec<&&str> = commands
                    .iter()
                    .filter(|c| c.starts_with(&app.input))
                    .collect();
                if matches.len() == 1 {
                    app.input = matches[0].to_string();
                    app.cursor_pos = app.input.len();
                }
            }
            true
        }
        _ => false,
    }
}

fn is_overlay_open(mode: &AppMode) -> bool {
    matches!(
        mode,
        AppMode::Help
            | AppMode::Settings(_)
            | AppMode::ModelSelect(_)
            | AppMode::ProviderSelect(_)
    )
}

async fn handle_command(app: &mut App, cmd: &str) {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    let command = parts[0].to_lowercase();
    let args = parts.get(1).unwrap_or(&"").trim();

    match command.as_str() {
        "/exit" | "/quit" => {
            app.should_quit = true;
        }
        "/help" => {
            if !is_overlay_open(&app.mode) {
                app.mode = AppMode::Help;
            }
        }
        "/clear" => {
            app.messages.clear();
            app.buffered_response.clear();
            app.pending_messages.clear();
            // Reset the shared agent (clear conversation history)
            {
                let mut agent = app.shared_agent.lock().await;
                agent.clear_history();
            }
            app.messages
                .push(ChatMessage::System("Chat cleared. Starting fresh.".into()));
            app.set_status("Chat cleared — Ready");
        }
        "/settings" => {
            if !is_overlay_open(&app.mode) {
                let state = SettingsState::from_config(&app.config);
                app.mode = AppMode::Settings(state);
            }
        }
        "/model" => {
            if !args.is_empty() {
                app.config.model = args.to_string();
                let _ = app.config.save();
                app.messages
                    .push(ChatMessage::System(format!("Model set to: {}", args)));
                app.set_status(&format!("Model: {}", args));
            } else if !is_overlay_open(&app.mode) {
                // Open model selection overlay
                app.mode = AppMode::ModelSelect(ModelSelectState {
                    models: vec![],
                    selected: 0,
                    loading: true,
                });
            }
        }
        "/provider" => {
            if !is_overlay_open(&app.mode) {
                app.mode = AppMode::ProviderSelect(ProviderSelectState::new(
                    &app.config.provider,
                    false,
                ));
            }
        }
        "/system" => {
            if !args.is_empty() {
                let prompt = args.to_string();
                {
                    let mut agent = app.shared_agent.lock().await;
                    agent.set_system_prompt(prompt);
                }
                app.messages
                    .push(ChatMessage::System("Custom system prompt set.".into()));
            } else {
                app.messages.push(ChatMessage::System(
                    "Usage: /system <your custom system prompt>".into(),
                ));
            }
        }
        "/import" => {
            if args.is_empty() {
                app.messages.push(ChatMessage::System(
                    "Usage: /import <file_path>  — injects file contents into agent context".into(),
                ));
                return;
            }
            let path = crate::tools::shellexpand(args);
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    const MAX_IMPORT_BYTES: usize = 100 * 1024;
                    if content.len() > MAX_IMPORT_BYTES {
                        app.messages.push(ChatMessage::Error(format!(
                            "File too large to import ({} KB). Maximum is 100 KB.",
                            content.len() / 1024
                        )));
                        return;
                    }
                    let lines = content.lines().count();
                    let bytes = content.len();
                    {
                        let mut agent = app.shared_agent.lock().await;
                        agent.inject_context(args, &content);
                    }
                    app.messages.push(ChatMessage::System(format!(
                        "Imported: {args} ({lines} lines, {bytes} bytes) — now in context. Ask me about it."
                    )));
                }
                Err(e) => {
                    app.messages.push(ChatMessage::Error(format!(
                        "Cannot import '{args}': {e}"
                    )));
                }
            }
        }
        _ => {
            app.messages.push(ChatMessage::Error(format!(
                "Unknown command: {}. Type /help for available commands.",
                command
            )));
        }
    }
}

fn handle_settings_key(app: &mut App, key: KeyEvent) {
    let state = match &mut app.mode {
        AppMode::Settings(s) => s,
        _ => return,
    };

    if state.editing {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                state.editing = false;
            }
            KeyCode::Char(c) => {
                state.fields[state.selected_field].value.push(c);
            }
            KeyCode::Backspace => {
                state.fields[state.selected_field].value.pop();
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.selected_field > 0 {
                state.selected_field -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
            if state.selected_field < state.fields.len() - 1 {
                state.selected_field += 1;
            }
        }
        KeyCode::Enter => {
            if state.fields[state.selected_field].key == "provider" {
                let current = state.fields[state.selected_field].value.clone();
                app.mode = AppMode::ProviderSelect(ProviderSelectState::new(&current, true));
            } else {
                let state = match &mut app.mode {
                    AppMode::Settings(s) => s,
                    _ => return,
                };
                state.editing = true;
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            // Save settings
            let state_clone = state.clone();
            state_clone.apply_to_config(&mut app.config);
            let _ = app.config.save();
            app.messages.push(ChatMessage::System(format!(
                "Settings saved. Provider: {}",
                app.config.provider_display()
            )));
            app.set_status(&format!("Settings saved — {}", app.config.provider_display()));
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_provider_select_key(app: &mut App, key: KeyEvent) {
    let state = match &mut app.mode {
        AppMode::ProviderSelect(s) => s.clone(),
        _ => return,
    };

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            if state.return_to_settings {
                app.mode = AppMode::Settings(SettingsState::from_config(&app.config));
            } else {
                app.mode = AppMode::Normal;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let AppMode::ProviderSelect(s) = &mut app.mode {
                if s.selected > 0 {
                    s.selected -= 1;
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let AppMode::ProviderSelect(s) = &mut app.mode {
                if s.selected < s.providers.len().saturating_sub(1) {
                    s.selected += 1;
                }
            }
        }
        KeyCode::Enter => {
            if let Some(provider) = state.providers.get(state.selected) {
                app.config.provider = provider.clone();
                let _ = app.config.save();
                app.messages.push(ChatMessage::System(format!(
                    "Provider set to: {provider}"
                )));
                app.set_status(&format!("Provider: {}", app.config.provider_display()));
            }
            if state.return_to_settings {
                app.mode = AppMode::Settings(SettingsState::from_config(&app.config));
            } else {
                app.mode = AppMode::Normal;
            }
        }
        _ => {}
    }
}

fn handle_model_select_key(app: &mut App, key: KeyEvent) {
    let selected_model = match &mut app.mode {
        AppMode::ModelSelect(s) => {
            if s.loading {
                if key.code == KeyCode::Esc {
                    app.mode = AppMode::Normal;
                }
                return;
            }

            let selected_model = match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    app.mode = AppMode::Normal;
                    None
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if s.selected > 0 {
                        s.selected -= 1;
                    }
                    None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if s.selected < s.models.len().saturating_sub(1) {
                        s.selected += 1;
                    }
                    None
                }
                KeyCode::Enter => {
                    let model = s.models.get(s.selected).cloned();
                    if model.is_some() {
                        app.mode = AppMode::Normal;
                    }
                    model
                }
                _ => None,
            };
            selected_model
        }
        _ => return,
    };

    if let Some(model) = selected_model {
        app.config.model = model.clone();
        let _ = app.config.save();
        app.messages
            .push(ChatMessage::System(format!("Model set to: {model}")));
        app.set_status(&format!("Model: {model}"));
    }
}
