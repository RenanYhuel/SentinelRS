use super::colors;

pub enum Component {
    Server,
    Agent,
    Worker,
    Cli,
}

impl Component {
    fn label(&self) -> &'static str {
        match self {
            Self::Server => "Server",
            Self::Agent => "Agent",
            Self::Worker => "Worker",
            Self::Cli => "CLI",
        }
    }
}

pub fn print_banner(component: Component, version: &str) {
    let label = component.label();

    let c = colors::BRIGHT_CYAN;
    let b = colors::BOLD;
    let d = colors::DIM;
    let w = colors::BRIGHT_WHITE;
    let r = colors::RESET;

    eprintln!();
    eprintln!("    {c}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{r}");
    eprintln!("     {w}{b}◆  S E N T I N E L R S{r}");
    eprintln!("     {d}{label}  v{version}{r}");
    eprintln!("    {c}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{r}");
    eprintln!();
}
