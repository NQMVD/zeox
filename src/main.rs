use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use requestty::prompt;
use std::{
    io,
    process::Command,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

enum Screen {
    Main,
    List,
    Stats,
}

struct App {
    current_screen: Screen,
    tracking_status: String,
    list_output: String,
    stats_output: String,
}

impl App {
    fn new() -> Self {
        Self {
            current_screen: Screen::Main,
            tracking_status: String::new(),
            list_output: String::new(),
            stats_output: String::new(),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create application state
    let mut app = App::new();

    // Run the application
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Handle any errors
    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let (tx, rx): (Sender<()>, Receiver<()>) = mpsc::channel();

    // Spawn a thread to update the tracking status every second
    let tracking_tx = tx.clone();
    thread::spawn(move || loop {
        tracking_tx.send(()).unwrap();
        thread::sleep(Duration::from_secs(1));
    });

    loop {
        // Check for tracking status updates
        while let Ok(_) = rx.try_recv() {
            app.tracking_status = get_current_tracking();
            if let Screen::Main = app.current_screen {
                // Redraw the UI if we're on the main screen
                terminal.draw(|f| ui(f, app))?;
            }
        }

        // Draw the UI
        terminal.draw(|f| ui(f, app))?;

        // Handle input events
        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.current_screen {
                    Screen::Main => match key.code {
                        KeyCode::Char('q') => {
                            // Exit the application
                            break;
                        }
                        KeyCode::Char('s') => {
                            // Start tracking
                            start_tracking();
                            app.tracking_status = get_current_tracking();
                        }
                        KeyCode::Char('f') => {
                            // Finish tracking
                            finish_tracking();
                            app.tracking_status = get_current_tracking();
                        }
                        KeyCode::Char('l') => {
                            // Switch to list screen
                            app.current_screen = Screen::List;
                            app.list_output = get_list_output();
                        }
                        KeyCode::Char('d') => {
                            // Switch to stats screen
                            app.current_screen = Screen::Stats;
                            app.stats_output = get_stats_output();
                        }
                        _ => {}
                    },
                    Screen::List | Screen::Stats => match key.code {
                        KeyCode::Char('b') => {
                            // Go back to main screen
                            app.current_screen = Screen::Main;
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        widgets::{Block, Borders, Paragraph, Wrap},
    };

    let size = f.area();

    match app.current_screen {
        Screen::Main => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
                .split(size);

            let block = Block::default().title("Zeit Tracker").borders(Borders::ALL);

            let paragraph = Paragraph::new(app.tracking_status.clone())
                .block(block)
                .wrap(Wrap { trim: true });

            f.render_widget(paragraph, chunks[0]);

            let instructions =
                Paragraph::new("q: quit • s: start • f: finish • l: list • d: stats")
                    .wrap(Wrap { trim: true });

            f.render_widget(instructions, chunks[1]);
        }
        Screen::List => {
            let block = Block::default()
                .title("Tracked Activities")
                .borders(Borders::ALL);

            let paragraph = Paragraph::new(app.list_output.clone())
                .block(block)
                .wrap(Wrap { trim: true });

            f.render_widget(paragraph, size);

            let instructions = Paragraph::new("b: back").wrap(Wrap { trim: true });

            f.render_widget(instructions, size);
        }
        Screen::Stats => {
            let block = Block::default().title("Statistics").borders(Borders::ALL);

            let paragraph = Paragraph::new(app.stats_output.clone())
                .block(block)
                .wrap(Wrap { trim: true });

            f.render_widget(paragraph, size);

            let instructions = Paragraph::new("b: back").wrap(Wrap { trim: true });

            f.render_widget(instructions, size);
        }
    }
}

fn get_current_tracking() -> String {
    // Execute 'zeit tracking' and capture the output
    let output = Command::new("zeit")
        .arg("tracking")
        .arg("--no-colors") // Added '--no-colors' flag
        .output()
        .expect("Failed to execute 'zeit tracking'");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if stdout.trim().is_empty() {
            "No active tracking.".to_string()
        } else {
            stdout
        }
    } else {
        "Error getting tracking status.".to_string()
    }
}

fn start_tracking() {
    // Temporarily disable raw mode and leave alternate screen
    disable_raw_mode().unwrap();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();

    // Prompt for project name and task name using requestty
    let project_question = requestty::Question::input("project")
        .message("Enter project name:")
        .validate(|input, _| {
            if input.trim().is_empty() {
                Err("Project name cannot be empty".to_string()) // Appended .to_string()
            } else {
                Ok(())
            }
        })
        .build();

    let task_question = requestty::Question::input("task")
        .message("Enter task name (optional):")
        .build();

    let begin_question = requestty::Question::input("begin")
        .message("Enter start time (e.g., '16:00' or '-0:15', leave empty for now):")
        .build();

    let answers = requestty::prompt(vec![project_question, task_question, begin_question]).unwrap();

    // Restore terminal settings
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
    enable_raw_mode().unwrap();

    // Build the command arguments
    let mut args = vec!["track"];

    if let Some(project_name) = answers.get("project").and_then(|a| a.as_string()) {
        args.push("--project");
        args.push(project_name);
    }

    if let Some(task_name) = answers.get("task").and_then(|a| a.as_string()) {
        if !task_name.trim().is_empty() {
            args.push("--task");
            args.push(task_name);
        }
    }

    if let Some(begin_time) = answers.get("begin").and_then(|a| a.as_string()) {
        if !begin_time.trim().is_empty() {
            args.push("--begin");
            args.push(begin_time);
        }
    }

    args.push("--no-colors"); // Added '--no-colors' flag

    // Start tracking the specified project and task
    let output = Command::new("zeit")
        .args(&args)
        .output()
        .expect("Failed to execute 'zeit track'");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Failed to start tracking: {}", stderr);
    }
}

fn finish_tracking() {
    // Temporarily disable raw mode and leave alternate screen
    disable_raw_mode().unwrap();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();

    // Prompt for optional task and time adjustments using requestty
    let task_question = requestty::Question::input("task")
        .message("Enter new task name (optional):")
        .build();

    let begin_question = requestty::Question::input("begin")
        .message("Adjust start time (optional):")
        .build();

    let finish_question = requestty::Question::input("finish")
        .message("Adjust finish time (optional):")
        .build();

    let answers = requestty::prompt(vec![task_question, begin_question, finish_question]).unwrap();

    // Restore terminal settings
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
    enable_raw_mode().unwrap();

    // Build the command arguments
    let mut args = vec!["finish"];

    if let Some(task_name) = answers.get("task").and_then(|a| a.as_string()) {
        if !task_name.trim().is_empty() {
            args.push("--task");
            args.push(task_name);
        }
    }

    if let Some(begin_time) = answers.get("begin").and_then(|a| a.as_string()) {
        if !begin_time.trim().is_empty() {
            args.push("--begin");
            args.push(begin_time);
        }
    }

    if let Some(finish_time) = answers.get("finish").and_then(|a| a.as_string()) {
        if !finish_time.trim().is_empty() {
            args.push("--finish");
            args.push(finish_time);
        }
    }

    args.push("--no-colors"); // Added '--no-colors' flag

    // Finish the current tracking session
    let output = Command::new("zeit")
        .args(&args)
        .output()
        .expect("Failed to execute 'zeit finish'");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Failed to finish tracking: {}", stderr);
    }
}

fn get_list_output() -> String {
    // Execute 'zeit list' and capture the output
    let output = Command::new("zeit")
        .arg("list")
        .arg("--no-colors") // Added '--no-colors' flag
        .output()
        .expect("Failed to execute 'zeit list'");

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).into_owned()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        format!("Error getting list: {}", stderr)
    }
}

fn get_stats_output() -> String {
    // Execute 'zeit stats' and capture the output
    let output = Command::new("zeit")
        .arg("stats")
        .arg("--no-colors") // Added '--no-colors' flag
        .output()
        .expect("Failed to execute 'zeit stats'");

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).into_owned()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        format!("Error getting stats: {}", stderr)
    }
}
