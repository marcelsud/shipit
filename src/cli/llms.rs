use anyhow::Result;

use crate::cli::LlmsAction;
use crate::llms;

pub fn run(action: &LlmsAction) -> Result<()> {
    match action {
        LlmsAction::Index { json } => {
            if *json {
                println!("{}", llms::index_json()?);
            } else {
                println!("{}", llms::index());
            }
        }
        LlmsAction::Get { topic, json } => {
            if *json {
                println!("{}", llms::get_json(topic)?);
            } else {
                let t = llms::get(topic)?;
                print!("{}", t.content.unwrap_or_default());
            }
        }
        LlmsAction::Full { json } => {
            if *json {
                println!("{}", llms::full_json()?);
            } else {
                print!("{}", llms::full());
            }
        }
    }
    Ok(())
}
