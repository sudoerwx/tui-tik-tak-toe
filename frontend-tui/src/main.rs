use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    DefaultTerminal, Frame,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
struct ApiGame {
    id: String,
    mode: String,
    name: Option<String>,
    #[serde(rename = "hostPlayerId")]
    host_player_id: String,
    #[serde(rename = "guestPlayerId")]
    guest_player_id: Option<String>,
    board: Vec<Option<String>>,
    #[serde(rename = "currentTurn")]
    current_turn: String,
    status: String,
    winner: Option<String>,
    #[serde(rename = "hasPassword")]
    has_password: bool,
}

#[derive(Debug, Serialize)]
struct CreateSoloRequest {
    #[serde(rename = "playerId")]
    player_id: String,
    #[serde(rename = "clientName")]
    client_name: String,
}

#[derive(Debug, Serialize)]
struct CreatePvpRequest {
    #[serde(rename = "playerId")]
    player_id: String,
    name: String,
    password: Option<String>,
}

#[derive(Debug, Serialize)]
struct JoinPvpRequest {
    #[serde(rename = "playerId")]
    player_id: String,
    password: Option<String>,
}

#[derive(Debug, Serialize)]
struct PlayMoveRequest {
    #[serde(rename = "playerId")]
    player_id: String,
    index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    SoloGame,
    PvpLobby,
    PvpCreate,
    PvpGame,
    GameOver,
    Info,
}

struct App {
    client: Client,
    base_url: String,
    player_id: String,
    screen: Screen,
    home_index: usize,
    board_cursor: usize,
    solo_game: Option<ApiGame>,
    pvp_game: Option<ApiGame>,
    pvp_games: Vec<ApiGame>,
    pvp_selected_index: usize,
    create_name: String,
    create_password: String,
    create_field_index: usize,
    join_password: String,
    editing_join_password: bool,
    game_over_message: String,
    info_message: String,
    should_quit: bool,
    last_poll_at: Instant,
}

impl App {
    fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            player_id: Uuid::new_v4().to_string(),
            screen: Screen::Home,
            home_index: 0,
            board_cursor: 0,
            solo_game: None,
            pvp_game: None,
            pvp_games: Vec::new(),
            pvp_selected_index: 0,
            create_name: String::new(),
            create_password: String::new(),
            create_field_index: 0,
            join_password: String::new(),
            editing_join_password: false,
            game_over_message: String::new(),
            info_message: String::new(),
            should_quit: false,
            last_poll_at: Instant::now(),
        }
    }

    async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            // Polling is intentionally done in the UI loop for simplicity.
            // In bigger apps, move this to a background task + message channel.
            self.refresh_remote_state_if_needed().await;
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(120))? {
                if let Event::Key(key_event) = event::read()? {
                    self.handle_key(key_event).await;
                }
            }
        }

        Ok(())
    }

    async fn refresh_remote_state_if_needed(&mut self) {
        if self.last_poll_at.elapsed() < Duration::from_secs(1) {
            return;
        }

        match self.screen {
            Screen::PvpLobby => {
                if let Ok(games) = self.list_open_pvp_games().await {
                    self.pvp_games = games;
                    if self.pvp_selected_index >= self.pvp_games.len() {
                        self.pvp_selected_index = self.pvp_games.len().saturating_sub(1);
                    }
                }
            }
            Screen::PvpGame => {
                // Polling lets a player see opponent moves without websockets.
                if let Some(game_id) = self.pvp_game.as_ref().map(|g| g.id.clone()) {
                    if let Ok(game) = self.get_game(&game_id).await {
                        if Self::is_game_finished(&game) {
                            self.open_game_over(&game, "PvP");
                        }
                        self.pvp_game = Some(game);
                    }
                }
            }
            _ => {}
        }

        self.last_poll_at = Instant::now();
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        match self.screen {
            Screen::Home => self.handle_home_key(key).await,
            Screen::SoloGame => self.handle_solo_key(key).await,
            Screen::PvpLobby => self.handle_pvp_lobby_key(key).await,
            Screen::PvpCreate => self.handle_pvp_create_key(key).await,
            Screen::PvpGame => self.handle_pvp_game_key(key).await,
            Screen::GameOver => self.handle_game_over_key(key),
            Screen::Info => self.handle_info_key(key),
        }
    }

    async fn handle_home_key(&mut self, key: KeyEvent) {
        let home_items = ["Solo vs Computer", "PvP", "Exit"];
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Up => {
                self.home_index = self.home_index.saturating_sub(1);
            }
            KeyCode::Down => {
                if self.home_index + 1 < home_items.len() {
                    self.home_index += 1;
                }
            }
            KeyCode::Enter => match self.home_index {
                0 => {
                    match self.create_solo_game().await {
                        Ok(game) => {
                            self.solo_game = Some(game);
                            self.board_cursor = 0;
                            self.screen = Screen::SoloGame;
                        }
                        Err(err) => {
                            self.show_error(format!("Could not start solo game: {err}"));
                        }
                    }
                }
                1 => {
                    match self.list_open_pvp_games().await {
                        Ok(games) => {
                            self.pvp_games = games;
                            self.pvp_selected_index = 0;
                            self.screen = Screen::PvpLobby;
                        }
                        Err(err) => {
                            self.show_error(format!("Could not load PvP games: {err}"));
                        }
                    }
                }
                _ => self.should_quit = true,
            },
            _ => {}
        }
    }

    async fn handle_solo_key(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Char('b')) {
            self.screen = Screen::Home;
            return;
        }

        if matches!(key.code, KeyCode::Char('q')) {
            self.should_quit = true;
            return;
        }

        self.update_board_cursor(key.code);

        let Some(game) = self.solo_game.clone() else {
            return;
        };

        if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
            let player_turn = game.current_turn == "X";
            let game_running = game.status == "IN_PROGRESS";
            if player_turn && game_running {
                match self.play_move(&game.id, self.board_cursor).await {
                    Ok(updated) => {
                        if Self::is_game_finished(&updated) {
                            self.open_game_over(&updated, "Solo");
                        }
                        self.solo_game = Some(updated);
                    }
                    Err(err) => self.show_error(format!("Move failed: {err}")),
                }
            }
        }
    }

    async fn handle_pvp_lobby_key(&mut self, key: KeyEvent) {
        if self.editing_join_password {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.editing_join_password = false,
                KeyCode::Backspace => {
                    self.join_password.pop();
                }
                KeyCode::Char(ch) => {
                    if self.join_password.len() < 32 {
                        self.join_password.push(ch);
                    }
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('b') => self.screen = Screen::Home,
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Up => {
                self.pvp_selected_index = self.pvp_selected_index.saturating_sub(1);
            }
            KeyCode::Down => {
                if self.pvp_selected_index + 1 < self.pvp_games.len() {
                    self.pvp_selected_index += 1;
                }
            }
            KeyCode::Char('r') => match self.list_open_pvp_games().await {
                Ok(games) => {
                    self.pvp_games = games;
                    self.pvp_selected_index = 0;
                }
                Err(err) => self.show_error(format!("Refresh failed: {err}")),
            },
            KeyCode::Char('c') => {
                self.create_name.clear();
                self.create_password.clear();
                self.create_field_index = 0;
                self.screen = Screen::PvpCreate;
            }
            KeyCode::Char('p') => self.editing_join_password = true,
            KeyCode::Char('j') | KeyCode::Enter => {
                if self.pvp_games.is_empty() {
                    return;
                }

                if let Some(game) = self.pvp_games.get(self.pvp_selected_index) {
                    let password = if game.has_password {
                        if self.join_password.is_empty() {
                            None
                        } else {
                            Some(self.join_password.clone())
                        }
                    } else {
                        None
                    };

                    match self.join_pvp_game(&game.id, password).await {
                        Ok(joined) => {
                            self.pvp_game = Some(joined);
                            self.board_cursor = 0;
                            self.screen = Screen::PvpGame;
                        }
                        Err(err) => {
                            self.show_error(format!("Join failed: {err}"));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    async fn handle_pvp_create_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('b') => self.screen = Screen::PvpLobby,
            KeyCode::Tab | KeyCode::Down | KeyCode::Up => {
                self.create_field_index = (self.create_field_index + 1) % 2;
            }
            KeyCode::Backspace => {
                if self.create_field_index == 0 {
                    self.create_name.pop();
                } else {
                    self.create_password.pop();
                }
            }
            KeyCode::Enter => {
                if self.create_name.trim().len() < 3 {
                    self.show_error("Game name must be at least 3 chars".to_string());
                    return;
                }

                let password = if self.create_password.trim().is_empty() {
                    None
                } else {
                    Some(self.create_password.trim().to_string())
                };

                match self.create_pvp_game(self.create_name.trim(), password).await {
                    Ok(game) => {
                        self.pvp_game = Some(game);
                        self.screen = Screen::PvpGame;
                    }
                    Err(err) => self.show_error(format!("Create game failed: {err}")),
                }
            }
            KeyCode::Char(ch) => {
                if self.create_field_index == 0 {
                    if self.create_name.len() < 40 {
                        self.create_name.push(ch);
                    }
                } else if self.create_password.len() < 32 {
                    self.create_password.push(ch);
                }
            }
            _ => {}
        }
    }

    async fn handle_pvp_game_key(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Char('b')) {
            self.screen = Screen::PvpLobby;
            return;
        }

        if matches!(key.code, KeyCode::Char('q')) {
            self.should_quit = true;
            return;
        }

        self.update_board_cursor(key.code);

        let Some(game) = self.pvp_game.clone() else {
            return;
        };

        let player_symbol = self.player_symbol_for(&game);
        let my_turn = player_symbol == game.current_turn;

        if matches!(key.code, KeyCode::Enter | KeyCode::Char(' '))
            && game.status == "IN_PROGRESS"
            && my_turn
        {
            match self.play_move(&game.id, self.board_cursor).await {
                Ok(updated) => {
                    if Self::is_game_finished(&updated) {
                        self.open_game_over(&updated, "PvP");
                    }
                    self.pvp_game = Some(updated);
                }
                Err(err) => self.show_error(format!("Move failed: {err}")),
            }
        }
    }

    fn handle_game_over_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('b') | KeyCode::Char('m') => {
                self.screen = Screen::Home;
            }
            _ => {}
        }
    }

    fn handle_info_key(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('b')) {
            self.screen = Screen::Home;
        }
    }

    fn update_board_cursor(&mut self, key: KeyCode) {
        let row = self.board_cursor / 3;
        let col = self.board_cursor % 3;

        let (next_row, next_col) = match key {
            KeyCode::Left => (row, col.saturating_sub(1)),
            KeyCode::Right => (row, (col + 1).min(2)),
            KeyCode::Up => (row.saturating_sub(1), col),
            KeyCode::Down => ((row + 1).min(2), col),
            KeyCode::Char(ch) if ('1'..='9').contains(&ch) => {
                let index = ch as usize - '1' as usize;
                self.board_cursor = index;
                return;
            }
            _ => (row, col),
        };

        self.board_cursor = next_row * 3 + next_col;
    }

    fn player_symbol_for(&self, game: &ApiGame) -> String {
        if game.host_player_id == self.player_id {
            "X".to_string()
        } else if game.guest_player_id.as_deref() == Some(self.player_id.as_str()) {
            "O".to_string()
        } else {
            "?".to_string()
        }
    }

    fn draw(&self, frame: &mut Frame<'_>) {
        match self.screen {
            Screen::Home => self.draw_home(frame),
            Screen::SoloGame => self.draw_game(frame, self.solo_game.as_ref(), "Solo Mode"),
            Screen::PvpLobby => self.draw_pvp_lobby(frame),
            Screen::PvpCreate => self.draw_pvp_create(frame),
            Screen::PvpGame => self.draw_game(frame, self.pvp_game.as_ref(), "PvP Mode"),
            Screen::GameOver => self.draw_game_over(frame),
            Screen::Info => self.draw_info(frame),
        }
    }

    fn draw_home(&self, frame: &mut Frame<'_>) {
        let area = centered_rect(70, 65, frame.area());
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(area);

        let title = Paragraph::new("Tic-Tac-Toe (NestJS + Rust TUI)")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Home"));
        frame.render_widget(title, chunks[0]);

        let items = ["Solo vs Computer", "PvP", "Exit"];
        let menu_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(idx, label)| {
                let line = if idx == self.home_index {
                    Line::from(vec![Span::styled(
                        format!("> {label}"),
                        Style::default().add_modifier(Modifier::BOLD),
                    )])
                } else {
                    Line::from(format!("  {label}"))
                };
                ListItem::new(line)
            })
            .collect();

        let list = List::new(menu_items).block(Block::default().borders(Borders::ALL).title("Menu"));
        frame.render_widget(list, chunks[1]);

        let help = Paragraph::new(
            "Arrow Up/Down + Enter to select.\nq exits from anywhere.\nPlayer session id is generated once per app launch.",
        )
        .block(Block::default().borders(Borders::ALL).title("Help"));
        frame.render_widget(help, chunks[2]);
    }

    fn draw_game(&self, frame: &mut Frame<'_>, game: Option<&ApiGame>, title: &str) {
        let area = centered_rect(80, 90, frame.area());
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(11),
                Constraint::Length(5),
                Constraint::Min(1),
            ])
            .split(area);

        let Some(game) = game else {
            frame.render_widget(
                Paragraph::new("No active game.")
                    .block(Block::default().borders(Borders::ALL).title(title)),
                area,
            );
            return;
        };

        let player_symbol = self.player_symbol_for(game);
        let status_line = if game.status == "WON" {
            format!("Status: WON | Winner: {}", game.winner.clone().unwrap_or_default())
        } else {
            format!("Status: {}", game.status)
        };

        let header = Paragraph::new(format!(
            "Game id: {}\nMode: {} | You are: {} | Current turn: {}\n{}",
            game.id, game.mode, player_symbol, game.current_turn, status_line
        ))
        .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(header, chunks[0]);

        let board_text = self.render_board_text(&game.board);
        let board = Paragraph::new(board_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Board (Arrows or 1..9, Enter to play)"),
        );
        frame.render_widget(board, chunks[1]);

        let hint = Paragraph::new(
            "Controls: Enter/Space = move, b = back, q = exit.\nPvP screen auto-refreshes each second for opponent moves.",
        )
        .block(Block::default().borders(Borders::ALL).title("Controls"));
        frame.render_widget(hint, chunks[2]);
    }

    fn draw_pvp_lobby(&self, frame: &mut Frame<'_>) {
        let area = centered_rect(90, 90, frame.area());
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
                Constraint::Length(5),
            ])
            .split(area);

        let title = Paragraph::new("Open PvP games")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("PvP Lobby"));
        frame.render_widget(title, chunks[0]);

        let items: Vec<ListItem> = if self.pvp_games.is_empty() {
            vec![ListItem::new("No open games")]
        } else {
            self.pvp_games
                .iter()
                .enumerate()
                .map(|(idx, game)| {
                    let prefix = if idx == self.pvp_selected_index { ">" } else { " " };
                    let name = game.name.clone().unwrap_or_else(|| "Untitled".to_string());
                    let pass = if game.has_password { "locked" } else { "open" };
                    ListItem::new(format!(
                        "{prefix} {name} | id={} | {pass}",
                        game.id
                    ))
                })
                .collect()
        };

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Games"));
        frame.render_widget(list, chunks[1]);

        let password_info = if self.join_password.is_empty() {
            "Join password: <empty>".to_string()
        } else {
            format!("Join password: {}", "*".repeat(self.join_password.len()))
        };
        let password_title = if self.editing_join_password {
            "Join Password (editing, Enter/Esc to stop)"
        } else {
            "Join Password (press p to edit)"
        };
        frame.render_widget(
            Paragraph::new(password_info).block(Block::default().borders(Borders::ALL).title(password_title)),
            chunks[2],
        );

        let help = Paragraph::new(
            "c=create game | p=edit join password | j/enter=join selected | r=refresh | b=home | q=exit",
        )
        .block(Block::default().borders(Borders::ALL).title("Help"));
        frame.render_widget(help, chunks[3]);
    }

    fn draw_pvp_create(&self, frame: &mut Frame<'_>) {
        let area = centered_rect(75, 65, frame.area());
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
            ])
            .split(area);

        frame.render_widget(
            Paragraph::new("Create PvP game")
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Create")),
            chunks[0],
        );

        let name_marker = if self.create_field_index == 0 { ">" } else { " " };
        let pass_marker = if self.create_field_index == 1 { ">" } else { " " };

        frame.render_widget(
            Paragraph::new(format!("{name_marker} Name (3..40): {}", self.create_name))
                .block(Block::default().borders(Borders::ALL).title("Name")),
            chunks[1],
        );

        frame.render_widget(
            Paragraph::new(format!(
                "{pass_marker} Password optional (3..32): {}",
                "*".repeat(self.create_password.len())
            ))
            .block(Block::default().borders(Borders::ALL).title("Password")),
            chunks[2],
        );

        frame.render_widget(
            Paragraph::new("Type text, Tab to switch field, Enter to create, Esc/b to go back")
                .block(Block::default().borders(Borders::ALL).title("Help")),
            chunks[3],
        );
    }

    fn draw_info(&self, frame: &mut Frame<'_>) {
        let area = centered_rect(75, 40, frame.area());
        frame.render_widget(
            Paragraph::new(self.info_message.as_str())
                .alignment(Alignment::Left)
                .block(Block::default().borders(Borders::ALL).title("Message")),
            area,
        );
    }

    fn draw_game_over(&self, frame: &mut Frame<'_>) {
        let area = centered_rect(70, 45, frame.area());
        frame.render_widget(
            Paragraph::new(format!(
                "{}\n\nPress Enter or b to return to Main Menu.\nPress q to exit.",
                self.game_over_message
            ))
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("Game Finished")),
            area,
        );
    }

    fn render_board_text(&self, board: &[Option<String>]) -> String {
        // This keeps board rendering explicit for learning purposes.
        // Each cell tracks two pieces of state: symbol value and cursor selection.
        let mut rows = Vec::new();

        for r in 0..3 {
            let mut cells = Vec::new();
            for c in 0..3 {
                let idx = r * 3 + c;
                let value = board[idx].as_deref().unwrap_or(" ");
                let label = if self.board_cursor == idx {
                    format!("[{value}]")
                } else {
                    format!(" {value} ")
                };
                cells.push(label);
            }
            rows.push(cells.join("|"));
        }

        format!(
            "{}\n-----------\n{}\n-----------\n{}\n\n1 2 3\n4 5 6\n7 8 9",
            rows[0], rows[1], rows[2]
        )
    }

    fn show_error(&mut self, message: String) {
        self.info_message = message;
        self.screen = Screen::Info;
    }

    fn is_game_finished(game: &ApiGame) -> bool {
        matches!(game.status.as_str(), "WON" | "DRAW")
    }

    fn open_game_over(&mut self, game: &ApiGame, mode_label: &str) {
        let result_line = if game.status == "WON" {
            let winner = game.winner.as_deref().unwrap_or("Unknown");
            let you = self.player_symbol_for(game);
            let outcome = if winner == you { "You won!" } else { "You lost." };
            format!("Winner: {winner} ({outcome})")
        } else {
            "Result: Draw".to_string()
        };

        self.game_over_message = format!(
            "{mode_label} game finished.\nGame id: {}\n{result_line}",
            game.id
        );
        self.screen = Screen::GameOver;
    }

    async fn create_solo_game(&self) -> Result<ApiGame> {
        let url = format!("{}/games/solo", self.base_url);
        let payload = CreateSoloRequest {
            player_id: self.player_id.clone(),
            client_name: "rust-tui-client".to_string(),
        };

        let response = self.client.post(url).json(&payload).send().await?;
        parse_json_response(response).await
    }

    async fn create_pvp_game(&self, name: &str, password: Option<String>) -> Result<ApiGame> {
        let url = format!("{}/games/pvp", self.base_url);
        let payload = CreatePvpRequest {
            player_id: self.player_id.clone(),
            name: name.to_string(),
            password,
        };

        let response = self.client.post(url).json(&payload).send().await?;
        parse_json_response(response).await
    }

    async fn list_open_pvp_games(&self) -> Result<Vec<ApiGame>> {
        let url = format!("{}/games/pvp/open", self.base_url);
        let response = self.client.get(url).send().await?;
        parse_json_response(response).await
    }

    async fn join_pvp_game(&self, game_id: &str, password: Option<String>) -> Result<ApiGame> {
        let url = format!("{}/games/pvp/{game_id}/join", self.base_url);
        let payload = JoinPvpRequest {
            player_id: self.player_id.clone(),
            password,
        };

        let response = self.client.post(url).json(&payload).send().await?;
        parse_json_response(response).await
    }

    async fn get_game(&self, game_id: &str) -> Result<ApiGame> {
        let url = format!("{}/games/{game_id}", self.base_url);
        let response = self.client.get(url).send().await?;
        parse_json_response(response).await
    }

    async fn play_move(&self, game_id: &str, index: usize) -> Result<ApiGame> {
        let url = format!("{}/games/{game_id}/move", self.base_url);
        let payload = PlayMoveRequest {
            player_id: self.player_id.clone(),
            index,
        };

        let response = self.client.post(url).json(&payload).send().await?;
        parse_json_response(response).await
    }
}

async fn parse_json_response<T: for<'de> Deserialize<'de>>(response: reqwest::Response) -> Result<T> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_else(|_| "<no body>".to_string());
        anyhow::bail!("request failed with {status}: {body}");
    }

    response
        .json::<T>()
        .await
        .context("invalid JSON response shape")
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;

    let mut terminal = ratatui::init();
    let mut app = App::new("http://localhost:3000");

    let run_result = app.run(&mut terminal).await;

    ratatui::restore();
    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen)?;

    run_result
}
