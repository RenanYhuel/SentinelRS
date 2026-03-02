use anyhow::Result;
use dialoguer::theme::ColorfulTheme;

pub fn text(prompt: &str, default: &str) -> Result<String> {
    let val = dialoguer::Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default.into())
        .interact_text()?;
    Ok(val)
}

pub fn text_required(prompt: &str) -> Result<String> {
    let val = dialoguer::Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact_text()?;
    if val.trim().is_empty() {
        anyhow::bail!("input cannot be empty");
    }
    Ok(val)
}

pub fn text_optional(prompt: &str) -> Result<Option<String>> {
    let val = dialoguer::Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .allow_empty(true)
        .interact_text()?;
    if val.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(val))
    }
}

pub fn number(prompt: &str, default: u64) -> Result<u64> {
    let val = dialoguer::Input::<u64>::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default)
        .interact_text()?;
    Ok(val)
}

pub fn password(prompt: &str) -> Result<String> {
    let val = dialoguer::Password::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact()?;
    Ok(val)
}
