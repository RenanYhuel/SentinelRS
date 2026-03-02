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

pub fn fuzzy_select(prompt: &str, items: &[String]) -> Option<usize> {
    let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&refs)
        .default(0)
        .interact_opt()
        .ok()
        .flatten()
}
