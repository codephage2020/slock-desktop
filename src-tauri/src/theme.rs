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
    let system = normalized_mode == "system";
    let dark = normalized_mode == "dark";
    let accent = if system {
        format!("light-dark({light_accent}, {dark_accent})")
    } else if dark {
        dark_accent.to_string()
    } else {
        light_accent.to_string()
    };
    let accent_soft = if system {
        format!(
            "light-dark(color-mix(in srgb, {light_accent} 12%, #ffffff), color-mix(in srgb, {dark_accent} 22%, #1f1f1c))"
        )
    } else if dark {
        format!("color-mix(in srgb, {accent} 22%, #1f1f1c)")
    } else {
        format!("color-mix(in srgb, {accent} 12%, #ffffff)")
    };

    ThemeDefinition {
        id: id.to_string(),
        name: name.to_string(),
        summary: summary.to_string(),
        mode: normalized_mode.to_string(),
        canvas: mode_color(system, dark, "#f7f7f5", "#1f1f1c"),
        surface: mode_color(system, dark, "#ffffff", "#252623"),
        surface_strong: mode_color(system, dark, "#f3f4f1", "#2f302c"),
        line: mode_color(system, dark, "#e2e4de", "#3e413a"),
        text: mode_color(system, dark, "#1f1f1c", "#f4f4ef"),
        muted: mode_color(system, dark, "#6b6f67", "#b7bbae"),
        accent: accent.clone(),
        accent_soft,
        preview: [
            mode_color(system, dark, "#f7f7f5", "#1f1f1c"),
            mode_color(system, dark, "#f3f4f1", "#2f302c"),
            accent,
        ],
    }
}

fn mode_color(system: bool, dark: bool, light_value: &str, dark_value: &str) -> String {
    if system {
        format!("light-dark({light_value}, {dark_value})")
    } else if dark {
        dark_value.to_string()
    } else {
        light_value.to_string()
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

    const markAvatarInitials = () => {{
      if (!document.body) return;
      const selector = [
        '[class*="rounded-full"]',
        '[class*="avatar"]',
        '[class*="Avatar"]',
        '[class*="bg-brutal-cyan"]',
        '[class*="bg-brutal-pink"]',
        '[class*="bg-brutal-lime"]',
        '[class*="bg-brutal-yellow"]',
        '[class*="bg-brutal-orange"]',
        '[class*="bg-brutal-lavender"]',
        '[class*="h-"][class*="w-"][class*="items-center"][class*="justify-center"]'
      ].join(',');

      const avatarStyleProps = [
        "inline-size",
        "block-size",
        "width",
        "height",
        "min-width",
        "min-height",
        "max-width",
        "max-height",
        "flex-basis",
        "aspect-ratio",
        "writing-mode",
        "transform",
        "line-height"
      ];

      const clearMarkedAvatar = (element, key) => {{
        delete element.dataset[key];
        delete element.dataset.slockDesktopAvatarHasImage;
        avatarStyleProps.forEach((property) => element.style.removeProperty(property));
        Array.from(element.children).slice(0, 3).forEach((child) => {{
          if (child instanceof HTMLElement) delete child.dataset.slockDesktopAvatarFallback;
        }});
      }};

      const isThreadAvatarScope = (element) => (
        element.closest('[class*="thread"],[class*="Thread"],[aria-label*="thread" i],[aria-label*="线程"],[href*="thread"]') ||
        element.closest('button.relative.flex.items-start.gap-3.border-2')
      );
      const isSidebarAvatarScope = (element) =>
        !!element.closest('nav,aside,[class*="sidebar"],[class*="Sidebar"]');

      const markAvatarElement = (element, key, size) => {{
        const hasImage = element.matches("img") || !!element.querySelector("img");
        element.dataset[key] = "true";
        if (hasImage) element.dataset.slockDesktopAvatarHasImage = "true";
        else delete element.dataset.slockDesktopAvatarHasImage;
        element.style.setProperty("inline-size", `${{size}}px`, "important");
        element.style.setProperty("block-size", `${{size}}px`, "important");
        element.style.setProperty("width", `${{size}}px`, "important");
        element.style.setProperty("height", `${{size}}px`, "important");
        element.style.setProperty("min-width", `${{size}}px`, "important");
        element.style.setProperty("min-height", `${{size}}px`, "important");
        element.style.setProperty("max-width", `${{size}}px`, "important");
        element.style.setProperty("max-height", `${{size}}px`, "important");
        element.style.setProperty("flex-basis", `${{size}}px`, "important");
        element.style.setProperty("aspect-ratio", "1 / 1", "important");
        element.style.setProperty("writing-mode", "horizontal-tb", "important");
        element.style.setProperty("transform", "none", "important");
        element.style.setProperty("line-height", `${{size}}px`, "important");

        Array.from(element.children).slice(0, 3).forEach((child) => {{
          if (!(child instanceof HTMLElement)) return;
          if (child.matches("path,defs,clipPath,mask")) return;
          child.dataset[key] = "true";
          delete child.dataset.slockDesktopAvatarFallback;
          if (hasImage && !child.matches("img,picture,svg") && !child.querySelector("img,picture")) {{
            child.dataset.slockDesktopAvatarFallback = "true";
          }}
          child.style.setProperty("writing-mode", "horizontal-tb", "important");
          child.style.setProperty("transform", "none", "important");
          child.style.setProperty("line-height", `${{size}}px`, "important");
        }});
      }};

      document.querySelectorAll(selector).forEach((element) => {{
        if (!(element instanceof HTMLElement)) return;
        if (element.closest('#slock-desktop-settings-host')) return;
        if (element.matches('button,[role="button"],input,textarea,select,svg')) return;

        const compactText = (element.textContent || "").replace(/\s+/g, "");
        const hasCompactText = compactText && compactText.length <= 3 && !/^[0-9]+$/.test(compactText);
        const isImage = element.matches("img");
        const hasGraphic = isImage || !!element.querySelector("img,svg");

        const className = String(element.className || "");
        const looksAvatar =
          /avatar|Avatar|initial|Initial|rounded|bg-brutal|bg-\[|bg-(cyan|blue|indigo|emerald|green|pink|rose|orange|amber|slate|gray)/.test(className);
        const looksSized =
          /(h-\[[0-9]+px\]|w-\[[0-9]+px\]|h-[2-9]|w-[2-9]|h-1[0-6]|w-1[0-6])/.test(className) &&
          /items-center|justify-center|inline-flex|flex|grid/.test(className);
        const nearThread = isThreadAvatarScope(element);
        const nearSidebar = isSidebarAvatarScope(element);

        if (nearThread && (looksAvatar || looksSized || isImage) && (hasCompactText || looksSized || isImage)) {{
          markAvatarElement(element, "slockDesktopAvatar", 28);
        }} else if (
          nearSidebar &&
          (looksAvatar || looksSized || isImage) &&
          (hasCompactText || hasGraphic || isImage) &&
          !/badge|Badge|status|Status|presence|Presence/.test(className)
        ) {{
          markAvatarElement(element, "slockDesktopSidebarAvatar", 32);
        }} else {{
          if (element.dataset.slockDesktopAvatar === "true") clearMarkedAvatar(element, "slockDesktopAvatar");
          if (element.dataset.slockDesktopSidebarAvatar === "true") clearMarkedAvatar(element, "slockDesktopSidebarAvatar");
        }}
      }});

      document.querySelectorAll('[data-slock-desktop-avatar="true"]').forEach((element) => {{
        if (element instanceof HTMLElement && !isThreadAvatarScope(element)) clearMarkedAvatar(element, "slockDesktopAvatar");
      }});
      document.querySelectorAll('[data-slock-desktop-sidebar-avatar="true"]').forEach((element) => {{
        if (element instanceof HTMLElement && !isSidebarAvatarScope(element)) clearMarkedAvatar(element, "slockDesktopSidebarAvatar");
      }});
    }};

    const markSemanticStatusTokens = () => {{
      if (!document.body) return;

      const brutalColors = ["yellow", "orange", "pink", "cyan", "lime", "lavender"];
      const resolveBrutalColor = (className) =>
        brutalColors.find((color) => className.includes(`brutal-${{color}}`)) || null;
      const resolveTaskState = (text) => {{
        const value = (text || "").trim().toLowerCase();
        if (!value) return null;
        if (/(^|[^a-z])(todo|to do)(?=$|[^a-z])/.test(value) || /待办/.test(value)) return "todo";
        if (/(^|[^a-z])(in[\s_-]?progress|doing|open)(?=$|[^a-z])/.test(value) || /进行中|处理中/.test(value)) return "in-progress";
        if (/(^|[^a-z])(in[\s_-]?review|review)(?=$|[^a-z])/.test(value) || /审核中|待审核|复核中/.test(value)) return "in-review";
        if (/(^|[^a-z])(done|completed|closed)(?=$|[^a-z])/.test(value) || /已完成|完成/.test(value)) return "done";
        return null;
      }};

      document
        .querySelectorAll(
          [
            '[class*="bg-brutal"]',
            '[class*="text-brutal"]',
            '[class*="border-brutal"]',
            '[data-state]',
            '[aria-label*="task" i]',
            '[aria-label*="任务"]'
          ].join(",")
        )
        .forEach((element) => {{
          if (!(element instanceof HTMLElement)) return;
          if (element.closest('#slock-desktop-settings-host')) return;

          const className = String(element.className || "");
          const text = (element.textContent || "").trim();
          const rect = element.getBoundingClientRect();
          const tinyDot = rect.width > 0 && rect.width <= 18 && rect.height > 0 && rect.height <= 18;
          const smallChip = rect.width > 0 && rect.width <= 144 && rect.height > 0 && rect.height <= 32;
          const badgeLike =
            tinyDot ||
            (
              smallChip &&
              (
                /badge|Badge|status|Status|presence|Presence|inline-flex|shrink-0|ml-auto/.test(className) ||
                text.length <= 24
              )
            );

          const brutalColor = resolveBrutalColor(className);
          if (brutalColor && badgeLike) {{
            element.dataset.slockDesktopSemanticColor = brutalColor;
            element.dataset.slockDesktopSemanticShape = tinyDot ? "dot" : "chip";
          }} else {{
            delete element.dataset.slockDesktopSemanticColor;
            delete element.dataset.slockDesktopSemanticShape;
          }}

          const numericCount = /^\\d+\\+?$/.test(text);
          if (badgeLike && numericCount && !tinyDot) {{
            element.dataset.slockDesktopCountTone = brutalColor ? "accent" : "plain";
          }} else {{
            delete element.dataset.slockDesktopCountTone;
          }}

          const taskState = resolveTaskState(text);
          if (taskState && badgeLike) {{
            element.dataset.slockDesktopTaskState = taskState;
          }} else {{
            delete element.dataset.slockDesktopTaskState;
          }}
        }});
    }};

    const markWorkspaceModuleSurfaces = () => {{
      if (!document.body) return;

      const normalizeText = (value) => (value || "").replace(/\s+/g, " ").trim().toLowerCase();
      const moduleNames = new Set(["chat", "member", "members", "task", "tasks", "聊天", "成员", "任务"]);
      const sectionNames = new Set([
        "channels",
        "channel",
        "direct messages",
        "direct message",
        "threads",
        "频道",
        "直接消息",
        "私信",
        "线程",
        "话题"
      ]);
      const sidebarSelector = 'nav,aside,[class*="sidebar"],[class*="Sidebar"]';
      const surfaceProps = [
        "slockDesktopModuleTabs",
        "slockDesktopModuleTab",
        "slockDesktopModuleTabLabel",
        "slockDesktopModuleTabsScope",
        "slockDesktopPrimaryList",
        "slockDesktopPrimaryRow",
        "slockDesktopPrimaryRowLayout",
        "slockDesktopPrimaryRowVariant",
        "slockDesktopMenuItem",
        "slockDesktopTaskRow",
        "slockDesktopTaskTitle",
        "slockDesktopTaskStateChip",
        "slockDesktopAccountDock",
        "slockDesktopAccountAction",
        "slockDesktopPlusAction"
      ];

      document.querySelectorAll('[data-slock-desktop-module-tabs],[data-slock-desktop-module-tab],[data-slock-desktop-module-tab-label],[data-slock-desktop-module-tabs-scope],[data-slock-desktop-primary-list],[data-slock-desktop-primary-row],[data-slock-desktop-primary-row-layout],[data-slock-desktop-primary-row-variant],[data-slock-desktop-menu-item],[data-slock-desktop-task-row],[data-slock-desktop-task-title],[data-slock-desktop-task-state-chip],[data-slock-desktop-account-dock],[data-slock-desktop-account-action],[data-slock-desktop-plus-action]').forEach((element) => {{
        if (!(element instanceof HTMLElement)) return;
        surfaceProps.forEach((key) => delete element.dataset[key]);
      }});

      const moduleButtons = Array.from(
        document.querySelectorAll("button,[role='tab'],[role='button']")
      ).filter((element) => {{
        if (!(element instanceof HTMLElement)) return false;
        if (element.closest('#slock-desktop-settings-host')) return false;
        return moduleNames.has(normalizeText(element.textContent));
      }});

      const moduleGroups = new Map();
      moduleButtons.forEach((button) => {{
        const group = button.parentElement;
        if (!group) return;
        const groupButtons = moduleGroups.get(group) || [];
        groupButtons.push(button);
        moduleGroups.set(group, groupButtons);
      }});

      moduleGroups.forEach((buttons, group) => {{
        const labels = new Set(buttons.map((button) => normalizeText(button.textContent)));
        const hasChat = labels.has("chat") || labels.has("聊天");
        const hasSecondary = labels.has("member") || labels.has("members") || labels.has("task") || labels.has("tasks") || labels.has("成员") || labels.has("任务");
        if (!hasChat || !hasSecondary || !(group instanceof HTMLElement)) return;

        group.dataset.slockDesktopModuleTabs = "true";
        group.dataset.slockDesktopModuleTabsScope = group.closest(sidebarSelector) ? "sidebar" : "content";
        const selectedButtons = buttons.filter((button) => {{
          const className = String(button.className || "");
          return (
            button.getAttribute("aria-selected") === "true" ||
            button.getAttribute("aria-current") === "page" ||
            button.dataset.state === "active" ||
            button.dataset.active === "true" ||
            /bg-brutal-(pink|lime|cyan|yellow|orange)|shadow-brutal|font-bold/.test(className)
          );
        }});
        buttons.forEach((button) => {{
          const selected = selectedButtons.length > 0 ? selectedButtons.includes(button) : button === buttons[0];

          button.dataset.slockDesktopModuleTab = selected ? "selected" : "icon";
          Array.from(button.querySelectorAll("span,div,p,strong")).forEach((child) => {{
            if (!(child instanceof HTMLElement)) return;
            const text = normalizeText(child.textContent);
            if (moduleNames.has(text)) child.dataset.slockDesktopModuleTabLabel = "true";
          }});
        }});
      }});

      const detectPrimaryRowLayout = (row) => {{
        const children = Array.from(row.children).filter((child) => child instanceof HTMLElement);
        const text = (row.textContent || "").replace(/\s+/g, " ").trim();
        const hasLeadingGraphic = children.slice(0, 2).some((child) => {{
          if (!(child instanceof HTMLElement)) return false;
          return (
            child.matches('svg,img,[data-slock-desktop-sidebar-avatar="true"],[data-slock-desktop-avatar="true"],[class*="avatar"],[class*="Avatar"],[class*="rounded-full"]') ||
            !!child.querySelector('svg,img,[data-slock-desktop-sidebar-avatar="true"],[data-slock-desktop-avatar="true"],[class*="avatar"],[class*="Avatar"],[class*="rounded-full"]')
          );
        }});

        if (!hasLeadingGraphic && children.length <= 1 && /^#\s*\S+/.test(text)) return "hash-text";
        return hasLeadingGraphic ? "icon" : "text";
      }};

      document.querySelectorAll(`${{sidebarSelector}} *`).forEach((element) => {{
        if (!(element instanceof HTMLElement)) return;
        if (element.closest('#slock-desktop-settings-host')) return;
        const text = normalizeText(element.textContent);
        if (!sectionNames.has(text)) return;
        if (element.matches("button,a,[role='button'],[role='menuitem']")) return;

        let container = element.parentElement;
        for (let depth = 0; container && depth < 5; depth += 1, container = container.parentElement) {{
          if (!(container instanceof HTMLElement)) continue;
          if (container.matches("nav,aside")) break;
          const interactiveCount = container.querySelectorAll("button,a,[role='button']").length;
          const hasNestedSection = Array.from(container.querySelectorAll("span,div,p,h1,h2,h3,h4,h5,h6")).some((child) => {{
            if (!(child instanceof HTMLElement) || child === element) return false;
            return sectionNames.has(normalizeText(child.textContent));
          }});
          if (interactiveCount > 0 && interactiveCount <= 16 && !hasNestedSection) {{
            container.dataset.slockDesktopPrimaryList = "true";
            break;
          }}
        }}
      }});

      document.querySelectorAll('[data-slock-desktop-primary-list="true"] button,[data-slock-desktop-primary-list="true"] a,[data-slock-desktop-primary-list="true"] [role="button"]').forEach((row) => {{
        if (!(row instanceof HTMLElement)) return;
        const text = (row.textContent || "").replace(/\s+/g, " ").trim();
        if (text) row.setAttribute("title", text);
        row.dataset.slockDesktopPrimaryRow = "true";
        row.dataset.slockDesktopPrimaryRowLayout = detectPrimaryRowLayout(row);
        if (/^#\s*all\b/i.test(text)) row.dataset.slockDesktopPrimaryRowVariant = "root-channel";
      }});

      document.querySelectorAll(`${{sidebarSelector}} button,${{sidebarSelector}} a,${{sidebarSelector}} [role="button"]`).forEach((row) => {{
        if (!(row instanceof HTMLElement)) return;
        if (row.closest('#slock-desktop-settings-host')) return;
        if (row.closest('[data-slock-desktop-module-tabs="true"]')) return;
        const text = (row.textContent || "").replace(/\s+/g, " ").trim();
        if (!text) return;
        const isChannelRow = /^#\\s*\\S+/.test(text) || !!row.querySelector("svg,path,[data-slock-desktop-sidebar-avatar='true'],img");
        if (!isChannelRow) return;
        if (text) row.setAttribute("title", text);
        row.dataset.slockDesktopPrimaryRow = "true";
        row.dataset.slockDesktopPrimaryRowLayout = detectPrimaryRowLayout(row);
        if (/^#\s*all\b/i.test(text)) row.dataset.slockDesktopPrimaryRowVariant = "root-channel";
      }});

      document.querySelectorAll('main button,main a,main [role="button"],main [class*="border-2"],main [class*="border"]').forEach((row) => {{
        if (!(row instanceof HTMLElement)) return;
        if (row.closest('#slock-desktop-settings-host,form,input,textarea,[contenteditable="true"]')) return;
        const text = (row.textContent || "").replace(/\s+/g, " ").trim();
        if (!text) return;
        if (row.dataset.slockDesktopTaskState) return;
        const rect = row.getBoundingClientRect();
        if (rect.width > 0 && rect.width < 240) return;
        const stateElement = row.dataset.slockDesktopTaskState
          ? row
          : row.querySelector("[data-slock-desktop-task-state]");
        const hasTaskState =
          !!stateElement ||
          /\\b(todo|to do|in progress|in review|done)\\b/i.test(text) ||
          /待办|进行中|待复核|完成/.test(text);
        const hasTaskMarker =
          /#\\d+/.test(text) ||
          row.matches('[aria-label*="task" i],[aria-label*="任务"],[class*="task"],[class*="Task"]') ||
          !!row.closest('[aria-label*="task" i],[aria-label*="任务"],[class*="task"],[class*="Task"]');
        if (!hasTaskState || !hasTaskMarker) return;

        row.dataset.slockDesktopTaskRow = "true";
        if (stateElement instanceof HTMLElement) stateElement.dataset.slockDesktopTaskStateChip = "true";
        const titleCandidate = Array.from(row.querySelectorAll("span,p,div,strong,button"))
          .filter((element) => {{
            if (!(element instanceof HTMLElement)) return false;
            if (element.dataset.slockDesktopTaskState) return false;
            if (element.closest('[data-slock-desktop-task-state]')) return false;
            const value = (element.textContent || "").replace(/\s+/g, " ").trim();
            if (!value || value.length < 3) return false;
            if (/^#\\d+$/.test(value)) return false;
            if (/\\b(todo|to do|in progress|in review|done)\\b/i.test(value) || /待办|进行中|待复核|完成/.test(value)) return false;
            return true;
          }})
          .sort((a, b) => (b.textContent || "").trim().length - (a.textContent || "").trim().length)[0];
        if (titleCandidate instanceof HTMLElement) titleCandidate.dataset.slockDesktopTaskTitle = "true";
      }});

      const markMenuItem = (element) => {{
        if (!(element instanceof HTMLElement)) return;
        if (element.closest('#slock-desktop-settings-host')) return;
        if (element.matches("input,textarea,select")) return;
        element.dataset.slockDesktopMenuItem = "true";
      }};

      document
        .querySelectorAll(
          [
            '[role="menu"] [role="menuitem"]',
            '[role="menu"] button',
            '[role="menu"] a',
            '[data-radix-popper-content-wrapper] [role="menuitem"]',
            '[data-radix-popper-content-wrapper] [role="button"]',
            '[data-radix-popper-content-wrapper] [data-radix-collection-item]',
            '[data-radix-popper-content-wrapper] button',
            '[data-radix-popper-content-wrapper] a',
            '[data-slot="popover-content"] [role="menuitem"]',
            '[data-slot="popover-content"] [role="button"]',
            '[data-slot="popover-content"] button',
            '[data-slot="popover-content"] a',
            '.fixed.z-50.card-brutal [role="menuitem"]',
            '.absolute.z-50.card-brutal [role="menuitem"]'
          ].join(",")
        )
        .forEach(markMenuItem);

      document.querySelectorAll('.fixed.z-50.card-brutal,.absolute.z-50.card-brutal').forEach((container) => {{
        if (!(container instanceof HTMLElement)) return;
        if (container.closest('#slock-desktop-settings-host')) return;
        if (container.querySelector("form,input,textarea,select,h1,h2,h3,p,[role='dialog']")) return;

        const actions = Array.from(container.querySelectorAll("button,a,[role='button'],[role='menuitem']"))
          .filter((element) => element instanceof HTMLElement && !element.closest("form"));
        if (actions.length === 0 || actions.length > 18) return;

        const semanticMenu =
          container.getAttribute("role") === "menu" ||
          !!container.querySelector("[role='menuitem'],[data-radix-collection-item]") ||
          /menu|Menu|popover|Popover|context|Context/.test(String(container.className || ""));
        if (!semanticMenu && actions.length < 2) return;

        actions.forEach(markMenuItem);
      }});

      document.querySelectorAll(sidebarSelector).forEach((sidebar) => {{
        if (!(sidebar instanceof HTMLElement)) return;
        if (sidebar.closest('#slock-desktop-settings-host')) return;
        const sidebarRect = sidebar.getBoundingClientRect();
        if (sidebarRect.height <= 0) return;

        const actions = Array.from(sidebar.querySelectorAll("button,a,[role='button']"))
          .filter((element) => element instanceof HTMLElement && !element.closest("form,input,textarea,select"));
        actions.forEach((action) => {{
          const rect = action.getBoundingClientRect();
          if (rect.height <= 0 || rect.width <= 0) return;
          if (rect.bottom < sidebarRect.bottom - 190) return;

          const label = normalizeText([
            action.textContent,
            action.getAttribute("aria-label"),
            action.getAttribute("title")
          ].filter(Boolean).join(" "));
          const hasAvatar =
            !!action.querySelector('[data-slock-desktop-sidebar-avatar="true"],img,[class*="avatar"],[class*="Avatar"],[class*="rounded-full"]');
          const looksSettingsAction =
            /settings|setting|profile|account|user|设置|个人|账号|账户/.test(label) ||
            (rect.width <= 68 && rect.height <= 68 && !!action.querySelector("svg"));
          if (!hasAvatar && !looksSettingsAction) return;

          action.dataset.slockDesktopAccountAction = "true";
          let container = action.parentElement;
          for (let depth = 0; container && depth < 4; depth += 1, container = container.parentElement) {{
            if (!(container instanceof HTMLElement)) continue;
            if (container.matches("nav,aside")) break;
            const containerRect = container.getBoundingClientRect();
            if (containerRect.bottom < sidebarRect.bottom - 210) continue;
            const interactiveCount = container.querySelectorAll("button,a,[role='button']").length;
            if (interactiveCount > 0 && interactiveCount <= 4) {{
              container.dataset.slockDesktopAccountDock = "true";
              break;
            }}
          }}
        }});
      }});

      document.querySelectorAll('button,a,[role="button"]').forEach((action) => {{
        if (!(action instanceof HTMLElement)) return;
        if (action.closest('#slock-desktop-settings-host')) return;
        const text = normalizeText(action.textContent);
        const label = normalizeText([
          action.textContent,
          action.getAttribute("aria-label"),
          action.getAttribute("title")
        ].filter(Boolean).join(" "));
        const containsPlusGlyph = /[+＋]/.test(action.textContent || "");
        const iconOnly = text === "" || text === "+" || text === "＋";
        const looksAddAction = /(add|new|create|plus|invite|添加|新建|创建|新增)/.test(label) || containsPlusGlyph;
        if ((iconOnly || containsPlusGlyph) && looksAddAction) {{
          action.dataset.slockDesktopPlusAction = "true";
        }}
      }});
    }};

    markAvatarInitials();
    markSemanticStatusTokens();
    markWorkspaceModuleSurfaces();
    if (!window.__slockDesktopAvatarObserver && document.body) {{
      let avatarPending = false;
      window.__slockDesktopAvatarObserver = new MutationObserver(() => {{
        if (avatarPending) return;
        avatarPending = true;
        requestAnimationFrame(() => {{
          avatarPending = false;
          markAvatarInitials();
          markSemanticStatusTokens();
          markWorkspaceModuleSurfaces();
        }});
      }});
      window.__slockDesktopAvatarObserver.observe(document.body, {{
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: ["class", "aria-label", "href"]
      }});
    }}
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
  --slock-desktop-toolbar-bg: {surface};
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
  --slock-desktop-topbar-height: 62px;
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
  --slock-semantic-yellow: #f2c86b;
  --slock-semantic-orange: #eb9d61;
  --slock-semantic-pink: #ef6f9b;
  --slock-semantic-cyan: #53c0df;
  --slock-semantic-lime: #79b56a;
  --slock-semantic-lavender: #9e90de;
}}

@media (prefers-color-scheme: dark) {{
  html[data-slock-desktop-mode="system"] {{
    --slock-desktop-canvas: #1f1f1c;
    --slock-desktop-app-bg: #1f1f1c;
    --slock-desktop-toolbar-bg: #252623;
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
  background: var(--slock-desktop-surface) !important;
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

[class*="border-2"] {{
  border-width: 1px !important;
}}

[class*="border-b-2"] {{
  border-bottom-width: 1px !important;
}}

[class*="border-t-2"] {{
  border-top-width: 1px !important;
}}

[class*="border-r-2"],
[class*="border-r-3"] {{
  border-right-width: 1px !important;
}}

[class*="border-l-2"],
[class*="border-l-3"],
[class*="border-l-4"],
.md\:border-l-3 {{
  border-left-width: 1px !important;
}}

.card-brutal,
.input-brutal,
.btn-brutal,
.btn-brutal-sm,
[class*="border-2"] {{
  border-width: 1px !important;
  border-color: var(--slock-desktop-line) !important;
}}

[class*="border-b-2"],
[class*="border-t-2"],
[class*="border-r-2"],
[class*="border-r-3"],
[class*="border-l-2"],
[class*="border-l-3"],
[class*="border-l-4"],
.md\:border-l-3 {{
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
  min-height: 34px !important;
  display: inline-flex !important;
  align-items: center !important;
  justify-content: center !important;
  gap: 8px !important;
  padding-inline: 12px !important;
  border-radius: var(--slock-desktop-radius-md) !important;
  line-height: 1.1 !important;
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

button[class*="h-8"][class*="w-8"],
button[class*="h-9"][class*="w-9"],
button[class*="size-"],
[role="button"][class*="h-8"][class*="w-8"],
[aria-label*="channel" i]:is(button),
[aria-label*="channel" i][role="button"],
[aria-label*="频道" i]:is(button) {{
  width: 34px !important;
  min-width: 34px !important;
  max-width: 34px !important;
  min-height: 34px !important;
  padding: 0 !important;
  border-radius: var(--slock-desktop-radius-sm) !important;
}}

[data-slock-desktop-plus-action="true"] {{
  width: 30px !important;
  min-width: 30px !important;
  max-width: 30px !important;
  height: 30px !important;
  min-height: 30px !important;
  max-height: 30px !important;
  padding: 0 !important;
  border-radius: var(--slock-desktop-radius-sm) !important;
}}

[data-slock-desktop-plus-action="true"] svg {{
  width: 14px !important;
  height: 14px !important;
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
.safe-top,
[class*="safe-top"],
[class*="titlebar"],
[class*="Titlebar"],
[class*="topbar"],
[class*="Topbar"],
[class*="toolbar"],
[class*="Toolbar"],
[role="banner"],
header,
.flex.h-\[62px\],
.flex.h-\[62px\].shrink-0,
.relative.flex.items-center,
.flex.overflow-x-auto,
.shrink-0.border-b-2,
.md\:hidden.shrink-0,
[class*="border-b-2"].bg-white {{
  background: var(--slock-desktop-surface) !important;
  border-color: var(--slock-desktop-line) !important;
  border-width: 0 0 1px 0 !important;
  border-bottom: 1px solid var(--slock-desktop-line) !important;
  border-bottom-color: var(--slock-desktop-line) !important;
  box-shadow: none !important;
  min-height: var(--slock-desktop-topbar-height) !important;
  height: var(--slock-desktop-topbar-height) !important;
  max-height: var(--slock-desktop-topbar-height) !important;
  align-items: center !important;
  padding-block: 0 !important;
}}

.safe-top [class*="border-b-2"],
.safe-top [class*="border-t-2"],
[class*="safe-top"] [class*="border-b-2"],
[class*="safe-top"] [class*="border-t-2"],
header [class*="border-b-2"],
header [class*="border-t-2"],
.flex.h-\[62px\] [class*="border-b-2"],
.flex.h-\[62px\] [class*="border-t-2"],
.relative.flex.items-center.border-b-2 > [class*="border-b-2"],
.relative.flex.items-center.border-b-2 > [class*="border-t-2"] {{
  border-top-width: 0 !important;
  border-bottom-width: 0 !important;
  box-shadow: none !important;
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
[class*="border-transparent"].mb-1.flex.w-full.items-center.gap-1\.5,
.mb-1.flex.w-full.items-center.gap-1\.5,
.w-full.border-2.mb-1.flex.w-full.items-center.gap-1\.5,
[class*="channel"],
[class*="Channel"],
[class*="thread"],
[class*="Thread"] {{
  border-radius: 10px !important;
  background: transparent !important;
  box-shadow: none !important;
  justify-content: flex-start !important;
  text-align: left !important;
  min-width: 0 !important;
}}

.mb-1.flex.w-full.items-center.gap-1\.5:not(.bg-brutal-pink),
button.mb-1.flex.w-full.items-center.gap-1\.5:not(.bg-brutal-pink),
a.mb-1.flex.w-full.items-center.gap-1\.5:not(.bg-brutal-pink),
nav .w-full.border-2:not(.bg-brutal-pink),
aside .w-full.border-2:not(.bg-brutal-pink),
[class*="sidebar"] .w-full.border-2:not(.bg-brutal-pink),
[class*="Sidebar"] .w-full.border-2:not(.bg-brutal-pink) {{
  background: transparent !important;
  border-color: transparent !important;
  box-shadow: none !important;
  border-radius: var(--slock-desktop-radius-sm) !important;
}}

.mb-1.flex.w-full.items-center.gap-1\.5:not(.bg-brutal-pink):hover,
button.mb-1.flex.w-full.items-center.gap-1\.5:not(.bg-brutal-pink):hover,
a.mb-1.flex.w-full.items-center.gap-1\.5:not(.bg-brutal-pink):hover,
nav .w-full.border-2:not(.bg-brutal-pink):hover,
aside .w-full.border-2:not(.bg-brutal-pink):hover,
[class*="sidebar"] .w-full.border-2:not(.bg-brutal-pink):hover,
[class*="Sidebar"] .w-full.border-2:not(.bg-brutal-pink):hover {{
  background: var(--slock-desktop-hover) !important;
  border-color: transparent !important;
  box-shadow: none !important;
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

nav button[aria-current="page"],
nav button[aria-selected="true"],
nav button[data-state="active"],
nav button[data-active="true"],
aside button[aria-current="page"],
aside button[aria-selected="true"],
aside button[data-state="active"],
aside button[data-active="true"],
.group.flex.items-center[aria-current="page"],
.group.flex.items-center[aria-selected="true"],
.group.flex.items-center[data-state="active"],
.group.flex.items-center[data-active="true"],
[class*="channel"][aria-current="page"],
[class*="channel"][aria-selected="true"],
[class*="channel"][data-state="active"],
[class*="channel"][data-active="true"],
[class*="Channel"][aria-current="page"],
[class*="Channel"][aria-selected="true"],
[class*="Channel"][data-state="active"],
[class*="Channel"][data-active="true"],
[class*="thread"][aria-current="page"],
[class*="thread"][aria-selected="true"],
[class*="thread"][data-state="active"],
[class*="thread"][data-active="true"],
[class*="Thread"][aria-current="page"],
[class*="Thread"][aria-selected="true"],
[class*="Thread"][data-state="active"],
[class*="Thread"][data-active="true"],
nav .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold,
aside .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold,
[class*="sidebar"] .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold,
[class*="Sidebar"] .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold,
.group.flex.items-center.border-black.bg-brutal-pink.shadow-brutal-sm.font-bold,
button.border-black.bg-brutal-pink.shadow-brutal-sm.font-bold,
.flex.w-full.items-center.gap-1\.5.border-black.bg-brutal-pink.shadow-brutal-sm.font-bold,
.flex.w-full.items-center.gap-2.border-black.bg-brutal-pink.shadow-brutal-sm.font-bold {{
  background: var(--slock-desktop-selection) !important;
  color: var(--slock-desktop-text) !important;
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--slock-desktop-accent) 20%, transparent) !important;
}}

nav button > *,
aside button > *,
.group.flex.items-center > *,
[class*="channel"] > *,
[class*="Channel"] > *,
[class*="thread"] > *,
[class*="Thread"] > * {{
  min-width: 0 !important;
}}

nav button > :first-child,
aside button > :first-child,
.group.flex.items-center > :first-child,
[class*="channel"] > :first-child,
[class*="Channel"] > :first-child,
[class*="thread"] > :first-child,
[class*="Thread"] > :first-child {{
  text-align: left !important;
}}

nav button > :last-child,
aside button > :last-child,
.group.flex.items-center > :last-child,
[class*="channel"] > :last-child,
[class*="Channel"] > :last-child,
[class*="thread"] > :last-child,
[class*="Thread"] > :last-child,
[class*="ml-auto"],
.ml-auto,
[class*="badge"],
[class*="Badge"],
[class*="count"],
[class*="Count"] {{
  margin-left: auto !important;
  justify-self: end !important;
  text-align: right !important;
}}

[class*="sidebar"] [class*="bg-brutal"],
[class*="Sidebar"] [class*="bg-brutal"],
aside [class*="bg-brutal"],
nav [class*="bg-brutal"] {{
  background: transparent !important;
}}

[class*="sidebar"] [aria-current="page"],
[class*="Sidebar"] [aria-current="page"],
[class*="sidebar"] [aria-selected="true"],
[class*="Sidebar"] [aria-selected="true"],
[class*="sidebar"] [data-state="active"],
[class*="Sidebar"] [data-state="active"],
aside [aria-current="page"],
aside [aria-selected="true"],
aside [data-state="active"],
nav [aria-current="page"],
nav [aria-selected="true"],
nav [data-state="active"],
nav .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold,
aside .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold,
[class*="sidebar"] .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold,
[class*="Sidebar"] .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold,
button.border-black.bg-brutal-pink.shadow-brutal-sm.font-bold,
.flex.w-full.items-center.gap-1\.5.border-black.bg-brutal-pink.shadow-brutal-sm.font-bold {{
  background: var(--slock-desktop-selection) !important;
}}

[data-slock-desktop-semantic-color="yellow"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-yellow);
}}

[data-slock-desktop-semantic-color="orange"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-orange);
}}

[data-slock-desktop-semantic-color="pink"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-pink);
}}

[data-slock-desktop-semantic-color="cyan"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-cyan);
}}

[data-slock-desktop-semantic-color="lime"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-lime);
}}

[data-slock-desktop-semantic-color="lavender"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-lavender);
}}

[data-slock-desktop-task-state="todo"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-cyan);
}}

[data-slock-desktop-task-state="in-progress"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-orange);
}}

[data-slock-desktop-task-state="in-review"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-yellow);
}}

[data-slock-desktop-task-state="done"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-lime);
}}

[data-slock-desktop-semantic-color][data-slock-desktop-semantic-shape="dot"] {{
  background: var(--slock-desktop-semantic-current) !important;
  border-color: color-mix(in srgb, var(--slock-desktop-surface) 72%, var(--slock-desktop-semantic-current)) !important;
  color: transparent !important;
  box-shadow: 0 0 0 2px var(--slock-desktop-surface) !important;
}}

[data-slock-desktop-semantic-color][data-slock-desktop-semantic-shape="chip"],
[data-slock-desktop-task-state] {{
  background: color-mix(in srgb, var(--slock-desktop-semantic-current) 18%, var(--slock-desktop-surface)) !important;
  border-color: color-mix(in srgb, var(--slock-desktop-semantic-current) 38%, var(--slock-desktop-line)) !important;
  color: color-mix(in srgb, var(--slock-desktop-semantic-current) 78%, var(--slock-desktop-text)) !important;
  box-shadow: none !important;
}}

[data-slock-desktop-count-tone] {{
  background: transparent !important;
  border-color: transparent !important;
  border-width: 0 !important;
  box-shadow: none !important;
  padding: 0 !important;
  min-height: auto !important;
  border-radius: 0 !important;
}}

[data-slock-desktop-count-tone="plain"] {{
  color: var(--slock-desktop-muted) !important;
  font-weight: 500 !important;
}}

[data-slock-desktop-count-tone="accent"] {{
  color: color-mix(in srgb, var(--slock-desktop-semantic-current) 78%, var(--slock-desktop-text)) !important;
  font-weight: 650 !important;
}}

[data-slock-desktop-avatar="true"][data-slock-desktop-avatar-has-image="true"],
[data-slock-desktop-sidebar-avatar="true"][data-slock-desktop-avatar-has-image="true"] {{
  background: transparent !important;
  color: transparent !important;
}}

[data-slock-desktop-avatar-fallback="true"] {{
  opacity: 0 !important;
  pointer-events: none !important;
}}

[data-slock-desktop-avatar="true"],
[data-slock-desktop-sidebar-avatar="true"],
[class*="thread"] [class*="avatar"],
[class*="Thread"] [class*="avatar"],
[class*="thread"] [class*="Avatar"],
[class*="Thread"] [class*="Avatar"],
[class*="thread"] [class*="rounded-full"][class*="h-"][class*="w-"],
[class*="Thread"] [class*="rounded-full"][class*="h-"][class*="w-"],
[class*="thread"] img[class*="avatar"],
[class*="Thread"] img[class*="avatar"],
[class*="thread"] img[class*="Avatar"],
[class*="Thread"] img[class*="Avatar"],
[class*="thread"] img[class*="rounded-full"][class*="h-"][class*="w-"],
[class*="Thread"] img[class*="rounded-full"][class*="h-"][class*="w-"],
[class*="thread"] img[alt*="avatar" i],
[class*="Thread"] img[alt*="avatar" i],
[class*="thread"] img[alt*="头像"],
[class*="Thread"] img[alt*="头像"],
[class*="thread"] img[title*="avatar" i],
[class*="Thread"] img[title*="avatar" i],
[class*="thread"] img[title*="头像"],
[class*="Thread"] img[title*="头像"],
[class*="thread"] img[data-avatar],
[class*="Thread"] img[data-avatar],
[class*="thread"] [data-avatar] img,
[class*="Thread"] [data-avatar] img {{
  width: 28px !important;
  min-width: 28px !important;
  max-width: 28px !important;
  height: 28px !important;
  min-height: 28px !important;
  max-height: 28px !important;
  aspect-ratio: 1 / 1 !important;
  flex: 0 0 28px !important;
  flex-basis: 28px !important;
  border-radius: var(--slock-desktop-radius-pill) !important;
  object-fit: cover !important;
  overflow: hidden !important;
  display: grid !important;
  place-items: center !important;
  padding: 0 !important;
  line-height: 1 !important;
  white-space: nowrap !important;
  text-align: center !important;
}}

[data-slock-desktop-sidebar-avatar="true"] {{
  width: 32px !important;
  min-width: 32px !important;
  max-width: 32px !important;
  height: 32px !important;
  min-height: 32px !important;
  max-height: 32px !important;
  aspect-ratio: 1 / 1 !important;
  flex: 0 0 32px !important;
  flex-basis: 32px !important;
  border-radius: var(--slock-desktop-radius-pill) !important;
  object-fit: cover !important;
  overflow: hidden !important;
  display: grid !important;
  place-items: center !important;
  padding: 0 !important;
  line-height: 1 !important;
  text-align: center !important;
  border-color: transparent !important;
  box-shadow: none !important;
}}

[data-slock-desktop-sidebar-avatar="true"] > img,
[data-slock-desktop-sidebar-avatar="true"] img,
img[data-slock-desktop-sidebar-avatar="true"] {{
  width: 100% !important;
  min-width: 100% !important;
  max-width: 100% !important;
  height: 100% !important;
  min-height: 100% !important;
  max-height: 100% !important;
  object-fit: cover !important;
}}

button.relative.flex.items-start.gap-3.border-2 .inline-flex.h-\[14px\].w-\[14px\],
button.relative.flex.items-start.gap-3.border-2 [data-slock-desktop-avatar="true"] {{
  width: 28px !important;
  min-width: 28px !important;
  max-width: 28px !important;
  height: 28px !important;
  min-height: 28px !important;
  max-height: 28px !important;
  flex: 0 0 28px !important;
  flex-basis: 28px !important;
  aspect-ratio: 1 / 1 !important;
  border-radius: var(--slock-desktop-radius-pill) !important;
  display: grid !important;
  place-items: center !important;
  line-height: 28px !important;
  writing-mode: horizontal-tb !important;
  transform: none !important;
  overflow: hidden !important;
}}

[data-slock-desktop-avatar="true"] > svg,
svg[data-slock-desktop-avatar="true"] {{
  width: 16px !important;
  min-width: 16px !important;
  max-width: 16px !important;
  height: 16px !important;
  min-height: 16px !important;
  max-height: 16px !important;
  flex: 0 0 16px !important;
}}

[data-slock-desktop-avatar="true"] > img,
[data-slock-desktop-avatar="true"] img,
img[data-slock-desktop-avatar="true"] {{
  width: 100% !important;
  min-width: 100% !important;
  max-width: 100% !important;
  height: 100% !important;
  min-height: 100% !important;
  max-height: 100% !important;
  object-fit: cover !important;
  display: block !important;
  position: relative !important;
  z-index: 1 !important;
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
form:has(textarea[placeholder*="Message" i]),
form:has(textarea[placeholder*="消息"]),
form:has(button[title*="Attach image" i]),
form:has(button[title*="附加图片"]) {{
  position: relative !important;
  display: flex !important;
  flex-direction: column !important;
  flex: 0 0 auto !important;
  background: var(--slock-desktop-surface) !important;
  border-color: var(--slock-desktop-line) !important;
  border-width: 1px 0 0 0 !important;
  border-radius: 0 !important;
  box-shadow: none !important;
  min-height: 96px !important;
  max-height: 154px !important;
  height: auto !important;
  padding: 12px 14px !important;
  gap: 8px !important;
  overflow: visible !important;
  align-self: stretch !important;
  margin-top: 0 !important;
}}

.relative.flex.items-center.border-t-2 textarea,
.flex.items-center.border-t-2 textarea,
form:has(textarea[placeholder*="Message" i]) textarea,
form:has(textarea[placeholder*="消息"]) textarea,
form:has(button[title*="Attach image" i]) textarea,
form:has(button[title*="附加图片"]) textarea,
form:has(textarea[placeholder*="Message" i]) [contenteditable="true"],
form:has(textarea[placeholder*="消息"]) [contenteditable="true"],
form:has(button[title*="Attach image" i]) [contenteditable="true"],
form:has(button[title*="附加图片"]) [contenteditable="true"] {{
  min-height: 44px !important;
  max-height: 86px !important;
  padding: 8px 12px !important;
  border: 0 !important;
  border-radius: 0 !important;
  background: transparent !important;
  box-shadow: none !important;
  line-height: 1.55 !important;
  overflow-y: auto !important;
  min-width: 0 !important;
  flex: 1 1 auto !important;
  width: auto !important;
  resize: none !important;
}}

.relative.flex.items-center.border-t-2 button[aria-label*="send" i],
.relative.flex.items-center.border-t-2 button[aria-label*="发送" i],
.relative.flex.items-center.border-t-2 button[type="submit"],
.flex.items-center.border-t-2 button[aria-label*="send" i],
.flex.items-center.border-t-2 button[aria-label*="发送" i],
.flex.items-center.border-t-2 button[type="submit"],
form:has(textarea[placeholder*="Message" i]) button[aria-label*="send" i],
form:has(textarea[placeholder*="消息"]) button[aria-label*="发送" i],
form:has(button[title*="Attach image" i]) button[aria-label*="send" i],
form:has(button[title*="附加图片"]) button[aria-label*="发送" i],
form:has(textarea[placeholder*="Message" i]) button[type="submit"],
form:has(textarea[placeholder*="消息"]) button[type="submit"],
form:has(button[title*="Attach image" i]) button[type="submit"],
form:has(button[title*="附加图片"]) button[type="submit"] {{
  position: relative !important;
  right: auto !important;
  bottom: auto !important;
  z-index: 3 !important;
  flex: 0 0 36px !important;
  width: 36px !important;
  min-width: 36px !important;
  max-width: 36px !important;
  min-height: 36px !important;
  padding: 0 !important;
  border-radius: var(--slock-desktop-radius-md) !important;
  background: var(--slock-desktop-accent) !important;
  color: var(--slock-desktop-surface) !important;
  border-color: transparent !important;
  box-shadow: none !important;
}}

form:has(textarea[placeholder*="Message" i]),
form:has(textarea[placeholder*="消息"]),
form:has(button[title*="Attach image" i]),
form:has(button[title*="附加图片"]) {{
  background: var(--slock-desktop-surface) !important;
  border-color: var(--slock-desktop-line) !important;
  border-width: 1px 0 0 0 !important;
  border-radius: 0 !important;
  box-shadow: none !important;
  min-height: 96px !important;
  max-height: 154px !important;
  height: auto !important;
  padding: 12px 14px !important;
  overflow: visible !important;
  align-self: stretch !important;
  margin-top: 0 !important;
}}

form:has(textarea[placeholder*="Message" i]) .flex.items-center.justify-between.gap-3,
form:has(textarea[placeholder*="消息"]) .flex.items-center.justify-between.gap-3,
form:has(button[title*="Attach image" i]) .flex.items-center.justify-between.gap-3,
form:has(button[title*="附加图片"]) .flex.items-center.justify-between.gap-3 {{
  display: flex !important;
  visibility: visible !important;
  opacity: 1 !important;
  position: relative !important;
  min-height: 32px !important;
  flex: 0 0 auto !important;
  align-items: center !important;
  width: 100% !important;
  margin-top: 0 !important;
  padding-inline: 2px !important;
  gap: 6px !important;
}}

form:has(textarea[placeholder*="Message" i]) button[title*="Attach" i],
form:has(textarea[placeholder*="消息"]) button[title*="附加"],
form:has(button[title*="Attach image" i]) button[title*="Attach" i],
form:has(button[title*="附加图片"]) button[title*="附加"],
form:has(textarea[placeholder*="Message" i]) button[type="submit"],
form:has(textarea[placeholder*="消息"]) button[type="submit"],
form:has(button[title*="Attach image" i]) button[type="submit"],
form:has(button[title*="附加图片"]) button[type="submit"] {{
  display: inline-flex !important;
  visibility: visible !important;
  opacity: 1 !important;
  align-items: center !important;
  justify-content: center !important;
  min-width: 32px !important;
  min-height: 32px !important;
  border-radius: var(--slock-desktop-radius-md) !important;
  box-shadow: none !important;
}}

form:has(textarea[placeholder*="Message" i]) .flex.items-center.justify-between.gap-3 button,
form:has(textarea[placeholder*="消息"]) .flex.items-center.justify-between.gap-3 button,
form:has(button[title*="Attach image" i]) .flex.items-center.justify-between.gap-3 button,
form:has(button[title*="附加图片"]) .flex.items-center.justify-between.gap-3 button {{
  min-height: 32px !important;
  max-height: 32px !important;
  padding-inline: 8px !important;
  border-radius: var(--slock-desktop-radius-sm) !important;
  box-shadow: none !important;
  white-space: nowrap !important;
  max-width: 82px !important;
  overflow: hidden !important;
  text-overflow: ellipsis !important;
}}

form:has(textarea[placeholder*="Message" i]) .flex.items-center.justify-between.gap-3 button:is([title*="Attach" i],[aria-label*="Attach" i],[title*="image" i],[aria-label*="image" i],[title*="file" i],[aria-label*="file" i],[title*="附加"],[aria-label*="附加"],[title*="图片"],[aria-label*="图片"],[title*="文件"],[aria-label*="文件"]):not([type="submit"]),
form:has(textarea[placeholder*="消息"]) .flex.items-center.justify-between.gap-3 button:is([title*="Attach" i],[aria-label*="Attach" i],[title*="image" i],[aria-label*="image" i],[title*="file" i],[aria-label*="file" i],[title*="附加"],[aria-label*="附加"],[title*="图片"],[aria-label*="图片"],[title*="文件"],[aria-label*="文件"]):not([type="submit"]),
form:has(button[title*="Attach image" i]) .flex.items-center.justify-between.gap-3 button:is([title*="Attach" i],[aria-label*="Attach" i],[title*="image" i],[aria-label*="image" i],[title*="file" i],[aria-label*="file" i],[title*="附加"],[aria-label*="附加"],[title*="图片"],[aria-label*="图片"],[title*="文件"],[aria-label*="文件"]):not([type="submit"]),
form:has(button[title*="附加图片"]) .flex.items-center.justify-between.gap-3 button:is([title*="Attach" i],[aria-label*="Attach" i],[title*="image" i],[aria-label*="image" i],[title*="file" i],[aria-label*="file" i],[title*="附加"],[aria-label*="附加"],[title*="图片"],[aria-label*="图片"],[title*="文件"],[aria-label*="文件"]):not([type="submit"]) {{
  width: 32px !important;
  min-width: 32px !important;
  max-width: 32px !important;
  padding: 0 !important;
}}

form:has(textarea[placeholder*="Message" i]) .border-2:has(textarea),
form:has(textarea[placeholder*="消息"]) .border-2:has(textarea),
form:has(button[title*="Attach image" i]) .border-2:has(textarea),
form:has(button[title*="附加图片"]) .border-2:has(textarea),
form:has(textarea[placeholder*="Message" i]) .border-2:has([contenteditable="true"]),
form:has(textarea[placeholder*="消息"]) .border-2:has([contenteditable="true"]),
form:has(button[title*="Attach image" i]) .border-2:has([contenteditable="true"]),
form:has(button[title*="附加图片"]) .border-2:has([contenteditable="true"]) {{
  background: var(--slock-desktop-surface-secondary) !important;
  border: 1px solid var(--slock-desktop-line) !important;
  border-radius: var(--slock-desktop-radius-lg) !important;
  box-shadow: none !important;
  min-height: 50px !important;
  max-height: 96px !important;
  padding: 0 !important;
  overflow: hidden !important;
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
.safe-top,
[class*="safe-top"],
[class*="titlebar"],
[class*="Titlebar"],
[class*="topbar"],
[class*="Topbar"],
[class*="toolbar"],
[class*="Toolbar"],
[role="banner"],
header,
.flex.h-\[62px\],
.flex.h-\[62px\].shrink-0,
.relative.flex.items-center.border-b-2,
.flex.items-start.gap-2.border-b-2,
.border-b-2.bg-white,
.border-b-2.bg-\[\#ffeefb\],
.flex.overflow-x-auto.border-b-2 {{
  background: var(--slock-desktop-surface) !important;
  border-color: var(--slock-desktop-line) !important;
  border-width: 0 0 1px 0 !important;
  border-bottom: 1px solid var(--slock-desktop-line) !important;
  border-bottom-color: var(--slock-desktop-line) !important;
  box-shadow: none !important;
  min-height: var(--slock-desktop-topbar-height) !important;
  height: var(--slock-desktop-topbar-height) !important;
  max-height: var(--slock-desktop-topbar-height) !important;
  align-items: center !important;
  padding-block: 0 !important;
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

nav .h-5.w-5,
nav .h-7.w-7,
nav .h-8.w-8,
nav .h-9.w-9,
nav .h-12.w-12,
aside .h-5.w-5,
aside .h-7.w-7,
aside .h-8.w-8,
aside .h-9.w-9,
aside .h-12.w-12,
[class*="sidebar"] .h-5.w-5,
[class*="sidebar"] .h-7.w-7,
[class*="sidebar"] .h-8.w-8,
[class*="sidebar"] .h-9.w-9,
[class*="sidebar"] .h-12.w-12,
[class*="Sidebar"] .h-5.w-5,
[class*="Sidebar"] .h-7.w-7,
[class*="Sidebar"] .h-8.w-8,
[class*="Sidebar"] .h-9.w-9,
[class*="Sidebar"] .h-12.w-12 {{
  border-color: transparent !important;
  box-shadow: none !important;
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

[data-slock-desktop-semantic-color][data-slock-desktop-semantic-shape="dot"] {{
  background: var(--slock-desktop-semantic-current) !important;
  border-color: color-mix(in srgb, var(--slock-desktop-surface) 72%, var(--slock-desktop-semantic-current)) !important;
  color: transparent !important;
  box-shadow: 0 0 0 2px var(--slock-desktop-surface) !important;
}}

[data-slock-desktop-semantic-color][data-slock-desktop-semantic-shape="chip"],
[data-slock-desktop-task-state] {{
  background: color-mix(in srgb, var(--slock-desktop-semantic-current) 18%, var(--slock-desktop-surface)) !important;
  border-color: color-mix(in srgb, var(--slock-desktop-semantic-current) 38%, var(--slock-desktop-line)) !important;
  color: color-mix(in srgb, var(--slock-desktop-semantic-current) 78%, var(--slock-desktop-text)) !important;
  box-shadow: none !important;
}}

[data-slock-desktop-module-tabs="true"] {{
  display: inline-flex !important;
  align-items: center !important;
  gap: 4px !important;
  padding: 2px !important;
  background: transparent !important;
  border: 0 !important;
  box-shadow: none !important;
}}

[data-slock-desktop-module-tabs-scope="content"] {{
  margin-inline-start: 28px !important;
}}

[data-slock-desktop-module-tab] {{
  min-height: 34px !important;
  border-radius: var(--slock-desktop-radius-pill) !important;
  border-color: transparent !important;
  box-shadow: none !important;
  transition:
    background 150ms ease,
    color 150ms ease,
    inline-size 150ms ease !important;
}}

[data-slock-desktop-module-tab="icon"] {{
  width: 34px !important;
  min-width: 34px !important;
  max-width: 34px !important;
  padding: 0 !important;
  background: transparent !important;
  color: var(--slock-desktop-muted) !important;
  font-size: 0 !important;
}}

[data-slock-desktop-module-tab="icon"] svg {{
  width: 17px !important;
  height: 17px !important;
  font-size: 17px !important;
}}

[data-slock-desktop-module-tab="icon"] [data-slock-desktop-module-tab-label="true"] {{
  position: absolute !important;
  inline-size: 1px !important;
  block-size: 1px !important;
  margin: -1px !important;
  overflow: hidden !important;
  clip: rect(0 0 0 0) !important;
  white-space: nowrap !important;
}}

[data-slock-desktop-module-tab="selected"] {{
  width: auto !important;
  min-width: 0 !important;
  max-width: none !important;
  padding-inline: 12px !important;
  background: var(--slock-desktop-selection) !important;
  border-color: color-mix(in srgb, var(--slock-desktop-accent) 24%, transparent) !important;
  color: var(--slock-desktop-text) !important;
  font-size: 13px !important;
  font-weight: 650 !important;
}}

[data-slock-desktop-account-dock="true"] :is(button,a,[role="button"]),
[data-slock-desktop-account-action="true"],
nav [class*="btn-brutal-sm"][data-slock-desktop-account-action="true"],
aside [class*="btn-brutal-sm"][data-slock-desktop-account-action="true"] {{
  border-color: transparent !important;
  border-width: 0 !important;
  box-shadow: none !important;
}}

[data-slock-desktop-primary-list="true"] {{
  background: color-mix(in srgb, var(--slock-desktop-surface) 78%, var(--slock-desktop-surface-strong)) !important;
  border: 1px solid color-mix(in srgb, var(--slock-desktop-line) 72%, transparent) !important;
  border-radius: var(--slock-desktop-radius-lg) !important;
  box-shadow: none !important;
  padding: 8px !important;
}}

[data-slock-desktop-primary-list="true"] :is(button,a,[role="button"]) {{
  display: grid !important;
  grid-template-columns: 28px minmax(0, 1fr) auto !important;
  align-items: center !important;
  justify-content: flex-start !important;
  column-gap: 8px !important;
  width: 100% !important;
  min-width: 0 !important;
  background: transparent !important;
  border-color: transparent !important;
  box-shadow: none !important;
  text-align: left !important;
}}

nav [data-slock-desktop-primary-row="true"],
aside [data-slock-desktop-primary-row="true"],
[class*="sidebar"] [data-slock-desktop-primary-row="true"],
[class*="Sidebar"] [data-slock-desktop-primary-row="true"] {{
  display: grid !important;
  grid-template-columns: 28px minmax(0, 1fr) auto !important;
  align-items: center !important;
  justify-content: stretch !important;
  column-gap: 8px !important;
  width: 100% !important;
  min-width: 0 !important;
  text-align: left !important;
}}

[data-slock-desktop-primary-row="true"] {{
  position: relative !important;
  overflow: hidden !important;
  padding-inline: 0 !important;
}}

[data-slock-desktop-primary-row-layout="text"] {{
  grid-template-columns: minmax(0, 1fr) auto !important;
}}

[data-slock-desktop-primary-row-layout="hash-text"] {{
  grid-template-columns: minmax(0, 1fr) auto !important;
  padding-inline-start: 24px !important;
}}

[data-slock-desktop-primary-row-variant="root-channel"] {{
  padding-inline-start: 24px !important;
}}

[data-slock-desktop-primary-row="true"]:hover {{
  z-index: 2 !important;
}}

[data-slock-desktop-primary-row="true"]::before {{
  content: none !important;
}}

[data-slock-desktop-primary-row="true"] > :first-child {{
  grid-column: 1 !important;
  justify-self: center !important;
  min-width: 0 !important;
}}

[data-slock-desktop-primary-row-layout="text"] > :first-child {{
  justify-self: start !important;
  text-align: left !important;
}}

[data-slock-desktop-primary-row-layout="hash-text"] > :first-child {{
  justify-self: start !important;
  text-align: left !important;
}}

[data-slock-desktop-primary-row="true"]:not(:has(> :nth-child(2))) > :first-child {{
  grid-column: 2 !important;
  justify-self: start !important;
}}

[data-slock-desktop-primary-row-layout="text"]:not(:has(> :nth-child(2))) > :first-child {{
  grid-column: 1 !important;
}}

[data-slock-desktop-primary-row-layout="hash-text"]:not(:has(> :nth-child(2))) > :first-child {{
  grid-column: 1 !important;
}}

[data-slock-desktop-primary-row="true"]:not(:has(> :nth-child(3))) > :last-child:not(:first-child) {{
  grid-column: 2 !important;
  justify-self: stretch !important;
}}

[data-slock-desktop-primary-row-layout="text"]:not(:has(> :nth-child(3))) > :last-child:not(:first-child) {{
  grid-column: 2 !important;
  justify-self: end !important;
}}

[data-slock-desktop-primary-row="true"]:has(> :nth-child(3)) > :last-child {{
  grid-column: 3 !important;
  justify-self: end !important;
}}

[data-slock-desktop-primary-row="true"]:has(> :nth-child(3)) > :not(:first-child):not(:last-child) {{
  grid-column: 2 !important;
  justify-self: stretch !important;
}}

[data-slock-desktop-primary-list="true"] :is(button,a,[role="button"]) > :not(:first-child):not(:last-child),
[data-slock-desktop-primary-list="true"] :is(button,a,[role="button"]) > :first-child:last-child,
nav :is(button,a,[role="button"]) > :not(:first-child):not(:last-child),
aside :is(button,a,[role="button"]) > :not(:first-child):not(:last-child),
[class*="sidebar"] :is(button,a,[role="button"]) > :not(:first-child):not(:last-child),
[class*="Sidebar"] :is(button,a,[role="button"]) > :not(:first-child):not(:last-child),
nav [data-slock-desktop-primary-row="true"] > :not(:first-child):not(:last-child),
aside [data-slock-desktop-primary-row="true"] > :not(:first-child):not(:last-child),
[class*="sidebar"] [data-slock-desktop-primary-row="true"] > :not(:first-child):not(:last-child),
[class*="Sidebar"] [data-slock-desktop-primary-row="true"] > :not(:first-child):not(:last-child) {{
  min-width: 0 !important;
  flex: 1 1 auto !important;
  overflow: hidden !important;
  text-overflow: ellipsis !important;
  white-space: nowrap !important;
  text-align: left !important;
}}

[data-slock-desktop-primary-list="true"] :is(button,a,[role="button"]):hover > :not(:last-child) {{
  overflow: hidden !important;
  text-overflow: ellipsis !important;
  white-space: nowrap !important;
}}

[data-slock-desktop-primary-list="true"] :is(.ml-auto,[class*="ml-auto"],[class*="badge"],[class*="Badge"],[class*="count"],[class*="Count"]),
nav :is(.ml-auto,[class*="ml-auto"],[class*="badge"],[class*="Badge"],[class*="count"],[class*="Count"]),
aside :is(.ml-auto,[class*="ml-auto"],[class*="badge"],[class*="Badge"],[class*="count"],[class*="Count"]),
nav [data-slock-desktop-primary-row="true"] > :last-child,
aside [data-slock-desktop-primary-row="true"] > :last-child,
[class*="sidebar"] [data-slock-desktop-primary-row="true"] > :last-child,
[class*="Sidebar"] [data-slock-desktop-primary-row="true"] > :last-child {{
  margin-left: auto !important;
  flex: 0 0 auto !important;
  text-align: right !important;
}}

[data-slock-desktop-task-state] {{
  margin-left: 0 !important;
  justify-self: start !important;
  align-self: flex-start !important;
  text-align: left !important;
}}

[data-slock-desktop-task-row="true"] {{
  display: grid !important;
  grid-template-columns: 36px minmax(0, 1fr) minmax(128px, 176px) 48px !important;
  align-items: center !important;
  justify-content: flex-start !important;
  column-gap: 12px !important;
  text-align: left !important;
}}

[data-slock-desktop-task-row="true"] > * {{
  min-width: 0 !important;
  text-align: left !important;
  justify-content: flex-start !important;
}}

[data-slock-desktop-task-row="true"] > :first-child {{
  justify-self: center !important;
  text-align: center !important;
}}

[data-slock-desktop-task-row="true"] > :not(:first-child):not(:last-child) {{
  justify-self: start !important;
  text-align: left !important;
}}

[data-slock-desktop-task-row="true"] > :last-child {{
  justify-self: end !important;
}}

[data-slock-desktop-task-row="true"] :is(.text-center,[class*="text-center"],[class*="justify-center"]) {{
  text-align: left !important;
  justify-content: flex-start !important;
}}

[data-slock-desktop-task-title="true"] {{
  flex: 1 1 auto !important;
  min-width: 0 !important;
  text-align: left !important;
  justify-content: flex-start !important;
  justify-self: start !important;
}}

[data-slock-desktop-task-row="true"] [data-slock-desktop-task-state] {{
  flex: 0 0 auto !important;
  margin-inline: 0 !important;
  justify-self: start !important;
  align-self: center !important;
}}

[data-slock-desktop-task-state-chip="true"] {{
  align-self: center !important;
  justify-self: start !important;
  text-align: left !important;
  line-height: 1 !important;
  display: inline-flex !important;
  align-items: center !important;
  justify-content: flex-start !important;
}}

[class*="task"],
[class*="Task"],
[aria-label*="task" i],
[aria-label*="任务"] {{
  text-align: left !important;
  justify-content: flex-start !important;
}}

[data-slock-desktop-menu-item="true"] {{
  min-height: 32px !important;
  justify-content: flex-start !important;
  border-color: transparent !important;
  border-radius: var(--slock-desktop-radius-sm) !important;
  background: transparent !important;
  box-shadow: none !important;
  text-align: left !important;
}}

[data-slock-desktop-menu-item="true"]:hover,
[data-slock-desktop-menu-item="true"]:focus-visible,
[data-slock-desktop-menu-item="true"][data-highlighted],
[data-slock-desktop-menu-item="true"][data-state="checked"],
[data-slock-desktop-menu-item="true"][aria-selected="true"] {{
  background: var(--slock-desktop-hover) !important;
  border-color: transparent !important;
  box-shadow: none !important;
}}

[data-slock-desktop-menu-item="true"][data-slock-desktop-task-state] {{
  position: relative !important;
  color: var(--slock-desktop-text) !important;
}}

[data-slock-desktop-menu-item="true"][data-slock-desktop-task-state]::before {{
  content: "" !important;
  width: 7px !important;
  height: 7px !important;
  border-radius: var(--slock-desktop-radius-pill) !important;
  background: var(--slock-desktop-semantic-current) !important;
  flex: 0 0 7px !important;
}}

[data-slock-desktop-account-dock="true"] {{
  background: transparent !important;
  border-color: transparent !important;
  box-shadow: none !important;
}}

[data-slock-desktop-account-action="true"] {{
  background: transparent !important;
  border-color: transparent !important;
  box-shadow: none !important;
  border-radius: var(--slock-desktop-radius-md) !important;
}}

[data-slock-desktop-account-action="true"]:hover,
[data-slock-desktop-account-action="true"]:focus-visible {{
  background: var(--slock-desktop-hover) !important;
  border-color: transparent !important;
  box-shadow: none !important;
}}

.safe-top :is(h1,h2,h3,span,p,button),
[class*="safe-top"] :is(h1,h2,h3,span,p,button),
[class*="topbar"] :is(h1,h2,h3,span,p,button),
[class*="Topbar"] :is(h1,h2,h3,span,p,button),
.flex.h-\[62px\] :is(h1,h2,h3,span,p,button),
.relative.flex.items-center.border-b-2 :is(h1,h2,h3,span,p,button) {{
  line-height: 1.28 !important;
  overflow: visible !important;
  padding-bottom: 1px !important;
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
