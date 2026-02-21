use crate::models::{ConvertItem, GetItem};

pub fn print_get(item: &GetItem, quiet: bool) {
    if quiet {
        // Minimal output: base rate and date
        println!("{} {}", item.base.to_uppercase(), item.date);
        return;
    }

    println!("Base: {}", item.base.to_uppercase());
    println!("Date: {}", item.date);

    let mut entries: Vec<_> = item.rates.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    for (code, rate) in entries {
        println!("{}: {}", code.to_uppercase(), rate);
    }
}

pub fn print_convert(item: &ConvertItem, quiet: bool) {
    if quiet {
        // Minimal output: result amount and target currency
        println!("{} {}", item.result, item.to.to_uppercase());
        return;
    }

    println!(
        "{} {} = {} {} (rate: {}, date: {})",
        item.amount,
        item.from.to_uppercase(),
        item.result,
        item.to.to_uppercase(),
        item.rate,
        item.date
    );
}

pub fn print_list(items: &[String], quiet: bool) {
    // In quiet mode, print one currency code per line (same as normal mode)
    for item in items {
        println!("{}", item.to_uppercase());
    }
    if !quiet && items.is_empty() {
        println!("(no currencies found)");
    }
}
