use crate::user::User;

use serenity::{
	client::Context,
	model::{
		channel::Message,
		id::UserId as DiscordId,
	},
};
use std::{
	fs::File,
	io::{
		BufRead,
		BufReader,
		BufWriter,
		Write,
	},
};
use telegram_bot::{
	Integer,
	UserId as TeleId,
};

#[derive(Debug, PartialEq)]
pub struct DBVerification {
	pub magic: String,
	pub tele_id: TeleId,
	pub discord_tag: String,
}

#[derive(Debug)]
pub struct Database {
	users: Vec<User>,
	awaiting_reg: Vec<DBVerification>,
}

impl Database {
	pub fn from_file() -> Self {
		let users = BufReader::new(File::open("users.txt").unwrap());
		let users = users
			.lines()
			.map(|l| {
				let l = l.unwrap();
				let mut split = l.split(":");
				let tele_id = split.next().unwrap().parse::<Integer>().unwrap().into();
				let discord_id = DiscordId(split.next().unwrap().parse().unwrap());
				User::new(&tele_id, &discord_id)
			})
			.collect();
		Self {
			users,
			awaiting_reg: Vec::new(),
		}
	}

	pub fn tele_id(&self, tele_id: &TeleId) -> Option<&User> {
		self.users.iter().find(|&x| x.tele_id == *tele_id)
	}

	pub fn disc_id(&self, disc_id: &DiscordId) -> Option<&User> {
		self.users.iter().find(|x| x.discord_id == *disc_id)
	}

	pub fn remove_user(&mut self, tele_id: &TeleId) -> bool {
		let pos = self.users.iter().position(|x| x.tele_id == *tele_id);
		pos.map(|x| {
			self.users.remove(x);
			self.save();
			true
		})
		.unwrap_or(false)
	}

	pub fn verify_start(&mut self, ver: DBVerification) { self.awaiting_reg.push(ver) }

	pub fn remove_verify(&mut self, magic: &str) {
		let pos = self.awaiting_reg.iter().position(|x| x.magic == *magic);
		pos.map(|pos| self.awaiting_reg.remove(pos));
	}

	pub fn verify(
		&mut self,
		magic: &str,
		discord_id: &DiscordId,
		tag: &str,
		ctx: Context,
		new_message: &Message,
	) {
		let pos = {
			let reg = self.awaiting_reg.iter().find(|x| x.magic == *magic);
			let reg = match reg {
				Some(reg) =>
					if reg.discord_tag == *tag {
						Some(reg)
					} else {
						None
					},
				None => {
					new_message.reply(&ctx, "You either entered the wrong message or you have not begun verification.").unwrap();
					return;
				},
			};
			let reg = match reg {
				Some(reg) => reg,
				None => {
					new_message
						.reply(&ctx, "Attempting to register for the wrong Discord tag. Ensure that you entered your tag in the format of Name#0000.")
						.unwrap();
					return;
				},
			};
			self.users.push(User::new(&reg.tele_id, &discord_id));
			self.awaiting_reg.iter().position(|p| p == reg).unwrap()
		};
		self.awaiting_reg.remove(pos);
		self.save();
		new_message.reply(&ctx, "Successfully registered.").unwrap();
	}

	fn save(&self) {
		let mut out_file = BufWriter::new(File::create("users.txt").unwrap());
		for user in self.users.iter() {
			out_file
				.write(format!("{}:{}\n", user.tele_id, user.discord_id).as_bytes())
				.unwrap();
		}
	}
}
