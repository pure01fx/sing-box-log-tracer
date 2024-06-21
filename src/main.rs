mod app;
mod log;
mod ui;
mod window;

use std::time::Duration;

use anyhow::{Context, Result};
use app::App;
use surf::Url;
use tokio::sync::mpsc;
use ui::ui;
use window::tui_run;

enum Action {
    Quit,
}

struct TerminalApp {
    app: App,
    should_quit: bool,
}

impl TerminalApp {
    fn new(app: App) -> Self {
        Self {
            app,
            should_quit: false,
        }
    }
}

#[derive(Clone)]
struct InputEventHandler {}

impl window::InputEventHandler<Action> for InputEventHandler {
    fn handle_input_event(
        &self,
        event: crossterm::event::Event,
        tx: &mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        if let crossterm::event::Event::Key(key) = event {
            if key.kind == crossterm::event::KeyEventKind::Press {
                match key.code {
                    crossterm::event::KeyCode::Char('q') => {
                        tx.send(Action::Quit).unwrap();
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
}

impl window::TerminalApp for TerminalApp {
    type Action = Action;
    type InputEventHandler = InputEventHandler;

    fn create_input_event_handler(&self) -> Self::InputEventHandler {
        InputEventHandler {}
    }

    fn handle_input_event(&mut self, action: Self::Action) {
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
        }
    }

    async fn update(&mut self) -> Result<()> {
        self.app.fetch().await
    }

    fn draw(&self, f: &mut ratatui::prelude::Frame) {
        ui(f, &self.app);
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }
}

struct DebugAppConfig {}

impl app::AppConfig for DebugAppConfig {
    fn base_url(&self) -> Url {
        "http://localhost:9090".try_into().unwrap()
    }

    fn cache_size(&self) -> u64 {
        10000
    }

    fn time_to_idle(&self) -> Duration {
        Duration::from_secs(60)
    }

    fn secret(&self) -> Option<String> {
        None
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new(DebugAppConfig {})
        .await
        .context("Failed to create core app")?;

    loop {
        app.fetch().await?;
    }

    let app = TerminalApp::new(app);
    let result = tui_run(app).await;
    result
}
