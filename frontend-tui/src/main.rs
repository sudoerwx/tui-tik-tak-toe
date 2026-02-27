mod api;
mod app;
mod models;
mod ui;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;

    let mut terminal = ratatui::init();
    let mut app = App::new("http://localhost:3000");

    let run_result = app.run(&mut terminal).await;

    // Always restore terminal state, even if the app loop failed.
    ratatui::restore();
    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen)?;

    run_result
}
