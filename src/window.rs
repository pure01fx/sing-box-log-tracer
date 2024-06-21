use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use tokio::sync::mpsc;

pub fn initialize_panic_handler() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        shutdown().unwrap();
        original_hook(panic_info);
    }));
}

fn startup() -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;
    Ok(())
}

fn shutdown() -> Result<()> {
    crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

pub trait InputEventHandler<A>: Send + 'static
{
    fn handle_input_event(
        &self,
        event: crossterm::event::Event,
        tx: &mpsc::UnboundedSender<A>,
    ) -> Result<()>;
}

pub trait TerminalApp {
    type Action: Send + 'static;
    type InputEventHandler: InputEventHandler<Self::Action>;

    fn create_input_event_handler(&self) -> Self::InputEventHandler;
    fn handle_input_event(&mut self, action: Self::Action);
    async fn update(&mut self) -> Result<()>;
    fn draw(&self, f: &mut Frame);
    fn should_quit(&self) -> bool;
}

fn create_input_event_handler<T: Send + 'static>(
    input: impl InputEventHandler<T>,
    tx: mpsc::UnboundedSender<T>,
) -> tokio::task::JoinHandle<Result<()>> {
    let tick_rate = std::time::Duration::from_millis(250);
    tokio::spawn(async move {
        loop {
            if crossterm::event::poll(tick_rate).unwrap() {
                let event = crossterm::event::read().unwrap();
                input.handle_input_event(event, &tx)?;
            }
        }
    })
}

pub async fn tui_run(mut app: impl TerminalApp) -> Result<()> {
    initialize_panic_handler();
    startup()?;

    let mut t = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    let (action_tx, mut action_rx) = mpsc::unbounded_channel();

    let task = create_input_event_handler(app.create_input_event_handler(), action_tx);

    loop {
        loop {
            match action_rx.try_recv() {
                Ok(action) => app.handle_input_event(action),
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(e) => {
                    eprintln!("{}", anyhow::format_err!("Error receiving action: {}", e));
                    break;
                }
            }
        }

        if let Err(e) = app.update().await {
            eprintln!("{}", anyhow::format_err!("Error updating app: {}", e));
            break;
        }

        if task.is_finished() {
            break;
        }

        if app.should_quit() {
            break;
        }

        t.draw(|f| {
            app.draw(f);
        })?;
    }

    if task.is_finished() {
        match task.await {
            Ok(_) => eprintln!(
                "{}",
                anyhow::format_err!("Input event handler task finished unexpectedly")
            ),
            Err(e) => eprintln!(
                "{}",
                anyhow::format_err!("Error in input event handler: {}", e)
            ),
        }
    } else {
        task.abort();
        task.await??;
    }

    shutdown()?;
    Ok(())
}
