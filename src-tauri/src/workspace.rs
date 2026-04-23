use crate::theme;

pub fn settings_overlay_script(active_theme_id: &str) -> String {
    let themes = serde_json::to_string(&theme::meta_catalog()).unwrap_or_else(|_| "[]".into());
    let active_theme =
        serde_json::to_string(active_theme_id).unwrap_or_else(|_| "\"default\"".into());

    WORKSPACE_SETTINGS_SCRIPT
        .replace("__SLOCK_DESKTOP_THEMES__", &themes)
        .replace("__SLOCK_DESKTOP_ACTIVE_THEME__", &active_theme)
}

const WORKSPACE_SETTINGS_SCRIPT: &str = r#"
(() => {
  const hostId = "slock-desktop-settings-host";
  const themes = __SLOCK_DESKTOP_THEMES__;
  const initialThemeId = __SLOCK_DESKTOP_ACTIVE_THEME__;
  const existing = document.getElementById(hostId);
  const host = existing || document.createElement("div");

  if (!existing) {
    host.id = hostId;
    document.documentElement.appendChild(host);
  }

  const shadow = host.shadowRoot || host.attachShadow({ mode: "open" });
  let open = window.__slockDesktopSettingsOpen === true;
  let activeThemeId = initialThemeId;

  const css = `
    :host {
      --desktop-canvas: #f7f7f8;
      --desktop-surface: #ffffff;
      --desktop-surface-strong: #ececf1;
      --desktop-line: rgba(32, 33, 35, 0.14);
      --desktop-text: #202123;
      --desktop-muted: #6e6e80;
      --desktop-accent: #6a9f91;
      color: var(--desktop-text);
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }

    *, *::before, *::after {
      box-sizing: border-box;
    }

    .dock {
      position: fixed;
      right: 22px;
      bottom: 22px;
      z-index: 2147483647;
      display: grid;
      justify-items: end;
      gap: 10px;
      pointer-events: none;
    }

    button {
      font: inherit;
      touch-action: manipulation;
    }

    .launcher,
    .theme-option {
      pointer-events: auto;
      appearance: none;
      border: 0;
      cursor: pointer;
      transition:
        transform 150ms ease,
        background 150ms ease,
        box-shadow 150ms ease,
        opacity 150ms ease;
    }

    .launcher {
      min-height: 42px;
      display: inline-flex;
      align-items: center;
      gap: 9px;
      padding: 10px 14px;
      border-radius: 999px;
      background: var(--desktop-text);
      color: var(--desktop-surface);
      box-shadow: 0 14px 38px rgba(0, 0, 0, 0.18);
      font-weight: 650;
      letter-spacing: -0.01em;
    }

    .launcher-dot {
      width: 8px;
      height: 8px;
      border-radius: 999px;
      background: var(--desktop-accent);
      box-shadow: 0 0 0 4px rgba(106, 159, 145, 0.18);
    }

    .panel {
      pointer-events: auto;
      width: min(420px, calc(100vw - 28px));
      max-height: min(620px, calc(100vh - 96px));
      overflow: auto;
      border-radius: 24px;
      background: var(--desktop-surface);
      box-shadow:
        0 1px 2px rgba(0, 0, 0, 0.08),
        0 30px 90px rgba(0, 0, 0, 0.18);
      opacity: 0;
      transform: translateY(10px) scale(0.98);
      transition:
        opacity 180ms ease,
        transform 180ms ease;
    }

    .dock[data-open="true"] .panel {
      opacity: 1;
      transform: translateY(0) scale(1);
    }

    .panel-inner {
      display: grid;
      gap: 0;
      padding: 8px;
    }

    .panel-head {
      display: grid;
      gap: 6px;
      padding: 14px 14px 16px;
    }

    .eyebrow {
      margin: 0;
      color: var(--desktop-muted);
      font-size: 11px;
      font-weight: 700;
      letter-spacing: 0.08em;
      text-transform: uppercase;
    }

    h2 {
      margin: 0;
      color: var(--desktop-text);
      font-size: 18px;
      line-height: 1.25;
      letter-spacing: -0.012em;
    }

    .description {
      margin: 0;
      color: var(--desktop-muted);
      font-size: 13px;
      line-height: 1.5;
    }

    .settings-grid {
      display: grid;
      grid-template-columns: 118px minmax(0, 1fr);
      min-height: 320px;
      border-radius: 18px;
      background: var(--desktop-canvas);
      overflow: hidden;
    }

    .nav {
      display: grid;
      align-content: start;
      gap: 4px;
      padding: 10px;
      background: var(--desktop-surface-strong);
    }

    .nav-item {
      min-height: 38px;
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 8px;
      border-radius: 10px;
      color: var(--desktop-text);
      font-size: 13px;
      font-weight: 600;
    }

    .nav-item.active {
      background: var(--desktop-surface);
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
    }

    .content {
      display: grid;
      align-content: start;
      gap: 12px;
      padding: 14px;
    }

    .setting-title {
      margin: 0;
      color: var(--desktop-text);
      font-size: 13px;
      font-weight: 700;
    }

    .theme-list {
      display: grid;
      gap: 7px;
    }

    .theme-option {
      min-height: 58px;
      display: grid;
      grid-template-columns: 52px minmax(0, 1fr) 20px;
      align-items: center;
      gap: 10px;
      padding: 8px;
      border-radius: 14px;
      background: transparent;
      color: var(--desktop-text);
      text-align: left;
    }

    .theme-option.active {
      background: var(--desktop-surface);
      box-shadow:
        inset 0 0 0 1px rgba(106, 159, 145, 0.26),
        0 1px 2px rgba(0, 0, 0, 0.05);
    }

    .theme-option:disabled {
      cursor: wait;
      opacity: 0.72;
    }

    .swatch {
      display: grid;
      grid-template-columns: 1fr 1fr;
      grid-template-rows: 1fr 1fr;
      gap: 4px;
      min-height: 38px;
      padding: 4px;
      border-radius: 10px;
      background: var(--theme-canvas);
      box-shadow: inset 0 0 0 1px var(--theme-line);
    }

    .swatch span {
      border-radius: 6px;
      background: var(--theme-surface);
    }

    .swatch span:first-child {
      grid-row: span 2;
      background: var(--theme-strong);
    }

    .swatch span:last-child {
      background: var(--theme-accent);
    }

    .theme-copy {
      display: grid;
      gap: 2px;
      min-width: 0;
    }

    .theme-name {
      color: var(--desktop-text);
      font-size: 13px;
      font-weight: 700;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .theme-summary {
      color: var(--desktop-muted);
      font-size: 12px;
      line-height: 1.35;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .check {
      color: var(--desktop-accent);
      font-weight: 800;
      text-align: center;
    }

    .status {
      min-height: 32px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
      margin-top: 2px;
      padding: 8px 10px;
      border-radius: 14px;
      background: var(--desktop-surface);
      color: var(--desktop-muted);
      font-size: 12px;
    }

    @media (hover: hover) {
      .launcher:hover {
        transform: translateY(-1px);
      }

      .theme-option:hover {
        background: var(--desktop-surface);
      }
    }

    .launcher:active,
    .theme-option:active {
      transform: scale(0.96);
    }

    @media (max-width: 520px) {
      .dock {
        right: 12px;
        bottom: 12px;
      }

      .settings-grid {
        grid-template-columns: 1fr;
      }

      .nav {
        grid-auto-flow: column;
        overflow-x: auto;
      }
    }

    @media (prefers-reduced-motion: reduce) {
      .launcher,
      .theme-option,
      .panel {
        transition-duration: 1ms;
      }
    }
  `;

  const render = () => {
    shadow.innerHTML = "";

    const style = document.createElement("style");
    style.textContent = css;
    shadow.appendChild(style);

    const dock = document.createElement("div");
    dock.className = "dock";
    dock.dataset.open = String(open);

    const panel = document.createElement("section");
    panel.className = "panel";
    panel.hidden = !open;
    panel.setAttribute("role", "dialog");
    panel.setAttribute("aria-label", "Desktop settings");

    const inner = document.createElement("div");
    inner.className = "panel-inner";
    inner.innerHTML = `
      <div class="panel-head">
        <p class="eyebrow">Slock Desktop</p>
        <h2>Desktop Settings</h2>
        <p class="description">Appearance settings apply to this workspace window immediately and persist locally.</p>
      </div>
      <div class="settings-grid">
        <nav class="nav" aria-label="Desktop settings sections">
          <div class="nav-item active">Appearance</div>
          <div class="nav-item">Service</div>
          <div class="nav-item">Updates</div>
        </nav>
        <div class="content">
          <p class="setting-title">Theme</p>
          <div class="theme-list" role="radiogroup" aria-label="Workspace theme"></div>
          <div class="status">
            <span>Saved in desktop config</span>
            <span>${themes.length} themes</span>
          </div>
        </div>
      </div>
    `;

    const list = inner.querySelector(".theme-list");

    themes.forEach((theme) => {
      const selected = theme.id === activeThemeId;
      const option = document.createElement("button");
      option.className = `theme-option${selected ? " active" : ""}`;
      option.type = "button";
      option.setAttribute("role", "radio");
      option.setAttribute("aria-checked", String(selected));
      option.style.setProperty("--theme-canvas", theme.canvas);
      option.style.setProperty("--theme-surface", theme.surface);
      option.style.setProperty("--theme-strong", theme.surfaceStrong);
      option.style.setProperty("--theme-line", theme.line);
      option.style.setProperty("--theme-accent", theme.accent);

      const swatch = document.createElement("span");
      swatch.className = "swatch";
      swatch.setAttribute("aria-hidden", "true");
      swatch.innerHTML = "<span></span><span></span><span></span>";

      const copy = document.createElement("span");
      copy.className = "theme-copy";

      const name = document.createElement("span");
      name.className = "theme-name";
      name.textContent = theme.name;

      const summary = document.createElement("span");
      summary.className = "theme-summary";
      summary.textContent = theme.summary;

      const check = document.createElement("span");
      check.className = "check";
      check.setAttribute("aria-hidden", "true");
      check.textContent = selected ? "OK" : "";

      copy.append(name, summary);
      option.append(swatch, copy, check);
      option.addEventListener("click", () => setTheme(theme.id));
      list.appendChild(option);
    });

    panel.appendChild(inner);

    const launcher = document.createElement("button");
    launcher.className = "launcher";
    launcher.type = "button";
    launcher.setAttribute("aria-expanded", String(open));
    launcher.innerHTML = '<span class="launcher-dot"></span><span>Desktop Settings</span>';
    launcher.addEventListener("click", () => {
      open = !open;
      window.__slockDesktopSettingsOpen = open;
      render();
    });

    dock.append(panel, launcher);
    shadow.appendChild(dock);
  };

  const setTheme = async (themeId) => {
    activeThemeId = themeId;
    render();

    try {
      const invoke = window.__TAURI__?.core?.invoke;
      if (typeof invoke !== "function") {
        throw new Error("Tauri invoke API is unavailable");
      }
      await invoke("set_theme", { themeId });
    } catch (error) {
      console.error("[Slock Desktop] theme update failed", error);
    }
  };

  if (!window.__slockDesktopSettingsEscapeBound) {
    window.__slockDesktopSettingsEscapeBound = true;
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && window.__slockDesktopSettingsOpen) {
        open = false;
        window.__slockDesktopSettingsOpen = false;
        const activeHost = document.getElementById(hostId);
        if (activeHost) {
          const activeDock = activeHost.shadowRoot?.querySelector(".dock");
          const activePanel = activeHost.shadowRoot?.querySelector(".panel");
          const activeLauncher = activeHost.shadowRoot?.querySelector(".launcher");
          if (activeDock) activeDock.dataset.open = "false";
          if (activePanel) activePanel.hidden = true;
          if (activeLauncher) activeLauncher.setAttribute("aria-expanded", "false");
        }
      }
    });
  }

  render();
})();
"#;
