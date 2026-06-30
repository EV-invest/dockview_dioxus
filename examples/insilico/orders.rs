use dioxus::prelude::*;

use crate::market::{DOWN, Market, SYM, UP};

#[component]
pub fn Orders(market: Signal<Market>) -> Element {
	let m = market.read();
	let px = m.px;
	let n = m.orders.len();
	if n == 0 {
		return rsx! {
			div { style: "display:flex; align-items:center; justify-content:center; height:100%; box-sizing:border-box; color:#5a6b66; font:12px ui-monospace, monospace; letter-spacing:.5px;",
				"No open orders"
			}
		};
	}
	let th = "position:sticky; top:0; background:#0e1513; color:#5a6b66; text-align:right; padding:5px 7px; font-weight:500; border-bottom:1px solid #2c3a36; z-index:1;";
	let th_l = "position:sticky; top:0; background:#0e1513; color:#5a6b66; text-align:left; padding:5px 7px; font-weight:500; border-bottom:1px solid #2c3a36; z-index:1;";
	let td = "text-align:right; padding:4px 7px; border-bottom:1px solid #16201d;";
	let td_l = "text-align:left; padding:4px 7px; border-bottom:1px solid #16201d;";
	rsx! {
		style { {"#ob tbody tr:hover{{background:#101b18;}} #ob button:hover{{background:#1a2622;border-color:#ff5d6c;}}"} }
		div { id: "ob", style: "display:flex; flex-direction:column; height:100%; box-sizing:border-box; font:11px ui-monospace, monospace; color:#c9d6d2; background:#0b1110;",
			div { style: "display:flex; align-items:center; justify-content:space-between; padding:6px 9px; border-bottom:1px solid #2c3a36; flex:0 0 auto;",
				span { style: "color:#63e9cd; letter-spacing:.5px;",
					"OPEN ORDERS "
					span { style: "color:#5a6b66;", "({n})" }
				}
				button {
					style: "cursor:pointer; border:1px solid #43242a; background:transparent; color:#ff5d6c; border-radius:3px; font:10px ui-monospace; padding:3px 9px; letter-spacing:.5px; transition:background .12s,border-color .12s;",
					onclick: move |_| {
						let ids: Vec<u64> = market.read().orders.iter().map(|o| o.id).collect();
						let mut mk = market;
						let mut w = mk.write();
						for id in ids { w.cancel(id); }
					},
					"Cancel All"
				}
			}
			div { style: "flex:1 1 auto; overflow:auto;",
				table { style: "width:100%; border-collapse:collapse; font:11px ui-monospace, monospace;",
					thead {
						tr {
							th { style: "{th_l}", "Symbol" }
							th { style: "{th_l}", "Side" }
							th { style: "{th_l}", "Type" }
							th { style: "{th}", "Price" }
							th { style: "{th}", "Qty" }
							th { style: "{th}", "Filled" }
							th { style: "{th}", "Distance" }
							th { style: "{th_l}", "Status" }
							th { style: "{th}", "" }
						}
					}
					tbody {
						for o in m.orders.clone() {
							{
								let col = if o.buy { UP } else { DOWN };
								let side = if o.buy { "BUY" } else { "SELL" };
								let diff = (px - o.px) / o.px;
								let bps = diff.abs() * 10_000.0;
								// warmer as it nears the fill; crossed-but-unfilled shows live as it resolves
								let dcol = if bps < 5.0 { DOWN } else if bps < 25.0 { "#f0a23c" } else { "#5a6b66" };
								let dist = format!("{:+.1} bps", diff * 10_000.0);
								let status = if bps < 5.0 { "Working" } else { "Open" };
								rsx! {
									tr {
										td { style: "{td_l} color:#c9d6d2;", "{SYM}" }
										td { style: "{td_l} color:{col}; font-weight:600;", "{side}" }
										td { style: "{td_l} color:#5a6b66;", "Limit" }
										td { style: "{td} color:#9fd; font-variant-numeric:tabular-nums;", "{o.px:.2}" }
										td { style: "{td} font-variant-numeric:tabular-nums;", "{o.qty:.1}" }
										td { style: "{td} color:#5a6b66; font-variant-numeric:tabular-nums;", "0.0/{o.qty:.1}" }
										td { style: "{td} color:{dcol}; font-variant-numeric:tabular-nums;", "{dist}" }
										td { style: "{td_l} color:#5a6b66;", "{status}" }
										td { style: "{td}",
											button {
												style: "cursor:pointer; border:1px solid #2c3a36; background:transparent; color:#ff5d6c; border-radius:3px; font:10px ui-monospace; padding:2px 8px; transition:background .12s,border-color .12s;",
												onclick: move |_| { let mut mk = market; mk.write().cancel(o.id); },
												"Cancel"
											}
										}
									}
								}
							}
						}
					}
				}
			}
		}
	}
}
