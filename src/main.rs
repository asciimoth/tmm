use teloxide::prelude::*;
use teloxide::types::{MessageKind, MediaKind};
use dotenv::dotenv;
use std::env;
use std::process::exit;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use pickledb::error::Result as PResult;
use pickledb::error::ErrorType as PErrorType;

// Load or create DB by path
fn get_db(path: &str) -> PResult<PickleDb> {
    match PickleDb::load(
        path, 
        PickleDbDumpPolicy::DumpUponRequest,
        SerializationMethod::Cbor
    ) {
        Ok(db) => Ok(db),
        Err(err) => {
            match err.get_type() {
                PErrorType::Io => {
                    let mut db = PickleDb::new(
                        path,
                        PickleDbDumpPolicy::DumpUponRequest,
                        SerializationMethod::Cbor,
                    );
                    match db.dump() {
                        Ok(_) => Ok(db),
                        Err(err) => Err(err),
                    }
                }
                _ => Err(err)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();
    let db_file = match env::var("DB") {
        Ok(file) => file,
        Err(_) => {
            log::error!("There is no 'DB' env var");
            exit(1);
        }
    };
    log::info!("Env readed");
    let mut db = match get_db(&db_file) {
        Ok(db) => db,
        Err(err) => {
            log::error!("Cannot load db: {}", err);
            exit(1);
        }
    };
    log::info!("Db loaded");
    let bot = Bot::from_env().auto_send();
    log::info!("Starting tg bot");
    teloxide::repl(bot, move |msg: Message, bot: AutoSend<Bot>| async move {
        let chat_id = msg.chat.id;
        log::info!("{:#?}", msg);
        //log::info!("{:#?}", bot.get_chat(chat_id).await?);
        respond(())
    }).await;
    log::info!("Dumping db");
    match db.dump() {
        Ok(_) => log::info!("Db dumped successfully"),
        Err(err) => log::error!("Cannot dump db: {}", err),
    }
}
