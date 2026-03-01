use dialoguer::{theme::ColorfulTheme, Confirm};

pub fn confirm_action(msg: &str) -> bool {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(msg)
        .default(false)
        .interact()
        .unwrap_or(false)
}

pub fn confirm_default_yes(msg: &str) -> bool {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(msg)
        .default(true)
        .interact()
        .unwrap_or(true)
}
