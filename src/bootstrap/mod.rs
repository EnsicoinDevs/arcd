use ensicoin_serializer::{Serialize, Sha256Result};

use std::fs;
mod irc;
pub use self::irc::irc_listener;

pub fn clean(data_dir: std::path::PathBuf) -> Result<(), String> {
    let mut settings = data_dir.clone();
    settings.push("settings.json");

    let mut blockchain_dir = std::path::PathBuf::new();
    blockchain_dir.push(data_dir.clone());
    blockchain_dir.push("blockchain");

    let mut utxo_dir = std::path::PathBuf::new();
    utxo_dir.push(data_dir.clone());
    utxo_dir.push("utxo");

    let mut rev_dir = std::path::PathBuf::new();
    rev_dir.push(data_dir.clone());
    rev_dir.push("reverse_chain");

    let mut spent_tx_dir = std::path::PathBuf::new();
    spent_tx_dir.push(data_dir.clone());
    spent_tx_dir.push("spent_tx");

    let mut stats_dir = std::path::PathBuf::new();
    stats_dir.push(data_dir.clone());
    stats_dir.push("stats");

    let mut work_dir = std::path::PathBuf::new();
    work_dir.push(data_dir);
    work_dir.push("work");

    match std::fs::remove_dir_all(utxo_dir)
        .and(std::fs::remove_dir_all(rev_dir))
        .and(std::fs::remove_dir_all(spent_tx_dir))
        .and(std::fs::remove_dir_all(stats_dir))
        .and(std::fs::remove_dir_all(blockchain_dir.clone()))
        .and(std::fs::remove_file(settings.clone()))
        .and(std::fs::remove_dir_all(work_dir))
    {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Can't clean data_dir: {}", e)),
    }
}

pub fn bootstrap(data_dir: &std::path::PathBuf) -> Result<(), String> {
    let mut settings = std::path::PathBuf::new();
    settings.push(data_dir);
    settings.push("settings.json");

    let mut blockchain_dir = std::path::PathBuf::new();
    blockchain_dir.push(data_dir);
    blockchain_dir.push("blockchain");

    let mut stats_dir = std::path::PathBuf::new();
    stats_dir.push(data_dir);
    stats_dir.push("stats");

    let mut work_dir = std::path::PathBuf::new();
    work_dir.push(data_dir);
    work_dir.push("work");
    let work = sled::Db::start_default(work_dir).unwrap();

    let settings = match fs::File::create(settings) {
        Ok(f) => f,
        Err(e) => {
            return Err(format!("Can't bootstrap at that location: {}", e));
        }
    };

    let defaults: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    serde_json::to_writer(settings, &defaults).unwrap();

    let genesis = ensicoin_messages::resource::Block {
        header: ensicoin_messages::resource::BlockHeader {
            version: 0,
            flags: vec!["ici cest limag".to_string()],
            prev_block: Sha256Result::from([0; 32]),
            merkle_root: Sha256Result::from([0; 32]),
            timestamp: 1558540052,
            nonce: 42,
            height: 0,
            target: Sha256Result::from([
                0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0,
            ]),
        },
        txs: Vec::new(),
    };
    let genesis_hash = genesis
        .double_hash()
        .iter()
        .map(|b| format!("{:x}", b))
        .fold(String::new(), |mut acc, mut v| {
            acc.push_str(&mut v);
            acc
        });
    println!("Welcome to ensicoin ! Setting up the DB and storing settings");
    println!("Genesis hash: {}", &genesis_hash);
    let blockchain_db = match sled::Db::start_default(blockchain_dir) {
        Ok(db) => db,
        Err(e) => {
            return Err(format!("Can't open blockchain database: {}", e));
        }
    };
    let stats_db = match sled::Db::start_default(stats_dir) {
        Ok(db) => db,
        Err(e) => {
            return Err(format!("Can't open blockchain database: {}", e));
        }
    };
    if let Err(e) = blockchain_db.set(
        genesis.double_hash().serialize().to_vec(),
        genesis.serialize().to_vec(),
    ) {
        return Err(format!(
            "Could not insert genesis block in blockchain: {}",
            e
        ));
    };

    if let Err(e) = stats_db.set("genesis_block", genesis.double_hash().serialize().to_vec()) {
        return Err(format!("Could not insert genesis hash in stats: {}", e));
    };

    if let Err(e) = work.set(
        genesis.double_hash().serialize().to_vec(),
        vec![0 as u8].serialize().to_vec(),
    ) {
        return Err(format!("Could not insert base work: {}", e));
    }

    if let Err(e) = stats_db.set("best_block", genesis.double_hash().serialize().to_vec()) {
        return Err(format!(
            "Could not insert genesis hash in best_block stats: {}",
            e
        ));
    };

    if let Err(e) = stats_db.set("10_last", vec![genesis.double_hash()].serialize().to_vec()) {
        return Err(format!(
            "Could not insert genesis hash in best_block stats: {}",
            e
        ));
    };

    Ok(())
}
