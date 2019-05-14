use crate::TELEGRAM_TOKEN;
use futures::future::Future;
use parking_lot::{
	Condvar,
	Mutex,
};
use std::{
	collections::HashMap,
	fs::OpenOptions,
	io::BufWriter,
	sync::Arc,
};
use telegram_bot::{
	Api,
	GetFile,
	Sticker,
};
use tokio_core::reactor::Handle;

use image::png::PNGEncoder;
use libwebp_sys::*;
use std::path::PathBuf;

pub struct Cache {
	io: Arc<Mutex<HashMap<String, Arc<Condvar>>>>,
	reqwest: Arc<reqwest::Client>,
}

impl Cache {
	pub fn new() -> Self {
		let io = Arc::new(Mutex::new(HashMap::new()));
		let reqwest = Arc::new(reqwest::Client::new());
		Self { io, reqwest }
	}

	fn path_from_sticker(sticker: &str) -> String { format!("./cache/{}.png", sticker) }

	pub fn get_sticker(&self, sticker: &Sticker) -> PathBuf {
		let path = Self::path_from_sticker(&sticker.file_id);
		let io = self.io.lock().get(&path).map(Arc::clone);
		match io {
			Some(io) => {
				let mutex = Mutex::new(());
				io.wait(&mut mutex.lock());
			},
			None => (),
		};
		PathBuf::from(path)
	}

	pub fn cache_sticker(&self, sticker: Sticker, handle: &Handle, api: &Api) {
		let io = self.io.clone();
		let io2 = self.io.clone();
		let path = Self::path_from_sticker(&sticker.file_id);
		let mut lock = io.lock();
		OpenOptions::new()
			.write(true)
			.create_new(true)
			.open(&path)
			.map(move |file| {
				let cond_var = lock
					.entry(path.clone())
					.or_insert(Arc::new(Condvar::new()))
					.clone();
				drop(lock);
				let req = self.reqwest.clone();
				let fut = api
					.send(GetFile::new(sticker))
					.map_err(|_| ())
					.and_then(move |f| {
						let webp_data = {
							let url = f.get_url(&TELEGRAM_TOKEN).unwrap();
							let mut res = req.get(&url).send().unwrap();
							let mut data = Vec::new();
							res.copy_to(&mut data).unwrap();
							data
						};
						unsafe {
							let mut width = 0;
							let mut height = 0;
							let data_ptr = WebPDecodeRGBA(
								webp_data.as_ptr(),
								webp_data.len(),
								&mut width,
								&mut height,
							);
							let data_len = width * height * 4;
							{
								let data_slice =
									std::slice::from_raw_parts(data_ptr, data_len as usize);
								let png = PNGEncoder::new(BufWriter::new(file));
								png.encode(
									data_slice,
									width as u32,
									height as u32,
									image::ColorType::RGBA(8),
								)
								.unwrap();
							}
							io2.lock().remove(&path);
							cond_var.notify_all();
							WebPFree(data_ptr as *mut _);
						}
						Ok(())
					});
				handle.spawn(fut);
			})
			.unwrap_or_default();
	}
}
