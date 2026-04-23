use serde::Serialize;

#[derive(Debug, Clone)]
pub struct ThemeDefinition {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub mode: String,
    pub canvas: String,
    pub surface: String,
    pub surface_strong: String,
    pub line: String,
    pub text: String,
    pub muted: String,
    pub accent: String,
    pub accent_soft: String,
    pub preview: [String; 3],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeMeta {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub mode: String,
    pub canvas: String,
    pub surface: String,
    pub surface_strong: String,
    pub line: String,
    pub text: String,
    pub muted: String,
    pub accent: String,
    pub accent_soft: String,
    pub preview: [String; 3],
}

#[derive(Debug, Clone)]
pub struct CustomThemeInput {
    pub name: String,
    pub accent: String,
}

#[derive(Debug, Clone, Copy)]
struct ThemePreset {
    id: &'static str,
    name: &'static str,
    summary: &'static str,
    accent: &'static str,
    dark_accent: &'static str,
}

const PRESETS: [ThemePreset; 5] = [
    ThemePreset {
        id: "default",
        name: "Default",
        summary: "Restrained green accent for daily desktop work.",
        accent: "#10a37f",
        dark_accent: "#19c99b",
    },
    ThemePreset {
        id: "light",
        name: "Mist",
        summary: "Soft blue accent for quiet operational views.",
        accent: "#3b82f6",
        dark_accent: "#74a8ff",
    },
    ThemePreset {
        id: "dark",
        name: "Indigo",
        summary: "Muted indigo accent for structured focus.",
        accent: "#6366f1",
        dark_accent: "#9ea2ff",
    },
    ThemePreset {
        id: "graphite",
        name: "Graphite",
        summary: "Low-saturation slate accent for long sessions.",
        accent: "#64748b",
        dark_accent: "#a8b3c5",
    },
    ThemePreset {
        id: "crimson",
        name: "Rose",
        summary: "Warm rose accent for editorial workspace depth.",
        accent: "#c05a6f",
        dark_accent: "#e0a2ae",
    },
];

pub fn normalize_mode(mode: &str) -> &'static str {
    match mode {
        "light" => "light",
        "dark" => "dark",
        "system" => "system",
        _ => "system",
    }
}

pub fn meta_catalog(mode: &str, custom: &CustomThemeInput) -> Vec<ThemeMeta> {
    PRESETS
        .iter()
        .map(|preset| materialize_preset(*preset, mode).into())
        .chain(std::iter::once(materialize_custom(custom, mode).into()))
        .collect()
}

pub fn resolve_theme(id: &str, mode: &str, custom: &CustomThemeInput) -> ThemeDefinition {
    PRESETS
        .iter()
        .find(|theme| theme.id == id)
        .map(|preset| materialize_preset(*preset, mode))
        .unwrap_or_else(|| {
            if id == "custom" {
                materialize_custom(custom, mode)
            } else {
                materialize_preset(PRESETS[0], mode)
            }
        })
}

fn materialize_preset(preset: ThemePreset, mode: &str) -> ThemeDefinition {
    materialize_theme(
        preset.id,
        preset.name,
        preset.summary,
        preset.accent,
        preset.dark_accent,
        mode,
    )
}

fn materialize_custom(custom: &CustomThemeInput, mode: &str) -> ThemeDefinition {
    let name = if custom.name.trim().is_empty() {
        "Custom"
    } else {
        custom.name.trim()
    };
    let accent = sanitize_hex(&custom.accent).unwrap_or_else(|| "#10a37f".to_string());
    materialize_theme(
        "custom",
        name,
        "User-defined accent theme.",
        &accent,
        &accent,
        mode,
    )
}

fn materialize_theme(
    id: &str,
    name: &str,
    summary: &str,
    light_accent: &str,
    dark_accent: &str,
    mode: &str,
) -> ThemeDefinition {
    let normalized_mode = normalize_mode(mode);
    let dark = normalized_mode == "dark";
    let accent = if dark { dark_accent } else { light_accent };
    let accent_soft = if dark {
        format!("color-mix(in srgb, {accent} 22%, #1f1f1c)")
    } else {
        format!("color-mix(in srgb, {accent} 12%, #ffffff)")
    };

    ThemeDefinition {
        id: id.to_string(),
        name: name.to_string(),
        summary: summary.to_string(),
        mode: normalized_mode.to_string(),
        canvas: if dark { "#1f1f1c" } else { "#f7f7f5" }.to_string(),
        surface: if dark { "#252623" } else { "#ffffff" }.to_string(),
        surface_strong: if dark { "#2f302c" } else { "#f3f4f1" }.to_string(),
        line: if dark { "#3e413a" } else { "#e2e4de" }.to_string(),
        text: if dark { "#f4f4ef" } else { "#1f1f1c" }.to_string(),
        muted: if dark { "#b7bbae" } else { "#6b6f67" }.to_string(),
        accent: accent.to_string(),
        accent_soft,
        preview: [
            if dark { "#1f1f1c" } else { "#f7f7f5" }.to_string(),
            if dark { "#2f302c" } else { "#f3f4f1" }.to_string(),
            accent.to_string(),
        ],
    }
}

pub fn sanitize_hex(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    let valid_length = hex.len() == 6;
    let valid_digits = hex.chars().all(|ch| ch.is_ascii_hexdigit());

    if valid_length && valid_digits {
        Some(format!("#{}", hex.to_ascii_lowercase()))
    } else {
        None
    }
}

pub fn injected_script(theme: ThemeDefinition) -> String {
    let css_payload = serde_json::to_string(&remote_css(&theme)).unwrap_or_else(|_| "\"\"".into());
    let theme_id = serde_json::to_string(&theme.id).unwrap_or_else(|_| "\"default\"".into());
    let mode = serde_json::to_string(&theme.mode).unwrap_or_else(|_| "\"system\"".into());
    let color_scheme =
        serde_json::to_string(color_scheme(&theme)).unwrap_or_else(|_| "\"light dark\"".into());

    format!(
        r#"
(() => {{
  const styleId = "slock-desktop-theme";
  const themeId = {theme_id};
  const mode = {mode};
  const colorScheme = {color_scheme};
  const css = {css_payload};

  const apply = () => {{
    document.documentElement.dataset.slockDesktopTheme = themeId;
    document.documentElement.dataset.slockDesktopMode = mode;
    document.documentElement.style.colorScheme = colorScheme;

    let style = document.getElementById(styleId);
    if (!style) {{
      style = document.createElement("style");
      style.id = styleId;
      document.head.appendChild(style);
    }}

    style.textContent = css;

    let themeMeta = document.querySelector('meta[name="theme-color"]');
    if (!themeMeta) {{
      themeMeta = document.createElement("meta");
      themeMeta.setAttribute("name", "theme-color");
      document.head.appendChild(themeMeta);
    }}
    themeMeta.setAttribute("content", {accent});
  }};

  if (document.readyState === "loading") {{
    document.addEventListener("DOMContentLoaded", apply, {{ once: true }});
  }}

  apply();
}})();
"#,
        accent = serde_json::to_string(&theme.accent).unwrap_or_else(|_| "\"#10a37f\"".into())
    )
}

fn color_scheme(theme: &ThemeDefinition) -> &'static str {
    if theme.mode == "system" {
        "light dark"
    } else if theme.mode == "dark" {
        "dark"
    } else {
        "light"
    }
}

fn remote_css(theme: &ThemeDefinition) -> String {
    format!(
        r#"
:root,
:host {{
  color-scheme: {mode};
  --slock-desktop-canvas: {canvas};
  --slock-desktop-app-bg: {canvas};
  --slock-desktop-toolbar-bg: {surface_strong};
  --slock-desktop-sidebar-bg: {surface_strong};
  --slock-desktop-panel-bg: color-mix(in srgb, {surface_strong} 72%, {canvas});
  --slock-desktop-surface: {surface};
  --slock-desktop-surface-strong: {surface_strong};
  --slock-desktop-surface-secondary: {surface_strong};
  --slock-desktop-surface-tertiary: color-mix(in srgb, {surface_strong} 72%, {canvas});
  --slock-desktop-line: {line};
  --slock-desktop-line-strong: color-mix(in srgb, {line} 82%, {text});
  --slock-desktop-text: {text};
  --slock-desktop-muted: {muted};
  --slock-desktop-text-tertiary: color-mix(in srgb, {muted} 72%, {surface});
  --slock-desktop-accent: {accent};
  --slock-desktop-accent-soft: {accent_soft};
  --slock-desktop-accent-hover: color-mix(in srgb, {accent} 88%, {text});
  --slock-desktop-accent-active: color-mix(in srgb, {accent} 72%, {text});
  --slock-desktop-selection: {accent_soft};
  --slock-desktop-hover: color-mix(in srgb, {text} 4%, transparent);
  --slock-desktop-active: color-mix(in srgb, {text} 8%, transparent);
  --slock-desktop-focus-ring: color-mix(in srgb, {accent} 28%, transparent);
  --slock-desktop-shadow: 0 8px 24px rgba(0, 0, 0, 0.08);
  --slock-desktop-soft-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
  --slock-desktop-radius-xs: 8px;
  --slock-desktop-radius-sm: 10px;
  --slock-desktop-radius-md: 12px;
  --slock-desktop-radius-lg: 16px;
  --slock-desktop-radius-xl: 20px;
  --slock-desktop-radius-pill: 999px;
  --slock-desktop-readable-width: 840px;
  --font-display: Inter, "SF Pro Display", "PingFang SC", system-ui, sans-serif;
  --default-font-family: Inter, "SF Pro Display", "PingFang SC", system-ui, sans-serif;
  --default-mono-font-family: "JetBrains Mono", "SF Mono", ui-monospace, monospace;
  --color-white: {surface};
  --color-black: {text};
  --color-gray-400: {muted};
  --color-brutal-cream: {canvas};
  --color-brutal-yellow: {surface_strong};
  --color-brutal-pink: {accent_soft};
  --color-brutal-cyan: {accent_soft};
  --color-brutal-lime: {surface};
  --color-brutal-lavender: {line};
  --color-brutal-orange: {accent_soft};
}}

@media (prefers-color-scheme: dark) {{
  html[data-slock-desktop-mode="system"] {{
    --slock-desktop-canvas: #1f1f1c;
    --slock-desktop-app-bg: #1f1f1c;
    --slock-desktop-toolbar-bg: #2f302c;
    --slock-desktop-sidebar-bg: #2f302c;
    --slock-desktop-panel-bg: #282925;
    --slock-desktop-surface: #252623;
    --slock-desktop-surface-strong: #2f302c;
    --slock-desktop-surface-secondary: #2f302c;
    --slock-desktop-surface-tertiary: #383a34;
    --slock-desktop-line: #3e413a;
    --slock-desktop-line-strong: #51554b;
    --slock-desktop-text: #f4f4ef;
    --slock-desktop-muted: #b7bbae;
    --slock-desktop-text-tertiary: #8f9488;
    --slock-desktop-accent-soft: color-mix(in srgb, {accent} 22%, #1f1f1c);
    --slock-desktop-selection: color-mix(in srgb, {accent} 22%, #1f1f1c);
    --slock-desktop-hover: rgba(244, 244, 239, 0.06);
    --slock-desktop-active: rgba(244, 244, 239, 0.1);
  }}
}}

html,
body,
#root {{
  background: var(--slock-desktop-canvas) !important;
  color: var(--slock-desktop-text) !important;
}}

body {{
  accent-color: var(--slock-desktop-accent) !important;
  font-family: Inter, "SF Pro Display", "PingFang SC", system-ui, sans-serif !important;
  font-size: 14px !important;
  letter-spacing: -0.01em !important;
  text-rendering: optimizeLegibility !important;
}}

#root {{
  background-image: none !important;
}}

::selection {{
  background: var(--slock-desktop-accent-soft) !important;
  color: var(--slock-desktop-text) !important;
}}

a,
[role="link"] {{
  color: var(--slock-desktop-accent) !important;
}}

main,
section,
article,
aside,
nav,
header,
footer,
[role="dialog"],
[role="menu"],
[role="menuitem"],
[role="tabpanel"],
[role="listitem"] {{
  color: var(--slock-desktop-text) !important;
  border-color: var(--slock-desktop-line) !important;
}}

input,
textarea,
select,
button,
[role="button"] {{
  color: var(--slock-desktop-text) !important;
  border-color: var(--slock-desktop-line) !important;
}}

input,
textarea,
select {{
  background: var(--slock-desktop-surface) !important;
}}

[class*="sidebar"],
[class*="Sidebar"],
[class*="panel"],
[class*="Panel"],
[class*="thread"],
[class*="Thread"],
[class*="card"],
[class*="Card"],
[class*="composer"],
[class*="Composer"],
[class*="message"],
[class*="Message"],
.card-brutal,
[data-radix-popper-content-wrapper],
[data-state="open"],
[data-slot="popover-content"] {{
  color: var(--slock-desktop-text) !important;
  border-color: var(--slock-desktop-line) !important;
}}

[role="dialog"],
[role="menu"],
[class*="sidebar"],
[class*="Sidebar"],
[class*="panel"],
[class*="Panel"],
[class*="card"],
[class*="Card"],
[class*="composer"],
[class*="Composer"],
.card-brutal,
[data-radix-popper-content-wrapper],
[data-state="open"],
[data-slot="popover-content"] {{
  background: var(--slock-desktop-surface) !important;
  border-radius: var(--slock-desktop-radius-lg) !important;
  box-shadow: none !important;
}}

[class*="message"],
[class*="Message"],
[class*="thread"],
[class*="Thread"] {{
  border-radius: var(--slock-desktop-radius-lg) !important;
}}

[class*="message"],
[class*="Message"] {{
  background: transparent !important;
}}

.font-display,
.font-mono,
code,
pre {{
  font-family: inherit !important;
}}

.font-mono,
code,
pre {{
  font-family: "SFMono-Regular", "SF Mono", ui-monospace, monospace !important;
}}

.tilt-neg-2 {{
  transform: none !important;
}}

[class*="shadow-brutal"],
[class*="shadow-\["],
.shadow-brutal,
.shadow-brutal-sm,
.shadow-\[4px_4px_0_\#000\],
.hover\:shadow-brutal:hover,
.hover\:shadow-brutal-sm:hover,
.hover\:shadow-brutal-lg:hover,
.focus\:shadow-brutal:focus,
.focus\:shadow-brutal-sm:focus,
.focus-within\:shadow-brutal:focus-within,
.active\:shadow-brutal-active:active,
.active\:shadow-brutal-sm:active {{
  box-shadow: var(--slock-desktop-soft-shadow) !important;
}}

[class*="border-black"],
[class*="border-brutal"],
[class*="hover\:border-black"],
[class*="hover\:border-brutal"] {{
  border-color: var(--slock-desktop-line) !important;
}}

[class*="border-2"],
[class*="border-b-2"],
[class*="border-t-2"],
[class*="border-l-3"],
[class*="border-l-4"] {{
  border-width: 1px !important;
}}

.card-brutal,
.input-brutal,
.btn-brutal,
.btn-brutal-sm,
[class*="border-2"],
[class*="border-b-2"],
[class*="border-t-2"],
[class*="border-l-4"],
.md\:border-l-3 {{
  border-width: 1px !important;
  border-color: var(--slock-desktop-line) !important;
}}

.card-brutal,
[role="dialog"],
[role="menu"],
[data-radix-popper-content-wrapper],
[data-slot="popover-content"] {{
  border-radius: var(--slock-desktop-radius-lg) !important;
  background: var(--slock-desktop-surface) !important;
  box-shadow: var(--slock-desktop-shadow) !important;
}}

.input-brutal,
input,
textarea,
select,
[contenteditable="true"] {{
  border-radius: var(--slock-desktop-radius-lg) !important;
  background: var(--slock-desktop-surface-secondary) !important;
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--slock-desktop-line) 55%, transparent) !important;
}}

.input-brutal:focus,
input:focus,
textarea:focus,
select:focus,
[contenteditable="true"]:focus {{
  box-shadow:
    inset 0 0 0 1px color-mix(in srgb, var(--slock-desktop-accent) 40%, var(--slock-desktop-line)),
    0 0 0 3px color-mix(in srgb, var(--slock-desktop-accent) 12%, transparent) !important;
}}

.btn-brutal,
.btn-brutal-sm,
button,
[role="button"] {{
  border-radius: var(--slock-desktop-radius-md) !important;
  transform: none !important;
  transition:
    background 150ms ease,
    border-color 150ms ease,
    box-shadow 150ms ease,
    transform 150ms ease !important;
}}

.btn-brutal,
.btn-brutal-sm {{
  background: var(--slock-desktop-surface) !important;
  color: var(--slock-desktop-text) !important;
  box-shadow: var(--slock-desktop-soft-shadow) !important;
}}

.btn-brutal.bg-brutal-pink,
.btn-brutal.bg-brutal-lime,
.btn-brutal.bg-brutal-cyan,
.btn-brutal.bg-brutal-yellow,
.btn-brutal.bg-brutal-orange,
.btn-brutal-sm.bg-brutal-pink,
.btn-brutal-sm.bg-brutal-lime,
.btn-brutal-sm.bg-brutal-cyan,
.btn-brutal-sm.bg-brutal-yellow,
.btn-brutal-sm.bg-brutal-orange {{
  background: var(--slock-desktop-accent) !important;
  color: var(--slock-desktop-surface) !important;
  border-color: transparent !important;
}}

.btn-brutal:hover,
.btn-brutal-sm:hover,
button:hover,
[role="button"]:hover {{
  background: var(--slock-desktop-hover) !important;
  transform: none !important;
}}

.btn-brutal:active,
.btn-brutal-sm:active,
button:active,
[role="button"]:active {{
  transform: scale(0.97) !important;
}}

[class*="bg-brutal-yellow"],
[class*="bg-brutal-orange"],
[class*="hover\:bg-brutal-yellow"],
[class*="hover\:bg-brutal-orange"] {{
  background-color: var(--slock-desktop-surface-secondary) !important;
}}

[class*="bg-brutal-pink"],
[class*="bg-brutal-cyan"],
[class*="bg-brutal-lavender"],
[class*="hover\:bg-brutal-pink"],
[class*="hover\:bg-brutal-cyan"],
[class*="hover\:bg-brutal-lavender"] {{
  background-color: var(--slock-desktop-surface-tertiary) !important;
}}

[class*="bg-brutal-lime"],
[class*="hover\:bg-brutal-lime"] {{
  background-color: var(--slock-desktop-selection) !important;
}}

[class*="text-brutal-yellow"],
[class*="text-brutal-pink"],
[class*="text-brutal-cyan"],
[class*="text-brutal-lime"],
[class*="text-brutal-lavender"],
[class*="text-brutal-orange"],
[class*="hover\:text-brutal"] {{
  color: var(--slock-desktop-accent) !important;
}}

.bg-white,
.bg-white\/70,
.bg-white\/80 {{
  background-color: var(--slock-desktop-surface) !important;
}}

.bg-brutal-cream,
.bg-brutal-cream\/50,
.bg-brutal-cream\/60 {{
  background-color: var(--slock-desktop-canvas) !important;
}}

.bg-brutal-yellow,
.bg-brutal-yellow\/30,
.bg-brutal-yellow\/40,
.bg-brutal-yellow\/60,
.bg-brutal-orange,
.bg-brutal-orange\/20,
.bg-brutal-orange\/30 {{
  background-color: var(--slock-desktop-surface-secondary) !important;
}}

.bg-brutal-pink,
.bg-brutal-pink\/30,
.bg-brutal-cyan,
.bg-brutal-cyan\/30,
.bg-brutal-cyan\/40,
.bg-brutal-lavender,
.bg-brutal-lavender\/40 {{
  background-color: var(--slock-desktop-surface-tertiary) !important;
}}

.bg-brutal-lime,
.bg-brutal-lime\/20,
.bg-brutal-lime\/30 {{
  background-color: var(--slock-desktop-selection) !important;
}}

.hover\:bg-brutal-yellow:hover,
.hover\:bg-brutal-yellow\/30:hover,
.hover\:bg-brutal-yellow\/50:hover,
.hover\:bg-brutal-pink:hover,
.hover\:bg-brutal-pink\/60:hover,
.hover\:bg-brutal-cyan\/40:hover,
.hover\:bg-brutal-cyan\/60:hover,
.hover\:bg-brutal-lavender:hover,
.hover\:bg-brutal-orange:hover,
.hover\:bg-brutal-cream:hover,
.hover\:bg-brutal-cream\/40:hover,
.hover\:bg-white:hover,
.hover\:bg-white\/50:hover,
.hover\:bg-black\/5:hover,
.hover\:bg-black\/\[0\.03\]:hover {{
  background-color: var(--slock-desktop-hover) !important;
}}

.safe-top.safe-left.safe-right,
.flex.h-\[62px\],
.flex.h-\[62px\].shrink-0,
.relative.flex.items-center,
.flex.overflow-x-auto,
.shrink-0.border-b-2,
.md\:hidden.shrink-0,
[class*="border-b-2"].bg-white,
[class*="border-t-2"].bg-white {{
  background: var(--slock-desktop-surface) !important;
  border-color: var(--slock-desktop-line) !important;
  box-shadow: var(--slock-desktop-soft-shadow) !important;
}}

.relative.flex-1.overflow-hidden,
.min-h-0.flex-1.overflow-y-auto,
.flex-1.overflow-y-auto,
.absolute.inset-0.z-30,
.flex.min-h-0.flex-1.flex-col {{
  background: var(--slock-desktop-canvas) !important;
}}

nav,
aside,
[class*="sidebar"],
[class*="Sidebar"],
.absolute.inset-0.z-30 {{
  background: var(--slock-desktop-surface-strong) !important;
}}

nav button,
aside button,
.group.flex.items-center,
[class*="channel"],
[class*="Channel"],
[class*="thread"],
[class*="Thread"] {{
  border-radius: 10px !important;
}}

nav button:hover,
aside button:hover,
.group.flex.items-center:hover,
[class*="channel"]:hover,
[class*="Channel"]:hover,
[class*="thread"]:hover,
[class*="Thread"]:hover {{
  background: var(--slock-desktop-hover) !important;
}}

[id^="message-"],
[class*="message"],
[class*="Message"],
[class*="max-w-\[70\%\]"],
.max-w-\[70\%\] {{
  background: transparent !important;
  border-color: transparent !important;
  box-shadow: none !important;
}}

[id^="message-"] > *,
[class*="message"] > *,
[class*="Message"] > *,
.max-w-\[70\%\] {{
  border-radius: var(--slock-desktop-radius-lg) !important;
}}

.whitespace-pre-wrap,
[id^="message-"] .whitespace-pre-wrap,
[class*="message"] .whitespace-pre-wrap,
[class*="Message"] .whitespace-pre-wrap {{
  color: var(--slock-desktop-text) !important;
  line-height: 1.58 !important;
}}

.relative.flex.items-center.border-t-2,
.flex.items-center.border-t-2,
.border-t-2.bg-white,
[class*="composer"],
[class*="Composer"] {{
  background: var(--slock-desktop-surface) !important;
  border-color: var(--slock-desktop-line) !important;
  border-radius: var(--slock-desktop-radius-lg) var(--slock-desktop-radius-lg) 0 0 !important;
  box-shadow: var(--slock-desktop-soft-shadow) !important;
}}

.relative.flex.items-center.border-t-2 textarea,
.flex.items-center.border-t-2 textarea,
[class*="composer"] textarea,
[class*="Composer"] textarea {{
  min-height: 44px !important;
  border-radius: var(--slock-desktop-radius-lg) !important;
  background: var(--slock-desktop-surface-secondary) !important;
}}

.text-black,
.text-black\/80,
.text-black\/70,
.text-black\/60,
.text-black\/50,
.text-black\/40,
.text-black\/35,
.text-black\/30,
.text-black\/25,
.text-black\/20 {{
  color: var(--slock-desktop-text) !important;
}}

.text-black\/60,
.text-black\/50,
.text-black\/40,
.text-black\/35,
.text-black\/30,
.text-black\/25,
.text-black\/20,
.text-gray-400 {{
  color: var(--slock-desktop-muted) !important;
}}

.input-brutal {{
  background: var(--slock-desktop-surface) !important;
  color: var(--slock-desktop-text) !important;
  border-color: var(--slock-desktop-line) !important;
}}

.btn-brutal,
.btn-brutal-sm {{
  color: var(--slock-desktop-text) !important;
  border-color: var(--slock-desktop-line) !important;
}}

.bg-white {{
  background-color: var(--slock-desktop-surface) !important;
}}

.bg-brutal-cream {{
  background-color: var(--slock-desktop-canvas) !important;
}}

.text-black {{
  color: var(--slock-desktop-text) !important;
}}

.text-gray-400 {{
  color: var(--slock-desktop-muted) !important;
}}

.text-brutal-yellow,
.text-brutal-orange,
.text-brutal-pink,
.text-brutal-lime {{
  color: var(--slock-desktop-accent) !important;
}}

h1,
h2,
h3,
h4,
h5,
h6 {{
  color: var(--slock-desktop-text) !important;
  letter-spacing: -0.02em !important;
  text-wrap: balance !important;
}}

p,
li,
dd,
span {{
  text-wrap: pretty;
}}

.rounded,
.rounded-none,
.rounded-r,
.card-brutal,
.input-brutal,
.btn-brutal,
.btn-brutal-sm,
[role="dialog"],
[role="menu"],
[data-slot="popover-content"],
[data-radix-popper-content-wrapper] {{
  border-radius: var(--slock-desktop-radius-md) !important;
}}

.rounded-full,
[class*="rounded-full"] {{
  border-radius: var(--slock-desktop-radius-pill) !important;
}}

.w-full.border-2,
.max-w-sm.border-2,
.max-w-md.border-2,
.max-w-lg.border-2,
.overflow-hidden.border-2,
.max-h-72.overflow-y-auto,
.max-h-48.overflow-y-auto,
.fixed.z-50.card-brutal,
.absolute.top-full,
.absolute.bottom-full,
.absolute.left-0.top-\[calc\(100\%\+8px\)\] {{
  background: var(--slock-desktop-surface) !important;
  border-color: var(--slock-desktop-line) !important;
  border-radius: var(--slock-desktop-radius-lg) !important;
  box-shadow: var(--slock-desktop-shadow) !important;
}}

.flex.min-h-0.flex-1.items-center.justify-center,
.flex.min-h-0.flex-1.overflow-y-auto.bg-brutal-cream,
.flex.min-h-0.flex-1.items-center.justify-center.bg-brutal-cream {{
  background: var(--slock-desktop-canvas) !important;
}}

.flex.min-h-0.flex-1.items-center.justify-center .w-full.border-2,
.flex.min-h-0.flex-1.overflow-y-auto .w-full.border-2 {{
  max-width: 440px !important;
  padding: 28px !important;
}}

.inline-block.tilt-neg-2,
.relative.inline-flex.tilt-neg-2 {{
  border-radius: var(--slock-desktop-radius-md) !important;
  background: var(--slock-desktop-text) !important;
  color: var(--slock-desktop-surface) !important;
  box-shadow: var(--slock-desktop-soft-shadow) !important;
}}

.safe-top.safe-left.safe-right,
.flex.h-\[62px\],
.flex.h-\[62px\].shrink-0,
.relative.flex.items-center.border-b-2,
.flex.items-start.gap-2.border-b-2,
.border-b-2.bg-white,
.border-b-2.bg-\[\#ffeefb\],
.border-t-2.bg-white,
.flex.overflow-x-auto.border-b-2 {{
  background: var(--slock-desktop-surface) !important;
  border-color: var(--slock-desktop-line) !important;
  box-shadow: var(--slock-desktop-soft-shadow) !important;
}}

.flex.w-full.items-center.gap-2,
.group.flex.items-center,
.inline-flex.items-center,
.btn-brutal,
.btn-brutal-sm {{
  min-height: 34px;
}}

.flex.w-full.items-center.gap-2,
.group.flex.items-center {{
  border-radius: var(--slock-desktop-radius-sm) !important;
}}

.flex.w-full.items-center.gap-2:hover,
.group.flex.items-center:hover {{
  background: var(--slock-desktop-hover) !important;
}}

.grid-cols-1,
.sm\:grid-cols-3,
.md\:grid-cols-2 {{
  gap: 12px !important;
}}

.space-y-1 > :not(:last-child),
.space-y-1\.5 > :not(:last-child),
.space-y-2 > :not(:last-child),
.space-y-3 > :not(:last-child),
.space-y-4 > :not(:last-child) {{
  margin-block-end: 8px !important;
}}

.border-collapse,
table {{
  border-color: var(--slock-desktop-line) !important;
  border-radius: var(--slock-desktop-radius-md) !important;
  overflow: hidden !important;
}}

th,
td {{
  border-color: var(--slock-desktop-line) !important;
}}

pre,
code,
.my-2.overflow-x-auto {{
  border-color: var(--slock-desktop-line) !important;
  border-radius: var(--slock-desktop-radius-md) !important;
}}

.my-2.overflow-x-auto {{
  background: color-mix(in srgb, var(--slock-desktop-text) 7%, var(--slock-desktop-surface)) !important;
  color: var(--slock-desktop-text) !important;
}}

.max-w-\[70\%\] {{
  max-width: min(var(--slock-desktop-readable-width), 86%) !important;
}}

.max-w-\[70\%\].border-2,
.max-w-sm.border-2.bg-brutal-cyan,
.mx-auto.mb-3.max-w-md {{
  background: var(--slock-desktop-surface-secondary) !important;
  color: var(--slock-desktop-text) !important;
  border: 1px solid var(--slock-desktop-line) !important;
  border-radius: var(--slock-desktop-radius-lg) !important;
  box-shadow: var(--slock-desktop-soft-shadow) !important;
}}

.h-5.w-5,
.h-7.w-7,
.h-8.w-8,
.h-9.w-9,
.h-12.w-12,
.h-16.w-16 {{
  border-color: var(--slock-desktop-line) !important;
  box-shadow: var(--slock-desktop-soft-shadow) !important;
}}

.ml-auto.shrink-0.rounded,
.rounded.bg-brutal-pink,
.inline-flex.items-center.gap-1.border,
.inline-flex.items-center.gap-1\.5.border,
.inline-flex.items-center.px-1\.5,
.shrink-0.inline-flex.items-center {{
  border-color: var(--slock-desktop-line) !important;
  border-radius: var(--slock-desktop-radius-pill) !important;
  background: var(--slock-desktop-accent-soft) !important;
  color: var(--slock-desktop-text) !important;
}}

.fixed.z-50.card-brutal,
.absolute.z-50.card-brutal,
.absolute.left-0.top-\[calc\(100\%\+8px\)\],
.absolute.top-full,
.absolute.bottom-full {{
  overflow: hidden !important;
}}

.animate-pulse {{
  background: var(--slock-desktop-accent-soft) !important;
  border-color: color-mix(in srgb, var(--slock-desktop-accent) 24%, var(--slock-desktop-line)) !important;
}}

.select-none,
button,
[role="button"] {{
  touch-action: manipulation;
}}

@media (hover: hover) {{
  a:hover,
  [role="link"]:hover {{
    color: color-mix(in srgb, var(--slock-desktop-accent) 82%, var(--slock-desktop-text)) !important;
  }}
}}

@media (max-width: 768px) {{
  .max-w-\[70\%\] {{
    max-width: 92% !important;
  }}

  .safe-top.safe-left.safe-right,
  .relative.flex.items-center.border-t-2,
  .flex.items-center.border-t-2 {{
    border-radius: 0 !important;
  }}
}}

@media (prefers-reduced-motion: reduce) {{
  *,
  *::before,
  *::after {{
    transition-duration: 1ms !important;
    animation-duration: 1ms !important;
    animation-iteration-count: 1 !important;
  }}
}}

.border-black,
.md\:border-black,
.border-brutal-yellow,
.border-brutal-orange,
.border-brutal-pink {{
  border-color: var(--slock-desktop-line) !important;
}}

:focus-visible {{
  outline-color: var(--slock-desktop-accent) !important;
}}

svg {{
  color: inherit !important;
}}

::-webkit-scrollbar {{
  width: 10px;
  height: 10px;
}}

::-webkit-scrollbar-track {{
  background: var(--slock-desktop-canvas) !important;
}}

::-webkit-scrollbar-thumb {{
  background: var(--slock-desktop-line);
  border-radius: 999px;
}}
"#,
        mode = color_scheme(theme),
        canvas = theme.canvas.as_str(),
        surface = theme.surface.as_str(),
        surface_strong = theme.surface_strong.as_str(),
        line = theme.line.as_str(),
        text = theme.text.as_str(),
        muted = theme.muted.as_str(),
        accent = theme.accent.as_str(),
        accent_soft = theme.accent_soft.as_str()
    )
}

impl From<ThemeDefinition> for ThemeMeta {
    fn from(value: ThemeDefinition) -> Self {
        Self {
            id: value.id,
            name: value.name,
            summary: value.summary,
            mode: value.mode,
            canvas: value.canvas,
            surface: value.surface,
            surface_strong: value.surface_strong,
            line: value.line,
            text: value.text,
            muted: value.muted,
            accent: value.accent,
            accent_soft: value.accent_soft,
            preview: value.preview,
        }
    }
}
