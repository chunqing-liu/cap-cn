import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(
	path.dirname(fileURLToPath(import.meta.url)),
	"../..",
);
const sourceRoot = path.join(root, "apps/desktop-gpui/src");
const dictionary = JSON.parse(
	fs.readFileSync(new URL("./translations.json", import.meta.url), "utf8"),
);
const files = fs
	.readdirSync(sourceRoot, { recursive: true })
	.filter(
		(file) =>
			file.endsWith(".rs") &&
			!file.includes("tests") &&
			!file.includes("bench") &&
			!["assets.rs", "snapshot.rs"].includes(file),
	);
const displayArguments = new Map([
	["child", [0]],
	["label", [0]],
	["set_placeholder", [0]],
	["section", [0, 1]],
	["setting_row", [0, 1]],
	["slider_field", [0]],
	["tab", [0]],
	["copy", [0]],
	["heading", [0, 1]],
	["render_outcome_panel", [1, 2]],
	["Tooltip::new", [1]],
	["Field::stacked", [1]],
	["Subfield::plain", [1]],
	["MenuItem::action", [0]],
	["MenuItem::new", [0]],
	["MenuItem::os_action", [0]],
	["MenuItem::os_submenu", [0]],
	["SegmentOption::new", [0]],
	["Field::section", [1]],
	["Field::inline", [1]],
	["Field::plain", [1]],
	["Entry::item", [0]],
	["Entry::disabled", [0]],
	["export_section", [0]],
	["export_toggle_row", [1, 2]],
	["export_stat", [1]],
	["with_tooltip", [0]],
	["set_tooltip", [0]],
	["alert_dialog", [0, 1]],
	["group_label", [0]],
	["editor_field", [0]],
	["disclosure_row", [2]],
	["section_action", [2]],
	["kbd_tooltip", [1]],
	["tool_button", [3]],
	["confirm_dialog", [0, 1, 2, 3]],
	["button", [2]],
	["tooltip", [1]],
	["set_title", [0]],
	["set_window_title", [0]],
	["set_description", [0]],
	["add_filter", [0]],
	["info_dialog", [0]],
	["toast_loading", [0]],
	["toast_success", [0]],
	["toast_error", [0]],
	["set_error", [0]],
	["box_for", [0]],
	["FeedbackStatus::Error", [0]],
	["show_upload_failure", [2]],
]);
const displayFunctions = new Set([
	"screenshot_outcome",
	"picker_outcome",
	"shortcut_effect",
	"shortcut_effect_hint",
	"trigger_phrase",
	"trigger_noun",
	"action_short",
	"action_noun",
	"rule_summary",
	"rule_name",
	"default_rule_name",
	"field_placeholder",
	"account_name",
	"recordings_empty_title",
	"recordings_empty_message",
	"instant_link_button",
	"format_storage_usage",
	"format_usage",
	"aspect_ratio_options",
	"rule_display_name",
	"auto_rule_name",
	"zoom_button",
	"history_button",
	"camera3d_blur_sliders",
	"button_content",
]);
const displayCatalogues = new Set([
	"MAX_FPS_OPTIONS",
	"COUNTDOWN_OPTIONS",
	"STUDIO_QUALITY_TIERS",
	"INSTANT_RESOLUTION_TIERS",
	"UPDATE_CHANNEL_DESCRIPTIONS",
	"TEXT_SEGMENT_WEIGHTS",
	"TEXT_ANIMATIONS",
	"TEXT_BACKGROUND_STYLES",
	"TEXT_LAYOUTS",
	"TEXT_PRESET_GROUPS",
	"TEXT_PRESETS",
	"GENERIC_FONTS",
	"SCENE_MODES",
	"MOTION_EASINGS",
	"CAMERA3D_FOCUS_MODES",
	"CAMERA3D_DEFAULT_LOOK_NAME",
	"BACKGROUND_THEMES",
	"CAMERA_SHAPES",
	"CAMERA_BLUR_MODES",
	"CORNER_STYLES",
	"STEREO_MODES",
	"CURSOR_STYLES",
	"TEXT_WEIGHTS",
	"KEYBOARD_POSITIONS",
	"CAPTION_POSITIONS",
	"CAPTION_ANIMATIONS",
	"CAPTION_HIGHLIGHT_STYLES",
	"CAPTION_LANGUAGES",
	"S3_FIELDS",
	"S3_PROVIDERS",
	"NEEDS_MODIFIER",
	"UNSUPPORTED_KEY",
	"RECORDING_HINT",
	"BLOCKED_WHILE_RECORDING",
	"UNAVAILABLE",
	"SWITCH_SENTENCES",
	"CAPTURE_TARGETS",
	"RECORDING_MODES",
	"CLIPBOARD_SOURCES",
	"COMPRESSIONS",
	"TEMPLATES",
	"ANGLE_PRESETS",
	"MOTION_TEMPLATES",
	"CAMERA3D_SCENES",
	"MODES",
	"FAQ",
	"FRAME_STYLES",
]);
const nonDisplayCalls = new Set([
	"id",
	"group",
	"key_context",
	"font_family",
	"font",
	"path",
	"get",
	"get_mut",
	"insert",
	"remove",
	"contains_key",
	"contains",
	"entry",
	"set_var",
	"var",
	"var_os",
	"set_store_setting",
	"json",
	"info",
	"warn",
	"error",
	"debug",
	"trace",
	"expect",
	"context",
	"with_context",
	"assert_eq",
	"assert_ne",
	"keys",
	"key",
	"kbd",
	"key_bindings",
	"key_binding",
	"keystroke",
	"shortcut",
]);

export function tokenize(source) {
	const tokens = [];
	let i = 0;
	while (i < source.length) {
		const start = i;
		if (/\s/.test(source[i])) {
			i++;
		} else if (source.startsWith("//", i)) {
			i = source.indexOf("\n", i);
			if (i < 0) break;
		} else if (source.startsWith("/*", i)) {
			i += 2;
			let depth = 1;
			while (i < source.length && depth) {
				if (source.startsWith("/*", i)) {
					depth++;
					i += 2;
				} else if (source.startsWith("*/", i)) {
					depth--;
					i += 2;
				} else i++;
			}
		} else if (/^r#*"/.test(source.slice(i, i + 12))) {
			const opening = source.slice(i).match(/^r(#*)"/)[0];
			const closing = `"${opening.slice(1, -1)}`;
			const end = source.indexOf(closing, i + opening.length);
			i = end < 0 ? source.length : end + closing.length;
			tokens.push({ kind: "raw", value: "", start, end: i });
		} else if (source[i] === '"') {
			i++;
			while (i < source.length) {
				if (source[i] === "\\") i += 2;
				else if (source[i++] === '"') break;
			}
			const literal = source.slice(start, i);
			let value;
			try {
				value = JSON.parse(literal.replace(/\\\r?\n\s*/g, ""));
			} catch {
				value = undefined;
			}
			tokens.push({ kind: "string", value, start, end: i });
		} else if (/^'(?:\\.|[^'\\\r\n])'/.test(source.slice(i, i + 8))) {
			i += source.slice(i).match(/^'(?:\\.|[^'\\\r\n])'/)[0].length;
		} else if (/[A-Za-z_]/.test(source[i])) {
			while (i < source.length && /[A-Za-z0-9_]/.test(source[i])) i++;
			tokens.push({
				kind: "identifier",
				value: source.slice(start, i),
				start,
				end: i,
			});
		} else {
			tokens.push({ kind: "punctuation", value: source[i], start, end: ++i });
		}
	}
	return tokens;
}

export function localize(source, file, translations = dictionary) {
	const tokens = tokenize(source);
	const stack = [];
	const edits = [];
	const candidates = [];
	let pendingFunction;
	let catalogue;
	for (let i = 0; i < tokens.length; i++) {
		const token = tokens[i];
		if (
			["const", "static"].includes(token.value) &&
			displayCatalogues.has(tokens[i + 1]?.value)
		) {
			catalogue = { depth: stack.length, name: tokens[i + 1].value };
		}
		if (token.value === ";" && catalogue?.depth === stack.length) {
			catalogue = undefined;
		}
		if (token.kind === "identifier" && token.value === "fn") {
			pendingFunction = tokens[i + 1]?.value;
		}
		if (token.kind === "punctuation" && ["(", "[", "{"].includes(token.value)) {
			let name = "";
			if (token.value === "(") {
				let j = tokens[i - 1]?.value === "!" ? i - 2 : i - 1;
				name = tokens[j]?.value ?? "";
				while (j >= 3 && tokens[j - 1]?.value === ":") {
					name = `${tokens[j - 3].value}::${name}`;
					j -= 3;
				}
			}
			stack.push({
				open: token.value,
				name,
				argument: 0,
				functionName: token.value === "{" ? pendingFunction : undefined,
			});
			if (token.value === "{") pendingFunction = undefined;
		} else if (
			token.kind === "punctuation" &&
			[")", "]", "}"].includes(token.value)
		) {
			stack.pop();
		} else if (token.kind === "punctuation" && token.value === ",") {
			const frame = stack.at(-1);
			if (frame?.open === "(") frame.argument++;
		} else if (token.kind === "string" && token.value !== undefined) {
			const inLabel = stack.some(
				(frame) =>
					frame.functionName?.endsWith("_label") ||
					displayFunctions.has(frame.functionName) ||
					[
						"label",
						"panel_title",
						"panel_description",
						"none_label",
						"empty_label",
						"empty_message",
						"tooltip",
						"title",
						"description",
						"empty_title",
						"placeholder",
						"button_label",
						"start_label",
					].includes(frame.functionName),
			);
			const call = stack.at(-1);
			const parent = stack.at(-2);
			const isDisplayCall = (frame) =>
				frame?.open === "(" &&
				[...displayArguments].some(
					([name, argumentsList]) =>
						frame.name.endsWith(name) && argumentsList.includes(frame.argument),
				);
			const isOptionalDescription =
				call?.name === "Some" && isDisplayCall(parent);
			const nearestCall = stack
				.toReversed()
				.find(
					(frame) =>
						frame.open === "(" && !["Some", "format"].includes(frame.name),
				);
			const isDisplayExpression = isDisplayCall(nearestCall);
			const inRender = stack.some(
				(frame) =>
					frame.functionName === "render" ||
					frame.functionName?.startsWith("render_") ||
					[
						"export_cta",
						"menu_items",
						"pages_menu_items",
						"panel_menu_items",
						"aspect_menu_items",
					].includes(frame.functionName),
			);
			const excludedCall = nonDisplayCalls.has(
				nearestCall?.name.split("::").at(-1),
			);
			const isMatchPattern =
				tokens[i + 1]?.value === "|" ||
				(tokens[i + 1]?.value === "=" && tokens[i + 2]?.value === ">");
			const isDisplayField =
				tokens[i - 1]?.value === ":" &&
				tokens[i - 2]?.kind === "identifier" &&
				["label", "hint", "title", "tagline", "description"].includes(
					tokens[i - 2].value,
				);
			const isDisplayError =
				/\.(?:error|last_error|failure)\s*=\s*Some\(\s*(?:format!\(\s*)?$/.test(
					source.slice(Math.max(0, token.start - 160), token.start),
				);
			const isSettingLabel =
				file === "store.rs" &&
				tokens[i - 1]?.value === "," &&
				tokens[i - 2]?.kind === "string" &&
				tokens[i - 3]?.value === "=";
			const isWindowIdentity =
				file === "app_windows.rs" &&
				stack.some((frame) => frame.functionName === "title");
			if (
				!excludedCall &&
				!isMatchPattern &&
				!isWindowIdentity &&
				(inLabel ||
					inRender ||
					catalogue ||
					isDisplayCall(call) ||
					isDisplayExpression ||
					isDisplayField ||
					isDisplayError ||
					isOptionalDescription ||
					isSettingLabel)
			) {
				candidates.push(token.value);
				if (Object.hasOwn(translations, token.value)) {
					let translated = translations[token.value];
					if (
						["TEXT_SEGMENT_WEIGHTS", "TEXT_WEIGHTS"].includes(catalogue?.name)
					) {
						translated =
							{
								Light: "细体",
								Regular: "常规",
								Medium: "中等",
								Semibold: "半粗",
								Bold: "粗体",
								"Extra Bold": "超粗",
								Black: "极粗",
								Normal: "常规",
							}[token.value] ?? translated;
					}
					edits.push({
						start: token.start,
						end: token.end,
						text: JSON.stringify(translated),
					});
				}
			}
		}
	}
	let output = source;
	for (const edit of edits.toReversed()) {
		output = output.slice(0, edit.start) + edit.text + output.slice(edit.end);
	}
	return { output, count: edits.length, candidates };
}

if (
	process.argv[1] &&
	path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
	const missing = new Set();
	let total = 0;
	for (const file of files) {
		const filename = path.join(sourceRoot, file);
		if (!fs.existsSync(filename)) continue;
		const source = fs.readFileSync(filename, "utf8");
		const result = localize(source, file);
		if (process.argv.includes("--list")) {
			for (const candidate of result.candidates) {
				if (
					/[A-Za-z]/.test(candidate) &&
					!/[\u3400-\u9fff]/.test(candidate) &&
					!Object.hasOwn(dictionary, candidate)
				) {
					missing.add(candidate);
				}
			}
		}
		if (process.argv.includes("--write") && result.count) {
			fs.writeFileSync(filename, result.output, "utf8");
		}
		total += result.count;
		console.log(`${file}: ${result.count}`);
	}
	if (missing.size)
		console.log(JSON.stringify([...missing].sort(), null, "\t"));
	console.log(`Translated UI literals: ${total}`);
}
