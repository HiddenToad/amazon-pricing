use docopt::Docopt;
use regex::Regex;
use reqwest;
use serde::Deserialize;
use std::process::exit;
use tokio::main;

const USAGE: &'static str = r"
Amazon Pricing Webscraper

Usage: ./amazon-pricing <query> [options]

Options: 
    -p <page>   Specify number of pages to look through
    --avg       Print average pricing for search term
    --raw       Print raw pricing data, default
";

#[derive(Debug, Deserialize, Default)]
struct Args {
    arg_query: String,
    flag_p: Option<u8>,
    flag_avg: bool,
    flag_raw: bool,
}

async fn amazon_search(string: &str, page: u8) -> reqwest::Result<String> {
    let body = reqwest::get(format!(
        "https://www.amazon.com/s?k={string}&page={page}&s=review-rank",
    ))
    .await?
    .text()
    .await?;
    Ok(format!("body: {:?}", body))
}

async fn amazon_search_up_to_nth_page(string: &str, page: u8) -> String {
    let mut res: String = String::new();
    for i in 1..=page {
        res += amazon_search(string, i).await.unwrap().as_str();
    }
    res
}

async fn amazon_get_pricing_raw(input: &str, page: u8) -> Vec<f32> {
    let result = amazon_search_up_to_nth_page(input, page).await;
    let whole_prices = Regex::new("a-price-whole\\\\+\">(\\d+(,\\d+)?)").unwrap();
    let frac_prices = Regex::new("a-price-fraction\\\\+\">(\\d+)").unwrap();
    let mut final_res: Vec<f32> = vec![];

    let whole_result = whole_prices.captures_iter(result.as_str());
    let frac_result = frac_prices.captures_iter(result.as_str());

    let mut it = whole_result.zip(frac_result);

    while let Some((wholeit, fracit)) = it.next() {
        let whole = wholeit.get(1).unwrap().as_str().replacen(",", "", 100);
        let frac = fracit.get(1).unwrap().as_str();

        final_res.push(format!("{}.{}", whole, frac).as_str().parse().unwrap());
    }
    final_res.sort_by(|a, b| a.partial_cmp(b).unwrap());
    final_res.dedup();
    final_res = final_res[((final_res.len() as f32 / 8.) as usize)..final_res.len() - 1].into();
    final_res
}

fn calc_avg_pricing(input: Vec<f32>) -> f32 {
    (((input.iter().sum::<f32>() / input.len() as f32) * 100.).round()) / 100.
}


#[main]
async fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|err| {
            println!("error: {}", err.to_string());
            exit(1);
        });

    let search_term = args.arg_query.as_str();

    let pages = match args.flag_p {
        Some(n) => n,
        None => 1,
    };

    let raw = amazon_get_pricing_raw(search_term, pages).await;

    if args.flag_avg {
        let avg = calc_avg_pricing(raw.clone());
        if args.flag_raw {
            println!("{:#?}", raw);
        }
        println!("{:#?}", avg);
    } else {
        println!("{:#?}", raw);
    }
}
