use mostro_core::Content;

use anyhow::Result;

use mostro_core::Message as MostroMessage;

use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{Client, Keys};

use crate::util::get_direct_messages;

pub async fn execute_get_dm(
    since: &i64,
    my_key: &Keys,
    mostro_key: XOnlyPublicKey,
    client: &Client,
) -> Result<()> {
    let dm = get_direct_messages(client, mostro_key, my_key, *since).await;
    if dm.is_empty() {
        println!();
        println!("No new messages from Mostro");
        println!();
    } else {
        for el in dm.iter() {
            match MostroMessage::from_json(&el.0) {
                Ok(m) => {
                    println!(
                        "Mostro sent you this message for order id: {}",
                        m.order_id.unwrap()
                    );
                    if let Some(Content::PaymentRequest(_, inv)) = m.content {
                        println!();
                        println!("Pay this invoice to continue --> {}", inv);
                        println!();
                    } else if let Some(Content::TextMessage(text)) = m.content {
                        println!();
                        println!("{text}");
                        println!();
                    } else {
                        println!();
                        println!("Action: {}", m.action);
                        println!("Content: {:#?}", m.content);
                        println!();
                    }
                }
                Err(_) => {
                    println!("Mostro sent you this message:");
                    println!();
                    println!("{}", el.0);
                    println!();
                }
            }
        }
    }
    Ok(())
}
