use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::io::stdout;
use crate::data::generate_mock_transactions;
use crate::core::NetworkStats;

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let txs = generate_mock_transactions(100);
    let stats = NetworkStats::from_transactions(&txs, 2_800_000);
    loop {
        terminal.draw(|f| render(f, &stats))?;
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') { break; }
        }
    }
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn render(f: &mut Frame, stats: &NetworkStats) {
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(3),
    ]).split(f.area());
    f.render_widget(
        Paragraph::new("ZecLedger Research Dashboard  [Q] quit")
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center),
        layout[0],
    );
    let cols = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ]).split(layout[1]);
    f.render_widget(Paragraph::new(vec![
        Line::from(format!("Total transactions : {}", stats.total_transactions)),
        Line::from(format!("Shielded           : {} ({:.1}%)", stats.shielded_count, stats.shield_rate_pct)),
        Line::from(format!("Transparent        : {}", stats.transparent_count)),
        Line::from(format!("Block height       : {}", stats.block_height)),
    ]).block(Block::default().title("Network Stats").borders(Borders::ALL)), cols[0]);
    f.render_widget(Paragraph::new(vec![
        Line::from(format!("Volume (ZEC)  : {:.4}", stats.total_volume_zec)),
        Line::from(format!("Volume (USD)  : ${:.2}", stats.total_volume_usd)),
        Line::from(format!("Avg tx size   : {:.4} ZEC", stats.avg_tx_size_zec)),
        Line::from(format!("Shield rate   : {:.1}%", stats.shield_rate_pct)),
    ]).block(Block::default().title("Volume").borders(Borders::ALL)), cols[1]);
    f.render_widget(
        Paragraph::new("Ask: zecledger ask \"What is the shield rate trend?\"")
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center),
        layout[2],
    );
}
