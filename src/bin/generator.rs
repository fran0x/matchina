use std::io;
use std::io::Write;
use std::fs::File;
use std::path::Path;
use std::io::{BufRead, BufReader};
use anyhow::Result;
use merx::order::OrderRequest;
use merx::order::util::generate;

fn main() -> Result<()> {
    let mut stdout = io::stdout();

    let input_exists = Path::new("./orders.json").try_exists();

    match input_exists {
        Ok(true) => {
            // Do nothing, continue with the code
        }
        Ok(false) => {
            let range = 1..=10_000_000;
            let mut file = File::create("./orders.json")?;

            for order in generate(range) {
                let order = serde_json::to_string(&order).ok().unwrap();
                writeln!(file, "{}", order)?;
            }
        }
        Err(_) => {
            // Error handling
        }
    }

    let input = File::open("./orders.json")?;
    let reader = BufReader::new(input);

    let mut orders: Vec<OrderRequest> = Vec::new();

    for line in reader.lines() {
        let line = line?;

        let order: OrderRequest = serde_json::from_str(&line)?;

        orders.push(order);
    }
    
    for order in &orders {
        let order_json = serde_json::to_string(&order)?;

        writeln!(stdout, "{}", order_json)?;
    }

    Ok(())
}
