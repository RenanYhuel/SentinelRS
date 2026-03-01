use dialoguer::{theme::ColorfulTheme, FuzzySelect, Select};

pub fn select_option(prompt: &str, items: &[&str]) -> Option<usize> {
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact_opt()
        .ok()
        .flatten()
}

#[allow(dead_code)]
pub fn fuzzy_select(prompt: &str, items: &[&str]) -> Option<usize> {
    FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact_opt()
        .ok()
        .flatten()
}
