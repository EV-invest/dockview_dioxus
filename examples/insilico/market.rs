//! The one world every pane renders. The ticker advances it; the place-order pane and the
//! per-order cancel buttons mutate it. Continuity across panes is therefore free — they are
//! all just different projections of this single state.

/// The one instrument the whole terminal trades, mirroring the screenshot.
pub const SYM: &str = "SOLUSDT";
pub const UP: &str = "#3ddc84";
pub const DOWN: &str = "#ff5d6c";

/// xorshift64 — a tiny no-dep PRNG so the sim and random tile sizes need no `rand`.
pub fn xorshift(s: &mut u64) -> u64 {
	*s ^= *s << 13;
	*s ^= *s >> 7;
	*s ^= *s << 17;
	*s
}

/// A single print on the tape (also how a user fill enters the world).
#[derive(Clone)]
pub struct Trade {
	pub t: u64,
	pub px: f64,
	pub qty: f64,
	pub buy: bool,
}

/// A resting user limit order, filled by [`Market::tick`] once price crosses it.
#[derive(Clone)]
pub struct Order {
	pub id: u64,
	pub px: f64,
	pub qty: f64,
	pub buy: bool,
}

#[derive(Clone)]
pub struct Market {
	pub t: u64,
	pub seed: u64,
	pub next_oid: u64,
	pub px: f64,
	pub hist: Vec<f64>,
	pub tape: Vec<Trade>,
	pub orders: Vec<Order>,
	pub pos_qty: f64,
	pub pos_avg: f64,
	pub cash: f64,
	pub log: Vec<(u64, String)>,
	pub chat: Vec<(&'static str, String)>,
	pub news: Vec<(&'static str, String)>,
}

impl Market {
	pub fn new() -> Self {
		let mut m = Market {
			t: 0,
			seed: 0x1234_5678_9abc_def0,
			next_oid: 1,
			px: 150.0,
			hist: vec![150.0; 48],
			tape: Vec::new(),
			orders: Vec::new(),
			pos_qty: 0.0,
			pos_avg: 0.0,
			cash: 10_000.0,
			log: Vec::new(),
			chat: vec![
				("yk", "change pairs".into()),
				("yk", "if you didnt accidentally print today ur ngmi".into()),
				("hr1q9j", "https://i.imgur.com/echg.png had one more scalp left in me".into()),
			],
			news: vec![
				("@TrumpTruthOnX", "Congressman Jeff Crank is doing a truly fantastic job representing CO-5.".into()),
				("Reuters", "SOL ETF inflows hit record as desks rotate out of majors.".into()),
			],
		};
		m.note("Terminal v5.3.2 — Bybit connected");
		m
	}

	pub fn rng(&mut self) -> u64 {
		xorshift(&mut self.seed)
	}

	/// uniform in [0,1)
	pub fn unit(&mut self) -> f64 {
		(self.rng() % 1_000_000) as f64 / 1_000_000.0
	}

	pub fn note(&mut self, s: impl Into<String>) {
		self.log.push((self.t, s.into()));
		if self.log.len() > 80 {
			let drop = self.log.len() - 80;
			self.log.drain(0..drop);
		}
	}

	pub fn print(&mut self, px: f64, qty: f64, buy: bool) {
		self.tape.insert(0, Trade { t: self.t, px, qty, buy });
		self.tape.truncate(40);
	}

	/// One simulation step: walk price, print a market trade, fill any crossed user orders.
	pub fn tick(&mut self) {
		self.t += 1;
		let drift = (self.unit() - 0.5) * 0.006;
		self.px = (self.px * (1.0 + drift)).max(1.0);
		self.hist.push(self.px);
		if self.hist.len() > 96 {
			self.hist.remove(0);
		}

		let buy = self.unit() > 0.5;
		let qty = (self.unit() * 40.0 + 10.0).round() / 10.0;
		let px = self.px * (1.0 + if buy { 0.0002 } else { -0.0002 });
		self.print(px, qty, buy);

		let px_now = self.px;
		let crossed: Vec<Order> = self.orders.iter().filter(|o| if o.buy { px_now <= o.px } else { px_now >= o.px }).cloned().collect();
		for o in crossed {
			self.orders.retain(|x| x.id != o.id);
			self.fill(o.buy, o.px, o.qty, "limit");
		}

		if self.t % 9 == 0 {
			let who = if self.unit() > 0.5 { "yk" } else { "hr1q9j" };
			self.chat.push((who, format!("{} {:.2}? gm", SYM, self.px)));
			self.chat.truncate_front(40);
		}
		if self.t % 23 == 0 {
			self.news.push(("Tape", format!("{} prints {:.2}, 24h vol building", SYM, self.px)));
			self.news.truncate_front(20);
		}
	}

	/// Apply a fill to the position/cash, booking realized PnL when it reduces. Logs to console.
	pub fn fill(&mut self, buy: bool, px: f64, qty: f64, via: &str) {
		let signed = if buy { qty } else { -qty };
		let same = self.pos_qty == 0.0 || (self.pos_qty > 0.0) == buy;
		if same {
			let abs = self.pos_qty.abs();
			self.pos_avg = (self.pos_avg * abs + px * qty) / (abs + qty);
			self.pos_qty += signed;
		} else {
			let closed = qty.min(self.pos_qty.abs());
			self.cash += closed * (px - self.pos_avg) * self.pos_qty.signum();
			let before = self.pos_qty;
			self.pos_qty += signed;
			if before.signum() != self.pos_qty.signum() && self.pos_qty != 0.0 {
				self.pos_avg = px; // flipped through flat → new side opens here
			}
		}
		self.print(px, qty, buy);
		self.note(format!("FILL {} {:.1} {} @ {:.2} ({via})", if buy { "BUY" } else { "SELL" }, qty, SYM, px));
	}

	/// Place a user order. `px == None` is a market order (fills now); a resting limit lands in
	/// the book. An oversized notional is rejected — the deliberate "false submit" path.
	pub fn submit(&mut self, buy: bool, px: Option<f64>, qty: f64) {
		let notional = qty * px.unwrap_or(self.px);
		if notional > self.cash * 20.0 {
			self.note(format!("REJECTED {} {:.1} {} — insufficient margin", if buy { "BUY" } else { "SELL" }, qty, SYM));
			return;
		}
		match px {
			None => self.fill(buy, self.px, qty, "market"),
			Some(p) => {
				let id = self.next_oid;
				self.next_oid += 1;
				self.orders.push(Order { id, px: p, qty, buy });
				self.note(format!("PLACED #{id} {} {:.1} @ {:.2}", if buy { "BUY" } else { "SELL" }, qty, p));
			}
		}
	}

	pub fn cancel(&mut self, id: u64) {
		self.orders.retain(|o| o.id != id);
		self.note(format!("CANCEL #{id}"));
	}

	pub fn unrealized(&self) -> f64 {
		self.pos_qty * (self.px - self.pos_avg)
	}
}

/// `Vec::truncate` keeps the head; chat/news want the newest tail kept.
pub trait TruncFront<T> {
	fn truncate_front(&mut self, keep: usize);
}
impl<T> TruncFront<T> for Vec<T> {
	fn truncate_front(&mut self, keep: usize) {
		if self.len() > keep {
			self.drain(0..self.len() - keep);
		}
	}
}
