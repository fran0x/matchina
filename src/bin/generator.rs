use std::io;
use std::io::Write;
use std::fs::File;
use std::io::{BufRead, BufReader};
use anyhow::Result;
use merx::order::OrderRequest;

fn main() -> Result<()> {
    let mut stdout = io::stdout();
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
