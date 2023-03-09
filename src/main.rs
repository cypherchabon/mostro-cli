use clap::Parser;
use dotenvy::{dotenv, var};
use nostr_sdk::prelude::*;
use std::env::set_var;
use std::io::{stdin, BufRead};

pub mod cli;
pub mod error;
pub mod lightning;
pub mod pretty_table;
pub mod types;
pub mod util;

use crate::types::Action;
use crate::types::Content;
use crate::types::Message;
use crate::types::Order;
use lightning::is_valid_invoice;
use pretty_table::*;
use util::*;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    // TODO: handle arguments
    let cli = cli::Cli::parse();
    //Init logger
    if cli.verbose {
        set_var("RUST_LOG", "info");
    }

    pretty_env_logger::init();

    // Mostro pubkey
    let pubkey = var("MOSTRO_PUBKEY").expect("$MOSTRO_PUBKEY env var needs to be set");

    // My key
    let my_key = crate::util::get_keys()?;

    // Used to get upper currency string to check against a list of tickers
    let mut upper_currency = None;

    // Call function to connect to relays
    let client = crate::util::connect_nostr().await?;

    match &cli.command {
        Some(cli::Commands::ListOrders {
            order_status,
            currency,
            kind_order,
        }) => {
            let mostro_key = XOnlyPublicKey::from_bech32(pubkey)?;

            // Uppercase currency
            if let Some(curr) = currency {
                upper_currency = Some(curr.to_uppercase());
            }

            println!(
                "Requesting orders from mostro pubId - {}",
                mostro_key.clone()
            );
            println!(
                "You are searching {:?} orders",
                order_status.unwrap().clone()
            );

            //Get orders from relays
            let table_of_orders = get_orders_list(
                mostro_key,
                order_status.to_owned(),
                upper_currency.clone(),
                *kind_order,
                &client,
            )
            .await?;
            let table = print_orders_table(table_of_orders)?;
            println!("{table}");
            std::process::exit(0);
        }
        Some(cli::Commands::TakeSell { order_id, invoice }) => {
            let mostro_key = XOnlyPublicKey::from_bech32(pubkey)?;

            println!(
                "Request of take sell order {} from mostro pubId {}",
                order_id,
                mostro_key.clone()
            );

            // Check invoice string
            let valid_invoice = is_valid_invoice(invoice);

            // Create takesell message
            let takesell_message = Message::new(
                0,
                Some(*order_id),
                Action::TakeSell,
                Some(Content::PaymentRequest(invoice.to_string())),
            )
            .as_json()
            .unwrap();

            match valid_invoice {
                Ok(_) => {
                    send_order_id_cmd(&client, &my_key, mostro_key, takesell_message).await?;
                    std::process::exit(0);
                }
                Err(e) => println!("{}", e),
            }
        }
        Some(cli::Commands::TakeBuy { order_id }) => {
            let mostro_key = XOnlyPublicKey::from_bech32(pubkey)?;

            println!(
                "Request of take buy order {} from mostro pubId {}",
                order_id,
                mostro_key.clone()
            );

            // Create takebuy message
            let takebuy_message = Message::new(0, Some(*order_id), Action::TakeBuy, None)
                .as_json()
                .unwrap();

            send_order_id_cmd(&client, &my_key, mostro_key, takebuy_message).await?;
            std::process::exit(0);
        }
        Some(cli::Commands::GetDm { since }) => {
            let mostro_key = XOnlyPublicKey::from_bech32(pubkey)?;

            let dm = get_direct_messages(&client, mostro_key, &my_key, *since).await;
            let mess = print_message_list(dm).unwrap();
            println!("{mess}");
            std::process::exit(0);
        }
        Some(cli::Commands::FiatSent { order_id }) | Some(cli::Commands::Release { order_id }) => {
            let mostro_key = XOnlyPublicKey::from_bech32(pubkey)?;

            // Get desised action based on command from CLI
            let requested_action = match &cli.command {
                Some(cli::Commands::FiatSent { order_id: _ }) => Action::FiatSent,
                Some(cli::Commands::Release { order_id: _ }) => Action::Release,
                _ => {
                    println!("Not a valid command!");
                    std::process::exit(0);
                }
            };

            println!(
                "Sending {} command for order {} to mostro pubId {}",
                requested_action,
                order_id,
                mostro_key.clone()
            );

            // Create fiat sent message
            let message = Message::new(0, Some(*order_id), requested_action, None)
                .as_json()
                .unwrap();

            send_order_id_cmd(&client, &my_key, mostro_key, message).await?;
            std::process::exit(0);
        }
        Some(cli::Commands::Neworder {
            kind,
            fiat_code,
            amount,
            fiat_amount,
            payment_method,
            prime,
            invoice,
        }) => {
            let mostro_key = XOnlyPublicKey::from_bech32(pubkey)?;

            // let mut yadio_API_exchange = String::from("https://api.yadio.io/exrates/");
            // yadio_API_exchange.push_str(&fiat_code);

            // //Request quotation in selected currency
            // let yadio_ans = reqwest::blocking::get(yadio_API_exchange).unwrap().json()?;

            // println!("{:?}",yadio_ans);

            let order_content = Content::Order(Order::new(
                None,
                kind.unwrap(),
                types::Status::Pending,
                *amount,
                fiat_code.to_owned(),
                *fiat_amount,
                payment_method.to_owned(),
                *prime,
                invoice.as_ref().to_owned().cloned(),
                None,
            ));

            // Print order preview
            let ord_preview = print_order_preview(order_content.clone()).unwrap();
            println!("{ord_preview}");
            let mut user_input = String::new();
            let _input = stdin();
            println!("Check your order! Is it correct? (Yes/No)");

            let mut answer = stdin().lock();
            answer.read_line(&mut user_input)?;

            match user_input.as_str().trim_end() {
                "Yes" => {},
                "No"  => {
                    println!("Try again!");
                    std::process::exit(0);
                },
                &_  =>{
                    println!("Can't get what you're sayin!");
                    std::process::exit(0);
                }
            };

            // Create fiat sent message
            let message = Message::new(0, None, Action::Order, Some(order_content))
                .as_json()
                .unwrap();

            send_order_id_cmd(&client, &my_key, mostro_key, message).await?;
            std::process::exit(0);
        }
        None => {}
    };

    println!("Bye Bye!");
    Ok(())
}
