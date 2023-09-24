use std::io;
use std::io::Write;

use anyhow::Result;
use exchange::order::util::generate;

fn main() -> Result<()> {
    let mut stdout = io::stdout();
    let range = 1..=10_000_000;
    for order in generate(range) {
        let order = serde_json::to_string(&order).ok().unwrap();
        writeln!(stdout, "{}", order)?;
    }

    Ok(())
}
