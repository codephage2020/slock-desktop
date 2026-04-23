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
        summary: "Paper-toned control room with warm gold accents.",
        mode: "light",
        canvas: "#f3eee3",
        surface: "#fbf7ef",
        surface_strong: "#efe4d2",
        line: "#d6c5aa",
        text: "#171411",
        muted: "#675d51",
        accent: "#c58a1f",
        accent_soft: "#f2ddb5",
        preview: ["#f3eee3", "#efe4d2", "#c58a1f"],
    },
    ThemeDefinition {
        id: "light",
        name: "Light",
        summary: "Pure light workspace with crisp neutrals and blue focus cues.",
        mode: "light",
        canvas: "#eef2f6",
        surface: "#fbfdff",
        surface_strong: "#ffffff",
        line: "#d5dde8",
        text: "#111827",
        muted: "#5f6b7a",
        accent: "#2f6df6",
        accent_soft: "#dbe7ff",
        preview: ["#eef2f6", "#ffffff", "#2f6df6"],
    },
    ThemeDefinition {
        id: "dark",
        name: "Dark",
        summary: "Pure dark workspace with neutral depth and cool blue signals.",
        mode: "dark",
        canvas: "#0f1319",
        surface: "#171c24",
        surface_strong: "#202734",
        line: "#384254",
        text: "#f5f7fb",
        muted: "#9aa5b5",
        accent: "#63a4ff",
        accent_soft: "#263650",
        preview: ["#0f1319", "#202734", "#63a4ff"],
    },
    ThemeDefinition {
        id: "graphite",
        name: "Graphite",
        summary: "Cool slate shell for long operational sessions.",
        mode: "dark",
        canvas: "#12151b",
        surface: "#1a1f28",
        surface_strong: "#222936",
        line: "#313948",
        text: "#eef2f8",
        muted: "#9ca6b5",
        accent: "#8aa5ff",
        accent_soft: "#233153",
        preview: ["#12151b", "#222936", "#8aa5ff"],
    },
    ThemeDefinition {
        id: "crimson",
        name: "Crimson",
        summary: "Dark editorial variant with sharper contrast and heat.",
        mode: "dark",
        canvas: "#1b1215",
        surface: "#24181d",
        surface_strong: "#331f26",
        line: "#53303c",
        text: "#fff3f3",
        muted: "#d0afb7",
        accent: "#ff6d7b",
        accent_soft: "#4d232c",
        preview: ["#1b1215", "#331f26", "#ff6d7b"],
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
        accent = serde_json::to_string(theme.accent).unwrap_or_else(|_| "\"#c58a1f\"".into())
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
  --font-display: "Space Grotesk", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --default-font-family: "Space Grotesk", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --default-mono-font-family: "Space Mono", ui-monospace, monospace;
  --color-white: {surface};
  --color-black: {text};
  --color-gray-400: {muted};
  --color-brutal-cream: {canvas};
  --color-brutal-yellow: {accent};
  --color-brutal-pink: {accent_soft};
  --color-brutal-cyan: {surface_strong};
  --color-brutal-lime: {surface};
  --color-brutal-lavender: {line};
  --color-brutal-orange: {accent};
}}

html,
body,
#root {{
  background: var(--slock-desktop-canvas) !important;
  color: var(--slock-desktop-text) !important;
}}

body {{
  accent-color: var(--slock-desktop-accent) !important;
  font-family: "Space Grotesk", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif !important;
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
