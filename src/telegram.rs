use std::sync::{Arc, Mutex};
use anyhow::{anyhow, Context};
use crossbeam_channel::Sender;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use log::{debug, info, warn, error};
use crate::format_error;

const POLL_TIMEOUT: u32 = 120;
const CONTEXT_PATH: &'static str = "telegram.json";

pub struct Telegram {
    api_url: String,
    context: Arc<Mutex<TelegramContext>>,
    context_path: PathBuf,
    sender: Sender<String>
}

impl Telegram {

    pub fn new(sender: &Sender<String>) -> Self {

        let token = std::env::var("GOTOBED_TELEGRAM_TOKEN")
            .expect("Environment variable GOTOBED_TELEGRAM_TOKEN not set");
        let api_url = format!("https://api.telegram.org/bot{}", token);

        let context_path = PathBuf::from(CONTEXT_PATH);
        debug!("Telegram context path: {}", context_path.to_string_lossy());

        let context = TelegramContext::restore(&context_path)
            .unwrap_or_else(|err| {
                warn!("No Telegram context restored: {}", err);
                info!("Creating new context");
                TelegramContext::new()
            });
        let context = Arc::new(Mutex::new(context));
        
        let telegram = Telegram { 
            api_url,
            context,
            context_path,
            sender: sender.clone()
        };

        telegram
    }

    pub fn send(&mut self, text: &str) {

        let context = self.context.lock().unwrap();

        || -> anyhow::Result<()>{

            let chat_id = context.chat_id
                .ok_or(anyhow!("no known ChatID stored"))?;
    
            let url = format!("{}/sendMessage", self.api_url);
            let json = ureq::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "HTML"
            });

            ureq::post(&url)
                .send_json(json)
                .context("call to Telegram API failed")?;

            Ok(())
        }()
        .unwrap_or_else(|err| error!(
            "Could not send Telegram message: {}",
            format_error(err)));
    }

    pub fn get_loop(&self) -> impl FnOnce() {

        let api_url = self.api_url.clone();
        let context = self.context.clone();
        let sender = self.sender.clone();
        let context_path = self.context_path.clone();

        move || {

            let mut offset = 0u32;
    
            let mut relay_updates = move || -> anyhow::Result<()> {
    
                let poll_url = format!(
                    "{}/getUpdates?offset={}&timeout={}&allowed_updates=[\"message\"]",
                    api_url, offset, POLL_TIMEOUT
                );
            
                let api_res: ReturnedUpdates = ureq::get(&poll_url)
                    .call()?
                    .into_json()?;
    
                let mut updates = api_res.result;
                updates.sort_by_key(|update| update.update_id);
    
                if let Some(latest_update) = updates.last() {
    
                    offset = latest_update.update_id + 1;
                    let chat_id = latest_update.message.chat.id;
    
                    {
                        let mut context = context.lock().unwrap();
                        let update = context.update_chat_id(chat_id);
                        if update { context.save(&context_path) };
                    }
                }
    
                updates.iter()
                    .for_each(|update| {
                        debug!("Telegram update: {:?}", update);
                        let text = update.message.text.clone();
                        info!("Received Telegram message: {}", text);
                        sender.send(text).unwrap()
                    });
    
                Ok(())
            };
    
            info!("Starting Telegram polling loop");
            loop {
                relay_updates()
                    .unwrap_or_else(|err| error!(
                        "Telegram: error retrieving updates: {}",
                        format_error(err)
                    ))
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ReturnedUpdates {
    _ok: bool,
    result: Vec<Update>
}

#[derive(Debug, Clone, Deserialize)]
struct Update {
    update_id: u32,
    message: Message
}

#[derive(Debug, Clone, Deserialize)]
struct Message {
    _message_id: u32,
    text: String,
    chat: Chat
}
#[derive(Debug, Clone, Deserialize)]
struct Chat {
    id: u32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TelegramContext {
    chat_id: Option<u32>
}

impl TelegramContext {

    fn restore(context_path: &Path) -> anyhow::Result<TelegramContext> {

        let path_str = context_path.to_string_lossy();

        info!(
            "Attempting to restore Telegram context from {}",
            path_str
        );
        let data = std::fs::read_to_string(&context_path)?;
        let context = serde_json::from_str(&data)
            .map_err(anyhow::Error::new)
            .expect(&format!("Error parsing Telegram context from {}", path_str));
        Ok(context)
    }

    fn new() -> Self {
        TelegramContext { chat_id: None }
    }

    fn save(&self, context_path: &Path) {

        let path_str = context_path.to_string_lossy();

        || -> anyhow::Result<()> {
            info!("Saving Telegram context to: {}", path_str);
            let data = serde_json::to_string_pretty(self)?;
            std::fs::write(&context_path, data)?;
            Ok(())
        }()
        .expect(&format!("Cannot save Telegram context to {}", path_str));
    }

    fn update_chat_id(&mut self, new_id: u32) -> bool{

        let update = self.chat_id.map_or(true, |chat_id| chat_id != new_id);
        if update {
            info!("Active ChatID changed to {}", new_id);
            self.chat_id = Some(new_id);
        }
        update
    }
}
