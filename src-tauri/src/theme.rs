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
pub struct CustomThemeItem {
    pub id: String,
    pub name: String,
    pub accent: String,
}

#[derive(Debug, Clone, Default)]
pub struct CustomThemeSet {
    pub items: Vec<CustomThemeItem>,
}

fn materialize_original() -> ThemeDefinition {
    ThemeDefinition {
        id: "original".to_string(),
        name: "Original".to_string(),
        summary: "Use Slock's native appearance without desktop theme overrides.".to_string(),
        mode: "system".to_string(),
        canvas: "light-dark(#f7f7f5, #1f1f1c)".to_string(),
        surface: "light-dark(#ffffff, #252623)".to_string(),
        surface_strong: "light-dark(#f3f4f1, #2f302c)".to_string(),
        line: "light-dark(#e2e4de, #3e413a)".to_string(),
        text: "light-dark(#1f1f1c, #f4f4ef)".to_string(),
        muted: "light-dark(#6b6f67, #b7bbae)".to_string(),
        accent: "light-dark(#10a37f, #19c99b)".to_string(),
        accent_soft:
            "light-dark(color-mix(in srgb, #10a37f 12%, #ffffff), color-mix(in srgb, #19c99b 22%, #1f1f1c))"
                .to_string(),
        preview: [
            "light-dark(#f7f7f5, #1f1f1c)".to_string(),
            "light-dark(#f3f4f1, #2f302c)".to_string(),
            "light-dark(#d7dbd2, #4a4d45)".to_string(),
        ],
    }
}

pub fn normalize_mode(mode: &str) -> &'static str {
    match mode {
        "light" => "light",
        "dark" => "dark",
        "system" => "system",
        _ => "system",
    }
}

pub fn meta_catalog(mode: &str, custom: &CustomThemeSet) -> Vec<ThemeMeta> {
    std::iter::once(materialize_original().into())
        .chain(
            custom
                .items
                .iter()
                .map(|item| materialize_custom_item(item, mode).into()),
        )
        .collect()
}

pub fn resolve_theme(id: &str, mode: &str, custom: &CustomThemeSet) -> ThemeDefinition {
    if id == "original" || id.is_empty() {
        return materialize_original();
    }

    custom
        .items
        .iter()
        .find(|item| item.id == id)
        .map(|item| materialize_custom_item(item, mode))
        .unwrap_or_else(materialize_original)
}

fn materialize_custom_item(item: &CustomThemeItem, mode: &str) -> ThemeDefinition {
    let name = if item.name.trim().is_empty() {
        "Custom"
    } else {
        item.name.trim()
    };
    let accent = sanitize_hex(&item.accent).unwrap_or_else(|| "#10a37f".to_string());
    materialize_theme(
        &item.id,
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
    if theme.id == "original" {
        return r#"
(() => {
  const styleId = "slock-desktop-theme";
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
  const cleanupKeys = [
    "slockDesktopAvatar",
    "slockDesktopSidebarAvatar",
    "slockDesktopAvatarHasImage",
    "slockDesktopAvatarFallback",
    "slockDesktopAvatarImageLayer",
    "slockDesktopSemanticColor",
    "slockDesktopSemanticShape",
    "slockDesktopCountTone",
    "slockDesktopTaskState",
    "slockDesktopMenuItem",
    "slockDesktopAccountDock",
    "slockDesktopAccountAction",
    "slockDesktopProfileControl",
    "slockDesktopLeftRail",
    "slockDesktopSidebarColumn",
    "slockDesktopPanelHeader",
    "slockDesktopMessageAffordance",
    "slockDesktopInlineReference",
    "slockDesktopTaskToolbar",
    "slockDesktopRoute"
  ];

  const style = document.getElementById(styleId);
  if (style) style.remove();

  document.documentElement.removeAttribute("data-slock-desktop-theme");
  document.documentElement.removeAttribute("data-slock-desktop-mode");
  document.documentElement.removeAttribute("data-slock-desktop-route");
  document.documentElement.style.removeProperty("color-scheme");

  document.querySelectorAll("*").forEach((element) => {
    if (!(element instanceof HTMLElement)) return;
    cleanupKeys.forEach((key) => delete element.dataset[key]);
    avatarStyleProps.forEach((property) => element.style.removeProperty(property));
  });
})();
"#
        .to_string();
    }

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
        '[class*="bg-brutal-red"]',
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
        Array.from(
          element.querySelectorAll(
            '[data-slock-desktop-avatar-fallback="true"],[data-slock-desktop-avatar-image-layer="true"]'
          )
        ).forEach((child) => {{
          if (!(child instanceof HTMLElement)) return;
          delete child.dataset.slockDesktopAvatarFallback;
          delete child.dataset.slockDesktopAvatarImageLayer;
        }});
      }};

      const isThreadAvatarScope = (element) => (
        element.closest('[class*="thread"],[class*="Thread"],[aria-label*="thread" i],[aria-label*="线程"],[href*="thread"]') ||
        element.closest('button.relative.flex.items-start.gap-3.border-2')
      );
      const isSidebarAvatarScope = (element) =>
        !!element.closest('nav,aside,[class*="sidebar"],[class*="Sidebar"]');

      const findAvatarImageLayer = (element) => {{
        const candidates = [element, ...Array.from(element.querySelectorAll("img,picture,canvas,div,span"))];
        return candidates.find((candidate) => {{
          if (!(candidate instanceof HTMLElement)) return false;
          if (candidate === element && !candidate.matches("img,picture,canvas")) return false;
          const computed = window.getComputedStyle(candidate);
          const backgroundImage = `${{candidate.style.backgroundImage || ""}} ${{computed.backgroundImage || ""}}`;
          return (
            candidate.matches("img,picture,canvas") ||
            /url\(/i.test(backgroundImage) ||
            candidate.hasAttribute("data-avatar-src") ||
            candidate.hasAttribute("data-profile-image")
          );
        }});
      }};

      const markAvatarElement = (element, key) => {{
        const imageLayer = findAvatarImageLayer(element);
        const hasImage = !!imageLayer;
        element.dataset[key] = "true";
        if (hasImage) element.dataset.slockDesktopAvatarHasImage = "true";
        else delete element.dataset.slockDesktopAvatarHasImage;

        if (imageLayer instanceof HTMLElement) {{
          let current = imageLayer;
          while (current instanceof HTMLElement && current !== element) {{
            current.dataset.slockDesktopAvatarImageLayer = "true";
            current = current.parentElement;
          }}
          imageLayer.dataset.slockDesktopAvatarImageLayer = "true";
        }}

        Array.from(element.children).slice(0, 4).forEach((child) => {{
          if (!(child instanceof HTMLElement)) return;
          if (child.matches("path,defs,clipPath,mask")) return;
          child.dataset[key] = "true";
          delete child.dataset.slockDesktopAvatarFallback;
          if (
            hasImage &&
            child !== imageLayer &&
            !child.contains(imageLayer) &&
            !child.matches("img,picture,canvas,svg") &&
            !child.querySelector("img,picture,canvas,[data-slock-desktop-avatar-image-layer='true']")
          ) {{
            child.dataset.slockDesktopAvatarFallback = "true";
          }}
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
          markAvatarElement(element, "slockDesktopAvatar");
        }} else if (
          nearSidebar &&
          (looksAvatar || looksSized || isImage) &&
          (hasCompactText || hasGraphic || isImage) &&
          !/badge|Badge|status|Status|presence|Presence/.test(className)
        ) {{
          markAvatarElement(element, "slockDesktopSidebarAvatar");
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

      const brutalColors = ["yellow", "orange", "red", "pink", "cyan", "lime", "lavender"];
      const resolveBrutalColor = (className) =>
        brutalColors.find((color) => className.includes(`brutal-${{color}}`)) || null;
      const resolveTaskState = (text, explicitState = "") => {{
        const state = String(explicitState || "").trim().toLowerCase().replace(/[_\s-]+/g, "-");
        if (state === "todo") return "todo";
        if (state === "in-progress" || state === "doing" || state === "open") return "in-progress";
        if (state === "in-review" || state === "review") return "in-review";
        if (state === "done" || state === "completed" || state === "closed") return "done";
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
            '[class*="ml-auto"]',
            '[data-state]',
            '[data-task-status]',
            '[data-message-affordance]',
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
          const filledCountChrome =
            /(?:^|\\s)bg-brutal-/.test(className) ||
            /(?:^|\\s)border-brutal-/.test(className);
          if (brutalColor && badgeLike) {{
            element.dataset.slockDesktopSemanticColor = brutalColor;
            element.dataset.slockDesktopSemanticShape = tinyDot ? "dot" : "chip";
          }} else {{
            delete element.dataset.slockDesktopSemanticColor;
            delete element.dataset.slockDesktopSemanticShape;
          }}

          const numericCount = /^\\d+\\+?$/.test(text);
          if (badgeLike && numericCount && !tinyDot) {{
            element.dataset.slockDesktopCountTone = filledCountChrome ? "plain" : "accent";
          }} else {{
            delete element.dataset.slockDesktopCountTone;
          }}

          const explicitTaskStatus = element.getAttribute("data-task-status");
          const taskState = resolveTaskState(text, explicitTaskStatus);
          if (taskState && (badgeLike || explicitTaskStatus)) {{
            element.dataset.slockDesktopTaskState = taskState;
          }} else {{
            delete element.dataset.slockDesktopTaskState;
          }}
        }});
    }};

    const markWorkspaceModuleSurfaces = () => {{
      if (!document.body) return;

      const routeMatch = window.location.pathname.match(/\/s\/[^/]+\/([^/?#]+)/i);
      const workspaceRoute = routeMatch?.[1]?.toLowerCase() || "";
      if (workspaceRoute) {{
        document.documentElement.dataset.slockDesktopRoute = workspaceRoute;
      }} else {{
        document.documentElement.removeAttribute("data-slock-desktop-route");
      }}

      const sidebarSelector = 'nav,aside,[class*="sidebar"],[class*="Sidebar"],.flex.h-full.w-full.flex-col[class*="border-r"],.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow,.bg-brutal-cream.relative.min-w-0.flex-1';
      const surfaceProps = [
        "slockDesktopMenuItem",
        "slockDesktopAccountDock",
        "slockDesktopAccountAction",
        "slockDesktopProfileControl",
        "slockDesktopLeftRail",
        "slockDesktopSidebarColumn",
        "slockDesktopPanelHeader",
        "slockDesktopMessageAffordance",
        "slockDesktopInlineReference",
        "slockDesktopTaskToolbar"
      ];

      document.querySelectorAll('[data-slock-desktop-menu-item],[data-slock-desktop-account-dock],[data-slock-desktop-account-action],[data-slock-desktop-profile-control],[data-slock-desktop-left-rail],[data-slock-desktop-sidebar-column],[data-slock-desktop-panel-header],[data-slock-desktop-message-affordance],[data-slock-desktop-inline-reference],[data-slock-desktop-task-toolbar]').forEach((element) => {{
        if (!(element instanceof HTMLElement)) return;
        surfaceProps.forEach((key) => delete element.dataset[key]);
      }});

      const markMenuItem = (element) => {{
        if (!(element instanceof HTMLElement)) return;
        if (element.closest('#slock-desktop-settings-host')) return;
        if (element.matches("input,textarea,select")) return;
        element.dataset.slockDesktopMenuItem = "true";
      }};

      const normalizeSurfaceText = (value) =>
        String(value || "").replace(/\s+/g, " ").trim().toLowerCase();
      document
        .querySelectorAll(
          [
            '[data-testid^="left-rail"]',
            '[data-testid="rail-bottom"]',
            '[data-testid="left-rail-settings"]',
            '[data-testid*="warning-trigger-rail"]'
          ].join(",")
        )
        .forEach((element) => {{
          if (!(element instanceof HTMLElement)) return;
          if (element.closest('#slock-desktop-settings-host')) return;
          element.dataset.slockDesktopLeftRail = "true";
          let container = element.parentElement;
          for (let depth = 0; container && depth < 5; depth += 1, container = container.parentElement) {{
            if (!(container instanceof HTMLElement)) continue;
            const rect = container.getBoundingClientRect();
            if (rect.width > 0 && rect.width <= 96 && rect.height >= 120) {{
              container.dataset.slockDesktopLeftRail = "true";
              break;
            }}
          }}
        }});

      document
        .querySelectorAll(
          [
            '.bg-brutal-cream.relative.min-w-0.flex-1',
            '.flex.h-panel-header.shrink-0.items-center.border-b-2.border-black.bg-brutal-cream',
            '.relative.flex.h-panel-header.shrink-0.items-center.gap-3.border-b-2.border-black.bg-brutal-yellow'
          ].join(",")
        )
        .forEach((element) => {{
          if (!(element instanceof HTMLElement)) return;
          if (element.closest('#slock-desktop-settings-host')) return;
          element.dataset.slockDesktopSidebarColumn = "true";
        }});

      document.querySelectorAll('.h-panel-header,[class*="h-panel-header"]').forEach((element) => {{
        if (!(element instanceof HTMLElement)) return;
        if (element.closest('#slock-desktop-settings-host')) return;
        element.dataset.slockDesktopPanelHeader = "true";
      }});

      document.querySelectorAll('[data-message-affordance]').forEach((element) => {{
        if (element instanceof HTMLElement) element.dataset.slockDesktopMessageAffordance = "true";
      }});

      document.querySelectorAll('a[data-channel],a[data-task-ref],a[data-thread-ref]').forEach((element) => {{
        if (element instanceof HTMLElement) element.dataset.slockDesktopInlineReference = "true";
      }});

      const taskToolbarSelectors = [
        'main .relative > .flex > .flex > .flex',
        'main .flex > .flex > .flex > .flex',
        'main .shrink-0 > .flex > .relative'
      ].join(',');
      document.querySelectorAll(taskToolbarSelectors).forEach((element) => {{
        if (!(element instanceof HTMLElement)) return;
        if (element.closest('#slock-desktop-settings-host,nav,aside,form')) return;
        const rect = element.getBoundingClientRect();
        const label = normalizeSurfaceText(element.textContent);
        const hasTaskAction = /new task|create task|add task|新建任务|创建任务|添加任务/.test(label);
        const hasTaskFilter =
          /(todo|to do|待办)/.test(label) &&
          /(in progress|doing|进行中)/.test(label) &&
          /(done|完成)/.test(label);
        const hasTaskViewToggle =
          /(board|kanban|看板)/.test(label) &&
          /(list|列表)/.test(label);
        const hasChannelFilter =
          /(channel|频道)/.test(label) &&
          element.querySelectorAll('button,[role="button"],.inline-flex').length <= 4;
        const matchesTaskToolbar =
          hasTaskAction && hasTaskFilter && rect.width >= 360 && rect.height >= 32 && rect.height <= 180;
        const matchesCompactControl =
          (hasTaskViewToggle || hasChannelFilter) && rect.width >= 56 && rect.width <= 420 && rect.height >= 24 && rect.height <= 96;
        if (matchesTaskToolbar || matchesCompactControl) {{
          element.dataset.slockDesktopTaskToolbar = "true";
        }}
      }});

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
          if (rect.bottom < sidebarRect.bottom - 128) return;

          action.dataset.slockDesktopAccountAction = "true";
          let container = action.parentElement;
          for (let depth = 0; container && depth < 4; depth += 1, container = container.parentElement) {{
            if (!(container instanceof HTMLElement)) continue;
            if (container.matches("nav,aside")) break;
            const containerRect = container.getBoundingClientRect();
            if (containerRect.bottom < sidebarRect.bottom - 128) continue;
            const interactiveCount = container.querySelectorAll("button,a,[role='button']").length;
            if (interactiveCount > 0 && interactiveCount <= 4) {{
              container.dataset.slockDesktopAccountDock = "true";
              break;
            }}
          }}
        }});
      }});

      const profileRoute = /[?&]profile=/.test(window.location.search) || /\/profile\b/i.test(window.location.pathname);
      if (profileRoute) {{
        document.querySelectorAll('main button, main [role="button"]').forEach((action) => {{
          if (!(action instanceof HTMLElement)) return;
          if (action.closest('#slock-desktop-settings-host,input,textarea,select,form')) return;
          const label = [
            action.textContent,
            action.getAttribute("aria-label"),
            action.getAttribute("title")
          ]
            .filter(Boolean)
            .join(" ")
            .replace(/\s+/g, " ")
            .trim()
            .toLowerCase();
          if (!label) return;
          if (
            /agent dms|agent 私信|reminders|提醒|message|发消息/.test(label) ||
            /@/.test(label)
          ) {{
            action.dataset.slockDesktopProfileControl = "true";
          }}
        }});
      }}

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
    if theme.id == "original" {
        return String::new();
    }

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
  --slock-desktop-tab-selected: color-mix(in srgb, {surface} 86%, {text} 14%);
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
  --color-brutal-red: color-mix(in srgb, #f97264 18%, {surface});
  --slock-semantic-yellow: #f2c86b;
  --slock-semantic-orange: #eb9d61;
  --slock-semantic-red: #d96657;
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
    --slock-desktop-tab-selected: color-mix(in srgb, #252623 84%, #f4f4ef 16%);
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

.border-2 > textarea.max-h-32.w-full.resize-none {{
  background: transparent !important;
  box-shadow: none !important;
  border-radius: 0 !important;
}}

.btn-brutal,
.btn-brutal-sm,
button,
[role="button"] {{
  border-radius: var(--slock-desktop-radius-md) !important;
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
.btn-brutal.bg-brutal-red,
.btn-brutal-sm.bg-brutal-pink,
.btn-brutal-sm.bg-brutal-lime,
.btn-brutal-sm.bg-brutal-cyan,
.btn-brutal-sm.bg-brutal-yellow,
.btn-brutal-sm.bg-brutal-orange,
.btn-brutal-sm.bg-brutal-red {{
  background: var(--slock-desktop-accent) !important;
  color: var(--slock-desktop-surface) !important;
  border-color: transparent !important;
}}

.btn-brutal.bg-brutal-red,
.btn-brutal-sm.bg-brutal-red {{
  background: var(--slock-semantic-red) !important;
  color: var(--slock-desktop-surface) !important;
}}

.btn-brutal:hover,
.btn-brutal-sm:hover,
button:hover,
[role="button"]:hover {{
  background: var(--slock-desktop-hover) !important;
}}

.btn-brutal.bg-brutal-red:hover,
.btn-brutal.bg-brutal-red:focus-visible,
.btn-brutal-sm.bg-brutal-red:hover,
.btn-brutal-sm.bg-brutal-red:focus-visible {{
  background: color-mix(in srgb, var(--slock-semantic-red) 86%, var(--slock-desktop-text)) !important;
  color: var(--slock-desktop-surface) !important;
}}

.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .relative.flex.items-center.border-b-2.border-black.px-4.py-3 > .relative.inline-flex.items-center.gap-1\.5.tilt-neg-2.border-2.border-black.bg-black,
.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .relative.flex.items-center.border-b-2.border-black.px-4.py-3 > .relative.inline-flex.items-center.gap-1\.5.tilt-neg-2.border-2.border-black.bg-black:hover,
.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .relative.flex.items-center.border-b-2.border-black.px-4.py-3 > .relative.inline-flex.items-center.gap-1\.5.tilt-neg-2.border-2.border-black.bg-black:focus-visible {{
  background: color-mix(in srgb, var(--slock-desktop-surface-strong) 88%, var(--slock-desktop-accent) 12%) !important;
  border-color: transparent !important;
  color: var(--slock-desktop-accent) !important;
}}

.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .flex.border-b-2.border-black > button.bg-white,
.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .flex.border-b-2.border-black > button.bg-white:hover,
.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .flex.border-b-2.border-black > button.bg-white:focus-visible {{
  background: var(--slock-desktop-selection) !important;
  color: var(--slock-desktop-text) !important;
  box-shadow: none !important;
}}

.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .flex.border-b-2.border-black > button.bg-brutal-yellow\/60 {{
  background: transparent !important;
  color: var(--slock-desktop-muted) !important;
}}

.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .flex.border-b-2.border-black > button.bg-brutal-yellow\/60:hover,
.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .flex.border-b-2.border-black > button.bg-brutal-yellow\/60:focus-visible {{
  background: var(--slock-desktop-hover) !important;
  color: var(--slock-desktop-text) !important;
}}

.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .relative.flex.items-center.border-b-2.border-black.px-4.py-3,
.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .flex.border-b-2.border-black,
.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .flex.border-b-2.border-black > button,
.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .flex.border-b-2.border-black > button:hover,
.flex.h-full.w-full.flex-col.border-r-3.border-black.bg-brutal-yellow > .flex.border-b-2.border-black > button:focus-visible {{
  border-color: transparent !important;
  box-shadow: none !important;
}}

.flex.min-h-0.flex-1.flex-col > .relative.flex.items-center.border-b-2.border-black,
.flex.min-h-0.flex-1.flex-col > .flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none {{
  border-color: transparent !important;
  box-shadow: none !important;
}}

.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.bg-brutal-yellow,
.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.bg-brutal-yellow:hover,
.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.bg-brutal-yellow:focus-visible {{
  background: var(--slock-desktop-selection) !important;
  color: var(--slock-desktop-text) !important;
  box-shadow: none !important;
}}

.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.bg-white,
.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.bg-white:hover,
.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.bg-white:focus-visible,
.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.hover\:bg-brutal-cream,
.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.hover\:bg-brutal-cream:hover,
.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.hover\:bg-brutal-cream:focus-visible {{
  background: transparent !important;
  color: var(--slock-desktop-muted) !important;
}}

.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.bg-white:hover,
.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.bg-white:focus-visible,
.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.hover\:bg-brutal-cream:hover,
.flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.hover\:bg-brutal-cream:focus-visible {{
  background: var(--slock-desktop-hover) !important;
  color: var(--slock-desktop-text) !important;
}}

[class*="bg-brutal-yellow"],
[class*="bg-brutal-orange"],
[class*="hover\:bg-brutal-yellow"],
[class*="hover\:bg-brutal-orange"] {{
  background-color: var(--slock-desktop-surface-secondary) !important;
}}

[class*="bg-brutal-red"],
[class*="hover\:bg-brutal-red"] {{
  background-color: color-mix(in srgb, var(--slock-semantic-red) 18%, var(--slock-desktop-surface)) !important;
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

[class*="bg-brutal-yellow\/40"],
[class*="bg-brutal-lavender\/40"],
[class*="bg-brutal-pink\/40"],
[class*="bg-brutal-cyan\/40"],
[class*="bg-brutal-lime\/40"],
[class*="bg-brutal-orange\/40"],
[class*="bg-brutal-red\/20"],
[class*="bg-brutal-red\/40"],
[class*="bg-brutal-red\/60"] {{
  background-color: transparent !important;
  border-color: transparent !important;
}}

[class*="bg-brutal-yellow\/40"]:hover,
[class*="bg-brutal-yellow\/40"]:focus-visible,
[class*="bg-brutal-lavender\/40"]:hover,
[class*="bg-brutal-lavender\/40"]:focus-visible,
[class*="bg-brutal-pink\/40"]:hover,
[class*="bg-brutal-pink\/40"]:focus-visible,
[class*="bg-brutal-cyan\/40"]:hover,
[class*="bg-brutal-cyan\/40"]:focus-visible,
[class*="bg-brutal-lime\/40"]:hover,
[class*="bg-brutal-lime\/40"]:focus-visible,
[class*="bg-brutal-orange\/40"]:hover,
[class*="bg-brutal-orange\/40"]:focus-visible,
[class*="bg-brutal-red\/20"]:hover,
[class*="bg-brutal-red\/20"]:focus-visible,
[class*="bg-brutal-red\/40"]:hover,
[class*="bg-brutal-red\/40"]:focus-visible,
[class*="bg-brutal-red\/60"]:hover,
[class*="bg-brutal-red\/60"]:focus-visible {{
  background-color: var(--slock-desktop-hover) !important;
  border-color: var(--slock-desktop-line) !important;
}}

span[class*="text-[10px]"][class*="border"][class*="bg-brutal-"],
span[class*="text-[10px]"][class*="border-black"][class*="bg-brutal-"] {{
  border-radius: 2px !important;
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

[class*="text-brutal-red"] {{
  color: color-mix(in srgb, var(--slock-semantic-red) 86%, var(--slock-desktop-text)) !important;
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

.bg-brutal-red,
.bg-brutal-red\/20,
.bg-brutal-red\/60 {{
  background-color: color-mix(in srgb, var(--slock-semantic-red) 18%, var(--slock-desktop-surface)) !important;
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
.hover\:bg-brutal-red:hover,
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
[data-slock-desktop-panel-header="true"],
[class*="titlebar"],
[class*="Titlebar"],
[class*="topbar"],
[class*="Topbar"],
[class*="toolbar"],
[class*="Toolbar"],
[role="banner"],
header,
.h-panel-header,
.flex.h-\[62px\],
.flex.h-\[54px\],
.flex.h-\[62px\].shrink-0,
.relative.flex.items-center,
.flex.overflow-x-auto,
.shrink-0.border-b-2,
.md\:hidden.shrink-0,
[class*="h-panel-header"],
[class*="border-b-2"].bg-white {{
  background: var(--slock-desktop-surface) !important;
  border-color: var(--slock-desktop-line) !important;
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

nav button[aria-current="page"]:not([data-slock-desktop-account-action="true"]),
nav button[aria-selected="true"]:not([data-slock-desktop-account-action="true"]),
nav button[data-state="active"]:not([data-slock-desktop-account-action="true"]),
nav button[data-active="true"]:not([data-slock-desktop-account-action="true"]),
aside button[aria-current="page"]:not([data-slock-desktop-account-action="true"]),
aside button[aria-selected="true"]:not([data-slock-desktop-account-action="true"]),
aside button[data-state="active"]:not([data-slock-desktop-account-action="true"]),
aside button[data-active="true"]:not([data-slock-desktop-account-action="true"]),
.group.flex.items-center[aria-current="page"]:not([data-slock-desktop-account-action="true"]),
.group.flex.items-center[aria-selected="true"]:not([data-slock-desktop-account-action="true"]),
.group.flex.items-center[data-state="active"]:not([data-slock-desktop-account-action="true"]),
.group.flex.items-center[data-active="true"]:not([data-slock-desktop-account-action="true"]),
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
nav .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]),
aside .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]),
[class*="sidebar"] .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]),
[class*="Sidebar"] .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]),
.group.flex.items-center.border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]),
button.border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]),
.flex.w-full.items-center.gap-1\.5.border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]),
.flex.w-full.items-center.gap-2.border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]) {{
  background: var(--slock-desktop-selection) !important;
  color: var(--slock-desktop-text) !important;
  box-shadow: none !important;
}}

[class*="sidebar"] [class*="bg-brutal"],
[class*="Sidebar"] [class*="bg-brutal"],
aside [class*="bg-brutal"],
nav [class*="bg-brutal"] {{
  background: transparent !important;
}}

button[data-slock-desktop-account-action="true"],
a[data-slock-desktop-account-action="true"],
[role="button"][data-slock-desktop-account-action="true"],
[data-slock-desktop-account-dock="true"] :is(button,a,[role="button"]) {{
  background: transparent !important;
  border-color: transparent !important;
  box-shadow: none !important;
}}

[class*="sidebar"] [aria-current="page"]:not([data-slock-desktop-account-action="true"]),
[class*="Sidebar"] [aria-current="page"]:not([data-slock-desktop-account-action="true"]),
[class*="sidebar"] [aria-selected="true"]:not([data-slock-desktop-account-action="true"]),
[class*="Sidebar"] [aria-selected="true"]:not([data-slock-desktop-account-action="true"]),
[class*="sidebar"] [data-state="active"]:not([data-slock-desktop-account-action="true"]),
[class*="Sidebar"] [data-state="active"]:not([data-slock-desktop-account-action="true"]),
aside [aria-current="page"]:not([data-slock-desktop-account-action="true"]),
aside [aria-selected="true"]:not([data-slock-desktop-account-action="true"]),
aside [data-state="active"]:not([data-slock-desktop-account-action="true"]),
nav [aria-current="page"]:not([data-slock-desktop-account-action="true"]),
nav [aria-selected="true"]:not([data-slock-desktop-account-action="true"]),
nav [data-state="active"]:not([data-slock-desktop-account-action="true"]),
nav .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]),
aside .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]),
[class*="sidebar"] .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]),
[class*="Sidebar"] .border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]),
button.border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]),
.flex.w-full.items-center.gap-1\.5.border-black.bg-brutal-pink.shadow-brutal-sm.font-bold:not([data-slock-desktop-account-action="true"]) {{
  background: var(--slock-desktop-selection) !important;
  color: var(--slock-desktop-text) !important;
  box-shadow: none !important;
}}

[data-slock-desktop-semantic-color="yellow"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-yellow);
}}

[data-slock-desktop-semantic-color="orange"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-orange);
}}

[data-slock-desktop-semantic-color="red"] {{
  --slock-desktop-semantic-current: var(--slock-semantic-red);
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

[data-slock-desktop-semantic-color][data-slock-desktop-semantic-shape="chip"]:not([data-slock-desktop-account-action="true"]),
[data-slock-desktop-task-state] {{
  background: color-mix(in srgb, var(--slock-desktop-semantic-current) 18%, var(--slock-desktop-surface)) !important;
  border-color: color-mix(in srgb, var(--slock-desktop-semantic-current) 38%, var(--slock-desktop-line)) !important;
  color: color-mix(in srgb, var(--slock-desktop-semantic-current) 78%, var(--slock-desktop-text)) !important;
  box-shadow: none !important;
}}

[data-slock-desktop-count-tone] {{
  background: transparent !important;
  border-color: transparent !important;
  box-shadow: none !important;
  border-radius: 0 !important;
}}

[data-slock-desktop-count-tone="plain"] {{
  color: var(--slock-desktop-muted) !important;
  font-weight: 500 !important;
}}

[data-slock-desktop-count-tone="accent"] {{
  color: color-mix(in srgb, var(--slock-semantic-pink) 90%, var(--slock-desktop-text)) !important;
  font-weight: 650 !important;
}}

:is(p, li, .break-words, [class*="select-text"]) :is(a, code)[data-slock-desktop-semantic-shape="chip"],
:is(p, li, .break-words, [class*="select-text"]) :is(a, code)[class*="bg-brutal-"] {{
  background: transparent !important;
  box-shadow: none !important;
}}

:is(p, li, .break-words, [class*="select-text"]) :is(a, code)[data-slock-desktop-semantic-shape="chip"]:hover,
:is(p, li, .break-words, [class*="select-text"]) :is(a, code)[data-slock-desktop-semantic-shape="chip"]:focus-visible,
:is(p, li, .break-words, [class*="select-text"]) :is(a, code)[class*="bg-brutal-"]:hover,
:is(p, li, .break-words, [class*="select-text"]) :is(a, code)[class*="bg-brutal-"]:focus-visible {{
  background: var(--slock-desktop-hover) !important;
}}

[data-slock-desktop-avatar="true"][data-slock-desktop-avatar-has-image="true"],
[data-slock-desktop-sidebar-avatar="true"][data-slock-desktop-avatar-has-image="true"] {{
  background: transparent !important;
  color: transparent !important;
}}

[data-slock-desktop-avatar-fallback="true"] {{
  opacity: 0 !important;
}}

[data-slock-desktop-avatar-image-layer="true"] {{
  border-radius: inherit !important;
  background-color: transparent !important;
  background-position: center !important;
  background-repeat: no-repeat !important;
  background-size: cover !important;
  color: transparent !important;
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

.text-brutal-red {{
  color: color-mix(in srgb, var(--slock-semantic-red) 86%, var(--slock-desktop-text)) !important;
}}

h1,
h2,
h3,
h4,
h5,
h6 {{
  color: var(--slock-desktop-text) !important;
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
  box-shadow: none !important;
}}

.flex.min-h-0.flex-1.flex-col > :is(.relative.flex.items-center, .flex.overflow-x-auto)[class*="border-b-2"],
.flex.min-h-0.flex-1.flex-col > :is(.relative.flex.items-center, .flex.overflow-x-auto)[class*="border-t-2"],
.flex.min-h-0.flex-1.flex-col > .flex.h-\[62px\],
.flex.min-h-0.flex-1.flex-col > .flex.h-\[62px\].shrink-0,
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .flex[class*="border"],
.flex.min-h-0.flex-1.flex-col > .flex > .flex > .flex > .flex[class*="border"],
.flex.min-h-0.flex-1.flex-col > .relative > .absolute > .flex.h-\[62px\][class*="border"],
.flex.min-h-0.flex-1.flex-col > .relative > .absolute > .flex > .flex[class*="border"],
.flex.min-h-0.flex-1.flex-col > .flex > .relative > .absolute > .flex[class*="border"],
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0[class*="border"],
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .shrink-0.border-b-2,
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .relative.flex.items-center.border-b-2,
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .flex.items-start.gap-2.border-b-2,
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .flex.h-\[62px\],
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .border-b-2.bg-white {{
  border-top-color: transparent !important;
  border-bottom-color: transparent !important;
  box-shadow: none !important;
}}

.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .flex,
.flex.min-h-0.flex-1.flex-col > .flex > .flex > .flex > .flex,
.flex.min-h-0.flex-1.flex-col > .relative > .absolute > .flex.h-\[62px\],
.flex.min-h-0.flex-1.flex-col > .relative > .absolute > .flex > .flex,
.flex.min-h-0.flex-1.flex-col > .flex > .relative > .absolute > .flex,
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0,
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .shrink-0.border-b-2,
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .relative.flex.items-center.border-b-2,
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .flex.items-start.gap-2.border-b-2,
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .flex.h-\[62px\],
.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .border-b-2.bg-white {{
  background: var(--slock-desktop-canvas) !important;
}}

main :where(.flex.min-h-0.flex-1.flex-col) > .relative > .flex > .flex,
main :where(.flex.min-h-0.flex-1.flex-col) > .relative > .flex > .flex > .flex {{
  background: transparent !important;
}}

html[data-slock-desktop-route="search"] main .relative > .flex > .flex > .shrink-0 {{
  background: var(--slock-desktop-canvas) !important;
  border-top-color: transparent !important;
  border-bottom-color: transparent !important;
  box-shadow: none !important;
}}

.flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .flex,
.flex.min-h-0.flex-1.flex-col > .flex > .flex > .flex > .flex {{
  background: transparent !important;
}}

[data-slock-desktop-task-toolbar="true"] {{
  background: var(--slock-desktop-canvas) !important;
  border-top-color: transparent !important;
  border-bottom-color: transparent !important;
  box-shadow: none !important;
}}

[data-slock-desktop-task-toolbar="true"] button.bg-white,
[data-slock-desktop-task-toolbar="true"] button.hover\:bg-brutal-cream,
[data-slock-desktop-task-toolbar="true"] button[class*="bg-brutal-yellow\/40"],
[data-slock-desktop-task-toolbar="true"] button[class*="bg-brutal-cream"],
[data-slock-desktop-task-toolbar="true"] > .inline-flex.bg-white,
[data-slock-desktop-task-toolbar="true"] > .inline-flex[class*="bg-brutal-yellow\/40"],
[data-slock-desktop-task-toolbar="true"] > [role="button"].bg-white,
[data-slock-desktop-task-toolbar="true"] > [role="button"][class*="bg-brutal-yellow\/40"] {{
  background: transparent !important;
  border-color: transparent !important;
  color: var(--slock-desktop-muted) !important;
  box-shadow: none !important;
}}

[data-slock-desktop-task-toolbar="true"] button.bg-white:hover,
[data-slock-desktop-task-toolbar="true"] button.bg-white:focus-visible,
[data-slock-desktop-task-toolbar="true"] button.hover\:bg-brutal-cream:hover,
[data-slock-desktop-task-toolbar="true"] button.hover\:bg-brutal-cream:focus-visible,
[data-slock-desktop-task-toolbar="true"] button[class*="bg-brutal-yellow\/40"]:hover,
[data-slock-desktop-task-toolbar="true"] button[class*="bg-brutal-yellow\/40"]:focus-visible,
[data-slock-desktop-task-toolbar="true"] > .inline-flex.bg-white:hover,
[data-slock-desktop-task-toolbar="true"] > .inline-flex.bg-white:focus-visible,
[data-slock-desktop-task-toolbar="true"] > .inline-flex[class*="bg-brutal-yellow\/40"]:hover,
[data-slock-desktop-task-toolbar="true"] > .inline-flex[class*="bg-brutal-yellow\/40"]:focus-visible,
[data-slock-desktop-task-toolbar="true"] > [role="button"].bg-white:hover,
[data-slock-desktop-task-toolbar="true"] > [role="button"].bg-white:focus-visible,
[data-slock-desktop-task-toolbar="true"] > [role="button"][class*="bg-brutal-yellow\/40"]:hover,
[data-slock-desktop-task-toolbar="true"] > [role="button"][class*="bg-brutal-yellow\/40"]:focus-visible {{
  background: var(--slock-desktop-hover) !important;
  color: var(--slock-desktop-text) !important;
}}

[data-slock-desktop-task-toolbar="true"] button.bg-brutal-yellow,
[data-slock-desktop-task-toolbar="true"] button[aria-selected="true"],
[data-slock-desktop-task-toolbar="true"] button[aria-pressed="true"],
[data-slock-desktop-task-toolbar="true"] button[data-state="active"],
[data-slock-desktop-task-toolbar="true"] button[data-active="true"] {{
  background: var(--slock-desktop-selection) !important;
  border-color: transparent !important;
  color: var(--slock-desktop-text) !important;
  box-shadow: none !important;
}}

[data-slock-desktop-task-toolbar="true"].inline-flex[class*="bg-brutal"]:not([class*="bg-brutal-yellow\/40"]):not([class*="bg-brutal-cream"]),
[data-slock-desktop-task-toolbar="true"][role="button"][class*="bg-brutal"]:not([class*="bg-brutal-yellow\/40"]):not([class*="bg-brutal-cream"]),
[data-slock-desktop-task-toolbar="true"] > .inline-flex[class*="bg-brutal"]:not([class*="bg-brutal-yellow\/40"]):not([class*="bg-brutal-cream"]),
[data-slock-desktop-task-toolbar="true"] > [role="button"][class*="bg-brutal"]:not([class*="bg-brutal-yellow\/40"]):not([class*="bg-brutal-cream"]) {{
  background: var(--slock-desktop-selection) !important;
  border-color: transparent !important;
  color: var(--slock-desktop-text) !important;
  box-shadow: none !important;
}}

[data-slock-desktop-left-rail="true"] {{
  background: color-mix(in srgb, var(--slock-desktop-surface-strong) 88%, var(--slock-desktop-accent) 12%) !important;
  border-color: var(--slock-desktop-line) !important;
  box-shadow: none !important;
}}

button[data-slock-desktop-left-rail="true"],
[role="button"][data-slock-desktop-left-rail="true"],
[data-slock-desktop-left-rail="true"] :is(button,a,[role="button"]) {{
  background: transparent !important;
  border-color: transparent !important;
  box-shadow: none !important;
  color: var(--slock-desktop-muted) !important;
}}

button[data-slock-desktop-left-rail="true"]:hover,
button[data-slock-desktop-left-rail="true"]:focus-visible,
[role="button"][data-slock-desktop-left-rail="true"]:hover,
[role="button"][data-slock-desktop-left-rail="true"]:focus-visible,
[data-slock-desktop-left-rail="true"] :is(button,a,[role="button"]):hover,
[data-slock-desktop-left-rail="true"] :is(button,a,[role="button"]):focus-visible {{
  background: var(--slock-desktop-hover) !important;
  color: var(--slock-desktop-text) !important;
}}

[data-slock-desktop-left-rail="true"] :is([aria-current="page"],[aria-selected="true"],[data-state="active"],[data-active="true"]),
button[data-slock-desktop-left-rail="true"][aria-current="page"],
button[data-slock-desktop-left-rail="true"][aria-selected="true"],
button[data-slock-desktop-left-rail="true"][data-state="active"],
button[data-slock-desktop-left-rail="true"][data-active="true"] {{
  background: var(--slock-desktop-selection) !important;
  color: var(--slock-desktop-text) !important;
}}

[data-slock-desktop-sidebar-column="true"] {{
  background: var(--slock-desktop-canvas) !important;
  border-color: var(--slock-desktop-line) !important;
  box-shadow: none !important;
}}

[data-slock-desktop-panel-header="true"] {{
  background: var(--slock-desktop-surface) !important;
  border-color: var(--slock-desktop-line) !important;
  box-shadow: none !important;
}}

.md\:bg-white {{
  background: var(--slock-desktop-surface) !important;
}}

[data-select-screenshot-root],
[data-slock-desktop-message-affordance="true"] {{
  background: var(--slock-desktop-surface) !important;
  border-color: var(--slock-desktop-line) !important;
  color: var(--slock-desktop-text) !important;
  box-shadow: var(--slock-desktop-soft-shadow) !important;
}}

[data-slock-desktop-message-affordance="true"]:hover,
[data-slock-desktop-message-affordance="true"]:focus-visible {{
  background: var(--slock-desktop-hover) !important;
}}

[data-slock-desktop-inline-reference="true"] {{
  display: inline-flex !important;
  align-items: center !important;
  max-width: 100% !important;
  min-height: 1.45em !important;
  padding: 0 0.36em !important;
  border: 1px solid color-mix(in srgb, var(--slock-desktop-accent) 30%, var(--slock-desktop-line)) !important;
  border-radius: var(--slock-desktop-radius-xs) !important;
  background: var(--slock-desktop-accent-soft) !important;
  color: color-mix(in srgb, var(--slock-desktop-accent) 72%, var(--slock-desktop-text)) !important;
  box-shadow: none !important;
  vertical-align: baseline !important;
}}

[data-slock-desktop-inline-reference="true"]:hover,
[data-slock-desktop-inline-reference="true"]:focus-visible {{
  background: color-mix(in srgb, var(--slock-desktop-accent) 18%, var(--slock-desktop-surface)) !important;
  color: var(--slock-desktop-text) !important;
}}

main .shrink-0 > .flex > .relative > .inline-flex[class*="bg-brutal"]:not([class*="bg-brutal-yellow\/40"]):not([class*="bg-brutal-cream"]),
main .shrink-0 > .flex > .relative > button.inline-flex[class*="bg-brutal"]:not([class*="bg-brutal-yellow\/40"]):not([class*="bg-brutal-cream"]),
main .shrink-0 > .flex > .relative > [role="button"].inline-flex[class*="bg-brutal"]:not([class*="bg-brutal-yellow\/40"]):not([class*="bg-brutal-cream"]),
main .shrink-0 > .flex > .relative > .inline-flex[class*="bg-brutal-yellow\/40"],
main .shrink-0 > .flex > .relative > button.inline-flex[class*="bg-brutal-yellow\/40"],
main .shrink-0 > .flex > .relative > [role="button"].inline-flex[class*="bg-brutal-yellow\/40"] {{
  background: var(--slock-desktop-selection) !important;
  border-color: transparent !important;
  color: var(--slock-desktop-text) !important;
  box-shadow: none !important;
}}

input.min-w-0[placeholder*="Search channels"],
input.min-w-0[placeholder*="搜索频道"],
input[placeholder*="Search channels, DMs, messages"],
input[placeholder*="搜索频道、私信、消息"],
main input.min-w-0[placeholder*="Search channels"],
main input.min-w-0[placeholder*="搜索频道"],
main input[placeholder*="Search channels, DMs, messages"],
main input[placeholder*="搜索频道、私信、消息"] {{
  border-radius: var(--slock-desktop-radius-xs) !important;
  background: transparent !important;
  background-color: transparent !important;
  background-clip: padding-box !important;
  padding-inline: 14px !important;
  box-shadow: none !important;
}}

.flex.w-full.items-center.gap-2,
.group.flex.items-center {{
  border-radius: var(--slock-desktop-radius-sm) !important;
}}

.flex.w-full.items-center.gap-2:hover,
.group.flex.items-center:hover {{
  background: var(--slock-desktop-hover) !important;
}}

.border-collapse,
table {{
  border-color: var(--slock-desktop-line) !important;
  border-radius: var(--slock-desktop-radius-md) !important;
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

.ml-auto.shrink-0.rounded:not([data-slock-desktop-count-tone]),
.rounded.bg-brutal-pink:not([data-slock-desktop-count-tone]),
.inline-flex.items-center.gap-1.border:not([data-slock-desktop-count-tone]),
.inline-flex.items-center.gap-1\.5.border:not([data-slock-desktop-count-tone]),
.inline-flex.items-center.px-1\.5:not([data-slock-desktop-count-tone]),
.shrink-0.inline-flex.items-center:not([data-slock-desktop-count-tone]) {{
  border-color: var(--slock-desktop-line) !important;
  border-radius: var(--slock-desktop-radius-pill) !important;
  background: var(--slock-desktop-accent-soft) !important;
  color: var(--slock-desktop-text) !important;
}}

.animate-pulse {{
  background: var(--slock-desktop-accent-soft) !important;
  border-color: color-mix(in srgb, var(--slock-desktop-accent) 24%, var(--slock-desktop-line)) !important;
}}

@media (hover: hover) {{
  a:hover,
  [role="link"]:hover {{
    color: color-mix(in srgb, var(--slock-desktop-accent) 82%, var(--slock-desktop-text)) !important;
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

[data-slock-desktop-semantic-color][data-slock-desktop-semantic-shape="chip"]:not([data-slock-desktop-account-action="true"]),
[data-slock-desktop-task-state] {{
  background: color-mix(in srgb, var(--slock-desktop-semantic-current) 18%, var(--slock-desktop-surface)) !important;
  border-color: color-mix(in srgb, var(--slock-desktop-semantic-current) 38%, var(--slock-desktop-line)) !important;
  color: color-mix(in srgb, var(--slock-desktop-semantic-current) 78%, var(--slock-desktop-text)) !important;
  box-shadow: none !important;
}}

[data-slock-desktop-account-dock="true"] :is(button,a,[role="button"]),
[data-slock-desktop-account-action="true"],
nav [class*="btn-brutal-sm"][data-slock-desktop-account-action="true"],
aside [class*="btn-brutal-sm"][data-slock-desktop-account-action="true"] {{
  border-color: transparent !important;
  box-shadow: none !important;
}}

[data-slock-desktop-profile-control="true"] {{
  border-color: transparent !important;
  box-shadow: none !important;
}}

[data-slock-desktop-account-dock="true"] :is(button,a,[role="button"],div,span)[class*="border"],
[data-slock-desktop-account-action="true"] :is(div,span)[class*="border"],
[data-slock-desktop-account-action="true"] :is(div,span)[class*="btn-brutal"] {{
  border-color: transparent !important;
  box-shadow: none !important;
  background: transparent !important;
}}

[data-slock-desktop-menu-item="true"] {{
  border-color: transparent !important;
  border-radius: var(--slock-desktop-radius-sm) !important;
  background: transparent !important;
  box-shadow: none !important;
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
  color: var(--slock-desktop-text) !important;
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

[data-slock-desktop-account-action="true"][class*="bg-brutal-pink"] {{
  background: var(--slock-desktop-accent) !important;
  color: var(--slock-desktop-surface) !important;
  border-color: transparent !important;
}}

[data-slock-desktop-account-action="true"]:hover,
[data-slock-desktop-account-action="true"]:focus-visible {{
  background: var(--slock-desktop-hover) !important;
  border-color: transparent !important;
  box-shadow: none !important;
}}

:focus-visible {{
  outline-color: var(--slock-desktop-accent) !important;
}}

svg {{
  color: inherit !important;
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

#[cfg(test)]
mod tests {
    use super::{injected_script, resolve_theme, CustomThemeItem, CustomThemeSet};

    fn fixture_set() -> CustomThemeSet {
        CustomThemeSet {
            items: vec![CustomThemeItem {
                id: "default".to_string(),
                name: "Custom".to_string(),
                accent: "#10a37f".to_string(),
            }],
        }
    }

    #[test]
    fn injected_script_contains_unread_count_tone_fixups() {
        let script = injected_script(resolve_theme("default", "light", &fixture_set()));

        assert!(script.contains("'[class*=\"ml-auto\"]'"));
        assert!(script.contains(
            "element.dataset.slockDesktopCountTone = filledCountChrome ? \"plain\" : \"accent\";"
        ));
        assert!(script.contains("var(--slock-semantic-pink) 90%"));
        assert!(script.contains(
            "[data-slock-desktop-account-action=\\\"true\\\"][class*=\\\"bg-brutal-pink\\\"]"
        ));
        assert!(!script.contains(
            "[data-slock-desktop-account-action=\\\"true\\\"][class*=\\\"bg-brutal-lime\\\"]"
        ));
    }

    #[test]
    fn injected_script_paints_inner_workspace_chrome_like_main_background() {
        let script = injected_script(resolve_theme("default", "light", &fixture_set()));

        assert!(
            script.contains(".flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .flex")
        );
        assert!(script.contains(
            ".flex.min-h-0.flex-1.flex-col > .flex > .flex > .flex > .flex {\\n  background: transparent !important;"
        ));
        assert!(script.contains(
            ".flex.min-h-0.flex-1.flex-col > .relative > .absolute > .flex.h-\\\\[62px\\\\]"
        ));
        assert!(script
            .contains(".flex.min-h-0.flex-1.flex-col > .relative > .absolute > .flex > .flex"));
        assert!(!script.contains(".flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex,\n"));
        assert!(!script.contains(".flex.min-h-0.flex-1.flex-col > .flex > .flex > .flex,\n"));
        assert!(script
            .contains(".flex.min-h-0.flex-1.flex-col > .flex > .relative > .absolute > .flex"));
        assert!(script.contains(
            r#".flex.min-h-0.flex-1.flex-col > .relative > .absolute > .flex > .flex[class*=\"border\"]"#
        ));
        assert!(script.contains(
            r#".flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0[class*=\"border\"]"#
        ));
        assert!(script.contains(
            ".flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .shrink-0.border-b-2"
        ));
        assert!(script.contains(
            ".flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .flex.items-start.gap-2.border-b-2"
        ));
        assert!(script.contains(
            ".flex.min-h-0.flex-1.flex-col > .relative > .flex > .flex > .shrink-0 > .border-b-2.bg-white"
        ));
        assert!(script.contains("slockDesktopTaskToolbar"));
        assert!(script.contains(".flex.h-full.w-full.flex-col[class*=\"border-r\"]"));
        assert!(script.contains("data-slock-desktop-task-toolbar"));
        assert!(script.contains("slockDesktopRoute"));
        assert!(script.contains(
            "html[data-slock-desktop-route=\\\"search\\\"] main .relative > .flex > .flex > .shrink-0"
        ));
        assert!(script.contains("background: var(--slock-desktop-canvas) !important;"));
        assert!(script.contains(r#"input.min-w-0[placeholder*=\"Search channels\"]"#));
        assert!(script.contains(r#"input.min-w-0[placeholder*=\"搜索频道\"]"#));
        assert!(script.contains(r#"main input.min-w-0[placeholder*=\"Search channels\"]"#));
        assert!(script.contains(r#"main input.min-w-0[placeholder*=\"搜索频道\"]"#));
        assert!(script.contains("padding-inline: 14px !important;"));
        assert!(!script.contains("canvasChromeSelectors"));
        assert!(!script.contains("querySelectorAll(canvasChromeSelectors)"));
        assert!(!script.contains("slockDesktopCanvasChrome"));
    }

    #[test]
    fn injected_script_normalizes_task_page_selected_controls() {
        let script = injected_script(resolve_theme("default", "light", &fixture_set()));

        assert!(script.contains(
            ".flex.overflow-x-auto.border-b-2.border-black.bg-white.scrollbar-none > button.bg-brutal-yellow"
        ));
        assert!(script
            .contains("[data-slock-desktop-task-toolbar=\\\"true\\\"] button.bg-brutal-yellow"));
        assert!(script.contains("hasTaskViewToggle"));
        assert!(script.contains("hasChannelFilter"));
        assert!(script.contains(
            "[data-slock-desktop-task-toolbar=\\\"true\\\"] > .inline-flex[class*=\\\"bg-brutal\\\"]"
        ));
        assert!(script.contains(
            "main .shrink-0 > .flex > .relative > .inline-flex[class*=\\\"bg-brutal\\\"]"
        ));
        assert!(script.contains(
            "main .shrink-0 > .flex > .relative > .inline-flex[class*=\\\"bg-brutal-yellow\\\\/40\\\"]"
        ));
        assert!(!script.contains("background: var(--slock-desktop-tab-selected) !important;"));
        assert!(!script.contains(
            "box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--slock-desktop-accent) 20%, transparent)"
        ));
    }

    #[test]
    fn injected_script_removes_channel_chrome_and_search_input_backgrounds() {
        let script = injected_script(resolve_theme("default", "light", &fixture_set()));

        assert!(script
            .contains("main :where(.flex.min-h-0.flex-1.flex-col) > .relative > .flex > .flex"));
        assert!(script.contains(".flex.min-h-0.flex-1.flex-col > .flex > .flex > .flex > .flex"));
        assert!(script.contains(
            "html[data-slock-desktop-route=\\\"search\\\"] main .relative > .flex > .flex > .shrink-0"
        ));
        assert!(script.contains(r#"input[placeholder*=\"搜索频道、私信、消息\"]"#));
        assert!(script.contains("background-color: transparent !important;"));
        assert!(script.contains("background-clip: padding-box !important;"));
    }

    #[test]
    fn injected_script_adapts_current_workspace_data_markers() {
        let script = injected_script(resolve_theme("default", "light", &fixture_set()));

        assert!(script.contains("bg-brutal-red"));
        assert!(script.contains("--color-brutal-red"));
        assert!(script.contains("--slock-semantic-red"));
        assert!(script.contains("[data-task-status]"));
        assert!(script.contains(r#"element.getAttribute("data-task-status")"#));
        assert!(script.contains(r#"[data-testid^="left-rail"]"#));
        assert!(script.contains("slockDesktopLeftRail"));
        assert!(script.contains("slockDesktopSidebarColumn"));
        assert!(script.contains("slockDesktopPanelHeader"));
        assert!(script.contains("h-panel-header"));
        assert!(script.contains(".md\\\\:bg-white"));
        assert!(script.contains("[data-message-affordance]"));
        assert!(script.contains("slockDesktopMessageAffordance"));
        assert!(script.contains("a[data-channel],a[data-task-ref],a[data-thread-ref]"));
        assert!(script.contains("slockDesktopInlineReference"));
    }

    #[test]
    fn original_theme_cleans_current_workspace_data_markers() {
        let script = injected_script(resolve_theme("original", "system", &fixture_set()));

        assert!(script.contains("slockDesktopLeftRail"));
        assert!(script.contains("slockDesktopSidebarColumn"));
        assert!(script.contains("slockDesktopPanelHeader"));
        assert!(script.contains("slockDesktopMessageAffordance"));
        assert!(script.contains("slockDesktopInlineReference"));
    }
}
