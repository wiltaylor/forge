mod forge;
mod screens;

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let res = screens::dashboard::run(&mut terminal);
    ratatui::restore();
    res
}
