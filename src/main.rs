use futures::{
	future::Future as _,
	stream::Stream as _,
};
use lazy_static::lazy_static;
use tokio_core::reactor::Core;

use serenity::{
	client::{
		Client,
		Context,
	},
	model::channel::Message,
	prelude::EventHandler,
};

use crate::database::DBVerification;
use crossbeam_channel::{
	unbounded,
	Receiver,
	Sender,
};
use database::Database;
use parking_lot::RwLock;
use rand::{
	distributions::Alphanumeric,
	thread_rng,
	Rng,
};
use serenity::utils::MessageBuilder;
use std::iter;
use telegram_bot::{
	Api,
	MessageKind,
	ParseMode,
	SendMessage,
	Sticker,
	UpdateKind,
};

mod cache;
mod database;
mod user;

use crate::cache::Cache;
use serenity::model::{
	gateway::Activity,
	prelude::Ready,
};

const TELEGRAM_TOKEN: &'static str = env!("TELE_TOKEN");
const DISCORD_TOKEN: &'static str = env!("DISC_TOKEN");

lazy_static! {
	static ref DB: RwLock<Database> = RwLock::new(Database::from_file());
	static ref DL_CHANNELS: (Sender<String>, Receiver<String>) = unbounded();
	static ref CACHE: Cache = Cache::new();
}

struct StickerHandler;

impl EventHandler for StickerHandler {
	fn message(&self, ctx: Context, new_message: Message) {
		if new_message.content.starts_with(".s") || new_message.content.starts_with(".sticker") {
			match DB.read().disc_id(&new_message.author.id) {
				Some(user) => match &*user.cur_sticker.read() {
					Some(stick) => {
						let sticker: &Sticker = stick;
						let path_str = CACHE.get_sticker(sticker);
						new_message
							.channel_id
							.send_files(&ctx.http, Some(&path_str), |m| {
								m.content({
									let mut resp = MessageBuilder::new();
									resp.mention(&new_message.author).push(" Sent a ");
									sticker.emoji.as_ref().map(|emoji| resp.push(emoji));
									resp.push(" sticker.");
									resp
								});
								m
							})
							.unwrap();
					},
					None => {
						new_message.reply(&ctx,
									  "You do not have a currently bound sticker. Please send a sticker to StickyCord on Telegram first."
					)
						.unwrap();
					},
				},
				None => {
					new_message
						.reply(
							&ctx,
							"You have not setup your Telegram with StickerCord yet.",
						)
						.unwrap();
				},
			}
			new_message.delete(&ctx).unwrap();
		} else if new_message.content.starts_with(".verify") {
			if new_message.is_private() {
				let tag = new_message.author.tag();
				let magic = &new_message.content.trim()[8..];
				let mut db = DB.write();
				db.verify(&magic, &new_message.author.id, &tag, ctx, &new_message);
			} else {
				new_message.delete(&ctx).unwrap();
				DB.write().remove_verify(&new_message.content.trim()[8..]);
				new_message.reply(&ctx, "Please send this to the bot as a *direct message*. You registration has been reset for security.").unwrap();
			}
		}
	}

	fn ready(&self, ctx: Context, _about: Ready) {
		ctx.set_activity(Activity::playing("http://t.me/stickercord_bot"));
	}
}

fn main() {
	let mut core = Core::new().unwrap();
	let core_handle = core.handle();

	let tele_client = Api::configure(&TELEGRAM_TOKEN)
		.build(core.handle())
		.unwrap();

	let _discord_thread = std::thread::spawn(|| {
		let mut discord_client = Client::new(&DISCORD_TOKEN, StickerHandler).unwrap();
		discord_client.start().unwrap();
	});

	let future = tele_client
        .stream()
        .for_each(|update| {
            if let UpdateKind::Message(message) = update.kind {
                match message.kind {
                    MessageKind::Text { data, .. } => {
                        let trimmed = data.trim();
                        let first_cmd = if trimmed.starts_with('/') {
                            trimmed.split_whitespace().next().map(|x| &x[1..])
                        } else {
                            None
                        };
                        match first_cmd {
                            Some("help") | Some("start") => {
                                let mut reply = SendMessage::new(message.chat, include_str!("../help.txt"));
                                reply.parse_mode(ParseMode::Markdown);
                                tele_client.spawn(reply)
                            }
							Some("register") => {
								if DB.read().tele_id(&message.from.id).is_some() {
									tele_client.spawn(SendMessage::new(message.chat, "This account is already registered. Please /unregister before trying to register."))
								} else {
									let discord_tag = &trimmed[10..];
									let mut rng = thread_rng();
									let magic: String = iter::repeat(()).map(|()| rng.sample(Alphanumeric)).take(64).collect();
									let verification_id = DBVerification {
										magic: magic.clone(),
										tele_id: message.from.id,
										discord_tag: String::from(discord_tag)
									};
									DB.write().verify_start(verification_id);
									let mut reply = SendMessage::new(message.chat,
																	 format!(
																		 "Verification has begun. Please send the following message to \
																	  *StickerCord#4070* on Discord:\n\n.verify {}",
																		 magic
																	 )
									);
									reply.parse_mode(ParseMode::Markdown);
									tele_client.spawn(reply);
								}
							}
							Some("unregister") => {
								let removed = DB.write().remove_user(&message.from.id);
								let msg = if removed {
									"Successfully unregistered accounts."
								} else {
									"This account was not registered."
								};
								tele_client.spawn(SendMessage::new(message.chat, msg));
							}
                            _ => tele_client.spawn(SendMessage::new(message.chat,
                                "Unknown command. Please either send /help, for help, or a sticker.",
                            )),
                        }
                    }
                    MessageKind::Sticker { data } => {
                        if let Some(user) = DB.read().tele_id(&message.from.id) {
                            let mut cur_sticker = user.cur_sticker.write();
							CACHE.cache_sticker(data.clone(), &core_handle, &tele_client);
							*cur_sticker = Some(data.clone());
                        } else {
                            tele_client.spawn(SendMessage::new(message.chat,
                                "Please register your Discord id before sending stickers",
                            ))
                        }
                    }
                    _ => (),
                }
            }
            Ok(())
        })
        .map_err(|_e| {
			()
		});

	core.run(future).unwrap();
}
