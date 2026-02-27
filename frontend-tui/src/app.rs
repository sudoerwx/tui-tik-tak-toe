use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{DefaultTerminal, Frame};
use uuid::Uuid;

use crate::{
    api::ApiClient,
    models::{ApiGame, Screen},
    ui,
};

// Main application state.
// If you know React: this is like one root component state + event handlers.
pub struct App {
    api: ApiClient,
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
    pub fn new(base_url: &str) -> Self {
        Self {
            api: ApiClient::new(base_url),
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

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            // Polling in main loop keeps architecture simple.
            // Production apps often move this to background tasks + channels.
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
                if let Ok(games) = self.api.list_open_pvp_games().await {
                    self.pvp_games = games;
                    if self.pvp_selected_index >= self.pvp_games.len() {
                        self.pvp_selected_index = self.pvp_games.len().saturating_sub(1);
                    }
                }
            }
            Screen::PvpGame => {
                // No websocket yet, so we poll server state.
                if let Some(game_id) = self.pvp_game.as_ref().map(|g| g.id.clone()) {
                    if let Ok(game) = self.api.get_game(&game_id).await {
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
                0 => match self.api.create_solo_game(&self.player_id).await {
                    Ok(game) => {
                        self.solo_game = Some(game);
                        self.board_cursor = 0;
                        self.screen = Screen::SoloGame;
                    }
                    Err(err) => {
                        self.show_error(format!("Could not start solo game: {err}"));
                    }
                },
                1 => match self.api.list_open_pvp_games().await {
                    Ok(games) => {
                        self.pvp_games = games;
                        self.pvp_selected_index = 0;
                        self.screen = Screen::PvpLobby;
                    }
                    Err(err) => {
                        self.show_error(format!("Could not load PvP games: {err}"));
                    }
                },
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
                match self
                    .api
                    .play_move(&self.player_id, &game.id, self.board_cursor)
                    .await
                {
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
            KeyCode::Char('r') => match self.api.list_open_pvp_games().await {
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

                    match self
                        .api
                        .join_pvp_game(&self.player_id, &game.id, password)
                        .await
                    {
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

                match self
                    .api
                    .create_pvp_game(&self.player_id, self.create_name.trim(), password)
                    .await
                {
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
            match self
                .api
                .play_move(&self.player_id, &game.id, self.board_cursor)
                .await
            {
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
            Screen::Home => ui::draw_home(frame, self.home_index),
            Screen::SoloGame => ui::draw_game(
                frame,
                self.solo_game.as_ref(),
                "Solo Mode",
                self.board_cursor,
                self.player_symbol_for_opt(self.solo_game.as_ref()),
            ),
            Screen::PvpLobby => ui::draw_pvp_lobby(
                frame,
                &self.pvp_games,
                self.pvp_selected_index,
                &self.join_password,
                self.editing_join_password,
            ),
            Screen::PvpCreate => ui::draw_pvp_create(
                frame,
                &self.create_name,
                &self.create_password,
                self.create_field_index,
            ),
            Screen::PvpGame => ui::draw_game(
                frame,
                self.pvp_game.as_ref(),
                "PvP Mode",
                self.board_cursor,
                self.player_symbol_for_opt(self.pvp_game.as_ref()),
            ),
            Screen::GameOver => ui::draw_game_over(frame, &self.game_over_message),
            Screen::Info => ui::draw_info(frame, &self.info_message),
        }
    }

    fn player_symbol_for_opt(&self, game: Option<&ApiGame>) -> String {
        game.map(|g| self.player_symbol_for(g))
            .unwrap_or_else(|| "?".to_string())
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
            let outcome = if winner == you {
                "You won!"
            } else {
                "You lost."
            };
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
}
