use teloxide::prelude::*;
use teloxide::types::{MessageKind, MediaKind, Update};
use dotenv::dotenv;
use std::env;
use std::process::exit;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use pickledb::error::Result as PResult;
use pickledb::error::ErrorType as PErrorType;
use std::sync::Arc;
use tokio::sync::Mutex;

// Load or create DB by path
fn get_db(path: &str) -> PResult<PickleDb> {
    match PickleDb::load(
        path, 
        PickleDbDumpPolicy::DumpUponRequest,
        SerializationMethod::Json
    ) {
        Ok(db) => Ok(db),
        Err(err) => {
            match err.get_type() {
                PErrorType::Io => {
                    let mut db = PickleDb::new(
                        path,
                        PickleDbDumpPolicy::DumpUponRequest,
                        SerializationMethod::Json,
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

fn my_db_unwrap<T>(msg: &str, r: PResult<T>) -> T {
    match r {
        Ok(v) => v,
        Err(err) => {
            log::error!("{};{}", msg, err);
            exit(1);
        }
    }
}

fn unbind(db: & mut PickleDb, s: i64) -> PResult<()> {
    if let Some(m) = db.get::<i64>(&format!("s{}", s)) {
        let mkey = format!("m{}", m);
        db.rem(&format!("s{}", s))?;
        let slaves = match db.get::<Vec<i64>>(&mkey) {
            Some(v) => {
                let mut ns = Vec::<i64>::new();
                for e in v {
                    if e != s {
                        ns.push(e);
                    }
                }
                if ns.len() > 0 {
                    db.set(&mkey, &ns)?;
                }else{
                    db.rem(&mkey)?;
                }
            }
            None => {},
        };
    };
    db.dump()
}

fn bind(db: & mut PickleDb, m: i64, s: i64) -> PResult<()>{
    unbind(db, s)?;
    let mkey = format!("m{}", m);
    let slaves = match db.get::<Vec<i64>>(&mkey) {
        Some(mut v) => {
            let mut p = true;
            for e in v.iter() {
                if *e == s {
                    p = false;
                    break;
                }
            }
            if p { v.push(s); }
            v
        }
        None => vec![s],
    };
    db.set(&format!("s{}", s), &m)?;
    db.set(&format!("m{}", m), &slaves)?;
    db.dump()
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
    let db:Arc<Mutex<PickleDb>> = Arc::new(Mutex::new(db));
    log::info!("Db loaded");
    let bot = Bot::from_env().auto_send();
    log::info!("Starting tg bot");
    let handler = Update::filter_message().endpoint(
        |message: Message, bot: AutoSend<Bot>, db: Arc<Mutex<PickleDb>>| async move {
        let chat_id = message.chat.id;
        let msg_id = message.id;
        if let Some(text) = message.text() {
            if text == "/getchatid" {
                bot.send_message(chat_id, format!("Chat id is {}", chat_id)).await?;
            } else if text == "/unbindfromall" {
                let mut db = db.lock().await;
                my_db_unwrap("Db error", unbind(& mut db, chat_id.0));
                bot.send_message(
                    chat_id,
                    "Unbinded dom all chats",
                ).await?;
            } else if text.starts_with("/bindtochat ") {
                let str_master_id = text.strip_prefix("/bindtochat ").unwrap();
                match str_master_id.parse::<i64>() {
                    Ok(master_id) => {
                        if master_id == chat_id.0 {
                            bot.send_message(
                                chat_id,
                                "Cannot bind chat to istself",
                            ).await?;
                        }else{
                            let mut db = db.lock().await;
                            my_db_unwrap("Db error", bind(& mut db, master_id, chat_id.0));
                            bot.send_message(
                                chat_id,
                                format!("Binded to {}", master_id),
                            ).await?;
                        }
                    }
                    Err(_) => {
                        bot.send_message(
                            chat_id,
                            format!(
                                "Cannot bid to '{}'; Chat id is invalid",
                                str_master_id,
                            ),
                        ).await?;
                    }
                }
            } else {
                log::debug!("Text: {}", text);
            }
        }
            respond(())
        },
    );
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db.clone()])
        .build()
        .dispatch()
        .await;
    let mut db = db.lock().await;
    log::info!("Dumping db");
    match db.dump() {
        Ok(_) => log::info!("Db dumped successfully"),
        Err(err) => log::error!("Cannot dump db: {}", err),
    }
}
