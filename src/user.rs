use parking_lot::RwLock;
use serenity::model::id::UserId as DiscordId;
use telegram_bot::{
	Sticker,
	UserId as TeleId,
};

#[derive(Debug)]
pub struct User {
	pub tele_id: TeleId,
	pub discord_id: DiscordId,
	pub cur_sticker: RwLock<Option<Sticker>>,
}

impl User {
	pub fn new(tele_id: &TeleId, discord_id: &DiscordId) -> Self {
		Self {
			tele_id: *tele_id,
			discord_id: *discord_id,
			cur_sticker: RwLock::new(None),
		}
	}
}
