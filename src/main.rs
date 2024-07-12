pub mod client;
pub mod storage;
pub mod types;
use axum::{
    extract::DefaultBodyLimit,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use client::{JsonRpcClient, RPC_ENDPOINT};
use colored::*;
use indicatif::ProgressBar;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use storage::MemoryDB;
use tokio::sync::RwLock;
use types::{Block, SolanaEpoch};

async fn gator_loop(database: Arc<Mutex<MemoryDB>>) {
    let client: JsonRpcClient =
        JsonRpcClient::new(RPC_ENDPOINT).expect("[Error] Failed to init RPC Client");
    let current_epoch: SolanaEpoch = client
        .get_current_epoch()
        .await
        .expect("[Error] Failed to get current Epoch");
    let epoch_blocks: Vec<u32> = client
        .get_current_era_blocks(current_epoch)
        .await
        .expect("[Error] Failed to get Blocks for ongoing Epoch");
    let progress_bar: ProgressBar = ProgressBar::new(epoch_blocks.len() as u64);
    println!("Epoch Blocks: {:?}", &epoch_blocks.len());
    for block_slot in epoch_blocks {
        // No fault tolerance for now
        let block: Option<Block> = match client.get_block_by_id(block_slot).await {
            Ok(block) => Some(block),
            Err(err) => {
                println!("Malformed or non-standard Block {}", &err.to_string().red());
                None
            }
        };
        match block {
            Some(block) => {
                let block_height = block.block_height.as_u64().unwrap();
                for transaction in block.transactions.clone() {
                    database.lock().unwrap().insert_transaction(
                        transaction.transaction.signatures[0].clone(),
                        transaction,
                        block_height,
                    );
                }
                database.lock().unwrap().insert_block(block_height, block);
            }
            None => {}
        }
        progress_bar.inc(1);
        //sleep(Duration::from_millis(10));
    }
    progress_bar.finish_with_message("Done fetching Blocks for Epoch!");
}
#[tokio::main]
async fn main() {
    println!(
        "{}",
        r#"
           .-._   _ _ _ _ _ _ _ _
.-''-.__.-'00  '-' ' ' ' ' ' ' ' '-.
'.___ '    .   .--_'-' '-' '-' _'-' '._
 V: V 'vv-'   '_   '.       .'  _..' '.'.
   '=.____.=_.--'   :_.__.__:_   '.   : :
           (((____.-'        '-.  /   : :
                             (((-'\ .' /
                           _____..'  .'
                          '-._____.-'"#
            .green()
            .bold()
    );
    println!(
        "{}{} {}!",
        "Data".green(),
        "Gator".yellow(),
        "says Hello".blue().italic()
    );
    let mut shared_memory_db: MemoryDB = MemoryDB {
        blocks: HashMap::new(),
        transactions: HashMap::new(),
        block_idx: 0,
    };
    let shared_state = Arc::new(Mutex::new(shared_memory_db));
    tokio::spawn(gator_loop(Arc::clone(&shared_state)));

    let app = Router::new()
        .route("/ping", get({ move || ping() }))
        //.route("/", post())
        //.route("/", post())
        .layer(DefaultBodyLimit::max(10000000))
        .layer(Extension(shared_state));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
    /*
        Since I am implementing this with a MemoryDB, there is no fallback options for cases where
        the service crashes. I wrote the code with SQL databases in mind though, therefore it is quite
        easy to setup a schema and replace MemoryDB. This is a common practice in Prototyping and POC architecture that
        I am personally a big fan of.

        If I had 1 week + to work on this project (and were paid for it), then I would consider setting up a Database schema and move away from
        in-memory storage. I think that for the scope of this project this is a fair assumption.
    */
}

// `&'static str` becomes a `200 OK` with `content-type: text/plain; charset=utf-8`
async fn ping() -> &'static str {
    "pong"
}
