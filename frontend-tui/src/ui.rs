// Importing UI rendering primitives from ratatui crate and our API game model
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect}, // Layout handles positioning and size of widgets
    style::{Modifier, Style}, // Style lets us control text formatting like bold
    text::{Line, Span}, // Line and Span let us create individual styled pieces of text
    widgets::{Block, Borders, List, ListItem, Paragraph}, // Various UI widgets for display
    Frame, // Frame is the canvas to render widgets onto
};

use crate::models::ApiGame; // Our own API game type

// Draw the home screen UI. home_index determines which menu item is highlighted.
/// Draws the main Home screen of the TUI application.
/// Arguments:
/// - `frame`: The drawing surface passed in each render cycle. Ratatui's Frame is what you use to render widgets.
/// - `home_index`: Which menu item to highlight (e.g. user selection).
pub fn draw_home(frame: &mut Frame<'_>, home_index: usize) {
    // Layout splits the rendering area vertically using percentage and fixed constraints
    let area = centered_rect(70, 65, frame.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),    // Title
            Constraint::Length(8),    // Menu
            Constraint::Length(3),    // Help area
            Constraint::Min(1),       // Fills remaining space
        ])
        .split(area);

    // Title with borders and centered alignment
    let title = Paragraph::new("Tic-Tac-Toe (NestJS + Rust TUI)")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Home"));
    frame.render_widget(title, chunks[0]);

    // Menu items for navigating different modes. ListItem allows custom highlighting.
    let items = ["Solo vs Computer", "PvP", "Exit"];
    let menu_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(idx, label)| {
            let line = if idx == home_index {
                // Highlight selected item with bold and prefix
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

    // Help paragraph, contains quick instructions for the user
    let help = Paragraph::new(
        "Arrow Up/Down + Enter to select.\nq exits from anywhere.\nPlayer session id is generated once per app launch.",
    )
    .block(Block::default().borders(Borders::ALL).title("Help"));
    frame.render_widget(help, chunks[2]);
}

/// Draws the main Tic-Tac-Toe gameplay UI.
/// Arguments:
/// - `frame`: Drawing surface passed each render cycle.
/// - `game`: Optionally references the game state (None: no game running).
/// - `title`: A string used in the UI block title.
/// - `board_cursor`: Which cell is 'hovered' for input.
/// - `player_symbol`: The player's game symbol (e.g. 'X' or 'O').
///
/// Rust lifetime syntax ('_): Means 'frame' can borrow from its context for as long as needed in this function.
pub fn draw_game(
    frame: &mut Frame<'_>,
    game: Option<&ApiGame>,
    title: &str,
    board_cursor: usize,
    player_symbol: String,
) {
    // Use centered_rect to calculate the display area: makes UI responsive to terminal size.
    let area = centered_rect(80, 90, frame.area());
    // Layout splits this area vertically for different widget blocks
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),     // Header
            Constraint::Length(11),    // Tic-tac-toe board
            Constraint::Length(5),     // Controls/hint
            Constraint::Min(1),        // Fills space
        ])
        .split(area);

    // If game is None, show empty message and return
    let Some(game) = game else {
        frame.render_widget(
            Paragraph::new("No active game.")
                .block(Block::default().borders(Borders::ALL).title(title)),
            area,
        );
        return;
    };

    // Status display: shows win, ongoing status, or winner
    let status_line = if game.status == "WON" {
        format!(
            "Status: WON | Winner: {}",
            game.winner.clone().unwrap_or_default()
        )
    } else {
        format!("Status: {}", game.status)
    };

    // Render header with game info
    let header = Paragraph::new(format!(
        "Game id: {}\nMode: {} | You are: {} | Current turn: {}\n{}",
        game.id, game.mode, player_symbol, game.current_turn, status_line
    ))
    .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(header, chunks[0]);

    // Render tic-tac-toe board (uses helper below to make board text)
    let board_text = render_board_text(&game.board, board_cursor);
    let board = Paragraph::new(board_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Board (Arrows or 1..9, Enter to play)"),
    );
    frame.render_widget(board, chunks[1]);

    // Input hint and PvP info
    let hint = Paragraph::new(
        "Controls: Enter/Space = move, b = back, q = exit.\nPvP screen auto-refreshes each second for opponent moves.",
    )
    .block(Block::default().borders(Borders::ALL).title("Controls"));
    frame.render_widget(hint, chunks[2]);
}

/// Draws the PvP lobby screen displaying available multiplayer games.
/// Arguments:
/// - `frame`: Drawing surface for rendering widgets (see ratatui Frame).
/// - `pvp_games`: Slice of available game objects for lobby display.
/// - `selected_index`: Which list item is highlighted (current selection).
/// - `join_password`: Current password input for joining a game.
/// - `editing_join_password`: Boolean, true if currently in password editing mode.
///
/// This function uses ratatui's List and Paragraph widgets extensively to visualize lobby options and information.
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

/// Draws the PvP game creation screen.
/// Arguments:
/// - `frame`: Drawing surface for rendering widgets.
/// - `create_name`: Current name input for new game.
/// - `create_password`: Current password input for new game.
/// - `create_field_index`: Which input field is selected (0 for name, 1 for password).
///
/// Explains input UX and visual feedback for both fields, including password hiding.
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

/// Shows a single informational message popup.
/// Arguments:
/// - `frame`: Drawing surface for widgets.
/// - `info_message`: The text to display.
///
/// Uses a simple paragraph block. This can be used for error messages, notifications, etc.
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

/// Constructs a string representation of the tic-tac-toe board for display in the UI.
/// Arguments:
/// - `board`: Represents the current board cell values. Each Option<String> is either Some(symbol) or None.
/// - `board_cursor`: Index (0..8) of the cell currently highlighted/selected.
/// Returns:
/// - String: Multi-line string representing the board layout.
///
/// This visualization is used for rendering the board in the terminal. Highlighted cells are bracketed.
fn render_board_text(board: &[Option<String>], board_cursor: usize) -> String {
    // Explicit board mapping to keep control flow easy to follow for beginners.
    let mut rows = Vec::new();

    for r in 0..3 {
        let mut cells = Vec::new();
        for c in 0..3 {
            let idx = r * 3 + c;
            let value = board[idx].as_deref().unwrap_or(" ");
            let label = if board_cursor == idx {
                format!("[{value}]") // Highlight selected cell with brackets
            } else {
                format!(" {value} ") // Unselected cell
            };
            cells.push(label);
        }
        rows.push(cells.join("|")); // row separator
    }

    // Headers for numeric cell input shortcuts
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
