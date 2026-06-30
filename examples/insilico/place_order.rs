use dioxus::prelude::*;

use crate::market::{DOWN, Market, SYM, UP};

const TEAL: &str = "#63e9cd";
const MUTED: &str = "#5a6b66";

/// The order ticket: Limit/Market BUY/SELL with size %, leverage, steppers and a summary,
/// plus an oversized button that exercises the rejection path — a "false submit" flagged red.
#[component]
pub fn PlaceOrder(market: Signal<Market>) -> Element {
	let px = market.read().px;
	let cash = market.read().cash;

	let mut is_limit = use_signal(|| true);
	let mut qty = use_signal(|| "10".to_string());
	let mut limit = use_signal(|| format!("{px:.2}"));
	let mut lev = use_signal(|| 5u32);
	let mut reduce_only = use_signal(|| false);
	let mut post_only = use_signal(|| false);

	let tick = (px * 0.0005).max(0.01);

	let parse = |s: &str| s.trim().parse::<f64>().ok().filter(|v| v.is_finite());
	let q_val = parse(&qty()).filter(|q| *q > 0.0);
	let l_val = parse(&limit()).filter(|p| *p > 0.0);
	let lim = is_limit();
	// effective fill price for valuation: limit px when limit, live mid when market.
	let eff_px = if lim { l_val.unwrap_or(px) } else { px };
	let valid = q_val.is_some() && (!lim || l_val.is_some());

	let q = q_val.unwrap_or(0.0);
	let lv = lev() as f64;
	let value = q * eff_px;
	let margin = value / lv;
	let fees = value * 0.00055;

	// `None` price = market order; aborts if the ticket can't be parsed/validated.
	let read_ticket = move || -> Option<(f64, Option<f64>)> {
		let q = qty().trim().parse::<f64>().ok().filter(|q| *q > 0.0 && q.is_finite())?;
		if is_limit() {
			let p = limit().trim().parse::<f64>().ok().filter(|p| *p > 0.0 && p.is_finite())?;
			Some((q, Some(p)))
		} else {
			Some((q, None))
		}
	};

	let field = "width:100%; box-sizing:border-box; background:#0b1411; border:1px solid #2c3a36; color:#ddd;
		border-radius:4px; padding:6px 8px; font:12px ui-monospace, monospace; outline:none;";
	let step_btn = "width:24px; flex:none; cursor:pointer; background:#13201c; border:1px solid #2c3a36; color:#bcd;
		border-radius:4px; font:13px ui-monospace, monospace; line-height:1;";

	let tab = |on: bool| -> String {
		format!(
			"flex:1; cursor:pointer; border:0; padding:6px 0; font:600 12px ui-monospace, monospace; border-radius:4px; \
			background:{}; color:{};",
			if on { "#1c2e29" } else { "transparent" },
			if on { TEAL } else { MUTED }
		)
	};
	let pct_btn = "flex:1; cursor:pointer; background:#0f1a17; border:1px solid #2c3a36; color:#9fb;
		border-radius:4px; padding:4px 0; font:11px ui-monospace, monospace;";

	let switch = |on: bool| -> String {
		format!(
			"width:30px; height:16px; border-radius:9px; position:relative; transition:.15s; flex:none; \
			background:{};",
			if on { TEAL } else { "#26332f" }
		)
	};
	let knob = |on: bool| -> String {
		format!(
			"position:absolute; top:2px; left:{}; width:12px; height:12px; border-radius:50%; background:#0b1411; transition:.15s;",
			if on { "16px" } else { "2px" }
		)
	};

	let buy_dis = !valid;
	let sell_dis = !valid;
	let buy_style = format!(
		"flex:1; border:0; border-radius:5px; padding:11px 0; font:700 13px ui-monospace, monospace; color:#062; \
		background:{}; cursor:{}; opacity:{};",
		UP,
		if buy_dis { "not-allowed" } else { "pointer" },
		if buy_dis { "0.4" } else { "1" }
	);
	let sell_style = format!(
		"flex:1; border:0; border-radius:5px; padding:11px 0; font:700 13px ui-monospace, monospace; color:#400; \
		background:{}; cursor:{}; opacity:{};",
		DOWN,
		if sell_dis { "not-allowed" } else { "pointer" },
		if sell_dis { "0.4" } else { "1" }
	);

	let row = "display:flex; align-items:center; justify-content:space-between; gap:8px;";
	let sum = "color:#8aa; font:11px ui-monospace, monospace;";

	rsx! {
		div { style: "padding:8px; font:12px ui-monospace, monospace; height:100%; box-sizing:border-box;
			display:flex; flex-direction:column; gap:7px; overflow:hidden;",

			div { style: "color:{TEAL}; font-weight:600; display:flex; justify-content:space-between;",
				span { "{SYM}" }
				span { style: "color:{MUTED};", "mkt {px:.2}" }
			}

			div { style: "display:flex; gap:4px; background:#0b1411; border:1px solid #2c3a36; border-radius:5px; padding:2px;",
				button { style: tab(lim), onclick: move |_| { is_limit.set(true); limit.set(format!("{:.2}", market.read().px)); }, "Limit" }
				button { style: tab(!lim), onclick: move |_| is_limit.set(false), "Market" }
			}

			if lim {
				div {
					label { style: "color:{MUTED}; font-size:11px;", "Price (USDT)" }
					div { style: "{row} margin-top:3px;",
						button { style: step_btn, onclick: move |_| { let v = parse(&limit()).unwrap_or(px); limit.set(format!("{:.2}", (v - tick).max(0.0))); }, "−" }
						input { style: "{field} text-align:center;", value: "{limit}", inputmode: "decimal", oninput: move |e| limit.set(e.value()) }
						button { style: step_btn, onclick: move |_| { let v = parse(&limit()).unwrap_or(px); limit.set(format!("{:.2}", v + tick)); }, "+" }
					}
				}
			} else {
				div { style: "color:{MUTED}; font-size:11px; padding:5px 0;", "Market — fills at {px:.2}" }
			}

			div {
				label { style: "color:{MUTED}; font-size:11px;", "Quantity ({SYM})" }
				div { style: "{row} margin-top:3px;",
					button { style: step_btn, onclick: move |_| { let v = parse(&qty()).unwrap_or(0.0); qty.set(format!("{:.2}", (v - 1.0).max(0.0))); }, "−" }
					input { style: "{field} text-align:center;", value: "{qty}", inputmode: "decimal", oninput: move |e| qty.set(e.value()) }
					button { style: step_btn, onclick: move |_| { let v = parse(&qty()).unwrap_or(0.0); qty.set(format!("{:.2}", v + 1.0)); }, "+" }
				}
			}

			div { style: "display:flex; gap:4px;",
				for frac in [0.25f64, 0.50, 0.75, 1.00] {
					button {
						style: pct_btn,
						onclick: move |_| {
							let bp = cash * lev() as f64 / eff_px.max(0.01);
							qty.set(format!("{:.2}", (bp * frac * 100.0).floor() / 100.0));
						},
						"{(frac * 100.0) as u32}%"
					}
				}
			}

			div { style: "{row}",
				span { style: "color:{MUTED}; font-size:11px;", "Leverage" }
				div { style: "display:flex; gap:4px;",
					for l in [1u32, 5, 10, 20] {
						button {
							style: format!("cursor:pointer; border:1px solid #2c3a36; border-radius:4px; padding:3px 7px; font:11px ui-monospace, monospace; background:{}; color:{};",
								if lev() == l { "#1c2e29" } else { "transparent" }, if lev() == l { TEAL } else { MUTED }),
							onclick: move |_| lev.set(l),
							"{l}x"
						}
					}
				}
			}

			div { style: "{row}",
				div { style: "display:flex; align-items:center; gap:6px; cursor:pointer;", onclick: move |_| { let v = reduce_only(); reduce_only.set(!v); },
					div { style: switch(reduce_only()), div { style: knob(reduce_only()) } }
					span { style: "color:{MUTED}; font-size:11px;", "Reduce-Only" }
				}
				div { style: "display:flex; align-items:center; gap:6px; cursor:pointer;", onclick: move |_| { let v = post_only(); post_only.set(!v); },
					div { style: switch(post_only()), div { style: knob(post_only()) } }
					span { style: "color:{MUTED}; font-size:11px;", "Post-Only" }
				}
			}

			div { style: "border-top:1px solid #1d2926; padding-top:6px; display:flex; flex-direction:column; gap:3px;",
				div { style: "{row}", span { style: sum, "Order Value" } span { style: sum, "{value:.2}" } }
				div { style: "{row}", span { style: sum, "Est. Margin" } span { style: sum, "{margin:.2}" } }
				div { style: "{row}", span { style: sum, "Est. Fees" } span { style: sum, "{fees:.3}" } }
				div { style: "{row}", span { style: sum, "Available" } span { style: sum, "{cash:.2}" } }
			}

			div { style: "display:flex; gap:6px; margin-top:auto;",
				button {
					style: buy_style,
					disabled: buy_dis,
					onclick: move |_| { if let Some((q, p)) = read_ticket() { let mut mk = market; mk.write().submit(true, p, q); } },
					"BUY"
				}
				button {
					style: sell_style,
					disabled: sell_dis,
					onclick: move |_| { if let Some((q, p)) = read_ticket() { let mut mk = market; mk.write().submit(false, p, q); } },
					"SELL"
				}
			}

			button {
				style: "cursor:pointer; border:1px dashed #6a4; background:transparent; color:#e8c06a; border-radius:4px; padding:5px 0; font:11px ui-monospace, monospace;",
				title: "Submits an oversized order to exercise the rejection path",
				onclick: move |_| { let q = read_ticket().map(|(q, _)| q).unwrap_or(10.0); let mut mk = market; mk.write().submit(true, None, q * 100_000.0); },
				"⚠ false submit (reject)"
			}
		}
	}
}
