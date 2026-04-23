use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub struct ThemeDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
    pub mode: &'static str,
    pub canvas: &'static str,
    pub surface: &'static str,
    pub surface_strong: &'static str,
    pub line: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub accent: &'static str,
    pub accent_soft: &'static str,
    pub preview: [&'static str; 3],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
    pub mode: &'static str,
    pub canvas: &'static str,
    pub surface: &'static str,
    pub surface_strong: &'static str,
    pub line: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub accent: &'static str,
    pub accent_soft: &'static str,
    pub preview: [&'static str; 3],
}

const THEMES: [ThemeDefinition; 5] = [
    ThemeDefinition {
        id: "default",
        name: "Default",
        summary: "Soft neutral workspace with restrained sage accents.",
        mode: "light",
        canvas: "#f4f3ef",
        surface: "#fbfaf7",
        surface_strong: "#eeeee8",
        line: "#d8d6cf",
        text: "#242424",
        muted: "#6f6e68",
        accent: "#6f9d92",
        accent_soft: "#dce8e4",
        preview: ["#f4f3ef", "#eeeee8", "#6f9d92"],
    },
    ThemeDefinition {
        id: "light",
        name: "Light",
        summary: "ChatGPT-inspired light mode with quiet gray-white surfaces.",
        mode: "light",
        canvas: "#f7f7f8",
        surface: "#fdfdfb",
        surface_strong: "#ececf1",
        line: "#d9d9e3",
        text: "#202123",
        muted: "#6e6e80",
        accent: "#6a9f91",
        accent_soft: "#e2eee9",
        preview: ["#f7f7f8", "#ececf1", "#6a9f91"],
    },
    ThemeDefinition {
        id: "dark",
        name: "Dark",
        summary: "VS Code-inspired dark mode with muted blue-gray depth.",
        mode: "dark",
        canvas: "#1e1e1e",
        surface: "#252526",
        surface_strong: "#2d2d30",
        line: "#3c3c3c",
        text: "#d4d4d4",
        muted: "#9da3ae",
        accent: "#75a6d8",
        accent_soft: "#2d4056",
        preview: ["#1e1e1e", "#2d2d30", "#75a6d8"],
    },
    ThemeDefinition {
        id: "graphite",
        name: "Graphite",
        summary: "Low-saturation slate shell for long operational sessions.",
        mode: "dark",
        canvas: "#1b1f24",
        surface: "#22272e",
        surface_strong: "#2b313a",
        line: "#3c444f",
        text: "#d8dee9",
        muted: "#98a2b3",
        accent: "#8fa8c8",
        accent_soft: "#2d3948",
        preview: ["#1b1f24", "#2b313a", "#8fa8c8"],
    },
    ThemeDefinition {
        id: "crimson",
        name: "Rose",
        summary: "Muted rose-gray variant with warm editorial depth.",
        mode: "dark",
        canvas: "#211c1f",
        surface: "#2a2428",
        surface_strong: "#332b30",
        line: "#4a3d43",
        text: "#eee7ea",
        muted: "#b8aab0",
        accent: "#d4a3ad",
        accent_soft: "#443138",
        preview: ["#211c1f", "#332b30", "#d4a3ad"],
    },
];

pub fn catalog() -> &'static [ThemeDefinition] {
    &THEMES
}

pub fn meta_catalog() -> Vec<ThemeMeta> {
    catalog().iter().copied().map(ThemeMeta::from).collect()
}

pub fn resolve_theme(id: &str) -> ThemeDefinition {
    catalog()
        .iter()
        .copied()
        .find(|theme| theme.id == id)
        .unwrap_or(THEMES[0])
}

pub fn injected_script(theme: ThemeDefinition) -> String {
    let css_payload = serde_json::to_string(&remote_css(theme)).unwrap_or_else(|_| "\"\"".into());
    let theme_id = serde_json::to_string(theme.id).unwrap_or_else(|_| "\"default\"".into());
    let mode = serde_json::to_string(theme.mode).unwrap_or_else(|_| "\"light\"".into());

    format!(
        r#"
(() => {{
  const styleId = "slock-desktop-theme";
  const themeId = {theme_id};
  const mode = {mode};
  const css = {css_payload};

  const apply = () => {{
    document.documentElement.dataset.slockDesktopTheme = themeId;
    document.documentElement.style.colorScheme = mode;

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
        accent = serde_json::to_string(theme.accent).unwrap_or_else(|_| "\"#6f9d92\"".into())
    )
}

fn remote_css(theme: ThemeDefinition) -> String {
    format!(
        r#"
:root,
:host {{
  color-scheme: {mode};
  --slock-desktop-canvas: {canvas};
  --slock-desktop-surface: {surface};
  --slock-desktop-surface-strong: {surface_strong};
  --slock-desktop-line: {line};
  --slock-desktop-text: {text};
  --slock-desktop-muted: {muted};
  --slock-desktop-accent: {accent};
  --slock-desktop-accent-soft: {accent_soft};
  --slock-desktop-hover: color-mix(in srgb, {accent_soft} 44%, {surface});
  --slock-desktop-active: color-mix(in srgb, {accent} 12%, {surface});
  --slock-desktop-shadow: 0 14px 46px -38px color-mix(in srgb, {text} 38%, transparent);
  --slock-desktop-soft-shadow: 0 1px 2px color-mix(in srgb, {text} 8%, transparent);
  --font-display: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --default-font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --default-mono-font-family: "SFMono-Regular", "SF Mono", ui-monospace, monospace;
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

html,
body,
#root {{
  background: var(--slock-desktop-canvas) !important;
  color: var(--slock-desktop-text) !important;
}}

body {{
  accent-color: var(--slock-desktop-accent) !important;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif !important;
  letter-spacing: -0.01em !important;
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
  border-radius: 18px !important;
  box-shadow: none !important;
}}

[class*="message"],
[class*="Message"],
[class*="thread"],
[class*="Thread"] {{
  background: var(--slock-desktop-surface-strong) !important;
  border-radius: 18px !important;
}}

[class*="message"],
[class*="Message"] {{
  background: var(--slock-desktop-surface-strong) !important;
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
  border-radius: 14px !important;
  background: var(--slock-desktop-surface) !important;
  box-shadow: var(--slock-desktop-shadow) !important;
}}

.input-brutal,
input,
textarea,
select,
[contenteditable="true"] {{
  border-radius: 14px !important;
  background: var(--slock-desktop-surface) !important;
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
  border-radius: 12px !important;
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
  transform: translateY(-1px) !important;
}}

.btn-brutal:active,
.btn-brutal-sm:active,
button:active,
[role="button"]:active {{
  transform: scale(0.97) !important;
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
.bg-brutal-pink,
.bg-brutal-pink\/30,
.bg-brutal-cyan,
.bg-brutal-cyan\/30,
.bg-brutal-cyan\/40,
.bg-brutal-lime,
.bg-brutal-lime\/20,
.bg-brutal-lime\/30,
.bg-brutal-lavender,
.bg-brutal-lavender\/40,
.bg-brutal-orange,
.bg-brutal-orange\/20,
.bg-brutal-orange\/30 {{
  background-color: var(--slock-desktop-accent-soft) !important;
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
  border-radius: 18px !important;
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
  border-radius: 18px 18px 0 0 !important;
  box-shadow: 0 -12px 42px -36px color-mix(in srgb, var(--slock-desktop-text) 35%, transparent) !important;
}}

.relative.flex.items-center.border-t-2 textarea,
.flex.items-center.border-t-2 textarea,
[class*="composer"] textarea,
[class*="Composer"] textarea {{
  min-height: 44px !important;
  border-radius: 16px !important;
  background: var(--slock-desktop-canvas) !important;
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
        mode = theme.mode,
        canvas = theme.canvas,
        surface = theme.surface,
        surface_strong = theme.surface_strong,
        line = theme.line,
        text = theme.text,
        muted = theme.muted,
        accent = theme.accent,
        accent_soft = theme.accent_soft
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
