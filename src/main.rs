use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Row, Sparkline, Table},
    DefaultTerminal, Frame,
};
use sysinfo::{Disks, System};

struct App {
    sys: System,
    disks: Disks,
    cpu_history: Vec<u64>,
    ram_history: Vec<u64>,
    last_tick: Instant,
}

impl App {
    fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        let disks = Disks::new_with_refreshed_list();

        Self {
            sys,
            disks,
            cpu_history: Vec::with_capacity(60),
            ram_history: Vec::with_capacity(60),
            last_tick: Instant::now(),
        }
    }

    fn update(&mut self) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.disks = Disks::new_with_refreshed_list();

        let global_cpu = self.sys.global_cpu_usage() as u64;
        let total_ram = self.sys.total_memory();
        let used_ram = self.sys.used_memory();
        let ram_pct = if total_ram > 0 {
            ((used_ram as f64 / total_ram as f64) * 100.0) as u64
        } else {
            0
        };

        // Maintain a sliding window of 60 seconds for historical graphs
        if self.cpu_history.len() >= 60 {
            self.cpu_history.remove(0);
        }
        self.cpu_history.push(global_cpu);

        if self.ram_history.len() >= 60 {
            self.ram_history.remove(0);
        }
        self.ram_history.push(ram_pct);
    }
}

fn main() -> io::Result<()> {
    // Initialize Ratatui terminal session
    let mut terminal = ratatui::init();
    let app = App::new();
    let result = run_app(&mut terminal, app);

    // Restore standard terminal configuration on exit
    ratatui::restore();
    result
}

fn run_app(terminal: &mut DefaultTerminal, mut app: App) -> io::Result<()> {
    let tick_rate = Duration::from_secs(1);

    loop {
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate.saturating_sub(app.last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        _ => {}
                    }
                }
            }
        }

        if app.last_tick.elapsed() >= tick_rate {
            app.update();
            app.last_tick = Instant::now();
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Footer
        ])
        .split(f.area());

    // --- Header ---
    let header = Paragraph::new(" System Monitor Dashboard  |  Press 'q' or 'Esc' to exit ")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
    f.render_widget(header, main_chunks[0]);

    // Split Body horizontally (50% left, 50% right)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    // --- Left Column: CPU & RAM Metrics ---
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // CPU Gauge
            Constraint::Length(4), // CPU Sparkline Graph
            Constraint::Length(3), // RAM Gauge
            Constraint::Length(4), // RAM Sparkline Graph
            Constraint::Min(0),    // Cores Table
        ])
        .split(body_chunks[0]);

    // Overall CPU Gauge
    let global_cpu = app.sys.global_cpu_usage();
    let cpu_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(format!(" CPU Load: {:.1}% ", global_cpu)))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(global_cpu.clamp(0.0, 100.0) as u16);
    f.render_widget(cpu_gauge, left_chunks[0]);

    // CPU History Graph
    let cpu_sparkline = Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title(" CPU History (60s) "))
        .data(&app.cpu_history)
        .max(100)
        .style(Style::default().fg(Color::LightGreen));
    f.render_widget(cpu_sparkline, left_chunks[1]);

    // RAM Usage Gauge
    let bytes_to_gb = 1_073_741_824.0;
    let total_ram = app.sys.total_memory() as f64 / bytes_to_gb;
    let used_ram = app.sys.used_memory() as f64 / bytes_to_gb;
    let ram_pct = if total_ram > 0.0 { (used_ram / total_ram) * 100.0 } else { 0.0 };

    let ram_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(format!(
            " RAM: {:.2} GB / {:.2} GB ({:.1}%) ",
            used_ram, total_ram, ram_pct
        )))
        .gauge_style(Style::default().fg(Color::Magenta))
        .percent(ram_pct.clamp(0.0, 100.0) as u16);
    f.render_widget(ram_gauge, left_chunks[2]);

    // RAM History Graph
    let ram_sparkline = Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title(" RAM History (60s) "))
        .data(&app.ram_history)
        .max(100)
        .style(Style::default().fg(Color::LightMagenta));
    f.render_widget(ram_sparkline, left_chunks[3]);

    // CPU Cores Table
    let core_rows: Vec<Row> = app
        .sys
        .cpus()
        .iter()
        .enumerate()
        .map(|(i, cpu)| {
            Row::new(vec![
                format!("Core {:<2}", i),
                format!("{:>5.1}%", cpu.cpu_usage()),
                format!("{} MHz", cpu.frequency()),
            ])
        })
        .collect();

    let core_table = Table::new(
        core_rows,
        [Constraint::Percentage(30), Constraint::Percentage(35), Constraint::Percentage(35)],
    )
    .header(Row::new(vec!["Core", "Load", "Frequency"]).style(Style::default().fg(Color::Yellow)))
    .block(Block::default().borders(Borders::ALL).title(" Logical Cores "));
    f.render_widget(core_table, left_chunks[4]);

    // --- Right Column: Disk Information ---
    let disk_rows: Vec<Row> = app
        .disks
        .iter()
        .map(|disk| {
            let total = disk.total_space() as f64 / bytes_to_gb;
            let avail = disk.available_space() as f64 / bytes_to_gb;
            let used = total - avail;
            let pct = if total > 0.0 { (used / total) * 100.0 } else { 0.0 };

            Row::new(vec![
                disk.mount_point().to_string_lossy().into_owned(),
                disk.file_system().to_string_lossy().into_owned(),
                format!("{:.1}/{:.1} GB", used, total),
                format!("{:.1}%", pct),
            ])
        })
        .collect();

    let disk_table = Table::new(
        disk_rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
        ],
    )
    .header(Row::new(vec!["Mount", "Type", "Used / Total", "Usage"]).style(Style::default().fg(Color::Yellow)))
    .block(Block::default().borders(Borders::ALL).title(" Disks "));
    f.render_widget(disk_table, body_chunks[1]);

    // --- Footer ---
    let footer = Paragraph::new(" Status: Running  |  Refresh Interval: 1000ms ")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, main_chunks[2]);
}