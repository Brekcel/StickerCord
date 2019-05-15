# StickerCord

A Telegram & Discord bot that jankily allows users to send Telegram stickers on Discord channels.

## Inviting to your server (What you probably want)

To invite the bot to your channel: 
[Link](https://discordapp.com/api/oauth2/authorize?client_id=577677485106528266&permissions=322624&scope=bot)

Note: There is no guarantee that the bot will be operating well at this time.

## Manually Running

This bot uses Rust. [You must have Rust installed in order to compile and run.](https://www.rust-lang.org/)

Once Rust is installed, set the Environment Variables as such:
```
TELE_TOKEN=Your Telegram bot's token
DISC_TOKEN=Your Discord bot's token
DISC_TAG=Your Discord bot's tag (In the form of NAME#0000)
```

Then run the following commands:
```
git clone https://github.com/Brekcel/StickerCord.git
git submodule update --init --recursive
cargo run --release
```
