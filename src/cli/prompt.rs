//! Interactive prompts read from the terminal.

use std::io::IsTerminal;

use anyhow::{Result, bail};
use dialoguer::Select;

/// Asks the user to pick one item, returning its index, with `default` preselected.
pub(crate) fn pick(prompt: &str, items: &[String], default: usize) -> Result<usize> {
    // dialoguer only checks the output terminal, so piped input would block forever.
    if !std::io::stdin().is_terminal() {
        bail!("not an interactive terminal, pass the choice as an option instead");
    }
    let selection = Select::new()
        .with_prompt(prompt)
        .items(items)
        .default(default)
        .interact_opt()?;
    match selection {
        Some(index) => Ok(index),
        None => bail!("no choice was made"),
    }
}
