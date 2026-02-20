use std::io;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use serde::Deserialize;

use crate::config::ShipitConfig;
use crate::ssh::SshSession;

// --- Data structures ---

#[derive(Debug, Deserialize)]
struct DockerPsEntry {
    #[serde(alias = "ID")]
    id: String,
    #[serde(alias = "Names")]
    names: String,
    #[serde(alias = "Image")]
    image: String,
    #[serde(alias = "Status")]
    status: String,
    #[serde(alias = "Ports")]
    ports: String,
    #[serde(alias = "State")]
    state: String,
}

#[derive(Debug, Deserialize)]
struct DockerStatsEntry {
    #[serde(alias = "Name")]
    name: String,
    #[serde(alias = "CPUPerc")]
    cpu_perc: String,
    #[serde(alias = "MemUsage")]
    mem_usage: String,
    #[serde(alias = "MemPerc")]
    mem_perc: String,
}

#[derive(Debug, Clone)]
struct ContainerInfo {
    name: String,
    image: String,
    status: String,
    state: String,
    cpu_perc: String,
    mem_usage: String,
    mem_perc: String,
    ports: String,
}

#[derive(Debug, Clone)]
struct DiskInfo {
    size: String,
    used: String,
    avail: String,
    use_percent: String,
}

#[derive(Debug, Clone)]
struct HostStatus {
    address: String,
    containers: Vec<ContainerInfo>,
    disk: Option<DiskInfo>,
    error: Option<String>,
}

struct AppState {
    hosts: Vec<HostStatus>,
    app_name: String,
    stage_name: String,
    interval: u64,
    last_update: String,
}

// --- Parsing ---

fn parse_ps(output: &str) -> Vec<DockerPsEntry> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<DockerPsEntry>(line).ok())
        .collect()
}

fn parse_stats(output: &str) -> Vec<DockerStatsEntry> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<DockerStatsEntry>(line).ok())
        .collect()
}

fn parse_disk(output: &str) -> Option<DiskInfo> {
    let line = output.trim();
    if line.is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 4 {
        Some(DiskInfo {
            size: parts[0].to_string(),
            used: parts[1].to_string(),
            avail: parts[2].to_string(),
            use_percent: parts[3].to_string(),
        })
    } else {
        None
    }
}

fn merge_ps_stats(
    ps_entries: Vec<DockerPsEntry>,
    stats_entries: Vec<DockerStatsEntry>,
) -> Vec<ContainerInfo> {
    let stats_map: std::collections::HashMap<String, &DockerStatsEntry> = stats_entries
        .iter()
        .map(|s| (s.name.clone(), s))
        .collect();

    ps_entries
        .into_iter()
        .map(|ps| {
            let stats = stats_map.get(&ps.names);
            ContainerInfo {
                name: ps.names,
                image: ps.image,
                status: ps.status,
                state: ps.state,
                cpu_perc: stats.map(|s| s.cpu_perc.clone()).unwrap_or_default(),
                mem_usage: stats
                    .map(|s| {
                        // Extract just the usage part before " / "
                        s.mem_usage
                            .split(" / ")
                            .next()
                            .unwrap_or(&s.mem_usage)
                            .to_string()
                    })
                    .unwrap_or_default(),
                mem_perc: stats.map(|s| s.mem_perc.clone()).unwrap_or_default(),
                ports: ps.ports,
            }
        })
        .collect()
}

// --- SSH polling ---

async fn poll_host(session: &SshSession, deploy_to: &str) -> HostStatus {
    let address = session.host().to_string();

    // docker ps
    let ps_result = session
        .exec("docker ps -a --format '{{json .}}'")
        .await;

    let ps_entries = match &ps_result {
        Ok(output) => parse_ps(output),
        Err(e) => {
            return HostStatus {
                address,
                containers: vec![],
                disk: None,
                error: Some(format!("docker ps failed: {}", e)),
            };
        }
    };

    // docker stats (only if there are running containers)
    let running_names: Vec<&str> = ps_entries
        .iter()
        .filter(|p| p.state == "running")
        .map(|p| p.names.as_str())
        .collect();

    let stats_entries = if running_names.is_empty() {
        vec![]
    } else {
        match session
            .exec("docker stats --no-stream --format '{{json .}}'")
            .await
        {
            Ok(output) => parse_stats(&output),
            Err(_) => vec![],
        }
    };

    // df
    let disk = match session
        .exec(&format!(
            "df -h {} --output=size,used,avail,pcent 2>/dev/null | tail -1",
            deploy_to
        ))
        .await
    {
        Ok(output) => parse_disk(&output),
        Err(_) => None,
    };

    let containers = merge_ps_stats(ps_entries, stats_entries);

    HostStatus {
        address,
        containers,
        disk,
        error: None,
    }
}

async fn poll_all(sessions: &[SshSession], state: &mut AppState, deploy_to: &str) {
    let mut futures = Vec::new();
    for session in sessions {
        futures.push(poll_host(session, deploy_to));
    }

    let results = futures::future::join_all(futures).await;
    state.hosts = results;
    state.last_update = chrono::Local::now().format("%H:%M:%S").to_string();
}

// --- TUI rendering ---

fn ui(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Outer block
    let title_left = format!(" shipit monitor — {} ({}) ", state.app_name, state.stage_name);
    let title_right = format!(" Updated: {} ", state.last_update);
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::styled(title_left, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]))
        .title(Line::from(vec![
            Span::styled(title_right, Style::default().fg(Color::DarkGray)),
        ]).alignment(ratatui::layout::Alignment::Right));

    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    let num_hosts = state.hosts.len().max(1);

    // Split inner area: host sections + footer
    // Use Fill(1) for each host so they share remaining space equally after the footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            std::iter::repeat(Constraint::Fill(1))
                .take(num_hosts)
                .chain(std::iter::once(Constraint::Length(1)))
                .collect::<Vec<_>>(),
        )
        .split(inner);

    // Render each host
    for (i, host) in state.hosts.iter().enumerate() {
        render_host(frame, chunks[i], host);
    }

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" = quit │ refreshing every "),
        Span::styled(format!("{}s", state.interval), Style::default().fg(Color::Yellow)),
    ]));
    frame.render_widget(footer, chunks[num_hosts]);
}

fn render_host(frame: &mut Frame, area: Rect, host: &HostStatus) {
    let disk_info = host
        .disk
        .as_ref()
        .map(|d| format!("Disk: {}/{} ({})", d.used, d.size, d.use_percent))
        .unwrap_or_else(|| "Disk: N/A".to_string());

    let title = format!(" {} ", host.address);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::styled(title, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]))
        .title(Line::from(vec![
            Span::styled(
                format!(" {} ", disk_info),
                Style::default().fg(Color::DarkGray),
            ),
        ]).alignment(ratatui::layout::Alignment::Right));

    if let Some(err) = &host.error {
        let error_text = Paragraph::new(Line::from(vec![
            Span::styled("ERROR: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(err.as_str(), Style::default().fg(Color::Red)),
        ]))
        .block(block);
        frame.render_widget(error_text, area);
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if host.containers.is_empty() {
        let empty = Paragraph::new(Span::styled(
            "  No containers found",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(empty, inner);
        return;
    }

    // Table header
    let header = Row::new(vec![
        Cell::from("NAME"),
        Cell::from("IMAGE"),
        Cell::from("STATUS"),
        Cell::from("CPU%"),
        Cell::from("MEM"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    // Table rows
    let rows: Vec<Row> = host
        .containers
        .iter()
        .map(|c| {
            let state_color = match c.state.as_str() {
                "running" => Color::Green,
                "exited" => Color::Red,
                "restarting" => Color::Yellow,
                _ => Color::DarkGray,
            };

            let status_display = if c.state == "running" {
                format!("{} ✓", c.status)
            } else {
                c.status.clone()
            };

            let name_display = truncate(&c.name, 24);
            let image_display = truncate(&c.image, 26);

            Row::new(vec![
                Cell::from(name_display),
                Cell::from(image_display),
                Cell::from(status_display).style(Style::default().fg(state_color)),
                Cell::from(c.cpu_perc.clone()),
                Cell::from(c.mem_usage.clone()),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(25),
            Constraint::Length(27),
            Constraint::Length(16),
            Constraint::Length(8),
            Constraint::Min(10),
        ],
    )
    .header(header);

    frame.render_widget(table, inner);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

// --- Entry point ---

pub async fn run(config: ShipitConfig, stage_name: &str, interval: u64) -> Result<()> {
    let stage = config.stage(stage_name)?;
    let user = stage.user.as_deref().unwrap_or("root");
    let port = stage.port;
    let deploy_to = config.app_path();

    // Connect SSH sessions
    let mut sessions = Vec::new();
    for host in &stage.hosts {
        let session = SshSession::connect(user, &host.address, port, stage.proxy.as_deref())
            .await
            .with_context(|| format!("Failed to connect to {}", host.address))?;
        sessions.push(session);
    }

    let mut state = AppState {
        hosts: stage
            .hosts
            .iter()
            .map(|h| HostStatus {
                address: h.address.clone(),
                containers: vec![],
                disk: None,
                error: None,
            })
            .collect(),
        app_name: config.app.name.clone(),
        stage_name: stage_name.to_string(),
        interval,
        last_update: "...".to_string(),
    };

    // Setup terminal with panic hook
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = ratatui::init();

    // Initial poll
    poll_all(&sessions, &mut state, &deploy_to).await;

    // Event loop
    let mut event_stream = EventStream::new();
    let mut poll_interval = tokio::time::interval(Duration::from_secs(interval));
    poll_interval.tick().await; // consume the first immediate tick

    loop {
        terminal.draw(|f| ui(f, &state))?;

        tokio::select! {
            _ = poll_interval.tick() => {
                poll_all(&sessions, &mut state, &deploy_to).await;
            }
            Some(Ok(event)) = event_stream.next() => {
                if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    ratatui::restore();

    // Close SSH sessions
    for session in sessions {
        let _ = session.close().await;
    }

    Ok(())
}
