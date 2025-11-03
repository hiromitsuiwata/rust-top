use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Row, Table},
};
use std::io;
use std::time::{Duration, Instant};
use sysinfo::{Product, System};

fn main() -> Result<(), io::Error> {
    // 端末をTUIモードに切り替える
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal);

    // 終了処理
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut sys = System::new_all();
    let tick_rate = Duration::from_secs(1);
    let mut last_tick = Instant::now();

    loop {
        // 情報更新
        sys.refresh_all();

        terminal.draw(|f| {
            let size = f.area();

            // レイアウト（縦分割）
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(8),
                    Constraint::Min(10),
                ])
                .split(size);

            // CPU情報
            let cpu_usage: f32 = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>();
            let all_cpu_usage: f32 = sys.cpus().len() as f32 * 100.0;
            let cpu_block = Paragraph::new(format!(
                "CPU Usage: {:.1}% / {}%",
                cpu_usage, all_cpu_usage
            ))
            .block(Block::default().borders(Borders::ALL).title("CPU"))
            .style(Style::default().fg(Color::Yellow));
            f.render_widget(cpu_block, chunks[0]);

            // メモリ情報
            let total_memory = sys.total_memory() / 1024 / 1024;
            let used_memory = (sys.used_memory()) / 1024 / 1024;
            // let free_swap = sys.free_swap();
            let total_swap = sys.total_swap() / 1024 / 1024;
            let used_swap = sys.used_swap() / 1024 / 1024;
            let mem_block = Paragraph::new(format!("Memory: {used_memory} MB / {total_memory} MB, Swap: {used_swap} MB / {total_swap} MB"))
                .block(Block::default().borders(Borders::ALL).title("Memory"))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(mem_block, chunks[1]);

            // プロセス情報（上位5件）
            let mut processes: Vec<_> = sys.processes().values().collect();
            processes.sort_by_key(|p| -(p.cpu_usage() as i32));
            let rows: Vec<Row> = processes
                .iter()
                .take(5)
                .map(|p| {
                    Row::new(vec![
                        p.pid().to_string(),
                        p.name().to_string_lossy().to_string(),
                        format!("{:.1}%", p.cpu_usage()),
                        format!("{:.1} MB", p.memory() as f64 / 1024.0),
                    ])
                })
                .collect();
            let table = Table::new(
                rows,
                [
                    Constraint::Length(8),
                    Constraint::Length(25),
                    Constraint::Length(10),
                    Constraint::Length(12),
                ],
            )
            .header(
                Row::new(vec!["PID", "Name", "CPU", "Memory"])
                    .style(Style::default().fg(Color::Green)),
            )
            .block(Block::default().borders(Borders::ALL).title("Processes"));
            f.render_widget(table, chunks[2]);


            // システム情報
            let mut info_rows: Vec<Row> = Vec::new();

            let number_of_cpus = sys.cpus().len().to_string();
            info_rows.push(Row::new(vec!["Number of cpus", number_of_cpus.as_str()]));

            let cpu_arch = System::cpu_arch();
            info_rows.push(Row::new(vec!["CPU Architecture", cpu_arch.as_str()]));

            let brand: &str = sys.cpus().get(0).map_or("Unknown", |c| c.brand());
            info_rows.push(Row::new(vec!["Brand", brand]));

            let uptime = System::uptime().to_string();
            info_rows.push(Row::new(vec!["Uptime", uptime.as_str()]));

            let kernel_long_version = System::kernel_long_version();
            info_rows.push(Row::new(vec!["kernel long version", kernel_long_version.as_str()]));

            let long_os_version = System::long_os_version();
            info_rows.push(Row::new(vec!["long os version", long_os_version.as_deref().unwrap_or("Unknown")]));

            let host_name = System::host_name();
            info_rows.push(Row::new(vec!["Host name", host_name.as_deref().unwrap_or("Unknown")]));

            let open_files_limit = System::open_files_limit();
            let open_files_limit_str = open_files_limit
                .map(|v| v.to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            info_rows.push(Row::new(vec!["Open files limit", open_files_limit_str.as_str()]));

            let product_name = Product::name();
            info_rows.push(Row::new(vec!["Product Name", product_name.as_deref().unwrap_or("Unknown")]));
            
            let vendor_name = Product::vendor_name();
            info_rows.push(Row::new(vec!["Vendor name", vendor_name.as_deref().unwrap_or("Unknown")]));

            // info_rows.push(Row::new(vec![]));
            // info_rows.push(Row::new(vec![]));

            let info_table = Table::new(info_rows, [
                Constraint::Length(25),
                Constraint::Length(60),
            ]).block(Block::default().borders(Borders::ALL).title("Info"));
            f.render_widget(info_table, chunks[3]);
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    Ok(())
}
