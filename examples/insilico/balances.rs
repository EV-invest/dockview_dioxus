use dioxus::prelude::*;

use crate::market::{DOWN, Market, SYM, UP};

/// Matches the sim's order-rejection cap (`notional > cash*20`).
const LEV: f64 = 20.0;

#[component]
pub fn Balances(market: Signal<Market>) -> Element {
	let m = market.read();
	let upnl = m.unrealized();
	let equity = m.cash + upnl;
	let notional = m.pos_qty.abs() * m.px;
	let used = notional / LEV;
	let buying_power = m.cash * LEV;
	let free = (buying_power - notional).max(0.0);
	let ratio = if equity > 0.0 { (used / equity).min(1.0) } else { 1.0 };
	let maint = notional * 0.005; // 0.5% maintenance, exchange convention
	let pos_val = m.pos_qty.abs() * m.px;

	let upnl_col = if upnl >= 0.0 { UP } else { DOWN };
	let eq_col = if upnl > 0.0 {
		"#7ff0c8"
	} else if upnl < 0.0 {
		"#ff8a96"
	} else {
		"#63e9cd"
	};

	// green → amber → red as the account leans on its margin.
	let (gauge, health) = if ratio < 0.4 {
		(UP, "HEALTHY")
	} else if ratio < 0.75 {
		("#e8c06a", "ELEVATED")
	} else {
		(DOWN, "AT RISK")
	};
	let pct = ratio * 100.0;

	// donut geometry: r=26, circumference ≈ 163.36; dash the filled arc.
	let circ = 2.0 * std::f64::consts::PI * 26.0;
	let dash = circ * ratio;
	let gap = circ - dash;

	let row = "display:flex; justify-content:space-between; align-items:baseline; padding:3px 0;";
	let key = "color:#5a6b66;";
	let val = "color:#cdd;";
	let div = "border-top:1px solid #1c2723; margin:5px 0;";
	rsx! {
		div { style: "padding:8px 10px; font:12px ui-monospace, monospace; height:100%; box-sizing:border-box; display:flex; flex-direction:column; color:#cdd; overflow:hidden;",
			div { style: "display:flex; align-items:center; gap:10px;",
				svg { width: "64", height: "64", view_box: "0 0 64 64", style: "flex:0 0 auto;",
					circle { cx: "32", cy: "32", r: "26", fill: "none", stroke: "#1c2723", stroke_width: "6" }
					circle {
						cx: "32", cy: "32", r: "26", fill: "none", stroke: "{gauge}", stroke_width: "6",
						stroke_linecap: "round", stroke_dasharray: "{dash:.2} {gap:.2}",
						transform: "rotate(-90 32 32)",
					}
					text { x: "32", y: "30", text_anchor: "middle", fill: "{gauge}", style: "font:700 12px ui-monospace, monospace;", "{pct:.0}%" }
					text { x: "32", y: "43", text_anchor: "middle", fill: "#5a6b66", style: "font:8px ui-monospace, monospace;", "MARGIN" }
				}
				div { style: "flex:1; min-width:0;",
					div { style: "color:#5a6b66; font-size:10px; letter-spacing:.06em;", "EQUITY · USDT" }
					div { style: "color:{eq_col}; font:700 22px ui-monospace, monospace; line-height:1.1;", "{equity:.2}" }
					div { style: "color:{upnl_col}; font-size:11px;", "{upnl:+.2} uPnL  ·  {health}" }
				}
			}

			div { style: "{div}" }

			Row { s: row, k: key, v: val, label: "Wallet / Cash", value: format!("{:.2}", m.cash) }
			div { style: "{row}",
				span { style: "{key}", "Unrealized PnL" }
				span { style: "color:{upnl_col};", "{upnl:+.2}" }
			}
			div { style: "{row}",
				span { style: "{key}", "Equity" }
				span { style: "color:#63e9cd;", "{equity:.2}" }
			}

			div { style: "{div}" }

			Row { s: row, k: key, v: val, label: "Used Margin", value: format!("{:.2}", used) }
			Row { s: row, k: key, v: val, label: "Free Margin", value: format!("{:.2}", free) }
			div { style: "{row}",
				span { style: "{key}", "Margin Ratio" }
				span { style: "color:{gauge};", "{pct:.2}%" }
			}
			Row { s: row, k: key, v: val, label: "Maint. Margin", value: format!("{:.2}", maint) }

			div { style: "{div}" }

			div { style: "color:#5a6b66; font-size:10px; letter-spacing:.06em; margin-bottom:2px;", "ASSETS" }
			div { style: "{row}",
				span { style: "{key}", "USDT" }
				span { style: "{val}", "{m.cash:.2}" }
			}
			div { style: "{row}",
				span { style: "{key}", "{SYM} mark" }
				span { style: "{val}", "{pos_val:.2}" }
			}
		}
	}
}

#[component]
fn Row(s: &'static str, k: &'static str, v: &'static str, label: &'static str, value: String) -> Element {
	rsx! {
		div { style: "{s}",
			span { style: "{k}", "{label}" }
			span { style: "{v}", "{value}" }
		}
	}
}
