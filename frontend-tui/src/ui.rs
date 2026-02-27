use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::models::ApiGame;

pub fn draw_home(frame: &mut Frame<'_>, home_index: usize) {
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
            let line = if idx == home_index {
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

pub fn draw_game(
    frame: &mut Frame<'_>,
    game: Option<&ApiGame>,
    title: &str,
    board_cursor: usize,
    player_symbol: String,
) {
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

    let status_line = if game.status == "WON" {
        format!(
            "Status: WON | Winner: {}",
            game.winner.clone().unwrap_or_default()
        )
    } else {
        format!("Status: {}", game.status)
    };

    let header = Paragraph::new(format!(
        "Game id: {}\nMode: {} | You are: {} | Current turn: {}\n{}",
        game.id, game.mode, player_symbol, game.current_turn, status_line
    ))
    .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(header, chunks[0]);

    let board_text = render_board_text(&game.board, board_cursor);
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

pub fn draw_pvp_lobby(
    frame: &mut Frame<'_>,
    pvp_games: &[ApiGame],
    selected_index: usize,
    join_password: &str,
    editing_join_password: bool,
) {
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

    let items: Vec<ListItem> = if pvp_games.is_empty() {
        vec![ListItem::new("No open games")]
    } else {
        pvp_games
            .iter()
            .enumerate()
            .map(|(idx, game)| {
                let prefix = if idx == selected_index { ">" } else { " " };
                let name = game.name.clone().unwrap_or_else(|| "Untitled".to_string());
                let pass = if game.has_password { "locked" } else { "open" };
                ListItem::new(format!("{prefix} {name} | id={} | {pass}", game.id))
            })
            .collect()
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Games"));
    frame.render_widget(list, chunks[1]);

    let password_info = if join_password.is_empty() {
        "Join password: <empty>".to_string()
    } else {
        format!("Join password: {}", "*".repeat(join_password.len()))
    };
    let password_title = if editing_join_password {
        "Join Password (editing, Enter/Esc to stop)"
    } else {
        "Join Password (press p to edit)"
    };
    frame.render_widget(
        Paragraph::new(password_info)
            .block(Block::default().borders(Borders::ALL).title(password_title)),
        chunks[2],
    );

    let help = Paragraph::new(
        "c=create game | p=edit join password | j/enter=join selected | r=refresh | b=home | q=exit",
    )
    .block(Block::default().borders(Borders::ALL).title("Help"));
    frame.render_widget(help, chunks[3]);
}

pub fn draw_pvp_create(
    frame: &mut Frame<'_>,
    create_name: &str,
    create_password: &str,
    create_field_index: usize,
) {
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

    let name_marker = if create_field_index == 0 { ">" } else { " " };
    let pass_marker = if create_field_index == 1 { ">" } else { " " };

    frame.render_widget(
        Paragraph::new(format!("{name_marker} Name (3..40): {create_name}"))
            .block(Block::default().borders(Borders::ALL).title("Name")),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new(format!(
            "{pass_marker} Password optional (3..32): {}",
            "*".repeat(create_password.len())
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

pub fn draw_info(frame: &mut Frame<'_>, info_message: &str) {
    let area = centered_rect(75, 40, frame.area());
    frame.render_widget(
        Paragraph::new(info_message)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("Message")),
        area,
    );
}

pub fn draw_game_over(frame: &mut Frame<'_>, game_over_message: &str) {
    let area = centered_rect(70, 45, frame.area());
    frame.render_widget(
        Paragraph::new(format!(
            "{game_over_message}\n\nPress Enter or b to return to Main Menu.\nPress q to exit."
        ))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Game Finished"),
        ),
        area,
    );
}

fn render_board_text(board: &[Option<String>], board_cursor: usize) -> String {
    // Explicit board mapping to keep control flow easy to follow for beginners.
    let mut rows = Vec::new();

    for r in 0..3 {
        let mut cells = Vec::new();
        for c in 0..3 {
            let idx = r * 3 + c;
            let value = board[idx].as_deref().unwrap_or(" ");
            let label = if board_cursor == idx {
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
