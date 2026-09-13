mod dist;
mod parser;

use std::env;
use std::process;

enum Query {
    AtLeast(i64),
    AtMost(i64),
    Exact(i64),
}

struct Options {
    expression: Option<String>,
    lenient: bool,
    query: Option<Query>,
}

fn print_usage() {
    println!("diceodds - exact probability distributions for dice notation\n");
    println!("usage:");
    println!("  diceodds <expression> [options]\n");
    println!("options:");
    println!("  --lenient          allow whitespace, uppercase D, leading zeros, omitted counts");
    println!("  --at-least <n>     print P(total >= n) instead of the full table");
    println!("  --at-most <n>      print P(total <= n) instead of the full table");
    println!("  --exact <n>        print P(total == n) instead of the full table");
    println!("  --help             print this message\n");
    println!("examples:");
    println!("  diceodds \"2d6+3\"");
    println!("  diceodds \"1d20\" --at-least 15");
    println!("  diceodds \"2D6 + 3\" --lenient");
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut expression = None;
    let mut lenient = false;
    let mut query = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--lenient" => lenient = true,
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            "--at-least" => {
                i += 1;
                let n = args.get(i).ok_or("--at-least needs a number")?;
                query = Some(Query::AtLeast(
                    n.parse().map_err(|_| format!("'{n}' is not a valid number"))?,
                ));
            }
            "--at-most" => {
                i += 1;
                let n = args.get(i).ok_or("--at-most needs a number")?;
                query = Some(Query::AtMost(
                    n.parse().map_err(|_| format!("'{n}' is not a valid number"))?,
                ));
            }
            "--exact" => {
                i += 1;
                let n = args.get(i).ok_or("--exact needs a number")?;
                query = Some(Query::Exact(
                    n.parse().map_err(|_| format!("'{n}' is not a valid number"))?,
                ));
            }
            other if !other.starts_with('-') && expression.is_none() => {
                expression = Some(other.to_string());
            }
            other => return Err(format!("unrecognized argument '{other}'")),
        }
        i += 1;
    }

    Ok(Options {
        expression,
        lenient,
        query,
    })
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let options = match parse_args(&args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(2);
        }
    };

    let expression = match options.expression {
        Some(e) => e,
        None => {
            print_usage();
            process::exit(2);
        }
    };

    let expr = match parser::parse(&expression, options.lenient) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    let distribution = dist::evaluate(&expr);

    match options.query {
        Some(Query::AtLeast(n)) => {
            println!("P(total >= {n}) = {:.4}%", distribution.prob_at_least(n) * 100.0);
        }
        Some(Query::AtMost(n)) => {
            println!("P(total <= {n}) = {:.4}%", distribution.prob_at_most(n) * 100.0);
        }
        Some(Query::Exact(n)) => {
            println!("P(total == {n}) = {:.4}%", distribution.prob_exact(n) * 100.0);
        }
        None => {
            println!("{:>8}  {:>10}", "value", "probability");
            for (i, p) in distribution.probs.iter().enumerate() {
                let value = distribution.min + i as i64;
                println!("{:>8}  {:>9.4}%", value, p * 100.0);
            }
            println!();
            println!(
                "min: {}  max: {}  mean: {:.2}",
                distribution.min,
                distribution.max(),
                distribution.mean()
            );
        }
    }
}
