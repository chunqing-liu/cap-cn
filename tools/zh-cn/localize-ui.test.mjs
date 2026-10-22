import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import { localize } from "./localize-ui.mjs";

test("only display arguments change, not persistence keys or user text", () => {
	const source = `
let mode = "Studio";
let json = serde_json::json!({"name": "Export", "mode": "studio"});
div().id("Export").child("Export");
ui::Button::new("Cancel").label("Cancel");
// div().child("Export")
let raw = r#"div().child("Export")"#;
`;
	const result = localize(source, "main_window.rs", {
		Export: "导出",
		Cancel: "取消",
		Studio: "本地录制",
	});
	assert.equal(result.count, 2);
	assert.ok(result.output.includes('let mode = "Studio";'));
	assert.ok(result.output.includes('.id("Export").child("导出")'));
	assert.ok(result.output.includes('new("Cancel").label("取消")'));
	assert.ok(result.output.includes('"name": "Export"'));
	assert.ok(result.output.includes('// div().child("Export")'));
	assert.ok(result.output.includes('r#"div().child("Export")"#'));
});

test("settings labels change without changing their serialized values", () => {
	const source = `
settings_enum! { Theme { Light = "light", "Light"; } }
impl Mode {
    fn slug(self) -> &'static str { "studio" }
    fn label(self) -> &'static str { match self { Self::Studio => "Studio" } }
}
self.setting_row("Export", Some("Cancel"), self.toggle("Light"));
`;
	const result = localize(source, "store.rs", {
		Light: "浅色",
		Studio: "本地录制",
		Export: "导出",
		Cancel: "取消",
	});
	assert.equal(result.count, 4);
	assert.ok(result.output.includes('Light = "light", "浅色"'));
	assert.ok(
		result.output.includes('fn slug(self) -> &\'static str { "studio" }'),
	);
	assert.ok(result.output.includes('Self::Studio => "本地录制"'));
	assert.ok(result.output.includes('self.toggle("Light")'));
});

test("conditional labels and helper text are translated", () => {
	const source = `
button.label(if busy { "Cancel" } else { "Export" });
let card = ModeCard { title: "Studio", slug: "studio" };
ui::Tooltip::new(&theme, "Export");
`;
	const result = localize(source, "main_window.rs", {
		Cancel: "取消",
		Export: "导出",
		Studio: "本地录制",
	});
	assert.equal(result.count, 4);
	assert.ok(result.output.includes('if busy { "取消" } else { "导出" }'));
	assert.ok(result.output.includes('slug: "studio"'));
});

test("render helpers preserve identifiers, protocol fields, shortcuts and match inputs", () => {
	const source = `
fn render_panel() {
    let title = "Frame";
    div().id("Frame").child(title).keys(&["End"]);
    let payload = serde_json::json!({"title": "Frame"});
    state.insert("Frame", 1);
    tracing::info!("Frame");
    match key { "Frame" => "End", _ => "Frame" }
}
`;
	const result = localize(source, "editor_window.rs", {
		Frame: "画框",
		End: "结束",
	});
	assert.ok(result.output.includes('let title = "画框"'));
	assert.ok(result.output.includes('.id("Frame")'));
	assert.ok(result.output.includes('.keys(&["End"])'));
	assert.ok(result.output.includes('json!({"title": "Frame"})'));
	assert.ok(result.output.includes('insert("Frame", 1)'));
	assert.ok(result.output.includes('info!("Frame")'));
	assert.ok(result.output.includes('"Frame" => "结束"'));
});

test("catalogue translation keeps stored enum values and font identities", () => {
	const source = `
const CAPTION_POSITIONS: [(&str, &str); 1] = [("bottom-center", "Bottom Center")];
const FONT_OPTIONS: [&str; 1] = ["System Serif"];
const TEXT_SEGMENT_WEIGHTS: [(f32, &str); 1] = [(300., "Light")];
`;
	const result = localize(source, "editor_tabs.rs", {
		"Bottom Center": "底部居中",
		"System Serif": "系统衬线字体",
		Light: "浅色",
	});
	assert.ok(result.output.includes('("bottom-center", "底部居中")'));
	assert.ok(
		result.output.includes('FONT_OPTIONS: [&str; 1] = ["System Serif"]'),
	);
	assert.ok(result.output.includes('(300., "细体")'));
});

test("translated templates preserve every Rust format argument", () => {
	const dictionary = JSON.parse(
		fs.readFileSync(new URL("./translations.json", import.meta.url), "utf8"),
	);
	const argumentsIn = (text) =>
		(
			text
				.replaceAll("{{", "")
				.replaceAll("}}", "")
				.match(/\{[^{}]*\}/g) ?? []
		).sort();
	for (const [english, chinese] of Object.entries(dictionary)) {
		assert.deepEqual(argumentsIn(chinese), argumentsIn(english), english);
	}
});

test("window exclusion identities and backend error needles remain unchanged", () => {
	const source = `
impl OwnWindow {
    pub fn title(self) -> &'static str { "Cap Settings" }
}
fn render() {
    let unavailable = error.contains("access is unavailable");
    div().child("Cap Settings");
}
`;
	const result = localize(source, "app_windows.rs", {
		"Cap Settings": "Cap 设置",
		"access is unavailable": "访问权限不可用",
	});
	assert.ok(
		result.output.includes(`fn title(self) -> &'static str { "Cap Settings" }`),
	);
	assert.ok(result.output.includes(`error.contains("access is unavailable")`));
	assert.ok(result.output.includes(`div().child("Cap 设置")`));
});
