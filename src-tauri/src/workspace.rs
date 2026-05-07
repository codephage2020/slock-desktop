use crate::theme;

pub fn settings_overlay_script(
    active_theme_id: &str,
    active_style_id: &str,
    active_theme_mode: &str,
    active_language: &str,
    resolved_language: &str,
    themes: &[theme::ThemeMeta],
    styles: &[theme::ThemeStyleMeta],
) -> String {
    let themes = serde_json::to_string(themes).unwrap_or_else(|_| "[]".into());
    let styles = serde_json::to_string(styles).unwrap_or_else(|_| "[]".into());
    let active_theme =
        serde_json::to_string(active_theme_id).unwrap_or_else(|_| "\"default\"".into());
    let active_style =
        serde_json::to_string(active_style_id).unwrap_or_else(|_| "\"default\"".into());
    let active_mode =
        serde_json::to_string(active_theme_mode).unwrap_or_else(|_| "\"system\"".into());
    let active_language =
        serde_json::to_string(active_language).unwrap_or_else(|_| "\"system\"".into());
    let resolved_language =
        serde_json::to_string(resolved_language).unwrap_or_else(|_| "\"en-US\"".into());

    WORKSPACE_SETTINGS_SCRIPT
        .replace("__SLOCK_DESKTOP_THEMES__", &themes)
        .replace("__SLOCK_DESKTOP_STYLES__", &styles)
        .replace("__SLOCK_DESKTOP_ACTIVE_THEME__", &active_theme)
        .replace("__SLOCK_DESKTOP_ACTIVE_STYLE__", &active_style)
        .replace("__SLOCK_DESKTOP_ACTIVE_MODE__", &active_mode)
        .replace("__SLOCK_DESKTOP_ACTIVE_LANGUAGE__", &active_language)
        .replace("__SLOCK_DESKTOP_RESOLVED_LANGUAGE__", &resolved_language)
}

const WORKSPACE_SETTINGS_SCRIPT: &str = r#"
(() => {
  const hostId = "slock-desktop-settings-host";
  const themes = __SLOCK_DESKTOP_THEMES__;
  const styles = __SLOCK_DESKTOP_STYLES__;
  const initialThemeId = __SLOCK_DESKTOP_ACTIVE_THEME__;
  const initialStyleId = __SLOCK_DESKTOP_ACTIVE_STYLE__;
  const initialMode = __SLOCK_DESKTOP_ACTIVE_MODE__;
  const initialLanguage = __SLOCK_DESKTOP_ACTIVE_LANGUAGE__;
  const initialResolvedLanguage = __SLOCK_DESKTOP_RESOLVED_LANGUAGE__;
  let serviceSnapshot = window.__slockDesktopServiceSnapshot || null;
  let themeCatalog = window.__slockDesktopThemeCatalog || themes;
  let styleCatalog = window.__slockDesktopStyleCatalog || styles;
  let updateSnapshot = window.__slockDesktopUpdateSnapshot || null;
  let serviceLogViewer = window.__slockDesktopServiceLogViewer || null;
  let serviceLogSearchTimer = null;
  let serviceLogSearchToken = 0;
  let releaseState = window.__slockDesktopReleaseState || {
    loading: false,
    installing: false,
    error: null,
    latest: null,
  };
  const hydrateReleaseStateFromUpdateSnapshot = () => {
    const latest = updateSnapshot?.latest;
    if (!latest) return;
    releaseState = {
      ...releaseState,
      loading: false,
      error: null,
      latest,
    };
    window.__slockDesktopReleaseState = releaseState;
  };
  if (releaseState.loading) {
    releaseState = { ...releaseState, loading: false };
    window.__slockDesktopReleaseState = releaseState;
  }
  hydrateReleaseStateFromUpdateSnapshot();
  let releaseCheckInFlight = false;
  let newThemeDraft = null;
  let serviceBusyAction = null;
  let serviceError = null;
  let appearanceBusyAction = null;
  let updateBusyAction = null;
  let titlebarThemeMenuOpen = false;
  let titlebarThemeWheelOpen = false;
  let titlebarStyleMenuOpen = false;
  let releaseNotesOpen = false;
  let agentCardTarget = null;
  let agentCardActivity = [];
  let agentCardLoading = false;
  let agentCardAction = null;
  let dashboardAgents = window.__slockDesktopDashboardAgents || [];
  const waitForNextPaint = () => new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  });
  const modes = [
    { id: "light", icon: "sun", key: "modeLight" },
    { id: "dark", icon: "moon", key: "modeDark" },
    { id: "system", icon: "display", key: "modeSystem" },
  ];
  const languages = [
    { id: "en-US", icon: "latin", key: "languageEnglish" },
    { id: "zh-CN", icon: "han", key: "languageChinese" },
    { id: "system", icon: "globe", key: "languageSystem" },
  ];
  const themeAccentPresets = [
    '#ff3b30',
    '#ff9500',
    '#ffcc00',
    '#34c759',
    '#32ade6',
    '#007aff',
    '#af52de',
  ];
  const optionIcon = (type, className = "option-icon") => {
    if (type === "han") {
      return `<svg class="${className} han-icon" aria-hidden="true" viewBox="0 0 1024 1024" fill="currentColor"><path d="M555.231787 330.203429v-107.997284h-68.202727v108.038827H263.433935v273.457531H487.02906v210.976899h68.202727V603.70431h224.21827V330.203429H555.231787z m-68.202727 209.074952h-157.337694v-144.605675h157.335888v144.605675z m226.131053 0H555.195662v-144.605675h157.962645v144.605675z"></path></svg>`;
    }
    if (type === "latin") {
      return `<svg class="${className}" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><path d="M7 18 12 6l5 12"></path><path d="M9.2 14h5.6"></path></svg>`;
    }
    if (type === "sun") {
      return `<svg class="${className}" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"></circle><path d="M12 2v2"></path><path d="M12 20v2"></path><path d="m4.9 4.9 1.4 1.4"></path><path d="m17.7 17.7 1.4 1.4"></path><path d="M2 12h2"></path><path d="M20 12h2"></path><path d="m4.9 19.1 1.4-1.4"></path><path d="m17.7 6.3 1.4-1.4"></path></svg>`;
    }
    if (type === "moon") {
      return `<svg class="${className}" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20.4 14.5A7.7 7.7 0 0 1 9.5 3.6 8.7 8.7 0 1 0 20.4 14.5Z"></path></svg>`;
    }
    if (type === "display") {
      return `<svg class="${className}" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="16" height="11" x="4" y="5" rx="2"></rect><path d="M12 16v3"></path><path d="M8 19h8"></path></svg>`;
    }
    return `<svg class="${className}" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"></circle><path d="M3 12h18"></path><path d="M12 3a14 14 0 0 1 0 18"></path><path d="M12 3a14 14 0 0 0 0 18"></path></svg>`;
  };
  const paletteIcon = () =>
    `<svg class="titlebar-theme-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="13.5" cy="6.5" r=".5" fill="currentColor"></circle><circle cx="17.5" cy="10.5" r=".5" fill="currentColor"></circle><circle cx="8.5" cy="7.5" r=".5" fill="currentColor"></circle><circle cx="6.5" cy="12.5" r=".5" fill="currentColor"></circle><path d="M12 22a10 10 0 1 1 10-10 4 4 0 0 1-4 4h-1.5a2.5 2.5 0 0 0 0 5H12Z"></path></svg>`;
  const logIcon = () =>
    `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="16" x="4" y="3" rx="2"></rect><path d="M8 8h6"></path><path d="M8 12h4"></path><circle cx="16.5" cy="16.5" r="2.5"></circle><path d="m18.5 18.5 2 2"></path></svg>`;
  const plusIcon = () =>
    `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14"></path><path d="M5 12h14"></path></svg>`;
  const editIcon = () =>
    `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"></path><path d="M16.5 3.5a2.1 2.1 0 1 1 3 3L7 19l-4 1 1-4Z"></path></svg>`;
  const closeIcon = () =>
    `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"></path><path d="m6 6 12 12"></path></svg>`;
  const checkIcon = () =>
    `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12 4 4 10-10"></path></svg>`;
  const searchIcon = () =>
    `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"></circle><path d="m20 20-3.2-3.2"></path></svg>`;
  const calendarIcon = () =>
    `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="16" height="16" x="4" y="5" rx="2"></rect><path d="M8 3v4"></path><path d="M16 3v4"></path><path d="M4 10h16"></path></svg>`;
  const clockIcon = () =>
    `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="8"></circle><path d="M12 8v5"></path><path d="m12 13 3 2"></path></svg>`;
  const chevronIcon = (direction) =>
    direction === "up"
      ? `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m18 15-6-6-6 6"></path></svg>`
      : `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"></path></svg>`;
  const backIcon = () =>
    `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"></path></svg>`;
  const styleIcon = () =>
    `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2.7 2 7l10 5 10-5-10-4.3Z"></path><path d="m2 17 10 5 10-5"></path><path d="m2 12 10 5 10-5"></path></svg>`;
  const spinnerIcon = () =>
    `<svg class="option-icon spin" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56"></path></svg>`;
  const normalizeHexColor = (value) => {
    const compact = String(value || "").trim().replace(/^#/, "");
    if (/^[0-9a-fA-F]{3}$/.test(compact)) {
      return `#${compact.split("").map((part) => `${part}${part}`).join("")}`.toLowerCase();
    }
    if (/^[0-9a-fA-F]{6}$/.test(compact)) {
      return `#${compact}`.toLowerCase();
    }
    return null;
  };
  const hexToRgb = (hex) => {
    const normalized = normalizeHexColor(hex) || '#10a37f';
    return {
      r: parseInt(normalized.slice(1, 3), 16),
      g: parseInt(normalized.slice(3, 5), 16),
      b: parseInt(normalized.slice(5, 7), 16),
    };
  };
  const rgbToHex = (r, g, b) =>
    `#${[r, g, b].map((value) => Number(value).toString(16).padStart(2, "0")).join("")}`;
  const hsvToHex = (hue, saturation, value) => {
    const chroma = value * saturation;
    const huePrime = ((((hue % 360) + 360) % 360) / 60);
    const x = chroma * (1 - Math.abs((huePrime % 2) - 1));
    const match = value - chroma;
    const [r1, g1, b1] =
      huePrime < 1
        ? [chroma, x, 0]
        : huePrime < 2
          ? [x, chroma, 0]
          : huePrime < 3
            ? [0, chroma, x]
            : huePrime < 4
              ? [0, x, chroma]
              : huePrime < 5
                ? [x, 0, chroma]
                : [chroma, 0, x];
    return rgbToHex(
      Math.round((r1 + match) * 255),
      Math.round((g1 + match) * 255),
      Math.round((b1 + match) * 255),
    );
  };
  const rgbToHsv = (r, g, b) => {
    const red = r / 255;
    const green = g / 255;
    const blue = b / 255;
    const max = Math.max(red, green, blue);
    const min = Math.min(red, green, blue);
    const delta = max - min;
    const saturation = max === 0 ? 0 : delta / max;
    let hue = 0;
    if (delta !== 0) {
      if (max === red) hue = 60 * (((green - blue) / delta) % 6);
      else if (max === green) hue = 60 * ((blue - red) / delta + 2);
      else hue = 60 * ((red - green) / delta + 4);
    }
    return { h: (hue + 360) % 360, s: saturation, v: max };
  };
  const accentFromWheelPointer = (event, target) => {
    const rect = target.getBoundingClientRect();
    const radius = Math.min(rect.width, rect.height) / 2;
    const dx = event.clientX - (rect.left + rect.width / 2);
    const dy = event.clientY - (rect.top + rect.height / 2);
    const distance = Math.min(radius, Math.hypot(dx, dy));
    const saturation = radius === 0 ? 0 : distance / radius;
    const hue = (Math.atan2(dy, dx) * 180) / Math.PI + 180;
    return hsvToHex(hue, saturation, 0.96);
  };
  const accentWheelMarkerStyle = (accent) => {
    const rgb = hexToRgb(accent);
    const hsv = rgbToHsv(rgb.r, rgb.g, rgb.b);
    const angle = (hsv.h - 180) * (Math.PI / 180);
    const radius = Math.max(0.08, Math.min(1, hsv.s)) * 46;
    const x = 50 + Math.cos(angle) * radius;
    const y = 50 + Math.sin(angle) * radius;
    return `--wheel-x:${x}%;--wheel-y:${y}%;--custom-accent:${escapeHtml(accent)}`;
  };
  const makeThemeDraft = (accent = '#10a37f', name = "") => {
    const normalized = normalizeHexColor(accent) || '#10a37f';
    const rgb = hexToRgb(normalized);
    return {
      name,
      accent: normalized,
      hexInput: normalized.toUpperCase(),
      rgbInput: {
        r: String(rgb.r),
        g: String(rgb.g),
        b: String(rgb.b),
      },
    };
  };
  const syncThemeDraftAccent = (draft, accent) => {
    const normalized = normalizeHexColor(accent) || draft.accent || '#10a37f';
    const rgb = hexToRgb(normalized);
    return {
      ...draft,
      accent: normalized,
      hexInput: normalized.toUpperCase(),
      rgbInput: {
        r: String(rgb.r),
        g: String(rgb.g),
        b: String(rgb.b),
      },
    };
  };
  const sanitizeRgbInput = (value) => String(value || "").replace(/\D/g, "").slice(0, 3);
  const parseRgbInput = (input) => {
    const r = Number(input.r);
    const g = Number(input.g);
    const b = Number(input.b);
    if (!input.r || !input.g || !input.b) return null;
    if ([r, g, b].some((value) => !Number.isInteger(value) || value < 0 || value > 255)) return null;
    return { r, g, b };
  };
  const copy = {
    "en-US": {
      eyebrow: "Slock Desktop",
      title: "Desktop Settings",
      settingsSections: "Desktop settings sections",
      description: "Appearance settings apply to this workspace window immediately and persist locally.",
      appearance: "Appearance",
      service: "Service",
      updates: "Updates",
      serverSettings: "Server settings",
      serverSettingsDescription: "Manage the selected desktop server from this workspace.",
      serverUrl: "Server URL",
      selectedServer: "Selected server",
      selectedServerPlaceholder: "Choose a server",
      serverSearch: "Find server",
      noMatchingServers: "No matching servers.",
      serviceStatus: "Service status",
      serviceRunning: "running",
      serviceIdle: "not running",
      serviceOffline: "not running",
      serviceNotLinked: "no local binding",
      serviceSignInRequired: "sign in required",
      machineStatus: "Machine status",
      noServers: "No servers available on this account yet.",
      serviceSignInHint: "Open Slock once, sign in, and the desktop settings will sync your server list.",
      loadingService: "Loading server settings...",
      refreshServers: "Refresh servers",
      refreshingServers: "Refreshing...",
      openServerLog: "View server logs",
      serverLogTitle: "Server logs",
      serverLogSearch: "Search logs",
      serverLogSearching: "Searching...",
      serverLogFrom: "From",
      serverLogTo: "To",
      serverLogRange: "Range",
      serverLogCustomRange: "Custom",
      serverLogRangeApply: "Load range",
      serverLogQuick30s: "30s",
      serverLogQuick1m: "1m",
      serverLogQuick5m: "5m",
      serverLogQuick30m: "30m",
      serverLogQuick1h: "1h",
      serverLogLoading: "Loading logs...",
      serverLogEmpty: "Log is empty.",
      serverLogPath: "Log file",
      serverLogTruncated: "Showing recent log tail",
      serverLogPreviousMatch: "Previous match",
      serverLogNextMatch: "Next match",
      serverLogNoMatches: "No matches",
      serverLogLines: "lines",
      startService: "Start service",
      closeServer: "Close server",
      closingServer: "Closing...",
      serviceNotRunning: "Selected server service is not running.",
      openSelectedServer: "Open selected server",
      openingServer: "Opening...",
      startingService: "Starting...",
      saving: "Saving...",
      mode: "Mode",
      modeLight: "Light",
      modeDark: "Dark",
      modeSystem: "System",
      theme: "Theme",
      language: "Language",
      languageEnglish: "English",
      languageChinese: "Chinese",
      languageSystem: "System",
      saved: "Saved in desktop config",
      themes: "themes",
      themeNewLabel: "New theme",
      themeCreate: "Create",
      themeRename: "Rename",
      themeDelete: "Delete",
      themeRenameSave: "Save",
      themeRenameCancel: "Cancel",
      themeEmptyHint: "No custom themes yet.",
      themeNamePlaceholder: "Untitled theme",
      themeAccent: "Accent",
      creatingTheme: "Creating...",
      deletingTheme: "Deleting...",
      updatesTitle: "Desktop version",
      currentVersion: "Current version",
      updateAvailable: "Update available",
      upToDate: "Up to date",
      notChecked: "Not checked",
      checkUpdates: "Check for updates",
      checkingUpdates: "Checking...",
      installUpdate: "Update",
      installingUpdate: "Updating...",
      updateCheckFailed: "Update check failed.",
      noReleaseNotes: "No release notes were provided.",
      releaseNotes: "Release notes",
      close: "Close",
      backToLauncher: "Back to launcher",
      themeStyle: "Style",
      themeStyleOriginalName: "Original style",
      themeStyleOriginalSummary: "Current web UI without desktop overrides.",
      themeStyleDefaultName: "Default style",
      themeStyleDefaultSummary: "Desktop refined style.",
      themeImportStyle: "Import style",
      themeExportStyle: "Export style",
      themeImportInvalid: "Invalid style file.",
      agentNoDescription: "No description",
      agentActivity: "Recent Activity",
      agentNoActivity: "No recent activity",
      agentStop: "Stop",
      agentStart: "Start",
      agentRestart: "Restart",
      agentStopping: "Stopping\u2026",
      agentStarting: "Starting\u2026",
      agents: "Agents",
      themeNames: {
        default: "Default accent",
        original: "Original",
      },
      themeSummaries: {
        default: "Slock green.",
        original: "Keep the native Slock look with no desktop theme injection.",
      },
    },
    "zh-CN": {
      eyebrow: "Slock 桌面端",
      title: "桌面设置",
      settingsSections: "桌面设置分区",
      description: "外观设置会立即应用到当前工作页窗口，并保存在本地。",
      appearance: "外观",
      service: "服务",
      updates: "更新",
      serverSettings: "Server 设置",
      serverSettingsDescription: "在当前工作页管理桌面端选中的 server。",
      serverUrl: "Server URL",
      selectedServer: "已选 Server",
      selectedServerPlaceholder: "选择一个 server",
      serverSearch: "搜索 server",
      noMatchingServers: "没有匹配的 server。",
      serviceStatus: "服务状态",
      serviceRunning: "运行中",
      serviceIdle: "未运行",
      serviceOffline: "未运行",
      serviceNotLinked: "未创建本地绑定",
      serviceSignInRequired: "需要登录",
      machineStatus: "本地 machine 状态",
      noServers: "当前账号下还没有可用 server。",
      serviceSignInHint: "先打开一次 Slock 并完成登录，桌面设置会同步 server 列表。",
      loadingService: "正在读取 server 设置...",
      refreshServers: "刷新 Server",
      refreshingServers: "刷新中...",
      openServerLog: "查看 server 日志",
      serverLogTitle: "Server 日志",
      serverLogSearch: "搜索日志",
      serverLogSearching: "搜索中...",
      serverLogFrom: "开始",
      serverLogTo: "结束",
      serverLogRange: "范围",
      serverLogCustomRange: "自定义",
      serverLogRangeApply: "加载时间范围",
      serverLogQuick30s: "30秒",
      serverLogQuick1m: "1分钟",
      serverLogQuick5m: "5分钟",
      serverLogQuick30m: "30分钟",
      serverLogQuick1h: "1小时",
      serverLogLoading: "正在读取日志...",
      serverLogEmpty: "日志为空。",
      serverLogPath: "日志文件",
      serverLogTruncated: "正在显示最近的日志尾部",
      serverLogPreviousMatch: "上一条匹配",
      serverLogNextMatch: "下一条匹配",
      serverLogNoMatches: "没有匹配项",
      serverLogLines: "行",
      startService: "启动服务",
      closeServer: "关闭 Server",
      closingServer: "关闭中...",
      serviceNotRunning: "所选 server 服务未运行。",
      openSelectedServer: "打开所选 Server",
      openingServer: "打开中...",
      startingService: "启动中...",
      saving: "保存中...",
      mode: "模式",
      modeLight: "亮色",
      modeDark: "暗黑",
      modeSystem: "系统",
      theme: "主题",
      language: "语言",
      languageEnglish: "英文",
      languageChinese: "中文",
      languageSystem: "系统",
      saved: "已保存到桌面配置",
      themes: "个主题",
      themeNewLabel: "新建主题",
      themeCreate: "创建",
      themeRename: "重命名",
      themeDelete: "删除",
      themeRenameSave: "保存",
      themeRenameCancel: "取消",
      themeEmptyHint: "还没有自定义主题。",
      themeNamePlaceholder: "未命名主题",
      themeAccent: "强调色",
      creatingTheme: "创建中...",
      deletingTheme: "删除中...",
      updatesTitle: "桌面版本",
      currentVersion: "当前版本",
      updateAvailable: "有可用更新",
      upToDate: "已是最新",
      notChecked: "未检查",
      checkUpdates: "检查更新",
      checkingUpdates: "检查中...",
      installUpdate: "更新",
      installingUpdate: "更新中...",
      updateCheckFailed: "更新检查失败。",
      noReleaseNotes: "此版本没有提供发布说明。",
      releaseNotes: "发布说明",
      close: "关闭",
      backToLauncher: "返回启动页",
      themeStyle: "样式",
      themeStyleOriginalName: "原样式",
      themeStyleOriginalSummary: "保留当前 Web UI 原始样式。",
      themeStyleDefaultName: "默认样式",
      themeStyleDefaultSummary: "Desktop 整理后的样式。",
      themeImportStyle: "导入样式",
      themeExportStyle: "导出样式",
      themeImportInvalid: "样式文件无效。",
      agentNoDescription: "无描述",
      agentActivity: "最近活动",
      agentNoActivity: "暂无活动记录",
      agentStop: "停止",
      agentStart: "启动",
      agentRestart: "重启",
      agentStopping: "停止中\u2026",
      agentStarting: "启动中\u2026",
      agents: "Agent",
      themeNames: {
        original: "原主题",
        default: "默认主题色",
        light: "雾蓝",
        dark: "靛蓝",
        graphite: "石墨",
        crimson: "玫瑰",
        custom: "自定义",
      },
      themeSummaries: {
        original: "保持 Slock 原生外观，不注入桌面主题样式。",
        default: "Slock 绿色。",
        light: "适合安静操作视图的柔和蓝色强调色。",
        dark: "适合结构化专注的低饱和靛蓝强调色。",
        graphite: "适合长时间会话的低饱和灰蓝强调色。",
        crimson: "适合编辑型工作区的温暖玫瑰强调色。",
        custom: "用户定义的个人强调色主题。",
      },
    },
  };
  const existing = document.getElementById(hostId);
  const host = existing || document.createElement("div");
  const chromeSafeAreaStyleId = "slock-desktop-titlebar-safe-area";

  if (!existing) {
    host.id = hostId;
    document.documentElement.appendChild(host);
  }

  const shadow = host.shadowRoot || host.attachShadow({ mode: "open" });
  let activeThemeId = initialThemeId;
  let activeStyleId = initialStyleId;
  let activeMode = initialMode;
  let activeLanguage = initialLanguage;
  const resolveLanguage = () => {
    if (activeLanguage === "zh-CN" || activeLanguage === "en-US") {
      return activeLanguage;
    }
    if (initialResolvedLanguage === "zh-CN" || initialResolvedLanguage === "en-US") {
      return initialResolvedLanguage;
    }
    return navigator.language?.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US";
  };
  const t = (key) => copy[resolveLanguage()][key];
  const escapeHtml = (value) =>
    String(value ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  const countLogLines = (content) => {
    const text = String(content || "");
    if (!text) return 0;
    let lines = 1;
    for (let index = 0; index < text.length; index += 1) {
      const code = text.charCodeAt(index);
      if (code === 10) {
        lines += 1;
      } else if (code === 13) {
        lines += 1;
        if (text.charCodeAt(index + 1) === 10) index += 1;
      }
    }
    return lines;
  };
  const emptyLogSearch = (query = "", activeMatchIndex = 0, searching = false) => ({
    query,
    activeMatchIndex,
    count: 0,
    activeStart: -1,
    activeEnd: -1,
    searching,
  });
  const currentLogSearch = (viewer) => {
    const query = String(viewer?.query || "").trim();
    const search = viewer?.search;
    if (search && search.query === query && search.activeMatchIndex === (viewer.activeMatchIndex || 0)) {
      return search;
    }
    return emptyLogSearch(query, viewer?.activeMatchIndex || 0, Boolean(query));
  };
  const scanLogMatchesInChunks = (content, query, activeMatchIndex, token) => {
    const text = String(content || "");
    const needle = String(query || "").trim().toLowerCase();
    if (!needle) return emptyLogSearch("", 0, false);

    const chunkSize = 64 * 1024;
    const needleLength = needle.length;
    let scanStart = 0;
    let count = 0;
    let activeStart = -1;
    let lastMatchStart = -1;

    const finish = (result) => {
      if (token !== serviceLogSearchToken) return;
      if (!serviceLogViewer || String(serviceLogViewer.query || "").trim() !== query) return;
      serviceLogViewer = { ...serviceLogViewer, search: result };
      window.__slockDesktopServiceLogViewer = serviceLogViewer;
      updateServiceLogSearchUi();
      applyServiceLogHighlight();
    };

    const scanNextChunk = () => {
      if (token !== serviceLogSearchToken) return;
      const acceptedEnd = Math.min(text.length, scanStart + chunkSize);
      const chunkEnd = Math.min(text.length, acceptedEnd + needleLength - 1);
      const chunk = text.slice(scanStart, chunkEnd).toLowerCase();
      let cursor = 0;
      while (cursor < chunk.length) {
        const next = chunk.indexOf(needle, cursor);
        if (next === -1) break;
        const matchStart = scanStart + next;
        if (matchStart >= acceptedEnd) break;
        if (count === activeMatchIndex) activeStart = matchStart;
        lastMatchStart = matchStart;
        count += 1;
        cursor = next + needleLength;
      }
      scanStart += chunkSize;
      if (scanStart < text.length) {
        serviceLogSearchTimer = window.setTimeout(scanNextChunk, 0);
        return;
      }
      const resolvedStart = activeStart >= 0 ? activeStart : lastMatchStart;
      finish({
        query,
        activeMatchIndex,
        count,
        activeStart: resolvedStart,
        activeEnd: resolvedStart >= 0 ? resolvedStart + needleLength : -1,
        searching: false,
      });
    };

    serviceLogSearchTimer = window.setTimeout(scanNextChunk, 0);
    return emptyLogSearch(query, activeMatchIndex, true);
  };
  const serviceLogStatus = (viewer) => {
    const content = viewer?.snapshot?.content || "";
    const query = String(viewer?.query || "").trim();
    const search = currentLogSearch(viewer);
    if (!query) return `${countLogLines(content)} ${t("serverLogLines")}`;
    if (search.searching) return t("serverLogSearching");
    if (search.count > 0) {
      return `${Math.min(viewer.activeMatchIndex || 0, search.count - 1) + 1}/${search.count}`;
    }
    return t("serverLogNoMatches");
  };
  const clearServiceLogHighlight = () => {
    shadow.querySelectorAll("mark[data-service-log-highlight]").forEach((mark) => {
      const parent = mark.parentNode;
      mark.replaceWith(document.createTextNode(mark.textContent || ""));
      parent?.normalize?.();
    });
  };
  const getServiceLogTextRange = (container, start, end) => {
    if (!container || start < 0 || end <= start) return null;
    const range = document.createRange();
    const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT);
    let offset = 0;
    let node = walker.nextNode();
    let started = false;
    while (node) {
      const length = node.textContent?.length || 0;
      const nextOffset = offset + length;
      if (!started && start >= offset && start <= nextOffset) {
        range.setStart(node, start - offset);
        started = true;
      }
      if (started && end >= offset && end <= nextOffset) {
        range.setEnd(node, end - offset);
        return range;
      }
      offset = nextOffset;
      node = walker.nextNode();
    }
    return null;
  };
  const scrollServiceLogRangeIntoView = (range, container) => {
    if (!range || !container) return;
    const rangeRect = range.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();
    if (rangeRect.width === 0 && rangeRect.height === 0) return;
    const top = rangeRect.top - containerRect.top + container.scrollTop - container.clientHeight / 2;
    container.scrollTo({ top: Math.max(0, top), behavior: "smooth" });
  };
  const applyServiceLogHighlight = () => {
    const viewer = serviceLogViewer;
    const search = currentLogSearch(viewer);
    const content = shadow.querySelector("[data-service-log-content]");
    if (!viewer || search.searching || search.activeStart < 0 || search.activeEnd <= search.activeStart) {
      clearServiceLogHighlight();
      return;
    }
    const range = getServiceLogTextRange(content, search.activeStart, search.activeEnd);
    if (!range) {
      clearServiceLogHighlight();
      return;
    }
    clearServiceLogHighlight();
    const mark = document.createElement("mark");
    mark.className = "active";
    mark.dataset.serviceLogHighlight = "true";
    try {
      range.surroundContents(mark);
    } catch (_) {
      const fragment = range.extractContents();
      mark.append(fragment);
      range.insertNode(mark);
    }
    scrollServiceLogRangeIntoView(range, content);
  };
  const updateServiceLogSearchUi = () => {
    const viewer = serviceLogViewer;
    if (!viewer) {
      clearServiceLogHighlight();
      return;
    }
    const search = currentLogSearch(viewer);
    const status = shadow.querySelector("[data-service-log-count]");
    if (status) status.textContent = serviceLogStatus(viewer);
    shadow.querySelectorAll("[data-service-log-step]").forEach((button) => {
      button.disabled = search.searching || search.count === 0;
    });
  };
  const scheduleServiceLogSearch = ({ immediate = false } = {}) => {
    const viewer = serviceLogViewer;
    window.clearTimeout(serviceLogSearchTimer);
    serviceLogSearchToken += 1;
    if (!viewer?.snapshot) {
      clearServiceLogHighlight();
      return;
    }
    const query = String(viewer.query || "").trim();
    if (!query) {
      serviceLogViewer = { ...viewer, search: emptyLogSearch() };
      window.__slockDesktopServiceLogViewer = serviceLogViewer;
      updateServiceLogSearchUi();
      clearServiceLogHighlight();
      return;
    }
    const token = serviceLogSearchToken;
    serviceLogViewer = {
      ...viewer,
      search: emptyLogSearch(query, viewer.activeMatchIndex || 0, true),
    };
    window.__slockDesktopServiceLogViewer = serviceLogViewer;
    updateServiceLogSearchUi();
    serviceLogSearchTimer = window.setTimeout(() => {
      scanLogMatchesInChunks(
        serviceLogViewer?.snapshot?.content || "",
        query,
        serviceLogViewer?.activeMatchIndex || 0,
        token,
      );
    }, immediate ? 0 : 120);
  };
  const invokeDesktop = async (command, args = {}) => {
    const invoke = window.__TAURI__?.core?.invoke;
    if (typeof invoke !== "function") {
      throw new Error("Tauri invoke API is unavailable");
    }
    return invoke(command, args);
  };
  const startWindowDrag = async () => {
    try {
      await invokeDesktop("start_window_drag", {});
    } catch (error) {
      console.warn("[Slock Desktop] window drag failed", error);
    }
  };
  const normalizeStatus = (status) => String(status || "").trim().toLowerCase();
	  const machineStatusLabel = (status) => {
	    const normalized = normalizeStatus(status);
	    if (["online", "running", "healthy", "idle", "ready"].includes(normalized)) return t("serviceRunning");
	    if (["not linked", "unbound", "missing"].includes(normalized)) return t("serviceNotLinked");
	    if (!normalized || ["offline", "stopped"].includes(normalized)) return t("serviceOffline");
	    return status;
	  };
	  const actionIcon = (name, busy = false) => {
	    if (busy) {
	      return `<svg class="service-action-icon spinning" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 0 1-9 9"></path><path d="M3 12a9 9 0 0 1 9-9"></path></svg>`;
	    }
	    if (name === "start") {
	      return `<svg class="service-action-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="9 7 17 12 9 17 9 7"></polygon></svg>`;
	    }
	    if (name === "stop") {
	      return `<svg class="service-action-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v9"></path><path d="M18.4 6.6a8 8 0 1 1-12.8 0"></path></svg>`;
	    }
	    return `<svg class="service-action-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 2v6h-6"></path><path d="M21 8A9 9 0 1 0 12 21a9 9 0 0 0 8.2-5.3"></path></svg>`;
	  };
	  const selectedServiceServer = () => {
    const service = serviceSnapshot;
    if (!service) return null;
    return (
      service.servers?.find((server) => server.slug === service.selectedServerSlug) ||
      service.servers?.find((server) => server.selected) ||
      null
    );
  };
  const serviceServerIsRunning = (service, serverSlug) => {
    const activeSlug = String(service?.activeServerSlug || "").trim();
    const selectedSlug = String(serverSlug || "").trim();
    return !!(service?.running && activeSlug && selectedSlug && activeSlug === selectedSlug);
  };
  const serviceStatusText = () => {
    const service = serviceSnapshot;
    if (!service) return t("loadingService");
    const selected = selectedServiceServer();
    const selectedServerSlug = selected?.slug || service.selectedServerSlug || "";
    const selectedRunning = serviceServerIsRunning(service, selectedServerSlug);
    if (selectedRunning) return t("serviceRunning");
    if (!service.authenticated) return t("serviceSignInRequired");
    if (selected) return service.configured ? t("serviceIdle") : t("serviceNotLinked");
    return service.configured ? t("serviceIdle") : t("serviceNotLinked");
  };
  const syncDesktopPayload = (payload) => {
    if (!payload) return;
    if (payload.service) {
      serviceSnapshot = payload.service;
      window.__slockDesktopServiceSnapshot = serviceSnapshot;
    }
    if (payload.themes) {
      themeCatalog = payload.themes;
      window.__slockDesktopThemeCatalog = themeCatalog;
    }
    if (payload.themeStyles) {
      styleCatalog = payload.themeStyles;
      window.__slockDesktopStyleCatalog = styleCatalog;
    }
    if (payload.updates) {
      updateSnapshot = payload.updates;
      window.__slockDesktopUpdateSnapshot = updateSnapshot;
      hydrateReleaseStateFromUpdateSnapshot();
    }
    if (payload.colorScheme) activeThemeId = payload.colorScheme;
    if (payload.styleScheme) activeStyleId = payload.styleScheme;
    if (payload.appearanceMode) activeMode = payload.appearanceMode;
    if (payload.language) activeLanguage = payload.language;
    serviceError = null;
  };
  const loadServiceSnapshot = async (command = "bootstrap", args = {}, busy = "service-load") => {
    serviceBusyAction = busy;
    render();
    try {
      const payload = await invokeDesktop(command, args);
      syncDesktopPayload(payload);
    } catch (error) {
      serviceError = error?.message || String(error);
      console.warn("[Slock Desktop] service settings sync failed", error);
    } finally {
      serviceBusyAction = null;
      render();
    }
  };
  const refreshServiceSnapshot = async () => {
    serviceBusyAction = "service-refresh";
    render();
    try {
      let payload = await invokeDesktop("refresh_service_server_catalog", {});
      syncDesktopPayload(payload);
      serviceBusyAction = "service-status";
      render();
      await waitForNextPaint();
      payload = await invokeDesktop("refresh_service_server_status", {});
      syncDesktopPayload(payload);
    } catch (error) {
      serviceError = error?.message || String(error);
      console.warn("[Slock Desktop] service settings sync failed", error);
    } finally {
      serviceBusyAction = null;
      render();
    }
  };
  const checkDesktopRelease = async ({ silent = false } = {}) => {
    if (releaseCheckInFlight) return;
    releaseCheckInFlight = true;
    releaseState = { ...releaseState, loading: !silent, error: silent ? releaseState.error : null };
    window.__slockDesktopReleaseState = releaseState;
    if (!silent) render();
    try {
      if (!updateSnapshot?.currentVersion) {
        const payload = await invokeDesktop("bootstrap", { refresh: false });
        syncDesktopPayload(payload);
      }
      const latest = await invokeDesktop("check_desktop_update", {});
      syncDesktopUpdateCheck(latest, false);
    } catch (error) {
      releaseState = {
        loading: false,
        installing: false,
        error: silent ? releaseState.error : error?.message || String(error || t("updateCheckFailed")),
        latest: silent ? releaseState.latest : null,
      };
      if (silent) console.warn("[Slock Desktop] automatic update check failed", error);
    } finally {
      releaseCheckInFlight = false;
    }
    window.__slockDesktopReleaseState = releaseState;
    render();
  };
  const syncDesktopUpdateCheck = (latest, shouldRender = true) => {
    if (!latest) return;
    updateSnapshot = {
      ...(updateSnapshot || { currentVersion: latest.currentVersion || "" }),
      currentVersion: latest.currentVersion || updateSnapshot?.currentVersion || "",
      latest,
    };
    window.__slockDesktopUpdateSnapshot = updateSnapshot;
    releaseState = {
      ...releaseState,
      loading: false,
      installing: false,
      error: null,
      latest,
    };
    window.__slockDesktopReleaseState = releaseState;
    if (shouldRender) render();
  };
  const installDesktopRelease = async () => {
    releaseState = { ...releaseState, installing: true, error: null };
    window.__slockDesktopReleaseState = releaseState;
    render();
    try {
      await invokeDesktop("install_desktop_update", {});
    } catch (error) {
      releaseState = {
        ...releaseState,
        installing: false,
        error: error?.message || String(error || t("updateCheckFailed")),
      };
      window.__slockDesktopReleaseState = releaseState;
      render();
    }
  };
  const syncAppearancePayload = (payload) => {
    syncDesktopPayload(payload);
    render();
    translateSlockMenus();
  };
  const createCustomTheme = async (name, accent) => {
    appearanceBusyAction = "theme-create";
    render();
    try {
      const payload = await invokeDesktop("create_custom_theme", { name, accent });
      newThemeDraft = null;
      titlebarThemeWheelOpen = false;
      syncAppearancePayload(payload);
    } catch (error) {
      console.warn("[Slock Desktop] custom theme create failed", error);
    } finally {
      appearanceBusyAction = null;
      render();
    }
  };
  const deleteCustomTheme = async (id) => {
    appearanceBusyAction = `theme-delete:${id}`;
    render();
    try {
      const payload = await invokeDesktop("delete_custom_theme", { id });
      syncAppearancePayload(payload);
    } catch (error) {
      console.warn("[Slock Desktop] custom theme delete failed", error);
    } finally {
      appearanceBusyAction = null;
      render();
    }
  };
  const setThemeStyle = async (styleId) => {
    appearanceBusyAction = `style:${styleId}`;
    render();
    try {
      const payload = await invokeDesktop("set_theme_style", { styleId });
      syncAppearancePayload(payload);
    } catch (error) {
      console.warn("[Slock Desktop] set theme style failed", error);
    } finally {
      appearanceBusyAction = null;
      render();
    }
  };
  const importThemeStyle = async (config) => {
    appearanceBusyAction = "import-style";
    render();
    try {
      const payload = await invokeDesktop("import_theme_style", { config });
      syncAppearancePayload(payload);
    } catch (error) {
      console.warn("[Slock Desktop] import theme style failed", error);
    } finally {
      appearanceBusyAction = null;
      render();
    }
  };
  const readThemeStyleConfig = (parsed) => {
    if (parsed && typeof parsed === "object") {
      if (parsed.style && typeof parsed.style === "object") return parsed.style;
      if (parsed.config && typeof parsed.config === "object") return parsed.config;
      if (parsed.id || parsed.name) return parsed;
    }
    throw new Error("Invalid style file");
  };
  const exportThemeStyleFile = (style) => {
    if (!style) return;
    const payload = { schema: "slock-desktop.theme-style.v1", style: style.config };
    const blob = new Blob([JSON.stringify(payload, null, 2) + "\n"], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    const slug = (style.name || style.id || "style").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");
    link.href = url;
    link.download = `${slug}.slock-style.json`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };
  const getThemeStyleName = (style) => {
    if (style.id === "original") return t("themeStyleOriginalName");
    if (style.id === "default") return t("themeStyleDefaultName");
    return style.name || style.id;
  };
  const getThemeStyleSummary = (style) => {
    if (style.id === "original") return t("themeStyleOriginalSummary");
    if (style.id === "default") return t("themeStyleDefaultSummary");
    return style.summary || "";
  };
  const selectedStyle = () => {
    return styleCatalog.find((s) => s.id === activeStyleId) || styleCatalog[0] || null;
  };
  const fetchDashboardAgents = async () => {
    const service = serviceSnapshot;
    const slug = service?.selectedServerSlug || "";
    if (!slug) return;
    try {
      const data = await invokeDesktop("fetch_dashboard", { serverSlug: slug });
      if (data && data.agents) {
        dashboardAgents = data.agents;
        window.__slockDesktopDashboardAgents = dashboardAgents;
        render();
      }
    } catch (error) {
      console.warn("[Slock Desktop] fetch dashboard agents failed", error);
    }
  };
  const handleAgentCardOpen = async (agent) => {
    if (agentCardTarget?.id === agent.id) {
      agentCardTarget = null;
      render();
      return;
    }
    agentCardTarget = agent;
    agentCardActivity = [];
    agentCardLoading = true;
    render();
    try {
      const slug = serviceSnapshot?.selectedServerSlug || "";
      if (slug) {
        const activity = await invokeDesktop("fetch_agent_activity", { serverSlug: slug, agentId: agent.id });
        agentCardActivity = (activity || []).slice(0, 5);
      }
    } catch {
      // Activity load failure is non-critical
    } finally {
      agentCardLoading = false;
      render();
    }
  };
  const handleAgentStop = async (agent) => {
    const slug = serviceSnapshot?.selectedServerSlug || "";
    if (!slug) return;
    try {
      agentCardAction = "stop";
      render();
      await invokeDesktop("stop_agent", { serverSlug: slug, agentId: agent.id });
      agentCardTarget = null;
      await fetchDashboardAgents();
    } catch (error) {
      console.warn("[Slock Desktop] agent stop failed", error);
    } finally {
      agentCardAction = null;
      render();
    }
  };
  const handleAgentStart = async (agent) => {
    const slug = serviceSnapshot?.selectedServerSlug || "";
    if (!slug) return;
    try {
      agentCardAction = "start";
      render();
      await invokeDesktop("start_agent", { serverSlug: slug, agentId: agent.id });
      agentCardTarget = null;
      await fetchDashboardAgents();
    } catch (error) {
      console.warn("[Slock Desktop] agent start failed", error);
    } finally {
      agentCardAction = null;
      render();
    }
  };
  const handleAgentRestart = async (agent) => {
    const slug = serviceSnapshot?.selectedServerSlug || "";
    if (!slug) return;
    try {
      agentCardAction = "restart";
      render();
      await invokeDesktop("stop_agent", { serverSlug: slug, agentId: agent.id });
      await invokeDesktop("start_agent", { serverSlug: slug, agentId: agent.id });
      agentCardTarget = null;
      await fetchDashboardAgents();
    } catch (error) {
      console.warn("[Slock Desktop] agent restart failed", error);
    } finally {
      agentCardAction = null;
      render();
    }
  };
  const formatRelativeTime = (dateStr) => {
    if (!dateStr) return "";
    try {
      const d = new Date(dateStr);
      const now = Date.now();
      const diff = Math.max(0, now - d.getTime());
      const s = Math.floor(diff / 1000);
      if (s < 60) return `${s}s`;
      const m = Math.floor(s / 60);
      if (m < 60) return `${m}m`;
      const h = Math.floor(m / 60);
      if (h < 24) return `${h}h`;
      const days = Math.floor(h / 24);
      return `${days}d`;
    } catch { return ""; }
  };
  const slockMenuCopy = {
    "en-US": {
      channels: "Channels",
      channelsUpper: "CHANNELS",
      newChannel: "New channel",
      createChannel: "Create Channel",
      createChannelUpper: "CREATE CHANNEL",
      createChannelPage: "Create channel",
      directMessages: "Direct messages",
      directMessagesTitle: "Direct Messages",
      directMessagesUpper: "DIRECT MESSAGES",
      directMessagesTypo: "DIERECT MESSAGES",
      chat: "Chat",
      newDirectMessage: "New Direct Message",
      startDirectMessage: "Start a direct message",
      searchDirectMessages: "Search direct messages",
      noDirectMessages: "No direct messages",
      threads: "Threads",
      threadsUpper: "THREADS",
      thread: "Thread",
      noActiveThreads: "No active threads",
      threadsAppearHere: "Threads you participate in will appear here.",
      markThreadDoneHint: "Marking a thread as done hides it until new messages arrive.",
      unfollowThread: "Unfollow Thread",
      loadMore: "Load More",
      replyLower: "reply",
      repliesLower: "replies",
      activeLower: "active",
      yesterday: "Yesterday",
      today: "Today",
      process: "Process",
      processes: "Processes",
      processStatus: "Process status",
      running: "Running",
      idle: "Idle",
      stopped: "Stopped",
      starting: "Starting",
      stopping: "Stopping",
      failed: "Failed",
      healthy: "Healthy",
      online: "Online",
      offline: "Offline",
      queued: "Queued",
      settings: "Settings",
      account: "Account",
      browser: "Browser",
      server: "Server",
      preferences: "Preferences",
      workspace: "Workspace",
      general: "General",
      appearance: "Appearance",
      language: "Language",
      theme: "Theme",
      security: "Security",
      billing: "Billing",
      planBilling: "Plan & Billing",
      dangerZone: "Danger Zone",
      integrations: "Integrations",
      accountSettings: "Account Settings",
      serverSettings: "Server Settings",
      saveChanges: "Save changes",
      workspaceSettings: "Workspace settings",
      profile: "Profile",
      notifications: "Notifications",
      pushNotifications: "Push Notifications",
      members: "Members",
      pendingInvites: "Pending Invites",
      joinLinks: "Join Links",
      onboardingAgent: "Onboarding Agent",
      invite: "Invite",
      search: "Search",
      searchGlobalPlaceholder: "Search channels, DMs, messages...",
      searchGlobalPlaceholderEllipsis: "Search channels, DMs, messages…",
      clearSearch: "Clear search",
      searchMyMessages: "My messages",
      searchMyMessagesUpper: "MY MESSAGES",
      searchAnyTime: "Any time",
      searchAnyTimeUpper: "ANY TIME",
      searchEverything: "Search everything",
      searchEverythingDescription: "Search channels, DMs, people, agents, and message history.",
      optional: "optional",
      optionalWrapped: "(optional)",
      channelNameExample: "e.g. ai-research",
      channelAboutPlaceholder: "What is this channel about?",
      searchMembersByName: "Search members by name",
      agentsUpper: "AGENTS",
      task: "Task",
      taskLower: "task",
      tasks: "Tasks",
      tasksUpper: "TASKS",
      channelTasks: "channel tasks",
      newTask: "New Task",
      newTaskSentence: "New task",
      createTask: "Create Task",
      createTaskSentence: "Create task",
      addTask: "Add Task",
      addTaskSentence: "Add task",
      taskTitle: "Task title",
      title: "Title",
      taskName: "Task name",
      taskDescription: "Task description",
      whatNeedsDoing: "What needs to be done?",
      describeTask: "Describe the task",
      addDetails: "Add details",
      assignTask: "Assign task",
      assignee: "Assignee",
      noAssignee: "No assignee",
      selectAssignee: "Select assignee",
      assignTo: "Assign to",
      channel: "Channel",
      selectChannel: "Select channel",
      status: "Status",
      priority: "Priority",
      dueDate: "Due date",
      tags: "Tags",
      todo: "Todo",
      todoUpper: "TODO",
      toDo: "To do",
      toDoTitle: "To Do",
      inProgress: "In progress",
      inProgressTitle: "In Progress",
      inProgressUpper: "IN PROGRESS",
      inReview: "In review",
      inReviewTitle: "In Review",
      inReviewUpper: "IN REVIEW",
      doneUpper: "DONE",
      board: "Board",
      list: "List",
      saved: "Saved",
      savedUpper: "SAVED",
      pinned: "Pinned",
      pinnedUpper: "PINNED",
      machines: "Machines",
      machinesUpper: "MACHINES",
      all: "All",
      mentions: "Mentions",
      unread: "Unread",
      activity: "Activity",
      inbox: "Inbox",
      agentDms: "Agent DMs",
      reminders: "Reminders",
      messageAction: "Message",
      people: "People",
      participants: "Participants",
      messages: "Messages",
      files: "Files",
      links: "Links",
      releaseNotes: "Release Notes",
      releaseNotesSentence: "Release notes",
      markAsRead: "Mark as Read",
      markAsUnread: "Mark as Unread",
      markAsReadSentence: "Mark as read",
      markAsUnreadSentence: "Mark as unread",
      copyLink: "Copy link",
      copyLinkTitle: "Copy Link",
      copyMarkdown: "Copy markdown",
      saveMessage: "Save message",
      removeFromSaved: "Remove from saved",
      markAsDone: "Mark as done",
      reopenTask: "Reopen task",
      asTask: "As Task",
      attachImage: "Attach image",
      attachFile: "Attach file",
      send: "Send",
      uploading: "Uploading...",
      viewParticipants: "View participants",
      viewParticipantsTitle: "View Participants",
      viewInChannel: "View in channel",
      closeThread: "Close thread",
      messageThread: "Message thread",
      edit: "Edit",
      delete: "Delete",
      reply: "Reply",
      replyInThread: "Reply in thread",
      openThread: "Open thread",
      close: "Close",
      cancel: "Cancel",
      addAnother: "Add another",
      done: "Done",
      doneLower: "done",
      save: "Save",
      saving: "Saving...",
      savedState: "Saved",
      updating: "Updating...",
      loading: "Loading...",
      connect: "Connect",
      connecting: "Connecting...",
      connectedAccounts: "Connected accounts",
      notConnected: "Not connected",
      changeAvatar: "Change Avatar on Gravatar",
      displayName: "Display Name",
      email: "Email",
      verified: "Verified",
      unverified: "Unverified",
      saveProfile: "Save Profile",
      changePassword: "Change Password",
      currentPassword: "Current Password",
      newPassword: "New Password",
      confirmPassword: "Confirm Password",
      minCharacters: "Min 8 characters",
      passwordUpdated: "Password updated!",
      name: "Name",
      slug: "Slug",
      maxUses: "Max Uses",
      expiresAt: "Expires At",
      unlimited: "Unlimited",
      leave: "Leave",
      leaveServer: "Leave Server",
      deleteServer: "Delete Server",
      createJoinLink: "Create Join Link",
      registeredClients: "Registered Clients",
      pendingApprovals: "Pending Approvals",
      activeConnections: "Active Connections",
      registerIntegrationClient: "Register Integration Client",
      serviceName: "Service Name",
      clientId: "Client ID",
      homepageUrl: "Homepage URL",
      description: "Description",
      createClient: "Create Client",
      currentPlan: "Current Plan:",
      plans: "Plans",
      history: "History",
      currentPlanBadge: "Current Plan",
      comingSoon: "Coming Soon",
      notAvailable: "Not Available",
      approve: "Approve",
      approveRemember: "Approve & Remember",
      deny: "Deny",
      revoke: "Revoke",
      revokeInvite: "Revoke Invite",
      revokeJoinLink: "Revoke Join Link",
      copyJoinLink: "Copy join link",
      dmsMentionsThreads: "DMs, direct mentions, and followed thread replies",
      pushDescription: "Uses web push so notifications can still arrive when the tab is in the background.",
      pushUnsupported: "This browser does not support service worker push notifications.",
      pushUnavailable: "Push notifications are not configured on this server yet.",
      pushDeniedHelp: "Browser permission is denied. Re-enable notifications in browser site settings, then come back here.",
      checking: "Checking...",
      unsupported: "Unsupported",
      enabled: "Enabled",
      readyToEnable: "Ready to enable",
      denied: "Denied",
      disabled: "Disabled",
      unavailable: "Unavailable",
      enablePush: "Enable Push Notifications",
      disablePush: "Disable Push Notifications",
      sendTestPush: "Send Test Push",
      noPendingInvites: "No pending invites.",
      noActiveJoinLinks: "No active join links.",
      revokeInviteTitle: "Revoke Invite",
      revokeJoinLinkTitle: "Revoke Join Link",
      selectOnboardingAgent: "Select one default onboarding agent for owner/member onboarding flows.",
      defaultOnboardingAgent: "Default (first available active agent)",
      onlyOwnerOnboarding: "Only server owner can change onboarding agent.",
      freeTrialActive: "Free Trial Active",
      founderPlan: "Founder Plan",
      planDowngraded: "Plan Downgraded",
      graceExpired: "Grace Period Expired",
      viewService: "View service",
      clientSecret: "Client Secret",
      pendingApprovalsDescription: "Requests waiting for an admin decision",
      activeConnectionsDescription: "Standing grants that let an agent log in without a new approval",
      registerIntegrationDescription: "Create client credentials for an external app or demo integration",
      onlyAdminsIntegrations: "Only server owners and admins can manage integrations.",
      whatIntegrationFor: "What this integration is for",
      convertToTask: "Convert to Task",
      convertToTaskSentence: "Convert to task",
      loadOlderMessages: "Load older messages",
      loadOlderMessagesTitle: "Load Older Messages",
      loadOlder: "Load older",
      help: "Help",
      signOut: "Sign out",
      logOut: "Log out",
      create: "Create",
      new: "New",
      more: "More",
      collapseSidebar: "Collapse sidebar",
      expandSidebar: "Expand sidebar",
    },
    "zh-CN": {
      channels: "频道",
      channelsUpper: "频道",
      newChannel: "新建频道",
      createChannel: "创建频道",
      createChannelUpper: "创建频道",
      createChannelPage: "创建频道",
      directMessages: "私信",
      directMessagesTitle: "私信",
      directMessagesUpper: "私信",
      directMessagesTypo: "私信",
      chat: "聊天",
      newDirectMessage: "新建私信",
      startDirectMessage: "开始私信",
      searchDirectMessages: "搜索私信",
      noDirectMessages: "暂无私信",
      threads: "线程",
      threadsUpper: "线程",
      thread: "线程",
      noActiveThreads: "暂无活跃线程",
      threadsAppearHere: "你参与的线程会显示在这里。",
      markThreadDoneHint: "标记完成后会隐藏线程，直到有新消息。",
      unfollowThread: "取消关注线程",
      loadMore: "加载更多",
      replyLower: "条回复",
      repliesLower: "条回复",
      activeLower: "活跃",
      yesterday: "昨天",
      today: "今天",
      process: "进程",
      processes: "进程",
      processStatus: "进程状态",
      running: "运行中",
      idle: "空闲",
      stopped: "已停止",
      starting: "启动中",
      stopping: "停止中",
      failed: "失败",
      healthy: "健康",
      online: "在线",
      offline: "离线",
      queued: "排队中",
      settings: "设置",
      account: "账号",
      browser: "浏览器",
      server: "服务器",
      preferences: "偏好设置",
      workspace: "工作区",
      general: "通用",
      appearance: "外观",
      language: "语言",
      theme: "主题",
      security: "安全",
      billing: "计费",
      planBilling: "方案与计费",
      dangerZone: "危险区域",
      integrations: "集成",
      accountSettings: "账号设置",
      serverSettings: "服务器设置",
      saveChanges: "保存更改",
      workspaceSettings: "工作区设置",
      profile: "个人资料",
      notifications: "通知",
      pushNotifications: "推送通知",
      members: "成员",
      pendingInvites: "待处理邀请",
      joinLinks: "加入链接",
      onboardingAgent: "入门 Agent",
      invite: "邀请",
      search: "搜索",
      searchGlobalPlaceholder: "搜索频道、私信、消息...",
      searchGlobalPlaceholderEllipsis: "搜索频道、私信、消息...",
      clearSearch: "清除搜索",
      searchMyMessages: "我的消息",
      searchMyMessagesUpper: "我的消息",
      searchAnyTime: "任意时间",
      searchAnyTimeUpper: "任意时间",
      searchEverything: "搜索全部",
      searchEverythingDescription: "搜索频道、私信、人员、Agent 和消息历史。",
      optional: "可选",
      optionalWrapped: "（可选）",
      channelNameExample: "例如：ai-research",
      channelAboutPlaceholder: "这个频道是做什么的？",
      searchMembersByName: "按名称搜索成员",
      agentsUpper: "智能体",
      task: "任务",
      taskLower: "任务",
      tasks: "任务",
      tasksUpper: "任务",
      channelTasks: "频道任务",
      newTask: "新建任务",
      newTaskSentence: "新建任务",
      createTask: "创建任务",
      createTaskSentence: "创建任务",
      addTask: "添加任务",
      addTaskSentence: "添加任务",
      taskTitle: "任务标题",
      title: "标题",
      taskName: "任务名称",
      taskDescription: "任务描述",
      whatNeedsDoing: "需要完成什么？",
      describeTask: "描述任务",
      addDetails: "添加详情",
      assignTask: "分配任务",
      assignee: "负责人",
      noAssignee: "无负责人",
      selectAssignee: "选择负责人",
      assignTo: "分配给",
      channel: "频道",
      selectChannel: "选择频道",
      status: "状态",
      priority: "优先级",
      dueDate: "截止日期",
      tags: "标签",
      todo: "待办",
      todoUpper: "待办",
      toDo: "待办",
      toDoTitle: "待办",
      inProgress: "进行中",
      inProgressTitle: "进行中",
      inProgressUpper: "进行中",
      inReview: "待复核",
      inReviewTitle: "待复核",
      inReviewUpper: "审核中",
      doneUpper: "已完成",
      board: "看板",
      list: "列表",
      saved: "已保存",
      savedUpper: "已保存",
      pinned: "已置顶",
      pinnedUpper: "已置顶",
      machines: "机器",
      machinesUpper: "机器",
      all: "全部",
      mentions: "提及",
      unread: "未读",
      activity: "动态",
      inbox: "收件箱",
      agentDms: "Agent 私信",
      reminders: "提醒",
      messageAction: "发消息",
      people: "人员",
      participants: "参与者",
      messages: "消息",
      files: "文件",
      links: "链接",
      releaseNotes: "发布说明",
      releaseNotesSentence: "发布说明",
      markAsRead: "标记为已读",
      markAsUnread: "标记为未读",
      markAsReadSentence: "标记为已读",
      markAsUnreadSentence: "标记为未读",
      copyLink: "复制链接",
      copyLinkTitle: "复制链接",
      copyMarkdown: "复制 Markdown",
      saveMessage: "保存消息",
      removeFromSaved: "从已保存中移除",
      markAsDone: "标记为完成",
      reopenTask: "重新打开任务",
      asTask: "作为任务",
      attachImage: "附加图片",
      attachFile: "附加文件",
      send: "发送",
      uploading: "上传中...",
      viewParticipants: "查看参与者",
      viewParticipantsTitle: "查看参与者",
      viewInChannel: "在频道中查看",
      closeThread: "关闭线程",
      messageThread: "发送线程消息",
      edit: "编辑",
      delete: "删除",
      reply: "回复",
      replyInThread: "在线程中回复",
      openThread: "打开线程",
      close: "关闭",
      cancel: "取消",
      addAnother: "再添加一个",
      done: "完成",
      doneLower: "完成",
      save: "保存",
      saving: "保存中...",
      savedState: "已保存",
      updating: "更新中...",
      loading: "加载中...",
      connect: "连接",
      connecting: "连接中...",
      connectedAccounts: "已连接账号",
      notConnected: "未连接",
      changeAvatar: "在 Gravatar 更换头像",
      displayName: "显示名称",
      email: "邮箱",
      verified: "已验证",
      unverified: "未验证",
      saveProfile: "保存资料",
      changePassword: "修改密码",
      currentPassword: "当前密码",
      newPassword: "新密码",
      confirmPassword: "确认密码",
      minCharacters: "至少 8 个字符",
      passwordUpdated: "密码已更新",
      name: "名称",
      slug: "标识",
      maxUses: "最大使用次数",
      expiresAt: "过期时间",
      unlimited: "无限制",
      leave: "离开",
      leaveServer: "离开服务器",
      deleteServer: "删除服务器",
      createJoinLink: "创建加入链接",
      registeredClients: "已注册客户端",
      pendingApprovals: "待审批",
      activeConnections: "活跃连接",
      registerIntegrationClient: "注册集成客户端",
      serviceName: "服务名称",
      clientId: "客户端 ID",
      homepageUrl: "主页 URL",
      description: "描述",
      createClient: "创建客户端",
      currentPlan: "当前方案：",
      plans: "方案",
      history: "历史",
      currentPlanBadge: "当前方案",
      comingSoon: "即将推出",
      notAvailable: "不可用",
      approve: "批准",
      approveRemember: "批准并记住",
      deny: "拒绝",
      revoke: "撤销",
      revokeInvite: "撤销邀请",
      revokeJoinLink: "撤销加入链接",
      copyJoinLink: "复制加入链接",
      dmsMentionsThreads: "私信、直接提及和关注线程回复",
      pushDescription: "使用 Web 推送，标签页在后台时也能收到通知。",
      pushUnsupported: "此浏览器不支持 Service Worker 推送通知。",
      pushUnavailable: "此服务器尚未配置推送通知。",
      pushDeniedHelp: "浏览器通知权限已拒绝，请在浏览器站点设置中重新启用。",
      checking: "检查中...",
      unsupported: "不支持",
      enabled: "已启用",
      readyToEnable: "可启用",
      denied: "已拒绝",
      disabled: "已关闭",
      unavailable: "不可用",
      enablePush: "启用推送通知",
      disablePush: "关闭推送通知",
      sendTestPush: "发送测试推送",
      noPendingInvites: "暂无待处理邀请。",
      noActiveJoinLinks: "暂无活跃加入链接。",
      revokeInviteTitle: "撤销邀请",
      revokeJoinLinkTitle: "撤销加入链接",
      selectOnboardingAgent: "为所有者/成员入门流程选择一个默认 Agent。",
      defaultOnboardingAgent: "默认（第一个可用活跃 Agent）",
      onlyOwnerOnboarding: "只有服务器所有者可以更改入门 Agent。",
      freeTrialActive: "免费试用中",
      founderPlan: "创始人方案",
      planDowngraded: "方案已降级",
      graceExpired: "宽限期已结束",
      viewService: "查看服务",
      clientSecret: "客户端密钥",
      pendingApprovalsDescription: "等待管理员决策的请求",
      activeConnectionsDescription: "允许 Agent 无需再次审批即可登录的长期授权",
      registerIntegrationDescription: "为外部应用或演示集成创建客户端凭据",
      onlyAdminsIntegrations: "只有服务器所有者和管理员可以管理集成。",
      whatIntegrationFor: "此集成的用途",
      convertToTask: "转换为任务",
      convertToTaskSentence: "转换为任务",
      loadOlderMessages: "加载更早消息",
      loadOlderMessagesTitle: "加载更早消息",
      loadOlder: "加载更早",
      help: "帮助",
      signOut: "退出登录",
      logOut: "退出登录",
      create: "创建",
      new: "新建",
      more: "更多",
      collapseSidebar: "收起侧边栏",
      expandSidebar: "展开侧边栏",
    },
  };
  const themeDisplay = (theme) => {
    const dictionary = copy[resolveLanguage()];
    return {
      name: theme.id === "custom"
        ? theme.name || dictionary.themeNames?.custom || "Custom"
        : dictionary.themeNames?.[theme.id] || theme.name,
      summary: dictionary.themeSummaries?.[theme.id] || theme.summary,
    };
  };
  const titlebarThemeLabel = (theme) => {
    const display = themeDisplay(theme).name || "";
    const trimmed = display.replace(/主题/g, "").replace(/\btheme\b/ig, "").trim();
    if (trimmed) return trimmed;
    return resolveLanguage() === "zh-CN" ? "原始" : "Original";
  };
  const titlebarThemeSwatch = (theme) => {
    if (theme?.id === "original") return '#ffd701';
    if (theme?.id === "default") return '#10a37f';
    return theme?.accent || "var(--desktop-accent)";
  };
  const selectedTheme = () =>
    themeCatalog.find((theme) => theme.id === activeThemeId) ||
    themeCatalog.find((theme) => theme.id === "default") ||
    themeCatalog.find((theme) => theme.id === "original") ||
    themeCatalog[0];
  const themeVarNames = [
    "--desktop-canvas",
    "--desktop-surface",
    "--desktop-surface-secondary",
    "--desktop-line",
    "--desktop-text",
    "--desktop-muted",
    "--desktop-selection",
  ];
  const translateSlockMenus = () => {
    const language = resolveLanguage();
    const target = slockMenuCopy[language];
    const translations = new Map();

    Object.keys(target).forEach((key) => {
      Object.values(slockMenuCopy).forEach((dictionary) => {
        if (dictionary[key] && dictionary[key] !== target[key] && !translations.has(dictionary[key])) {
          translations.set(dictionary[key], target[key]);
        }
      });
    });

    document.documentElement.lang = language;

    const escapeRegExp = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const partialTranslationKeys = [
      "thread",
      "replyLower",
      "repliesLower",
      "activeLower",
      "process",
      "processes",
      "processStatus",
      "running",
      "idle",
      "stopped",
      "starting",
      "stopping",
      "failed",
      "healthy",
      "online",
      "offline",
      "queued",
      "chat",
      "task",
      "taskLower",
      "channelTasks",
      "newTaskSentence",
      "createTaskSentence",
      "addTaskSentence",
      "todo",
      "todoUpper",
      "toDo",
      "toDoTitle",
      "inProgress",
      "inProgressTitle",
      "inProgressUpper",
      "inReview",
      "inReviewTitle",
      "inReviewUpper",
      "doneUpper",
      "board",
      "list",
      "addAnother",
      "done",
      "doneLower",
      "settings",
      "account",
      "browser",
      "server",
      "preferences",
      "workspace",
      "general",
      "appearance",
      "language",
      "theme",
      "security",
      "billing",
      "planBilling",
      "dangerZone",
      "integrations",
      "optionalWrapped",
      "optional",
    ];
    const partialTranslations = [];

    partialTranslationKeys.forEach((key) => {
      const replacement = target[key];
      if (!replacement) return;
      Object.values(slockMenuCopy).forEach((dictionary) => {
        const source = dictionary[key];
        if (source && source !== replacement) {
          partialTranslations.push([source, replacement]);
        }
      });
    });
    partialTranslations.sort((a, b) => b[0].length - a[0].length);

    const isControlElement = (element) => !!element.closest("input, textarea, select, [contenteditable='true']");
    const isExcludedTranslationTarget = (element, options = {}) => {
      if (host.contains(element)) return true;
      if (isControlElement(element)) return options.attribute !== true;
      if (element.closest("pre, code")) return true;
      if (element.closest("[data-slock-message-content], [data-message-content], [class*='message-content'], [class*='MessageContent']")) return true;
      if (element.closest("article") && !element.closest("header, nav, aside, [role='menu'], [role='menuitem'], [role='heading'], [data-radix-collection-item]")) return true;

      return false;
    };

    const canPartiallyTranslate = (element) => {
      const text = element.textContent?.trim() || "";
      if (!text || text.length > 96) return false;
      if (isExcludedTranslationTarget(element)) return false;
      if (/^\d+\s+active$/i.test(text)) return true;

      return element.matches([
        "header",
        "header *",
        "nav",
        "nav *",
        "aside",
        "aside *",
        "[role='heading']",
        "[role='heading'] *",
        "[role='menu'] *",
        "[role='menuitem']",
        "[role='menuitem'] *",
        "h1",
        "h1 *",
        "h2",
        "h2 *",
        "h3",
        "h3 *",
        "h4",
        "h4 *",
        "button",
        "button *",
        "label",
        "label *",
        "[data-radix-collection-item]",
        "[data-radix-collection-item] *",
        "[class*='uppercase']",
        "[class*='tracking-widest']",
        "[class*='status']",
        "[class*='status'] *",
        "[class*='badge']",
        "[class*='badge'] *",
        "[class*='chip']",
        "[class*='chip'] *",
        "[data-state]",
        "[data-state] *",
      ].join(",")) || element.hasAttribute("title") || element.hasAttribute("aria-label");
    };

    const replacePartialText = (value) => {
      let next = value;
      partialTranslations.forEach(([source, replacement]) => {
        if (/^[A-Za-z][A-Za-z0-9 &/.-]*$/.test(source)) {
          const pattern = new RegExp(`(^|[^A-Za-z0-9_])(${escapeRegExp(source)})(?=$|[^A-Za-z0-9_])`, "g");
          next = next.replace(pattern, `$1${replacement}`);
        } else {
          next = next.split(source).join(replacement);
        }
      });
      return next;
    };

    const translateText = (value, allowPartial = false) => {
      const text = value?.trim();
      if (!text) return value;
      if (translations.has(text)) {
        return value.replace(text, translations.get(text));
      }
      return allowPartial ? replacePartialText(value) : value;
    };

    const translateAttribute = (element, attribute) => {
      if (isExcludedTranslationTarget(element, { attribute: true })) return;
      const value = element.getAttribute(attribute)?.trim();
      if (!value) return;
      const translated = translateText(value, attribute !== "placeholder");
      if (translated !== value) {
        element.setAttribute(attribute, translated);
      }
    };

    const translateElementTextNodes = (element, allowPartial) => {
      const walker = document.createTreeWalker(
        element,
        NodeFilter.SHOW_TEXT,
        {
          acceptNode(node) {
            const text = node.textContent?.trim() || "";
            if (!text) return NodeFilter.FILTER_REJECT;
            const parent = node.parentElement;
            if (!parent) return NodeFilter.FILTER_REJECT;
            if (isExcludedTranslationTarget(parent)) return NodeFilter.FILTER_REJECT;
            return NodeFilter.FILTER_ACCEPT;
          },
        },
      );

      const pending = [];
      let node = walker.nextNode();
      while (node) {
        const text = node.textContent || "";
        const translated = translateText(text, allowPartial);
        if (translated !== text) {
          pending.push([node, translated]);
        }
        node = walker.nextNode();
      }

      pending.forEach(([targetNode, translated]) => {
        targetNode.textContent = translated;
      });
    };

    const collectSearchRoots = (root = document) => {
      const roots = [root];
      const startNode = root instanceof Document ? root.documentElement : root;
      if (!startNode) return roots;

      const walker = document.createTreeWalker(startNode, NodeFilter.SHOW_ELEMENT);
      let node = walker.currentNode;
      while (node) {
        if (node.shadowRoot) {
          roots.push(...collectSearchRoots(node.shadowRoot));
        }
        node = walker.nextNode();
      }

      return roots;
    };

    const selectors = [
      "[role='menuitem']",
      "[role='menu'] button",
      "[role='menu'] [role='button']",
      "nav a",
      "nav button",
      "nav span",
      "nav div",
      "aside a",
      "aside button",
      "aside span",
      "aside div",
      "a",
      "a span",
      "div[title]",
      "span[title]",
      "header button",
      "header span",
      "header div",
      "[role='heading']",
      "[role='heading'] span",
      "h1",
      "h1 span",
      "h2",
      "h2 span",
      "h3",
      "h3 span",
      "h4",
      "h4 span",
      "[class*='uppercase']",
      "[class*='tracking-widest']",
      "main [class*='font-bold']",
      "main [class*='font-bold'] span",
      "main [class*='font-semibold']",
      "main [class*='font-semibold'] span",
      "main [class*='text-xs']",
      "main [class*='text-xs'] span",
      "main p",
      "main p span",
      "main [class*='status']",
      "main [class*='status'] span",
      "main [class*='badge']",
      "main [class*='badge'] span",
      "main [class*='chip']",
      "main [class*='chip'] span",
      "main [data-state]",
      "main [data-state] span",
      "[class*='font-bold']",
      "[class*='font-bold'] span",
      "[class*='font-semibold']",
      "[class*='font-semibold'] span",
      "button",
      "button span",
      "button[title]",
      "button[type='submit']",
      "label",
      "label span",
      "input[placeholder]",
      "textarea[placeholder]",
      "[placeholder]",
      "[title]",
      "[data-radix-collection-item]",
      "button[aria-haspopup='menu']",
      "button[aria-label]",
      "[aria-label]",
    ].join(",");

    const seen = new Set();
    collectSearchRoots().forEach((root) => {
      root.querySelectorAll(selectors).forEach((element) => {
        if (seen.has(element)) return;
        seen.add(element);
        const excludedForText = isExcludedTranslationTarget(element);

        translateAttribute(element, "aria-label");
        translateAttribute(element, "title");
        translateAttribute(element, "placeholder");

        if (excludedForText) return;

        const allowPartial = canPartiallyTranslate(element);

        if (element.childElementCount === 0) {
          const text = element.textContent || "";
          const translated = translateText(text, allowPartial);
          if (translated !== text) element.textContent = translated;
          return;
        }

        translateElementTextNodes(element, allowPartial);
      });
    });

    const messageTimestampReplacements = [];
    ["yesterday", "today"].forEach((key) => {
      const replacement = target[key];
      if (!replacement) return;
      Object.values(slockMenuCopy).forEach((dictionary) => {
        const source = dictionary[key];
        if (source && source !== replacement) {
          messageTimestampReplacements.push([source, replacement]);
        }
      });
    });
    if (messageTimestampReplacements.length > 0) {
      collectSearchRoots().forEach((root) => {
        root.querySelectorAll("[class*='text-xs']").forEach((element) => {
          if (element.childElementCount !== 0) return;
          const text = element.textContent || "";
          if (!text) return;
          for (const [source, replacement] of messageTimestampReplacements) {
            if (text === source || text.startsWith(source + " ")) {
              element.textContent = text.replace(source, replacement);
              break;
            }
          }
        });
      });
    }

    const shouldTranslateSearchDescriptions = () =>
      window.location.pathname.split("/").includes("search");

    collectSearchRoots().forEach((root) => {
      if (!shouldTranslateSearchDescriptions()) return;
      root.querySelectorAll("p, div, span, [class*='empty-state'], [class*='mt-1']").forEach((element) => {
        if (!(element instanceof Element)) return;
        if (host.contains(element)) return;
        if (isControlElement(element)) return;
        if (element.closest("pre, code")) return;
        if (element.closest("[data-slock-message-content], [data-message-content], [class*='message-content'], [class*='MessageContent']")) return;
        if (element.childElementCount !== 0) return;
        const text = element.textContent?.trim();
        if (!text || text.length < 16) return;
        const translated = translations.get(text);
        if (translated && translated !== text) {
          element.textContent = (element.textContent || "").replace(text, translated);
        }
      });
    });
  };

  const bindSlockMenuTranslator = () => {
    if (!document.body) return;
    window.__slockDesktopTranslateMenus = translateSlockMenus;
    translateSlockMenus();

    if (!window.__slockDesktopRouteTranslatorBound) {
      const rerun = () => {
        requestAnimationFrame(() => {
          window.__slockDesktopTranslateMenus?.();
          window.setTimeout(() => window.__slockDesktopTranslateMenus?.(), 120);
        });
      };
      const wrapHistory = (method) => {
        const original = window.history?.[method];
        if (typeof original !== "function") return;
        window.history[method] = function slockDesktopHistoryTranslator(...args) {
          const result = original.apply(this, args);
          rerun();
          return result;
        };
      };
      wrapHistory("pushState");
      wrapHistory("replaceState");
      window.addEventListener("popstate", rerun);
      window.addEventListener("hashchange", rerun);
      window.__slockDesktopRouteTranslatorBound = true;
    }

    if (window.__slockDesktopMenuObserver) {
      window.__slockDesktopMenuObserver.disconnect();
    }

    let pending = false;
    window.__slockDesktopMenuObserver = new MutationObserver(() => {
      if (pending) return;
      pending = true;
      requestAnimationFrame(() => {
        pending = false;
        translateSlockMenus();
      });
    });
    window.__slockDesktopMenuObserver.observe(document.body, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ["aria-label", "title", "placeholder"],
    });
  };

  const syncWorkspaceChromeSafeArea = () => {
    document.documentElement.dataset.slockDesktopWorkspaceChrome = "true";

    let style = document.getElementById(chromeSafeAreaStyleId);
    if (!style) {
      style = document.createElement("style");
      style.id = chromeSafeAreaStyleId;
      document.head.appendChild(style);
    }

    style.textContent = `
      :root[data-slock-desktop-workspace-chrome="true"] {
        --slock-desktop-titlebar-height: 34px;
        scroll-padding-top: var(--slock-desktop-titlebar-height);
      }

      :root[data-slock-desktop-workspace-chrome="true"] body {
        box-sizing: border-box !important;
        min-height: 100dvh !important;
        overflow: hidden !important;
      }

      :root[data-slock-desktop-workspace-chrome="true"] body > #root,
      :root[data-slock-desktop-workspace-chrome="true"] body > [data-reactroot],
      :root[data-slock-desktop-workspace-chrome="true"] body > div:first-child {
        transform: translate3d(0, var(--slock-desktop-titlebar-height), 0) !important;
        transform-origin: top left !important;
        height: calc(100dvh - var(--slock-desktop-titlebar-height)) !important;
        min-height: calc(100dvh - var(--slock-desktop-titlebar-height)) !important;
        max-height: calc(100dvh - var(--slock-desktop-titlebar-height)) !important;
        overflow: hidden !important;
        isolation: isolate !important;
      }

      :root[data-slock-desktop-workspace-chrome="true"] body > #root > [class*="h-screen"],
      :root[data-slock-desktop-workspace-chrome="true"] body > #root > [class*="min-h-screen"],
      :root[data-slock-desktop-workspace-chrome="true"] body > #root > [class*="h-dvh"],
      :root[data-slock-desktop-workspace-chrome="true"] body > #root > [class*="min-h-dvh"],
      :root[data-slock-desktop-workspace-chrome="true"] body > div:first-child > [class*="h-screen"],
      :root[data-slock-desktop-workspace-chrome="true"] body > div:first-child > [class*="min-h-screen"],
      :root[data-slock-desktop-workspace-chrome="true"] body > div:first-child > [class*="h-dvh"],
      :root[data-slock-desktop-workspace-chrome="true"] body > div:first-child > [class*="min-h-dvh"] {
        height: calc(100dvh - var(--slock-desktop-titlebar-height)) !important;
        min-height: calc(100dvh - var(--slock-desktop-titlebar-height)) !important;
      }

      :root[data-slock-desktop-workspace-chrome="true"] body > #root[class*="fixed"][class*="inset-0"],
      :root[data-slock-desktop-workspace-chrome="true"] body > div:first-child[class*="fixed"][class*="inset-0"] {
        top: 0 !important;
        height: calc(100dvh - var(--slock-desktop-titlebar-height)) !important;
      }
    `;
  };

  const syncHostTheme = () => {
    const theme = themes.find((candidate) => candidate.id === activeThemeId) || themes[0];
    if (!theme) return;
    host.style.colorScheme = activeMode === "system" ? "light dark" : activeMode;

    if (activeMode === "system") {
      themeVarNames.forEach((name) => host.style.removeProperty(name));
    } else {
      host.style.setProperty("--desktop-canvas", theme.canvas);
      host.style.setProperty("--desktop-surface", theme.surface);
      host.style.setProperty("--desktop-surface-secondary", theme.surfaceStrong);
      host.style.setProperty("--desktop-line", theme.line);
      host.style.setProperty("--desktop-text", theme.text);
      host.style.setProperty("--desktop-muted", theme.muted);
      host.style.setProperty("--desktop-selection", theme.accentSoft);
    }

    host.style.setProperty("--desktop-accent", theme.accent);
  };

  const css = `
    :host {
      color-scheme: light dark;
      --desktop-canvas: #f7f7f5;
      --desktop-toolbar: #ecede8;
      --desktop-sidebar: #ecede8;
      --desktop-panel: #f1f2ee;
      --desktop-surface: #ffffff;
      --desktop-surface-secondary: #f3f4f1;
      --desktop-surface-tertiary: #ecefea;
      --desktop-line: #e2e4de;
      --desktop-line-strong: #d4d8d0;
      --desktop-text: #1f1f1c;
      --desktop-muted: #6b6f67;
      --desktop-tertiary: #8a8f86;
      --desktop-accent: #10a37f;
      --desktop-accent-hover: #0e8f70;
      --desktop-accent-active: #0c7a60;
      --desktop-selection: #e7f5f1;
      --desktop-hover: rgba(31, 31, 28, 0.04);
      --desktop-focus-ring: rgba(16, 163, 127, 0.28);
      --desktop-radius-xs: 8px;
      --desktop-radius-sm: 10px;
      --desktop-radius-md: 12px;
      --desktop-radius-lg: 16px;
      --desktop-radius-xl: 20px;
      --desktop-radius-pill: 999px;
      color: var(--desktop-text);
      font-family: Inter, "SF Pro Display", "PingFang SC", system-ui, sans-serif;
    }

    @media (prefers-color-scheme: dark) {
      :host {
        --desktop-canvas: #1f1f1c;
        --desktop-toolbar: #2f302c;
        --desktop-sidebar: #2f302c;
        --desktop-panel: #282925;
        --desktop-surface: #252623;
        --desktop-surface-secondary: #2f302c;
        --desktop-surface-tertiary: #383a34;
        --desktop-line: #3e413a;
        --desktop-line-strong: #51554b;
        --desktop-text: #f4f4ef;
        --desktop-muted: #b7bbae;
        --desktop-tertiary: #8f9488;
        --desktop-selection: color-mix(in srgb, var(--desktop-accent) 22%, #1f1f1c);
        --desktop-hover: rgba(244, 244, 239, 0.06);
      }
    }

    *, *::before, *::after {
      box-sizing: border-box;
    }

    .sr-only {
      position: absolute;
      width: 1px;
      height: 1px;
      padding: 0;
      margin: -1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }

    .dock {
      position: fixed;
      left: 0;
      top: 0;
      right: 0;
      z-index: 2147483647;
      height: 34px;
      pointer-events: none;
    }

    .titlebar-drag-strip {
      pointer-events: auto;
      position: absolute;
      inset: 0;
      z-index: 0;
      height: 34px;
      -webkit-user-select: none;
      user-select: none;
    }

    .titlebar-back {
      pointer-events: auto;
      position: absolute;
      top: 4px;
      left: 10px;
      z-index: 1;
    }

    .platform-macos .titlebar-back {
      left: 86px;
    }

    .titlebar-tools-inner {
      pointer-events: auto;
      position: absolute;
      top: 4px;
      right: 10px;
      z-index: 1;
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 0;
    }

    .titlebar-button,
    .titlebar-version {
      appearance: none;
      min-height: 26px;
      border: 1px solid color-mix(in srgb, var(--desktop-line) 58%, transparent);
      background: color-mix(in srgb, var(--desktop-surface-secondary) 64%, transparent);
      color: var(--desktop-muted);
      font: inherit;
      cursor: pointer;
      transition:
        transform 150ms ease,
        background 150ms ease,
        border-color 150ms ease,
        color 150ms ease;
    }

    .titlebar-button,
    .titlebar-version {
      display: inline-grid;
      place-items: center;
      padding: 0;
      border-radius: var(--desktop-radius-pill);
    }

    .titlebar-button {
      width: 28px;
      height: 26px;
      font-size: 12px;
      font-weight: 800;
    }

    .titlebar-button.language {
      width: 28px;
    }

    .titlebar-button.live {
      color: var(--desktop-accent);
      border-color: color-mix(in srgb, var(--desktop-accent) 30%, var(--desktop-line));
      background: var(--desktop-selection);
    }

    .titlebar-button svg,
    .titlebar-version svg,
    .option-icon {
      width: 14px;
      height: 14px;
      display: block;
    }

    .option-icon.han-icon {
      width: 16px;
      height: 16px;
    }

    .titlebar-theme-wrap {
      position: relative;
      display: inline-grid;
      place-items: center;
    }

    .titlebar-theme-button {
      width: 28px;
      height: 26px;
      display: inline-grid;
      place-items: center;
      border: 1px solid color-mix(in srgb, var(--desktop-line) 58%, transparent);
      border-radius: var(--desktop-radius-pill);
      background: color-mix(in srgb, var(--desktop-surface-secondary) 64%, transparent);
      color: var(--desktop-muted);
      cursor: pointer;
      transition:
        transform 150ms ease,
        background 150ms ease,
        border-color 150ms ease,
        color 150ms ease;
    }

    .titlebar-theme-swatch {
      width: 12px;
      height: 12px;
      border: 1px solid color-mix(in srgb, var(--desktop-line) 40%, transparent);
      border-radius: var(--desktop-radius-pill);
      background: var(--theme-accent, var(--desktop-accent));
      box-shadow: 0 0 0 1px color-mix(in srgb, var(--theme-accent, var(--desktop-accent)) 34%, transparent);
    }

    .titlebar-theme-menu {
      position: absolute;
      top: 32px;
      right: 0;
      z-index: 4;
      width: min(360px, calc(100vw - 20px));
      max-height: min(400px, calc(100vh - 60px));
      overflow: auto;
      display: flex;
      flex-wrap: wrap;
      align-items: flex-start;
      gap: 6px;
      padding: 8px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-md);
      background: var(--desktop-surface);
      box-shadow: 0 10px 28px rgba(0, 0, 0, 0.12);
    }

    .titlebar-theme-menu:has(.titlebar-accent-wheel-popover) {
      width: min(440px, calc(100vw - 20px));
      max-height: none;
      overflow: visible;
    }

    .titlebar-theme-option-wrap {
      position: relative;
      width: 24px;
      height: 24px;
      display: inline-grid;
      place-items: center;
    }

    .titlebar-theme-option {
      width: 24px;
      height: 24px;
      display: inline-grid;
      place-items: center;
      border: 1px solid color-mix(in srgb, var(--desktop-line) 70%, transparent);
      border-radius: var(--desktop-radius-pill);
      background: var(--theme-accent, var(--desktop-accent));
      color: var(--desktop-text);
      cursor: pointer;
      box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--desktop-surface) 58%, transparent);
    }

    .titlebar-theme-option-wrap.active .titlebar-theme-option {
      border-color: color-mix(in srgb, var(--theme-accent, var(--desktop-accent)) 55%, var(--desktop-line));
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--theme-accent, var(--desktop-accent)) 18%, transparent);
    }

    .titlebar-theme-option-swatch {
      width: 100%;
      height: 100%;
      border-radius: inherit;
      background: var(--theme-accent, var(--desktop-accent));
    }

    .titlebar-theme-option.add {
      flex: 0 0 24px;
      background: var(--desktop-surface-secondary);
      box-shadow: none;
      font-size: 16px;
      font-weight: 700;
      line-height: 1;
    }

    .titlebar-theme-delete {
      position: absolute;
      top: -6px;
      right: -6px;
      width: 16px;
      height: 16px;
      display: inline-grid;
      place-items: center;
      padding: 0;
      border: 1px solid color-mix(in srgb, #c24141 38%, var(--desktop-line));
      border-radius: var(--desktop-radius-pill);
      background: var(--desktop-surface);
      color: color-mix(in srgb, #c24141 86%, var(--desktop-text));
      opacity: 0;
      pointer-events: none;
      transition:
        opacity 150ms ease,
        transform 150ms ease;
    }

    .titlebar-theme-delete svg {
      width: 10px;
      height: 10px;
    }

    .titlebar-theme-option-wrap:hover .titlebar-theme-delete,
    .titlebar-theme-option-wrap:focus-within .titlebar-theme-delete {
      opacity: 1;
      pointer-events: auto;
    }

    .titlebar-theme-draft {
      position: relative;
      flex: 1 0 100%;
      display: grid;
      grid-template-columns: minmax(0, 1fr) 30px;
      grid-template-areas:
        "fields accent"
        "actions actions";
      align-items: start;
      gap: 8px 9px;
      margin-top: 2px;
      padding-top: 8px;
      border-top: 1px solid var(--desktop-line);
    }

    .titlebar-theme-draft:has(.titlebar-accent-wheel-popover) {
      z-index: 3;
      min-height: 148px;
      grid-template-columns: minmax(0, 1fr) 136px;
      grid-template-areas:
        "fields accent"
        "actions accent";
      align-items: center;
      gap: 10px 12px;
      padding-top: 10px;
    }

    .titlebar-theme-draft-accent {
      grid-area: accent;
      justify-self: end;
      margin-top: 1px;
      position: relative;
      z-index: 4;
    }

    .titlebar-theme-draft:has(.titlebar-accent-wheel-popover) .titlebar-theme-draft-accent {
      width: 124px;
      height: 124px;
      align-self: center;
      margin-top: 0;
    }

    .titlebar-theme-draft .theme-draft-fields {
      grid-area: fields;
    }

    .titlebar-theme-draft:has(.titlebar-accent-wheel-popover) .theme-draft-fields {
      display: grid;
      gap: 10px;
    }

    .theme-color-picker-label {
      color: var(--desktop-muted);
      font-size: 0.78rem;
      font-weight: 700;
    }

    .theme-preset-row {
      display: flex;
      align-items: center;
      gap: 7px;
    }

    .theme-preset-swatch {
      appearance: none;
      width: 20px;
      height: 20px;
      padding: 0;
      border: 1px solid color-mix(in srgb, var(--desktop-surface) 82%, transparent);
      border-radius: var(--desktop-radius-pill);
      background: var(--preset-accent);
      box-shadow:
        inset 0 0 0 1px color-mix(in srgb, var(--desktop-text) 10%, transparent),
        0 7px 14px -12px rgba(0, 0, 0, 0.48);
      cursor: pointer;
      transition:
        transform 160ms ease,
        box-shadow 160ms ease;
    }

    .theme-preset-swatch:hover,
    .theme-preset-swatch:focus-visible {
      transform: translateY(-1px);
      box-shadow:
        inset 0 0 0 1px color-mix(in srgb, var(--desktop-text) 10%, transparent),
        0 0 0 3px color-mix(in srgb, var(--preset-accent) 20%, transparent);
    }

    .theme-preset-swatch.selected {
      box-shadow:
        inset 0 0 0 1px color-mix(in srgb, var(--desktop-text) 10%, transparent),
        0 0 0 3px color-mix(in srgb, var(--preset-accent) 24%, transparent);
    }

    .titlebar-theme-draft .theme-color-inputs {
      grid-template-columns: minmax(104px, 1fr) repeat(3, minmax(50px, 0.42fr));
      gap: 6px;
    }

    .titlebar-theme-draft:has(.titlebar-accent-wheel-popover) .theme-color-inputs {
      grid-template-columns: minmax(112px, 1fr) repeat(3, minmax(44px, 0.34fr));
      gap: 6px;
    }

    .titlebar-theme-draft-actions {
      grid-area: actions;
      display: inline-flex;
      justify-content: flex-end;
      gap: 6px;
    }

    .titlebar-theme-draft .tiny-button {
      min-height: 28px;
      min-width: 46px;
      padding-inline: 10px;
    }

    .titlebar-accent-wheel {
      appearance: none;
      position: relative;
      z-index: 2;
      width: 30px;
      height: 30px;
      display: grid;
      place-items: center;
      padding: 0;
      border: 0;
      border-radius: var(--desktop-radius-pill);
      overflow: hidden;
      background: conic-gradient(
        from 180deg,
        #ff3b30,
        #ff9500,
        #ffcc00,
        #34c759,
        #32ade6,
        #007aff,
        #af52de,
        #ff2d55,
        #ff3b30
      );
      box-shadow:
        inset 0 0 0 1px color-mix(in srgb, var(--desktop-text) 10%, transparent),
        0 0 0 4px color-mix(in srgb, var(--custom-accent) 14%, transparent);
      cursor: pointer;
      transition:
        transform 180ms ease,
        box-shadow 180ms ease;
    }

    .titlebar-accent-wheel::before,
    .titlebar-accent-wheel-large::before {
      content: '';
      position: absolute;
      z-index: 1;
      border-radius: inherit;
      pointer-events: none;
      background:
        radial-gradient(
          circle at 50% 32%,
          #ffffff 0,
          rgba(255, 255, 255, 0.78) 24%,
          rgba(255, 255, 255, 0) 58%
        ),
        linear-gradient(to bottom, rgba(255, 255, 255, 0) 42%, rgba(0, 0, 0, 0.62) 100%),
        conic-gradient(
          from 180deg,
          #ff3b30,
          #ff9500,
          #ffcc00,
          #34c759,
          #32ade6,
          #007aff,
          #af52de,
          #ff2d55,
          #ff3b30
        );
    }

    .titlebar-accent-wheel::before {
      inset: 6px;
    }

    .titlebar-accent-wheel:hover {
      transform: scale(1.06);
    }

    .titlebar-accent-wheel.expanded {
      display: none;
    }

    .titlebar-accent-wheel span {
      display: none;
    }

    .titlebar-accent-wheel-popover {
      position: absolute;
      z-index: 1;
      top: 0;
      right: 0;
      margin: 0;
      width: 124px;
      height: 124px;
      padding: 0;
      border: 0;
      border-radius: var(--desktop-radius-pill);
      background: transparent;
      box-shadow: none;
      transform-origin: 50% 50%;
      will-change: transform, opacity;
      animation: accent-wheel-pop 180ms cubic-bezier(0.16, 1, 0.3, 1);
    }

    .titlebar-accent-wheel-large {
      position: relative;
      width: 124px;
      height: 124px;
      border-radius: var(--desktop-radius-pill);
      overflow: hidden;
      background: conic-gradient(
        from 180deg,
        #ff3b30,
        #ff9500,
        #ffcc00,
        #34c759,
        #32ade6,
        #007aff,
        #af52de,
        #ff2d55,
        #ff3b30
      );
      box-shadow:
        inset 0 0 0 1px color-mix(in srgb, var(--desktop-text) 10%, transparent),
        0 16px 32px -22px rgba(0, 0, 0, 0.32);
      cursor: pointer;
      touch-action: none;
      outline: none;
    }

    .titlebar-accent-wheel-large::before {
      inset: 24px;
    }

    .titlebar-accent-wheel-marker {
      position: absolute;
      z-index: 2;
      left: var(--wheel-x);
      top: var(--wheel-y);
      width: 14px;
      height: 14px;
      border: 2px solid var(--desktop-surface);
      border-radius: var(--desktop-radius-pill);
      background: var(--custom-accent);
      box-shadow: 0 1px 5px rgba(0, 0, 0, 0.22);
      transform: translate(-50%, -50%);
      pointer-events: none;
    }

    @keyframes accent-wheel-pop {
      0% {
        opacity: 0;
        transform: scale(0.96) rotate(-8deg);
      }
      100% {
        opacity: 1;
        transform: scale(1) rotate(0deg);
      }
    }

    @keyframes accent-wheel-origin-flyout {
      0% {
        opacity: 1;
        transform: translate3d(405px, -50%, 0) scale(1) rotate(0deg);
      }
      52% {
        opacity: 0.9;
        transform: translate3d(185px, -50%, 0) scale(2.55) rotate(-116deg);
      }
      100% {
        opacity: 0;
        transform: translate3d(-51px, -50%, 0) scale(4.4) rotate(-236deg);
      }
    }

    @media (prefers-reduced-motion: reduce) {
      .titlebar-accent-wheel.expanded {
        opacity: 0;
        animation: none;
        transform: none;
      }

      .titlebar-accent-wheel-popover {
        animation: none;
      }
    }

    .titlebar-version {
      position: relative;
      grid-auto-flow: column;
      gap: 4px;
      min-width: 58px;
      padding: 3px 9px;
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      font-size: 10px;
      font-weight: 700;
      letter-spacing: 0.03em;
    }

    .titlebar-version.has-update {
      color: var(--desktop-text);
      border-color: color-mix(in srgb, #d63232 36%, var(--desktop-line));
    }

    .titlebar-version.has-update::after {
      content: "";
      position: absolute;
      top: 3px;
      right: 5px;
      width: 7px;
      height: 7px;
      border-radius: var(--desktop-radius-pill);
      background: #d63232;
      box-shadow: 0 0 0 2px color-mix(in srgb, #d63232 16%, var(--desktop-surface));
    }

    .titlebar-version.error {
      border-color: color-mix(in srgb, #c24141 34%, var(--desktop-line));
    }

    .titlebar-release-popover {
      position: absolute;
      top: 32px;
      right: 0;
      z-index: 4;
      width: min(360px, calc(100vw - 20px));
      max-height: min(420px, calc(100vh - 52px));
      display: grid;
      gap: 10px;
      overflow: auto;
      padding: 12px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-lg);
      background: var(--desktop-surface);
      color: var(--desktop-text);
      box-shadow: 0 12px 34px rgba(0, 0, 0, 0.14);
    }

    .titlebar-release-head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
    }

    .titlebar-release-title {
      min-width: 0;
      display: grid;
      gap: 3px;
    }

    .titlebar-release-title strong {
      font-size: 13px;
      line-height: 1.2;
    }

    .titlebar-release-title span {
      color: var(--desktop-muted);
      font-size: 11px;
      line-height: 1.2;
    }

    .titlebar-release-body {
      max-height: 240px;
      overflow: auto;
      white-space: pre-wrap;
      color: var(--desktop-muted);
      font-size: 12px;
      line-height: 1.55;
    }

    .titlebar-release-actions {
      display: flex;
      justify-content: flex-end;
      gap: 8px;
    }

    .service-log-backdrop {
      position: fixed;
      inset: 0;
      z-index: 8;
      display: grid;
      place-items: center;
      padding: 44px 12px 12px;
      background: color-mix(in srgb, var(--desktop-canvas) 72%, transparent);
      backdrop-filter: blur(14px);
      pointer-events: auto;
    }

    .service-log-dialog {
      width: min(900px, calc(100vw - 24px));
      max-height: min(640px, calc(100vh - 58px));
      min-height: min(460px, calc(100vh - 58px));
      display: grid;
      grid-template-rows: auto auto minmax(0, 1fr) auto;
      gap: 7px;
      padding: 10px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-lg);
      background: var(--desktop-surface);
      color: var(--desktop-text);
      box-shadow: 0 20px 64px rgba(0, 0, 0, 0.18);
    }

    .service-log-head,
    .service-log-toolbar,
    .service-log-actions,
    .service-log-loading {
      display: flex;
      align-items: center;
    }

    .service-log-head {
      justify-content: space-between;
      gap: 10px;
    }

    .service-log-title {
      min-width: 0;
      display: grid;
      gap: 1px;
    }

    .service-log-title strong {
      overflow: hidden;
      font-size: 14px;
      line-height: 1.2;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .service-log-path {
      min-width: 0;
      overflow: hidden;
      color: var(--desktop-muted);
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      font-size: 11px;
      line-height: 1.3;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .service-log-controls {
      min-width: 0;
      display: grid;
      grid-template-columns: 1fr;
      gap: 7px;
      padding: 6px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-md);
      background: color-mix(in srgb, var(--desktop-surface-secondary) 70%, transparent);
    }

    .service-log-toolbar {
      display: grid;
      grid-template-columns: 1fr auto auto;
      align-items: center;
      gap: 7px;
    }

    .service-log-timebar {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto 28px;
      align-items: end;
      gap: 5px;
    }

    .service-log-time-field {
      min-width: 0;
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(0, 0.72fr);
      gap: 4px;
      margin: 0;
      padding: 0;
      border: 0;
      color: var(--desktop-muted);
      font-size: 9px;
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      font-weight: 700;
      letter-spacing: 0.04em;
      text-transform: uppercase;
    }

    .service-log-time-field legend {
      grid-column: 1 / -1;
      padding: 0;
    }

    .service-log-time-input {
      min-width: 0;
      height: 28px;
      display: grid;
      grid-template-columns: 14px minmax(0, 1fr);
      align-items: center;
      gap: 5px;
      padding: 0 6px;
      min-height: 28px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-sm);
      background: var(--desktop-surface-secondary);
      color: var(--desktop-text);
    }

    .service-log-time-input .option-icon {
      width: 14px;
      height: 14px;
      color: var(--desktop-muted);
    }

    .service-log-time-input input {
      width: 100%;
      min-width: 0;
      border: 0;
      outline: 0;
      background: transparent;
      color: var(--desktop-text);
      font: inherit;
      font-size: 10px;
      letter-spacing: 0;
      text-transform: none;
    }

    .service-log-range-select {
      grid-column: auto;
      min-width: 82px;
      height: 28px;
      display: grid;
      grid-template-columns: 14px minmax(0, 1fr);
      align-items: center;
      gap: 5px;
      padding: 0 6px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-sm);
      background: var(--desktop-surface-secondary);
      color: var(--desktop-muted);
    }

    .service-log-range-select .option-icon {
      width: 14px;
      height: 14px;
    }

    .service-log-range-select select {
      min-width: 0;
      border: 0;
      outline: 0;
      background: transparent;
      color: var(--desktop-text);
      font: inherit;
      font-size: 10px;
      cursor: pointer;
    }

    .service-log-range-button:disabled,
    .service-log-range-select:has(select:disabled),
    .service-log-range-select select:disabled,
    .service-log-time-input:has(input:disabled),
    .service-log-time-input input:disabled {
      cursor: not-allowed;
      opacity: 0.55;
    }

    .service-log-range-button {
      align-self: end;
      width: 28px;
      height: 28px;
      color: var(--desktop-accent);
    }

    .service-log-search {
      grid-column: 1 / -1;
      min-width: 0;
    }

    .service-log-count {
      min-width: 68px;
      justify-content: center;
    }

    .service-log-actions {
      gap: 4px;
    }

    .service-log-body {
      min-height: 0;
      overflow: hidden;
      display: grid;
    }

    .service-log-content {
      margin: 0;
      min-height: 0;
      overflow: auto;
      padding: 10px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-md);
      background: var(--desktop-surface-secondary);
      color: var(--desktop-text);
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      font-size: 11px;
      line-height: 1.55;
      white-space: pre-wrap;
      word-break: break-word;
      outline: none;
      scrollbar-gutter: stable;
    }

    .service-log-content:focus {
      border-color: color-mix(in srgb, var(--desktop-accent) 32%, var(--desktop-line));
      box-shadow: 0 0 0 3px color-mix(in srgb, var(--desktop-accent) 12%, transparent);
    }

    .service-log-content mark {
      border-radius: var(--desktop-radius-xs);
      background: color-mix(in srgb, #facc15 58%, transparent);
      color: inherit;
      padding: 0 2px;
    }

    .service-log-content mark.active {
      background: color-mix(in srgb, var(--desktop-accent) 38%, #facc15);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--desktop-accent) 24%, transparent);
    }

    .service-log-loading {
      gap: 8px;
      min-height: 40px;
      color: var(--desktop-muted);
      font-size: 12px;
    }

    .service-log-error,
    .service-log-empty,
    .service-log-truncated {
      margin: 0;
    }

    button {
      font: inherit;
      touch-action: manipulation;
    }

	    .mode-option,
	    .theme-option,
      .theme-row,
      .theme-select,
      .tiny-button,
	    .nav-item,
	    .settings-icon-button,
      .service-row-wrap,
      .service-row,
      .service-open-button,
      .titlebar-button,
      .titlebar-theme-button,
      .titlebar-theme-delete,
      .titlebar-theme-option,
      .titlebar-version {
	      pointer-events: auto;
	      appearance: none;
	      cursor: pointer;
	      transition:
        transform 150ms ease,
        background 150ms ease,
        box-shadow 150ms ease,
        opacity 150ms ease;
    }

    .panel {
      pointer-events: auto;
      position: absolute;
      right: 10px;
      top: 38px;
      bottom: auto;
      width: min(420px, calc(100vw - 28px));
      max-height: min(620px, calc(100vh - 96px));
      overflow: auto;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-xl);
      background: var(--desktop-surface);
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.08);
      opacity: 0;
      transform: translateY(10px) scale(0.98);
      transition:
        opacity 180ms ease,
        transform 180ms ease;
    }

    .dock[data-align="left"] .panel {
      left: auto;
      right: 10px;
    }

    .dock[data-vertical="top"] .panel {
      top: 38px;
      bottom: auto;
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
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-lg);
      background: var(--desktop-canvas);
      overflow: hidden;
    }

    .nav {
      display: grid;
      align-content: start;
      gap: 4px;
      padding: 10px;
      background: var(--desktop-sidebar);
    }

	    .nav-item {
	      appearance: none;
	      width: 100%;
	      min-height: 38px;
	      display: flex;
	      align-items: center;
	      gap: 8px;
	      padding: 8px;
	      border: 0;
	      border-radius: var(--desktop-radius-md);
	      background: transparent;
	      color: var(--desktop-text);
	      font-size: 13px;
	      font-weight: 600;
	      text-align: left;
	      cursor: pointer;
	    }

	    .nav-item.active {
	      background: var(--desktop-selection);
	      box-shadow: none;
	    }

	    .nav-item.inert {
	      cursor: default;
	      color: var(--desktop-muted);
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

    .content-head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
    }

    .theme-row,
    .theme-draft {
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      align-items: center;
      gap: 8px;
      padding: 6px;
      border: 1px solid color-mix(in srgb, var(--desktop-line) 72%, transparent);
      border-radius: var(--desktop-radius-lg);
      background: transparent;
    }

    .theme-row.active {
      border-color: color-mix(in srgb, var(--desktop-accent) 28%, var(--desktop-line));
      background: var(--desktop-selection);
    }

    .theme-select {
      appearance: none;
      min-width: 0;
      min-height: 46px;
      display: grid;
      grid-template-columns: 44px minmax(0, 1fr) 18px;
      align-items: center;
      gap: 9px;
      padding: 0;
      border: 0;
      background: transparent;
      color: var(--desktop-text);
      text-align: left;
      cursor: pointer;
    }

    .theme-select:disabled {
      cursor: default;
      opacity: 0.72;
    }

    .theme-actions {
      display: inline-flex;
      align-items: center;
      gap: 5px;
      opacity: 0;
      transition: opacity 150ms ease;
    }

    .theme-row:hover .theme-actions,
    .theme-row:focus-within .theme-actions {
      opacity: 1;
    }

    .theme-draft {
      grid-template-columns: 30px minmax(0, 1fr) auto auto;
      border-style: dashed;
      background: var(--desktop-surface-secondary);
    }

    .theme-draft-fields {
      min-width: 0;
      display: grid;
      gap: 6px;
    }

    .theme-color-inputs {
      display: grid;
      grid-template-columns: minmax(88px, 1fr) repeat(3, minmax(42px, 0.42fr));
      gap: 5px;
    }

    .theme-hex-input,
    .theme-rgb-input {
      min-width: 0;
      min-height: 26px;
      display: grid;
      grid-template-columns: auto minmax(0, 1fr);
      align-items: center;
      gap: 4px;
      padding: 3px 6px;
      border: 1px solid color-mix(in srgb, var(--desktop-line) 72%, transparent);
      border-radius: var(--desktop-radius-sm);
      background: var(--desktop-surface);
      color: var(--desktop-muted);
      font-size: 10px;
      font-weight: 700;
    }

    .theme-hex-input input,
    .theme-rgb-input input {
      width: 100%;
      min-width: 0;
      padding: 0;
      border: 0;
      outline: 0;
      background: transparent;
      color: var(--desktop-text);
      font: inherit;
      letter-spacing: 0;
    }

    .theme-hex-input:focus-within,
    .theme-rgb-input:focus-within {
      border-color: color-mix(in srgb, var(--desktop-accent) 34%, var(--desktop-line));
      box-shadow: 0 0 0 3px color-mix(in srgb, var(--desktop-accent) 12%, transparent);
    }

    .theme-name-input {
      width: 100%;
      min-width: 0;
      min-height: 30px;
      border: 1px solid color-mix(in srgb, var(--desktop-line) 78%, transparent);
      border-radius: var(--desktop-radius-sm);
      background: var(--desktop-surface);
      color: var(--desktop-text);
      padding: 4px 8px;
      outline: 0;
      font: inherit;
      font-size: 12px;
      font-weight: 650;
    }

    .theme-name-input:focus {
      border-color: color-mix(in srgb, var(--desktop-accent) 40%, var(--desktop-line));
      box-shadow: 0 0 0 3px color-mix(in srgb, var(--desktop-accent) 16%, transparent);
    }

    .accent-dot {
      position: relative;
      width: 24px;
      height: 24px;
      display: inline-block;
      border-radius: var(--desktop-radius-pill);
      background: var(--custom-accent);
      box-shadow:
        inset 0 0 0 1px color-mix(in srgb, var(--desktop-text) 12%, transparent),
        0 0 0 3px color-mix(in srgb, var(--desktop-surface) 70%, transparent);
      cursor: pointer;
    }

    .accent-dot.large {
      width: 28px;
      height: 28px;
    }

    .accent-dot input {
      position: absolute;
      inset: 0;
      width: 100%;
      height: 100%;
      opacity: 0;
      cursor: pointer;
    }

    .tiny-button {
      min-height: 28px;
      padding: 4px 9px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-sm);
      background: var(--desktop-surface-secondary);
      color: var(--desktop-text);
      font-size: 12px;
      font-weight: 650;
      cursor: pointer;
    }

    .tiny-button.accent {
      border-color: color-mix(in srgb, var(--desktop-accent) 45%, var(--desktop-line));
      background: var(--desktop-accent);
      color: #fff;
    }

    .tiny-button.muted {
      color: var(--desktop-muted);
      background: transparent;
    }

    .quick-controls {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      padding: 0 14px 14px;
    }

    .mode-list,
    .language-list {
      display: inline-flex;
      gap: 4px;
      padding: 3px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-md);
      background: var(--desktop-surface-secondary);
    }

    .mode-option {
      min-width: 34px;
      min-height: 30px;
      display: inline-grid;
      place-items: center;
      border: 0;
      border-radius: var(--desktop-radius-sm);
      background: transparent;
      color: var(--desktop-text);
      font-size: 12px;
      font-weight: 600;
    }

    .language-option {
      min-width: 34px;
      min-height: 30px;
      display: inline-grid;
      place-items: center;
      padding: 0;
      border: 0;
      border-radius: var(--desktop-radius-sm);
      background: transparent;
      color: var(--desktop-text);
      font-size: 12px;
      font-weight: 600;
      white-space: nowrap;
    }

    .language-option.active,
    .mode-option.active {
      background: var(--desktop-surface);
      color: var(--desktop-accent);
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
    }

	    .theme-option {
	      min-height: 58px;
	      display: grid;
      grid-template-columns: 52px minmax(0, 1fr) 20px;
      align-items: center;
	      gap: 10px;
	      padding: 8px;
	      border: 0;
	      border-radius: var(--desktop-radius-lg);
      background: transparent;
      color: var(--desktop-text);
      text-align: left;
    }

    .theme-option.active {
      background: var(--desktop-selection);
      box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--desktop-accent) 24%, var(--desktop-line));
    }

    .theme-option:disabled {
      cursor: default;
      opacity: 0.72;
    }

    .swatch {
      display: grid;
      grid-template-columns: 1fr 1fr;
      grid-template-rows: 1fr 1fr;
      gap: 4px;
      min-height: 38px;
      padding: 4px;
      border-radius: var(--desktop-radius-sm);
      background: var(--theme-canvas);
      box-shadow: inset 0 0 0 1px var(--theme-line);
    }

    .swatch span {
      border-radius: var(--desktop-radius-xs);
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
      border-radius: var(--desktop-radius-md);
      background: var(--desktop-surface-secondary);
      color: var(--desktop-muted);
	      font-size: 12px;
	    }

	    .service-panel {
	      display: grid;
	      gap: 12px;
	    }

	    .service-panel-head {
	      display: flex;
	      align-items: flex-start;
	      justify-content: space-between;
	      gap: 12px;
	    }

	    .service-description,
	    .service-empty {
	      margin: 4px 0 0;
	      color: var(--desktop-muted);
	      font-size: 12px;
	      line-height: 1.45;
	    }

	    .service-actions {
	      display: inline-flex;
        align-items: center;
	      gap: 6px;
	      flex: 0 0 auto;
	    }

      .service-toggle-button {
        width: 32px;
        height: 32px;
        display: inline-grid;
        place-items: center;
        padding: 0;
        border: 1px solid color-mix(in srgb, var(--desktop-accent) 42%, var(--desktop-line));
        border-radius: var(--desktop-radius-pill);
        background: var(--desktop-accent);
        color: #ffffff;
      }

      .service-toggle-button.running {
        color: var(--desktop-accent);
        background: var(--desktop-selection);
        border-color: color-mix(in srgb, var(--desktop-accent) 34%, var(--desktop-line));
      }

      .service-toggle-button svg {
        width: 16px;
        height: 16px;
        display: block;
      }

      .service-count {
        min-height: 28px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        padding: 4px 8px;
        border-radius: var(--desktop-radius-pill);
        background: var(--desktop-surface-secondary);
        color: var(--desktop-muted);
        font-size: 11px;
        font-weight: 700;
      }

		    .settings-icon-button {
		      width: 32px;
		      height: 32px;
		      display: inline-grid;
		      place-items: center;
		      padding: 0;
		      border: 1px solid color-mix(in srgb, var(--desktop-line) 72%, transparent);
		      border-radius: var(--desktop-radius-pill);
		      background: color-mix(in srgb, var(--desktop-surface-secondary) 58%, transparent);
		      color: var(--desktop-muted);
		      font-size: 14px;
		    }

        .settings-icon-button.compact {
          width: 28px;
          height: 28px;
          font-size: 13px;
        }

		    .settings-icon-button svg {
		      width: 16px;
		      height: 16px;
		      display: block;
		    }

		    .settings-icon-button.positive:hover {
		      color: var(--desktop-accent);
		      background: var(--desktop-selection);
		      border-color: color-mix(in srgb, var(--desktop-accent) 32%, var(--desktop-line));
		    }

		    .settings-icon-button.danger:hover {
		      color: color-mix(in srgb, #c24141 82%, var(--desktop-text));
		      background: color-mix(in srgb, #c24141 10%, var(--desktop-surface-secondary));
		      border-color: color-mix(in srgb, #c24141 30%, var(--desktop-line));
		    }

	    .settings-icon-button:disabled,
      .service-toggle-button:disabled,
	    .service-open-button:disabled,
	    .service-row:disabled {
	      cursor: default;
	      opacity: 0.64;
	      transform: none;
	    }

	    .service-facts {
	      display: grid;
	      grid-template-columns: 108px minmax(0, 1fr);
	      gap: 7px 10px;
	      padding: 10px;
	      border: 1px solid var(--desktop-line);
	      border-radius: var(--desktop-radius-md);
	      background: var(--desktop-surface-secondary);
	      font-size: 12px;
	    }

	    .service-facts span {
	      color: var(--desktop-muted);
	    }

	    .service-facts strong {
	      min-width: 0;
	      overflow: hidden;
	      color: var(--desktop-text);
	      text-overflow: ellipsis;
	      white-space: nowrap;
	    }

	    .service-list {
	      display: grid;
	      gap: 7px;
	      max-height: 190px;
	      overflow: auto;
	    }

      .server-search {
        min-height: 32px;
        display: grid;
        grid-template-columns: 16px minmax(0, 1fr);
        align-items: center;
        gap: 8px;
        padding: 0 10px;
        border: 1px solid color-mix(in srgb, var(--desktop-line) 66%, transparent);
        border-radius: var(--desktop-radius-md);
        background: var(--desktop-surface-secondary);
        color: var(--desktop-muted);
      }

      .server-search input {
        width: 100%;
        min-width: 0;
        border: 0;
        outline: 0;
        background: transparent;
        color: var(--desktop-text);
        font: inherit;
        font-size: 12px;
      }

      .server-search input::placeholder {
        color: color-mix(in srgb, var(--desktop-muted) 76%, transparent);
      }

      .server-search:focus-within {
        border-color: color-mix(in srgb, var(--desktop-accent) 32%, var(--desktop-line));
        box-shadow: 0 0 0 3px color-mix(in srgb, var(--desktop-accent) 12%, transparent);
      }

      .service-row-wrap {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: center;
        gap: 7px;
        border: 1px solid color-mix(in srgb, var(--desktop-line) 72%, transparent);
        border-radius: var(--desktop-radius-md);
        background: transparent;
      }

      .service-row-wrap.active {
        border-color: color-mix(in srgb, var(--desktop-accent) 28%, var(--desktop-line));
        background: var(--desktop-selection);
      }

	    .service-row {
	      display: grid;
	      grid-template-columns: minmax(0, 1fr) auto;
	      align-items: center;
	      gap: 10px;
	      min-height: 54px;
	      padding: 8px 10px;
	      border: 0;
	      border-radius: var(--desktop-radius-md);
	      background: transparent;
	      color: var(--desktop-text);
	      text-align: left;
	    }

	    .service-row-copy {
	      min-width: 0;
	      display: grid;
	      gap: 3px;
	    }

	    .service-row-name {
	      min-width: 0;
	      overflow: hidden;
	      text-overflow: ellipsis;
	      white-space: nowrap;
	      font-size: 13px;
	      font-weight: 700;
	    }

	    .service-row-meta {
	      min-width: 0;
	      overflow: hidden;
	      color: var(--desktop-muted);
	      font-size: 11px;
	      line-height: 1.35;
	      text-overflow: ellipsis;
	      white-space: nowrap;
	    }

	    .service-chip {
	      display: inline-flex;
	      align-items: center;
	      min-height: 24px;
	      padding: 0 8px;
	      border-radius: var(--desktop-radius-pill);
	      background: var(--desktop-surface-secondary);
	      color: var(--desktop-muted);
	      font-size: 11px;
	      font-weight: 700;
	      white-space: nowrap;
	    }

      .service-log-button {
        margin-right: 7px;
      }

	    .service-chip.live {
	      background: var(--desktop-selection);
	      color: var(--desktop-accent);
	    }

	    .service-open-button {
	      min-height: 34px;
	      border: 1px solid color-mix(in srgb, var(--desktop-accent) 32%, var(--desktop-line));
	      border-radius: var(--desktop-radius-md);
	      background: var(--desktop-accent);
	      color: #ffffff;
	      font-size: 12px;
	      font-weight: 700;
	    }

      .service-open-button.secondary {
        background: var(--desktop-surface-secondary);
        color: var(--desktop-text);
        border-color: var(--desktop-line);
      }

      .updates-panel {
        display: grid;
        gap: 12px;
      }

      .updates-actions {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
      }

      .updates-actions .service-open-button {
        padding: 0 12px;
      }

    @media (hover: hover) {
      .theme-option:hover {
        background: var(--desktop-hover);
      }

      .mode-option:hover {
        background: var(--desktop-hover);
      }

	      .language-option:hover {
	        background: var(--desktop-hover);
	      }

		      .nav-item:hover,
		      .settings-icon-button:hover,
		      .service-row-wrap:hover,
		      .service-row:hover {
		        background: var(--desktop-hover);
		      }

		      .settings-icon-button:hover {
		        border-color: color-mix(in srgb, var(--desktop-text) 18%, var(--desktop-line));
		      }

      .titlebar-button:hover,
      .titlebar-theme-button:hover,
      .titlebar-theme-delete:hover,
      .titlebar-theme-option:hover,
      .titlebar-version:hover {
        color: var(--desktop-text);
        background: color-mix(in srgb, var(--desktop-surface) 76%, transparent);
        border-color: color-mix(in srgb, var(--desktop-text) 18%, var(--desktop-line));
        transform: translateY(-1px);
      }

        .service-toggle-button:hover,
	      .service-open-button:hover {
	        background: var(--desktop-accent-hover);
	      }

        .service-toggle-button.running:hover {
          background: color-mix(in srgb, var(--desktop-accent) 16%, var(--desktop-surface-secondary));
        }
	    }

	    .language-option:active,
	    .mode-option:active,
	    .theme-option:active,
	    .nav-item:active,
		    .settings-icon-button:active,
        .service-toggle-button:active,
        .theme-row:active,
        .theme-select:active,
        .service-row-wrap:active,
		    .service-row:active,
        .service-open-button:active,
        .titlebar-button:active,
        .titlebar-theme-button:active,
        .titlebar-theme-delete:active,
        .titlebar-theme-option:active,
        .titlebar-version:active {
		      transform: scale(0.97);
		    }

		    .service-action-icon.spinning {
		      animation: service-action-spin 900ms linear infinite;
		    }

		    @keyframes service-action-spin {
		      to {
		        transform: rotate(360deg);
		      }
		    }

    @media (max-width: 520px) {
      .titlebar-back {
        left: 8px;
      }

      .platform-macos .titlebar-back {
        left: 86px;
      }

      .titlebar-tools-inner {
        right: 8px;
        gap: 4px;
      }

      .titlebar-button {
        width: 26px;
        height: 24px;
      }

      .titlebar-button.language {
        width: 26px;
      }

      .titlebar-theme-button {
        width: 26px;
        height: 24px;
      }

      .titlebar-version {
        min-width: 52px;
        padding-inline: 7px;
      }

      .settings-grid {
        grid-template-columns: 1fr;
      }

      .nav {
        grid-auto-flow: column;
        overflow-x: auto;
      }

      .service-log-dialog {
        width: calc(100vw - 16px);
        max-height: calc(100vh - 48px);
        min-height: calc(100vh - 48px);
      }

      .service-log-controls {
        grid-template-columns: minmax(0, 1fr);
      }

      .service-log-toolbar {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
      }

      .service-log-timebar {
        grid-template-columns: minmax(0, 1fr) 28px;
      }

      .service-log-time-field {
        grid-column: 1 / -1;
      }

      .service-log-range-select {
        grid-column: 1;
      }

      .service-log-range-button {
        grid-column: 2;
        grid-row: 3;
      }

      .service-log-search {
        grid-column: 1 / -1;
      }
    }

    .titlebar-style-wrap {
      position: relative;
      display: inline-grid;
      place-items: center;
    }

    .titlebar-style-panel {
      position: absolute;
      top: 32px;
      right: 0;
      z-index: 5;
      width: min(320px, calc(100vw - 20px));
      max-height: min(400px, calc(100vh - 60px));
      overflow: auto;
      padding: 10px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-md);
      background: var(--desktop-surface);
      box-shadow: 0 10px 28px rgba(0, 0, 0, 0.12);
    }

    .titlebar-style-head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 8px;
    }

    .titlebar-style-eyebrow {
      font-size: 11px;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      color: var(--desktop-muted);
    }

    .titlebar-style-head-actions {
      display: flex;
      gap: 8px;
    }

    .text-action-button {
      appearance: none;
      border: none;
      background: none;
      color: var(--desktop-accent);
      font: inherit;
      font-size: 11px;
      cursor: pointer;
      padding: 0;
    }

    .text-action-button:hover {
      text-decoration: underline;
    }

    .text-action-button:disabled {
      opacity: 0.5;
      cursor: default;
      text-decoration: none;
    }

    .titlebar-style-list {
      display: flex;
      flex-direction: column;
      gap: 4px;
    }

    .titlebar-style-row {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 6px 8px;
      border-radius: var(--desktop-radius-sm);
      cursor: pointer;
      transition: background 150ms ease;
    }

    .titlebar-style-row:hover {
      background: var(--desktop-surface-secondary);
    }

    .titlebar-style-row.selected {
      background: var(--desktop-selection);
    }

    .titlebar-style-preview {
      display: flex;
      gap: 2px;
      flex-shrink: 0;
    }

    .titlebar-style-preview span {
      width: 10px;
      height: 22px;
      border-radius: 3px;
    }

    .titlebar-style-copy {
      flex: 1;
      min-width: 0;
      display: flex;
      flex-direction: column;
      gap: 1px;
    }

    .titlebar-style-name {
      font-size: 12px;
      font-weight: 600;
      color: var(--desktop-text);
    }

    .titlebar-style-summary {
      font-size: 11px;
      color: var(--desktop-muted);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .titlebar-style-actions {
      flex-shrink: 0;
      width: 14px;
      height: 14px;
    }

    .spin {
      animation: spin-anim 1s linear infinite;
    }

    @keyframes spin-anim {
      to { transform: rotate(360deg); }
    }

    .titlebar-agent-wrap {
      position: relative;
      display: inline-flex;
      align-items: center;
      margin-left: 2px;
      padding-left: 8px;
      border-left: 1px solid color-mix(in srgb, var(--desktop-line) 50%, transparent);
    }

    .titlebar-agent-list {
      display: inline-flex;
      align-items: center;
      gap: 4px;
    }

    .titlebar-agent-row {
      position: relative;
      display: inline-grid;
      place-items: center;
    }

    .agent-avatar-button {
      appearance: none;
      width: 22px;
      height: 22px;
      display: inline-grid;
      place-items: center;
      padding: 0;
      border: 1px solid color-mix(in srgb, var(--desktop-line) 58%, transparent);
      border-radius: var(--desktop-radius-pill);
      background: color-mix(in srgb, var(--desktop-surface-secondary) 64%, transparent);
      cursor: pointer;
      transition: background 150ms ease, border-color 150ms ease;
    }

    .agent-avatar-button:hover {
      background: var(--desktop-surface-secondary);
      border-color: var(--desktop-line);
    }

    .agent-status-dot {
      width: 8px;
      height: 8px;
      border-radius: var(--desktop-radius-pill);
      background: #10a37f;
    }

    .agent-status-dot.offline {
      background: var(--desktop-muted);
    }

    .agent-status-dot.online {
      background: #10a37f;
    }

    .agent-card {
      position: absolute;
      top: 30px;
      right: 0;
      z-index: 6;
      width: min(280px, calc(100vw - 20px));
      padding: 12px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-md);
      background: var(--desktop-surface);
      box-shadow: 0 10px 28px rgba(0, 0, 0, 0.12);
    }

    .agent-card-header {
      display: flex;
      align-items: center;
      gap: 6px;
      margin-bottom: 6px;
    }

    .agent-card-name {
      font-size: 13px;
      font-weight: 600;
      color: var(--desktop-text);
      flex: 1;
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .agent-card-status {
      font-size: 11px;
      color: var(--desktop-muted);
      text-transform: capitalize;
    }

    .agent-card-description {
      font-size: 11px;
      color: var(--desktop-muted);
      margin: 0 0 8px;
      line-height: 1.4;
    }

    .agent-card-activity {
      margin-bottom: 10px;
    }

    .agent-card-activity-title {
      font-size: 11px;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      color: var(--desktop-muted);
      margin: 0 0 4px;
    }

    .agent-card-activity-list {
      list-style: none;
      padding: 0;
      margin: 0;
      display: flex;
      flex-direction: column;
      gap: 3px;
    }

    .agent-card-activity-list li {
      display: flex;
      justify-content: space-between;
      gap: 8px;
      font-size: 11px;
    }

    .activity-text {
      color: var(--desktop-text);
      flex: 1;
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .activity-time {
      color: var(--desktop-muted);
      flex-shrink: 0;
    }

    .inline-note {
      font-size: 11px;
      color: var(--desktop-muted);
      margin: 0;
    }

    .agent-card-actions {
      display: flex;
      gap: 6px;
    }

    .agent-card-button {
      appearance: none;
      flex: 1;
      padding: 4px 8px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-sm);
      background: var(--desktop-surface-secondary);
      color: var(--desktop-text);
      font: inherit;
      font-size: 11px;
      font-weight: 600;
      cursor: pointer;
      transition: background 150ms ease;
    }

    .agent-card-button:hover {
      background: color-mix(in srgb, var(--desktop-surface-secondary) 80%, var(--desktop-line));
    }

    .agent-card-button.danger {
      color: #c24141;
      border-color: color-mix(in srgb, #c24141 30%, var(--desktop-line));
    }

    .agent-card-button.accent {
      color: var(--desktop-accent);
      border-color: color-mix(in srgb, var(--desktop-accent) 30%, var(--desktop-line));
    }

    .agent-card-button:disabled {
      opacity: 0.5;
      cursor: default;
    }

	    @media (prefers-reduced-motion: reduce) {
	      .language-option,
	      .mode-option,
	      .theme-option,
        .theme-row,
        .theme-select,
        .tiny-button,
	      .nav-item,
	      .settings-icon-button,
        .service-row-wrap,
        .service-row,
        .service-open-button,
        .titlebar-button,
        .titlebar-version,
        .titlebar-theme-button,
        .titlebar-theme-delete,
        .titlebar-theme-option,
	      .panel {
        transition-duration: 1ms;
      }
	    }
	  `;

  const isCustomTheme = (theme) => theme.id !== "original" && theme.id !== "default";
  const currentModeOption = () =>
    modes.find((mode) => mode.id === activeMode) || modes.find((mode) => mode.id === "system") || modes[0];
  const nextModeId = () => {
    const index = modes.findIndex((mode) => mode.id === activeMode);
    return modes[(index + 1) % modes.length]?.id || "system";
  };
  const currentLanguageOption = () =>
    languages.find((language) => language.id === activeLanguage) || languages.find((language) => language.id === "system") || languages[0];
  const nextLanguageId = () => {
    const index = languages.findIndex((language) => language.id === activeLanguage);
    return languages[(index + 1) % languages.length]?.id || "system";
  };
  const selectedServiceRunning = () => {
    const service = serviceSnapshot;
    const selected = selectedServiceServer();
    const selectedSlug = selected?.slug || service?.selectedServerSlug || "";
    return serviceServerIsRunning(service, selectedSlug);
  };
  const releaseUpdateAvailable = () =>
    !!(releaseState.latest?.available ?? releaseState.latest?.updateAvailable);
  const latestReleaseVersion = () =>
    releaseState.latest?.version || releaseState.latest?.tagName || "";
  const releaseStatusTitle = () => {
    if (releaseState.error) return releaseState.error;
    if (releaseState.loading) return t("checkingUpdates");
    if (releaseState.installing) return t("installingUpdate");
    if (releaseUpdateAvailable()) {
      const version = latestReleaseVersion();
      return version ? `${t("updateAvailable")}: ${version}` : t("updateAvailable");
    }
    if (releaseState.latest) return t("upToDate");
    return t("notChecked");
  };
  const releaseNotesText = () =>
    releaseState.latest?.body || releaseState.latest?.name || t("noReleaseNotes");
  const titlebarToolsContent = () => {
    const service = serviceSnapshot;
    const selected = selectedServiceServer();
    const selectedSlug = selected?.slug || service?.selectedServerSlug || "";
    const running = selectedServiceRunning();
    const serviceBusy = ["service-start", "service-stop", "service-load", "service-refresh", "service-status"].includes(serviceBusyAction || "");
    const mode = currentModeOption();
    const language = currentLanguageOption();
    const version = updateSnapshot?.currentVersion || releaseState.latest?.currentVersion || "";
    const versionText = version ? `v${version}` : "v...";
    const checking = releaseState.loading || updateBusyAction === "release-check";
    const installing = releaseState.installing || updateBusyAction === "release-install";
    const updateReady = releaseUpdateAvailable();
    const theme = selectedTheme();
    const themeAccent = titlebarThemeSwatch(theme);
    const themeLabel = theme ? titlebarThemeLabel(theme) : t("theme");
    const themeOptions = themeCatalog
      .map((theme) => {
        const selected = theme.id === (activeThemeId || "default");
        const swatch = titlebarThemeSwatch(theme);
        const custom = isCustomTheme(theme);
        const deleting = appearanceBusyAction === `theme-delete:${theme.id}`;
        const label = titlebarThemeLabel(theme);
        return `
          <span class="titlebar-theme-option-wrap${selected ? " active" : ""}" style="--theme-accent:${escapeHtml(swatch)}">
            <button
              class="titlebar-theme-option"
              type="button"
              data-titlebar-theme-option="${escapeHtml(theme.id)}"
              title="${escapeHtml(label)}"
              aria-label="${escapeHtml(label)}"
              ${appearanceBusyAction ? "disabled" : ""}
            ><span class="titlebar-theme-option-swatch" aria-hidden="true"></span></button>
            ${
              custom
                ? `<button class="titlebar-theme-delete" type="button" data-titlebar-theme-delete="${escapeHtml(theme.id)}" title="${t("themeDelete")}" aria-label="${t("themeDelete")}: ${escapeHtml(label)}" ${deleting ? "disabled" : ""}>${deleting ? actionIcon("refresh", true) : closeIcon()}</button>`
                : ""
            }
          </span>
        `;
      })
      .join("");
    const draftRgb = newThemeDraft ? hexToRgb(newThemeDraft.accent) : null;
    const themePresetOptions = newThemeDraft
      ? themeAccentPresets
          .map((accent) => {
            const selected = normalizeHexColor(accent) === newThemeDraft.accent;
            return `<button class="theme-preset-swatch${selected ? " selected" : ""}" type="button" data-titlebar-theme-preset="${accent}" style="--preset-accent:${accent}" title="${accent.toUpperCase()}" aria-label="${accent.toUpperCase()}"></button>`;
          })
          .join("")
      : "";
    const themeDraft = newThemeDraft
      ? `<div class="titlebar-theme-draft">
          <div class="titlebar-theme-draft-accent" style="--custom-accent:${escapeHtml(newThemeDraft.accent)}">
            <button class="titlebar-accent-wheel${titlebarThemeWheelOpen ? " expanded" : ""}" type="button" data-titlebar-theme-wheel-toggle aria-label="${t("themeAccent")}" aria-expanded="${titlebarThemeWheelOpen}"><span aria-hidden="true"></span></button>
            ${titlebarThemeWheelOpen ? `<div class="titlebar-accent-wheel-popover"><div class="titlebar-accent-wheel-large" role="slider" tabindex="0" data-titlebar-theme-wheel aria-label="${t("themeAccent")}" aria-valuetext="${escapeHtml(newThemeDraft.hexInput)}" style="${accentWheelMarkerStyle(newThemeDraft.accent)}"><span class="titlebar-accent-wheel-marker" aria-hidden="true"></span></div></div>` : ""}
          </div>
          <div class="theme-draft-fields">
            <div class="theme-color-picker-label">${t("themeAccent")}</div>
            <div class="theme-preset-row" aria-label="${t("themeAccent")}">${themePresetOptions}</div>
            <input class="theme-name-input" data-titlebar-theme-draft-name value="${escapeHtml(newThemeDraft.name)}" placeholder="${t("themeNamePlaceholder")}" aria-label="${t("themeNewLabel")}">
            <div class="theme-color-inputs">
              <label class="theme-hex-input"><span>HEX</span><input data-titlebar-theme-draft-hex value="${escapeHtml(newThemeDraft.hexInput || newThemeDraft.accent.toUpperCase())}" spellcheck="false" aria-label="HEX"></label>
              <label class="theme-rgb-input"><span>R</span><input data-titlebar-theme-draft-rgb="r" value="${escapeHtml(newThemeDraft.rgbInput?.r || String(draftRgb.r))}" inputmode="numeric" aria-label="R"></label>
              <label class="theme-rgb-input"><span>G</span><input data-titlebar-theme-draft-rgb="g" value="${escapeHtml(newThemeDraft.rgbInput?.g || String(draftRgb.g))}" inputmode="numeric" aria-label="G"></label>
              <label class="theme-rgb-input"><span>B</span><input data-titlebar-theme-draft-rgb="b" value="${escapeHtml(newThemeDraft.rgbInput?.b || String(draftRgb.b))}" inputmode="numeric" aria-label="B"></label>
            </div>
          </div>
          <div class="titlebar-theme-draft-actions">
            <button class="tiny-button accent" type="button" data-titlebar-theme-create ${appearanceBusyAction === "theme-create" ? "disabled" : ""}>${appearanceBusyAction === "theme-create" ? t("creatingTheme") : t("themeCreate")}</button>
            <button class="tiny-button muted" type="button" data-titlebar-theme-draft-cancel>${t("themeRenameCancel")}</button>
          </div>
        </div>`
      : "";
    const releaseNote = escapeHtml(releaseNotesText());
    const releaseVersion = latestReleaseVersion();

    const activeStyle = selectedStyle();
    const activeIsOriginal = activeStyleId === "original" || !activeStyleId;
    const styleOptions = styleCatalog
      .map((style) => {
        const sel = style.id === activeStyleId || (style.id === "original" && activeIsOriginal);
        const busy = appearanceBusyAction === `style:${style.id}`;
        const name = getThemeStyleName(style);
        const summary = getThemeStyleSummary(style);
        return `
          <div class="titlebar-style-row${sel ? " selected" : ""}" role="radio" aria-checked="${sel}" tabindex="0" data-titlebar-style-option="${escapeHtml(style.id)}">
            <span class="titlebar-style-preview" aria-hidden="true">
              ${(style.preview || []).map((color) => `<span style="background:${escapeHtml(color)}"></span>`).join("")}
            </span>
            <span class="titlebar-style-copy">
              <span class="titlebar-style-name">${escapeHtml(name)}</span>
              <span class="titlebar-style-summary">${escapeHtml(summary)}</span>
            </span>
            <span class="titlebar-style-actions">${busy ? spinnerIcon() : ""}</span>
          </div>
        `;
      })
      .join("");

    const agentItems = dashboardAgents
      .map((agent) => {
        const isTarget = agentCardTarget?.id === agent.id;
        const isOnline = agent.status !== "offline";
        const displayName = agent.displayName || agent.name;
        let cardHtml = "";
        if (isTarget) {
          let activityHtml;
          if (agentCardLoading) {
            activityHtml = spinnerIcon();
          } else if (agentCardActivity.length > 0) {
            activityHtml = `<ul class="agent-card-activity-list">${agentCardActivity.map((e) => `<li><span class="activity-text">${escapeHtml(e.activity)}</span><span class="activity-time">${formatRelativeTime(e.createdAt)}</span></li>`).join("")}</ul>`;
          } else {
            activityHtml = `<p class="inline-note">${t("agentNoActivity")}</p>`;
          }
          let actionsHtml;
          if (isOnline) {
            actionsHtml = `
              <button class="agent-card-button danger" type="button" data-titlebar-agent-stop="${escapeHtml(agent.id)}" ${agentCardAction ? "disabled" : ""}>${agentCardAction === "stop" ? t("agentStopping") : t("agentStop")}</button>
              <button class="agent-card-button" type="button" data-titlebar-agent-restart="${escapeHtml(agent.id)}" ${agentCardAction ? "disabled" : ""}>${agentCardAction === "restart" ? t("agentStarting") : t("agentRestart")}</button>
            `;
          } else {
            actionsHtml = `
              <button class="agent-card-button accent" type="button" data-titlebar-agent-start="${escapeHtml(agent.id)}" ${agentCardAction ? "disabled" : ""}>${agentCardAction === "start" ? t("agentStarting") : t("agentStart")}</button>
            `;
          }
          cardHtml = `
            <div class="agent-card" role="dialog" aria-label="${escapeHtml(displayName)}">
              <div class="agent-card-header">
                <span class="agent-status-dot ${isOnline ? "online" : "offline"}"></span>
                <span class="agent-card-name">${escapeHtml(displayName)}</span>
                <span class="agent-card-status">${escapeHtml(agent.status)}</span>
              </div>
              <p class="agent-card-description">${escapeHtml(agent.description || t("agentNoDescription"))}</p>
              <div class="agent-card-activity">
                <p class="agent-card-activity-title">${t("agentActivity")}</p>
                ${activityHtml}
              </div>
              <div class="agent-card-actions">${actionsHtml}</div>
            </div>
          `;
        }
        return `
          <div class="titlebar-agent-row">
            <button class="agent-avatar-button" type="button" data-titlebar-agent-card="${escapeHtml(agent.id)}" title="${escapeHtml(displayName)}">
              <span class="agent-status-dot ${isOnline ? "online" : "offline"}"></span>
            </button>
            ${cardHtml}
          </div>
        `;
      })
      .join("");
    const hasAgents = dashboardAgents.length > 0;

    return `
      <div class="titlebar-drag-strip" data-titlebar-drag data-tauri-drag-region aria-hidden="true"></div>
      <button
        class="titlebar-button titlebar-back"
        type="button"
        data-titlebar-back
        title="${t("backToLauncher")}"
        aria-label="${t("backToLauncher")}"
      >${backIcon()}</button>
      <div class="titlebar-tools-inner">
        <button
          class="titlebar-button${running ? " live" : ""}"
          type="button"
          data-titlebar-service
          title="${running ? t("closeServer") : t("startService")}"
          aria-label="${running ? t("closeServer") : t("startService")}"
          ${!selectedSlug || serviceBusy ? "disabled" : ""}
        >${actionIcon(running ? "stop" : "start", serviceBusy)}</button>
        <button
          class="titlebar-button"
          type="button"
          data-titlebar-log
          title="${t("openServerLog")}"
          aria-label="${t("openServerLog")}"
          ${!selectedSlug ? "disabled" : ""}
        >${logIcon()}</button>
        <div class="titlebar-style-wrap">
          <button
            class="titlebar-button"
            type="button"
            data-titlebar-style-toggle
            title="${t("themeStyle")}"
            aria-label="${t("themeStyle")}"
            aria-expanded="${titlebarStyleMenuOpen}"
          >${styleIcon()}</button>
          ${titlebarStyleMenuOpen ? `<div class="titlebar-style-panel" aria-label="${t("themeStyle")}" data-titlebar-style-panel>
            <div class="titlebar-style-head">
              <span class="titlebar-style-eyebrow">${t("themeStyle")}</span>
              <span class="titlebar-style-head-actions">
                <button class="text-action-button" type="button" data-titlebar-style-import ${appearanceBusyAction === "import-style" ? "disabled" : ""}>${t("themeImportStyle")}</button>
                <button class="text-action-button" type="button" data-titlebar-style-export ${!activeStyle ? "disabled" : ""}>${t("themeExportStyle")}</button>
              </span>
            </div>
            <input class="sr-only" type="file" accept="application/json,.json" data-titlebar-style-file-input>
            <div class="titlebar-style-list" role="radiogroup" aria-label="${t("themeStyle")}">
              ${styleOptions}
            </div>
          </div>` : ""}
        </div>
        <div class="titlebar-theme-wrap" style="--theme-accent:${escapeHtml(themeAccent)}">
          <button class="titlebar-theme-button" type="button" data-titlebar-theme-toggle title="${escapeHtml(themeLabel)}" aria-label="${t("theme")}">
            <span class="titlebar-theme-swatch"></span>
          </button>
          ${titlebarThemeMenuOpen ? `<div class="titlebar-theme-menu" role="menu" aria-label="${t("theme")}" data-titlebar-theme-menu>
            ${themeOptions}
            <button class="titlebar-theme-option add" type="button" data-titlebar-theme-new title="${t("themeNewLabel")}" aria-label="${t("themeNewLabel")}">${plusIcon()}</button>
            ${themeDraft}
          </div>` : ""}
        </div>
        <button
          class="titlebar-button"
          type="button"
          data-titlebar-mode
          title="${t(mode.key)}"
          aria-label="${t("mode")}"
        >${optionIcon(mode.icon)}</button>
        <button
          class="titlebar-button language"
          type="button"
          data-titlebar-language
          title="${t(language.key)}"
          aria-label="${t("language")}"
        >${optionIcon(language.icon)}</button>
        <button
          class="titlebar-version${updateReady ? " has-update" : ""}${releaseState.error ? " error" : ""}"
          type="button"
          data-titlebar-update
          title="${escapeHtml(releaseStatusTitle())}"
          aria-label="${t("updatesTitle")}: ${escapeHtml(versionText)}"
          ${checking || installing ? "disabled" : ""}
        >${checking || installing ? actionIcon("refresh", true) : ""}<span>${escapeHtml(versionText)}</span></button>
        ${releaseNotesOpen && updateReady ? `
          <div class="titlebar-release-popover" role="dialog" aria-label="${t("releaseNotes")}">
            <div class="titlebar-release-head">
              <span class="titlebar-release-title">
                <strong>${t("releaseNotes")}</strong>
                <span>${escapeHtml(releaseVersion ? `v${releaseVersion}` : t("updateAvailable"))}</span>
              </span>
              <button class="settings-icon-button compact" type="button" data-titlebar-release-close aria-label="${t("close")}">${closeIcon()}</button>
            </div>
            <div class="titlebar-release-body">${releaseNote}</div>
            <div class="titlebar-release-actions">
              <button class="service-open-button" type="button" data-titlebar-release-install ${installing ? "disabled" : ""}>${installing ? t("installingUpdate") : t("installUpdate")}</button>
            </div>
          </div>
        ` : ""}
        ${hasAgents ? `
          <div class="titlebar-agent-wrap">
            <div class="titlebar-agent-list" data-titlebar-agent-list>
              ${agentItems}
            </div>
          </div>
        ` : ""}
      </div>
    `;
  };
  const toggleSelectedService = async () => {
    const service = serviceSnapshot;
    const selected = selectedServiceServer();
    const selectedSlug = selected?.slug || service?.selectedServerSlug || "";
    if (!selectedSlug) {
      serviceError = t("selectedServerPlaceholder");
      render();
      return;
    }
    const command = selectedServiceRunning() ? "stop_service" : "start_service";
    const busy = selectedServiceRunning() ? "service-stop" : "service-start";
    await loadServiceSnapshot(command, { selectedServerSlug: selectedSlug }, busy);
  };
  const serviceLogQuickRanges = [
    { key: "serverLogQuick30s", durationMs: 30 * 1000 },
    { key: "serverLogQuick1m", durationMs: 60 * 1000 },
    { key: "serverLogQuick5m", durationMs: 5 * 60 * 1000 },
    { key: "serverLogQuick30m", durationMs: 30 * 60 * 1000 },
    { key: "serverLogQuick1h", durationMs: 60 * 60 * 1000 },
  ];
  const toDatetimeLocalValue = (date) => {
    const pad = (value) => String(value).padStart(2, "0");
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
  };
  const datetimeDatePart = (value) => String(value || "").split("T")[0] || "";
  const datetimeTimePart = (value) => {
    const time = String(value || "").split("T")[1] || "";
    return time.length === 5 ? `${time}:00` : time;
  };
  const normalizeTimeInput = (value) => value.length === 5 ? `${value}:00` : value;
  const updateDatetimeLocalPart = (value, part, nextValue) => {
    const fallback = toDatetimeLocalValue(new Date());
    const date = datetimeDatePart(value) || datetimeDatePart(fallback);
    const time = datetimeTimePart(value) || "00:00:00";
    return part === "date"
      ? `${nextValue || date}T${time}`
      : `${date}T${normalizeTimeInput(nextValue || time)}`;
  };
  const serviceLogRangeForDuration = (durationMs) => {
    const end = new Date();
    const start = new Date(end.getTime() - durationMs);
    return { rangeStart: toDatetimeLocalValue(start), rangeEnd: toDatetimeLocalValue(end) };
  };
  const epochFromDatetimeLocal = (value) => {
    const time = new Date(value).getTime();
    return Number.isFinite(time) ? time : null;
  };
  const setServiceLogViewer = (next) => {
    serviceLogViewer = next;
    window.__slockDesktopServiceLogViewer = serviceLogViewer;
    render();
  };
  const openServiceLogViewer = async (serverSlug, rangeOverride = null) => {
    const server = serviceSnapshot?.servers?.find((item) => item.slug === serverSlug);
    const serverName = server?.name || serverSlug;
    const query = serviceLogViewer?.serverSlug === serverSlug ? serviceLogViewer.query : "";
    const preservedRange = serviceLogViewer?.serverSlug === serverSlug;
    const defaultRange = serviceLogRangeForDuration(30 * 60 * 1000);
    const rangeStart = rangeOverride?.rangeStart || (preservedRange ? serviceLogViewer.rangeStart : defaultRange.rangeStart);
    const rangeEnd = rangeOverride?.rangeEnd || (preservedRange ? serviceLogViewer.rangeEnd : defaultRange.rangeEnd);
    const rangePresetMs =
      rangeOverride?.rangePresetMs ?? (preservedRange ? serviceLogViewer.rangePresetMs : 30 * 60 * 1000);
    setServiceLogViewer({
      loading: true,
      snapshot: null,
      serverSlug,
      serverName,
      query,
      rangeStart,
      rangeEnd,
      rangePresetMs,
      activeMatchIndex: 0,
      search: emptyLogSearch(),
      error: null,
    });
    try {
      const snapshot = await invokeDesktop("open_service_log", {
        serverSlug,
        fromEpochMs: epochFromDatetimeLocal(rangeStart),
        toEpochMs: epochFromDatetimeLocal(rangeEnd),
      });
      if (!serviceLogViewer || serviceLogViewer.serverSlug !== serverSlug) return;
      setServiceLogViewer({
        ...serviceLogViewer,
        loading: false,
        snapshot,
        serverName,
        rangeStart,
        rangeEnd,
        rangePresetMs,
        activeMatchIndex: 0,
        search: emptyLogSearch(),
        error: null,
      });
      scheduleServiceLogSearch({ immediate: true });
    } catch (error) {
      if (!serviceLogViewer || serviceLogViewer.serverSlug !== serverSlug) return;
      setServiceLogViewer({
        ...serviceLogViewer,
        loading: false,
        error: error?.message || String(error),
      });
    }
  };
  const updateServiceLogQuery = (query) => {
    if (!serviceLogViewer) return;
    serviceLogViewer = { ...serviceLogViewer, query, activeMatchIndex: 0 };
    window.__slockDesktopServiceLogViewer = serviceLogViewer;
    scheduleServiceLogSearch();
  };
  const updateServiceLogRangePart = (field, part, value) => {
    if (!serviceLogViewer) return;
    serviceLogViewer = {
      ...serviceLogViewer,
      [field]: updateDatetimeLocalPart(serviceLogViewer[field], part, value),
      rangePresetMs: null,
    };
    window.__slockDesktopServiceLogViewer = serviceLogViewer;
  };
  const applyServiceLogRangePreset = (durationMs) => {
    if (!serviceLogViewer?.serverSlug) return;
    const range = { ...serviceLogRangeForDuration(durationMs), rangePresetMs: durationMs };
    void openServiceLogViewer(serviceLogViewer.serverSlug, range);
  };
  const stepServiceLogMatch = (direction) => {
    if (!serviceLogViewer?.snapshot) return;
    const search = currentLogSearch(serviceLogViewer);
    if (search.searching || search.count === 0) return;
    serviceLogViewer = {
      ...serviceLogViewer,
      activeMatchIndex:
        ((serviceLogViewer.activeMatchIndex || 0) + direction + search.count) % search.count,
    };
    window.__slockDesktopServiceLogViewer = serviceLogViewer;
    scheduleServiceLogSearch({ immediate: true });
  };
  const closeServiceLogViewer = () => {
    window.clearTimeout(serviceLogSearchTimer);
    serviceLogSearchToken += 1;
    clearServiceLogHighlight();
    setServiceLogViewer(null);
  };
  const openSelectedServiceLog = async () => {
    const service = serviceSnapshot;
    const selected = selectedServiceServer();
    const selectedSlug = selected?.slug || service?.selectedServerSlug || "";
    if (!selectedSlug) {
      serviceError = t("selectedServerPlaceholder");
      render();
      return;
    }
    await openServiceLogViewer(selectedSlug);
  };

  const serviceLogViewerContent = () => {
    const viewer = serviceLogViewer;
    if (!viewer) return "";
    const content = viewer.snapshot?.content || "";
    const search = currentLogSearch(viewer);
    const status = serviceLogStatus(viewer);
    const path = viewer.snapshot?.path || viewer.serverSlug;
    const rangeOptions = [
      `<option value="" ${viewer.rangePresetMs ? "" : "selected"}>${t("serverLogCustomRange")}</option>`,
      ...serviceLogQuickRanges.map((range) =>
        `<option value="${range.durationMs}" ${viewer.rangePresetMs === range.durationMs ? "selected" : ""}>${t(range.key)}</option>`,
      ),
    ]
      .join("");
    const body = viewer.loading
      ? `<div class="service-log-loading" role="status">${actionIcon("refresh", true)}<span>${t("serverLogLoading")}</span></div>`
      : content
        ? `<pre class="service-log-content" data-service-log-content tabindex="0">${escapeHtml(content)}</pre>`
        : `<p class="service-empty service-log-empty">${t("serverLogEmpty")}</p>`;

    return `
      <section class="service-log-backdrop" data-service-log-backdrop>
        <section class="service-log-dialog" role="dialog" aria-modal="true" aria-label="${t("serverLogTitle")}">
          <header class="service-log-head">
            <span class="service-log-title">
              <span class="eyebrow">${t("serverLogTitle")}</span>
              <strong>${escapeHtml(viewer.serverName || viewer.serverSlug)}</strong>
              <code class="service-log-path" title="${escapeHtml(path)}">${escapeHtml(path)}</code>
            </span>
            <button class="settings-icon-button compact" type="button" data-service-log-close title="${t("close")}" aria-label="${t("close")}">${closeIcon()}</button>
          </header>
          <div class="service-log-controls">
            <div class="service-log-toolbar">
              <label class="server-search service-log-search">
                ${searchIcon()}
                <span class="sr-only">${t("serverLogSearch")}</span>
                <input data-service-log-search value="${escapeHtml(viewer.query || "")}" placeholder="${t("serverLogSearch")}" aria-label="${t("serverLogSearch")}" ${viewer.loading || !viewer.snapshot ? "disabled" : ""}>
              </label>
              <span class="service-chip service-log-count" data-service-log-count>${escapeHtml(status)}</span>
              <div class="service-log-actions">
                <button class="settings-icon-button compact" type="button" data-service-log-step="-1" title="${t("serverLogPreviousMatch")}" aria-label="${t("serverLogPreviousMatch")}" ${search.searching || search.count === 0 ? "disabled" : ""}>${chevronIcon("up")}</button>
                <button class="settings-icon-button compact" type="button" data-service-log-step="1" title="${t("serverLogNextMatch")}" aria-label="${t("serverLogNextMatch")}" ${search.searching || search.count === 0 ? "disabled" : ""}>${chevronIcon("down")}</button>
              </div>
            </div>
            <div class="service-log-timebar">
              <fieldset class="service-log-time-field">
                <legend>${t("serverLogFrom")}</legend>
                <label class="service-log-time-input">${calendarIcon()}<input type="date" data-service-log-range="rangeStart" data-service-log-range-part="date" value="${escapeHtml(datetimeDatePart(viewer.rangeStart))}" aria-label="${t("serverLogFrom")} date" ${viewer.loading ? "disabled" : ""}></label>
                <label class="service-log-time-input">${clockIcon()}<input type="time" step="1" data-service-log-range="rangeStart" data-service-log-range-part="time" value="${escapeHtml(datetimeTimePart(viewer.rangeStart))}" aria-label="${t("serverLogFrom")} time" ${viewer.loading ? "disabled" : ""}></label>
              </fieldset>
              <fieldset class="service-log-time-field">
                <legend>${t("serverLogTo")}</legend>
                <label class="service-log-time-input">${calendarIcon()}<input type="date" data-service-log-range="rangeEnd" data-service-log-range-part="date" value="${escapeHtml(datetimeDatePart(viewer.rangeEnd))}" aria-label="${t("serverLogTo")} date" ${viewer.loading ? "disabled" : ""}></label>
                <label class="service-log-time-input">${clockIcon()}<input type="time" step="1" data-service-log-range="rangeEnd" data-service-log-range-part="time" value="${escapeHtml(datetimeTimePart(viewer.rangeEnd))}" aria-label="${t("serverLogTo")} time" ${viewer.loading ? "disabled" : ""}></label>
              </fieldset>
              <label class="service-log-range-select">${clockIcon()}<select data-service-log-preset aria-label="${t("serverLogRange")}" title="${t("serverLogRange")}" ${viewer.loading ? "disabled" : ""}>${rangeOptions}</select></label>
              <button class="settings-icon-button compact service-log-range-button" type="button" data-service-log-apply-range title="${t("serverLogRangeApply")}" aria-label="${t("serverLogRangeApply")}" ${viewer.loading ? "disabled" : ""}>${actionIcon("refresh", viewer.loading)}</button>
            </div>
          </div>
          ${viewer.error ? `<p class="service-empty service-log-error" role="alert">${escapeHtml(viewer.error)}</p>` : ""}
          <div class="service-log-body">${body}</div>
          ${viewer.snapshot?.truncated ? `<p class="service-empty service-log-truncated">${t("serverLogTruncated")}</p>` : ""}
        </section>
      </section>
    `;
  };

  const render = () => {
    syncHostTheme();
    shadow.innerHTML = "";

    const style = document.createElement("style");
    style.textContent = css;
    shadow.appendChild(style);

    const dock = document.createElement("div");
    dock.className = "dock";
    if (navigator.platform?.startsWith("Mac") || navigator.userAgent?.includes("Macintosh")) {
      dock.classList.add("platform-macos");
    }

    const toolbar = document.createElement("div");
    toolbar.className = "titlebar-tools";
    toolbar.innerHTML = titlebarToolsContent();
    toolbar.querySelector("[data-titlebar-drag]")?.addEventListener("mousedown", (event) => {
      if (event.button !== 0) return;
      event.preventDefault();
      void startWindowDrag();
    });
    toolbar.querySelector("[data-titlebar-back]")?.addEventListener("click", async () => {
      try {
        await invokeDesktop("exit_workspace", {});
      } catch (err) {
        console.error("[desktop] exit_workspace failed:", err);
      }
    });
    toolbar.querySelector("[data-titlebar-service]")?.addEventListener("click", () => {
      void toggleSelectedService();
    });
    toolbar.querySelector("[data-titlebar-log]")?.addEventListener("click", () => {
      void openSelectedServiceLog();
    });
    toolbar.querySelector("[data-titlebar-style-toggle]")?.addEventListener("click", () => {
      titlebarStyleMenuOpen = !titlebarStyleMenuOpen;
      titlebarThemeMenuOpen = false;
      releaseNotesOpen = false;
      newThemeDraft = null;
      titlebarThemeWheelOpen = false;
      render();
    });
    toolbar.querySelectorAll("[data-titlebar-style-option]").forEach((row) => {
      const handler = () => {
        const styleId = row.getAttribute("data-titlebar-style-option");
        titlebarStyleMenuOpen = false;
        if (styleId) setThemeStyle(styleId);
      };
      row.addEventListener("click", handler);
      row.addEventListener("keydown", (event) => {
        if (event.key === "Enter" || event.key === " ") { event.preventDefault(); handler(); }
      });
    });
    toolbar.querySelector("[data-titlebar-style-import]")?.addEventListener("click", () => {
      const fileInput = shadow.querySelector("[data-titlebar-style-file-input]");
      if (fileInput) fileInput.click();
    });
    shadow.querySelector("[data-titlebar-style-file-input]")?.addEventListener("change", async (event) => {
      const file = event.target.files?.[0];
      event.target.value = "";
      if (!file) return;
      try {
        const text = await file.text();
        const parsed = JSON.parse(text);
        const config = readThemeStyleConfig(parsed);
        await importThemeStyle(config);
      } catch {
        console.warn("[Slock Desktop] invalid style file");
      }
    });
    toolbar.querySelector("[data-titlebar-style-export]")?.addEventListener("click", () => {
      const style = selectedStyle();
      exportThemeStyleFile(style);
    });
    toolbar.querySelector("[data-titlebar-theme-toggle]")?.addEventListener("click", () => {
      const nextOpen = !titlebarThemeMenuOpen;
      titlebarThemeMenuOpen = nextOpen;
      if (!nextOpen) {
        newThemeDraft = null;
        titlebarThemeWheelOpen = false;
      }
      titlebarStyleMenuOpen = false;
      releaseNotesOpen = false;
      render();
    });
    toolbar.querySelectorAll("[data-titlebar-theme-option]").forEach((button) => {
      button.addEventListener("click", () => {
        const themeId = button.getAttribute("data-titlebar-theme-option");
        titlebarThemeMenuOpen = false;
        newThemeDraft = null;
        titlebarThemeWheelOpen = false;
        if (themeId) setTheme(themeId);
      });
    });
    toolbar.querySelector("[data-titlebar-theme-new]")?.addEventListener("click", () => {
      titlebarThemeMenuOpen = true;
      titlebarThemeWheelOpen = true;
      releaseNotesOpen = false;
      newThemeDraft = makeThemeDraft();
      render();
      queueMicrotask(() => shadow.querySelector("[data-titlebar-theme-draft-name]")?.focus());
    });
    toolbar.querySelectorAll("[data-titlebar-theme-delete]").forEach((button) => {
      button.addEventListener("click", (event) => {
        event.stopPropagation();
        const themeId = button.getAttribute("data-titlebar-theme-delete");
        if (themeId) deleteCustomTheme(themeId);
      });
    });
    toolbar.querySelector("[data-titlebar-theme-wheel-toggle]")?.addEventListener("click", () => {
      titlebarThemeWheelOpen = !titlebarThemeWheelOpen;
      render();
    });
    toolbar.querySelectorAll("[data-titlebar-theme-preset]").forEach((button) => {
      button.addEventListener("click", () => {
        const accent = button.getAttribute("data-titlebar-theme-preset");
        if (!accent) return;
        newThemeDraft = syncThemeDraftAccent(newThemeDraft || makeThemeDraft(), accent);
        titlebarThemeWheelOpen = true;
        render();
      });
    });
    toolbar.querySelector("[data-titlebar-theme-wheel]")?.addEventListener("pointerdown", (event) => {
      event.preventDefault();
      event.currentTarget.setPointerCapture?.(event.pointerId);
      newThemeDraft = syncThemeDraftAccent(
        newThemeDraft || makeThemeDraft(),
        accentFromWheelPointer(event, event.currentTarget),
      );
      render();
    });
    toolbar.querySelector("[data-titlebar-theme-wheel]")?.addEventListener("pointermove", (event) => {
      if (event.buttons !== 1) return;
      newThemeDraft = syncThemeDraftAccent(
        newThemeDraft || makeThemeDraft(),
        accentFromWheelPointer(event, event.currentTarget),
      );
      render();
    });
    toolbar.querySelector("[data-titlebar-theme-draft-hex]")?.addEventListener("input", (event) => {
      const value = String(event.target.value || "").toUpperCase();
      const normalized = normalizeHexColor(value);
      newThemeDraft = {
        ...(newThemeDraft || makeThemeDraft()),
        hexInput: value,
      };
      if (normalized) {
        newThemeDraft = syncThemeDraftAccent(newThemeDraft, normalized);
      }
      render();
      queueMicrotask(() => {
        const input = shadow.querySelector("[data-titlebar-theme-draft-hex]");
        if (input) {
          input.focus();
          input.setSelectionRange(input.value.length, input.value.length);
        }
      });
    });
    toolbar.querySelectorAll("[data-titlebar-theme-draft-rgb]").forEach((input) => {
      input.addEventListener("input", (event) => {
        const channel = input.getAttribute("data-titlebar-theme-draft-rgb");
        const value = sanitizeRgbInput(event.target.value);
        const draft = newThemeDraft || makeThemeDraft();
        const rgbInput = {
          ...draft.rgbInput,
          [channel]: value,
        };
        newThemeDraft = { ...draft, rgbInput };
        const rgb = parseRgbInput(rgbInput);
        if (rgb) {
          newThemeDraft = syncThemeDraftAccent(newThemeDraft, rgbToHex(rgb.r, rgb.g, rgb.b));
        }
        render();
        queueMicrotask(() => {
          const nextInput = shadow.querySelector(`[data-titlebar-theme-draft-rgb="${channel}"]`);
          if (nextInput) {
            nextInput.focus();
            nextInput.setSelectionRange(nextInput.value.length, nextInput.value.length);
          }
        });
      });
    });
    toolbar.querySelector("[data-titlebar-theme-draft-name]")?.addEventListener("input", (event) => {
      newThemeDraft = {
        ...(newThemeDraft || makeThemeDraft()),
        name: event.target.value,
      };
    });
    toolbar.querySelector("[data-titlebar-theme-draft-name]")?.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        if (newThemeDraft) createCustomTheme(newThemeDraft.name, newThemeDraft.accent);
      }
      if (event.key === "Escape") {
        event.preventDefault();
        newThemeDraft = null;
        titlebarThemeWheelOpen = false;
        render();
      }
    });
    toolbar.querySelector("[data-titlebar-theme-create]")?.addEventListener("click", () => {
      if (newThemeDraft) createCustomTheme(newThemeDraft.name, newThemeDraft.accent);
    });
    toolbar.querySelector("[data-titlebar-theme-draft-cancel]")?.addEventListener("click", () => {
      newThemeDraft = null;
      titlebarThemeWheelOpen = false;
      render();
    });
    toolbar.querySelector("[data-titlebar-mode]")?.addEventListener("click", () => {
      titlebarThemeMenuOpen = false;
      newThemeDraft = null;
      titlebarThemeWheelOpen = false;
      releaseNotesOpen = false;
      setMode(nextModeId());
    });
    toolbar.querySelector("[data-titlebar-language]")?.addEventListener("click", () => {
      titlebarThemeMenuOpen = false;
      newThemeDraft = null;
      titlebarThemeWheelOpen = false;
      releaseNotesOpen = false;
      setLanguage(nextLanguageId());
    });
    toolbar.querySelector("[data-titlebar-release-close]")?.addEventListener("click", () => {
      releaseNotesOpen = false;
      render();
    });
    toolbar.querySelector("[data-titlebar-release-install]")?.addEventListener("click", () => {
      releaseNotesOpen = false;
      installDesktopRelease();
    });
    toolbar.querySelector("[data-titlebar-update]")?.addEventListener("click", () => {
      if (releaseUpdateAvailable()) {
        titlebarThemeMenuOpen = false;
        newThemeDraft = null;
        titlebarThemeWheelOpen = false;
        releaseNotesOpen = !releaseNotesOpen;
        render();
      } else {
        titlebarThemeMenuOpen = false;
        newThemeDraft = null;
        titlebarThemeWheelOpen = false;
        releaseNotesOpen = false;
        checkDesktopRelease();
      }
    });

    toolbar.querySelectorAll("[data-titlebar-agent-card]").forEach((button) => {
      button.addEventListener("click", () => {
        const agentId = button.getAttribute("data-titlebar-agent-card");
        const agent = dashboardAgents.find((a) => a.id === agentId);
        if (agent) handleAgentCardOpen(agent);
      });
    });
    toolbar.querySelectorAll("[data-titlebar-agent-stop]").forEach((button) => {
      button.addEventListener("click", () => {
        const agentId = button.getAttribute("data-titlebar-agent-stop");
        const agent = dashboardAgents.find((a) => a.id === agentId);
        if (agent) handleAgentStop(agent);
      });
    });
    toolbar.querySelectorAll("[data-titlebar-agent-start]").forEach((button) => {
      button.addEventListener("click", () => {
        const agentId = button.getAttribute("data-titlebar-agent-start");
        const agent = dashboardAgents.find((a) => a.id === agentId);
        if (agent) handleAgentStart(agent);
      });
    });
    toolbar.querySelectorAll("[data-titlebar-agent-restart]").forEach((button) => {
      button.addEventListener("click", () => {
        const agentId = button.getAttribute("data-titlebar-agent-restart");
        const agent = dashboardAgents.find((a) => a.id === agentId);
        if (agent) handleAgentRestart(agent);
      });
    });

    const logViewerContainer = document.createElement("div");
    logViewerContainer.innerHTML = serviceLogViewerContent();
    const logViewerElement = logViewerContainer.firstElementChild;
    if (logViewerElement) {
      logViewerElement.addEventListener("mousedown", (event) => {
        if (event.target === logViewerElement) closeServiceLogViewer();
      });
      logViewerElement.querySelector("[data-service-log-close]")?.addEventListener("click", () => {
        closeServiceLogViewer();
      });
      logViewerElement.querySelector("[data-service-log-apply-range]")?.addEventListener("click", () => {
        if (serviceLogViewer?.serverSlug) void openServiceLogViewer(serviceLogViewer.serverSlug);
      });
      logViewerElement.querySelector("[data-service-log-preset]")?.addEventListener("change", (event) => {
        const durationMs = Number(event.target.value);
        if (durationMs > 0) {
          applyServiceLogRangePreset(durationMs);
        }
      });
      logViewerElement.querySelectorAll("[data-service-log-range]").forEach((input) => {
        input.addEventListener("input", (event) => {
          updateServiceLogRangePart(
            input.getAttribute("data-service-log-range"),
            input.getAttribute("data-service-log-range-part"),
            event.target.value,
          );
          const preset = logViewerElement.querySelector("[data-service-log-preset]");
          if (preset) preset.value = "";
        });
      });
      logViewerElement.querySelectorAll("[data-service-log-step]").forEach((button) => {
        button.addEventListener("click", () => {
          stepServiceLogMatch(Number(button.getAttribute("data-service-log-step")) || 1);
        });
      });
      logViewerElement.querySelector("[data-service-log-search]")?.addEventListener("input", (event) => {
        updateServiceLogQuery(event.target.value);
      });
      logViewerElement.querySelector("[data-service-log-search]")?.addEventListener("keydown", (event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          stepServiceLogMatch(event.shiftKey ? -1 : 1);
        }
      });
      queueMicrotask(() => {
        applyServiceLogHighlight();
        const search = shadow.querySelector("[data-service-log-search]");
        if (search && !search.disabled) {
          const query = String(serviceLogViewer?.query || "");
          search.focus();
          search.setSelectionRange?.(query.length, query.length);
        }
      });
    }

    dock.append(toolbar);
    if (logViewerElement) dock.appendChild(logViewerElement);
    shadow.appendChild(dock);
  };

  const setTheme = async (themeId) => {
    activeThemeId = themeId;
    render();

    try {
      const payload = await invokeDesktop("set_theme", { themeId });
      syncDesktopPayload(payload);
      render();
    } catch (error) {
      console.error("[Slock Desktop] theme update failed", error);
    }
  };

  const setMode = async (mode) => {
    activeMode = mode;
    render();

    try {
      const payload = await invokeDesktop("set_theme_mode", { themeMode: mode });
      syncDesktopPayload(payload);
      render();
    } catch (error) {
      console.error("[Slock Desktop] theme mode update failed", error);
    }
  };

  const setLanguage = async (language) => {
    activeLanguage = language;
    render();
    translateSlockMenus();

    try {
      const payload = await invokeDesktop("set_language", { language });
      syncDesktopPayload(payload);
      render();
    } catch (error) {
      console.error("[Slock Desktop] language update failed", error);
    }
  };

  const accountText = (value) => typeof value === "string" ? value.trim() : "";
  const accountField = (source, keys) => {
    if (!source || typeof source !== "object") return "";
    for (const key of keys) {
      const value = accountText(source[key]);
      if (value) return value;
    }
    return "";
  };
  const collectSessionAccount = () => {
    const account = {
      displayName: "",
      email: "",
      avatarUrl: "",
    };
    const readCandidate = (candidate) => {
      if (!candidate || typeof candidate !== "object") return;
      const sources = [candidate, candidate.data, candidate.result, candidate.user, candidate.profile, candidate.account, candidate.currentUser, candidate.me];
      for (const source of sources) {
        if (!source || typeof source !== "object") continue;
        account.displayName ||= accountField(source, ["displayName", "display_name", "fullName", "name", "username"]);
        account.email ||= accountField(source, ["email", "emailAddress", "email_address"]);
        account.avatarUrl ||= accountField(source, ["avatarUrl", "avatar_url", "picture", "image", "profileImage"]);
      }
    };

    for (let index = 0; index < localStorage.length; index += 1) {
      const key = localStorage.key(index) || "";
      const raw = localStorage.getItem(key) || "";
      if (!raw || raw.length > 50000) continue;
      if (!/(user|profile|account|auth|session|slock)/i.test(`${key} ${raw.slice(0, 200)}`)) continue;
      try {
        readCandidate(JSON.parse(raw));
      } catch (_) {}
    }

    return account;
  };

  const syncSessionTokens = async () => {
    try {
      const accessToken = localStorage.getItem("slock_access_token");
      const refreshToken = localStorage.getItem("slock_refresh_token");
      if (!accessToken || !refreshToken) return;

      const nextSignature = `${accessToken}::${refreshToken}`;
      if (window.__slockDesktopSessionSignature === nextSignature) return;

      const invoke = window.__TAURI__?.core?.invoke;
      if (typeof invoke !== "function") {
        throw new Error("Tauri invoke API is unavailable");
      }

      const wasUnauthenticated = serviceSnapshot?.authenticated === false;
      const account = collectSessionAccount();
      await invoke("save_session_tokens", {
        accessToken,
        refreshToken,
        displayName: account.displayName || null,
        email: account.email || null,
        avatarUrl: account.avatarUrl || null,
      });
      window.__slockDesktopSessionSignature = nextSignature;
      if (wasUnauthenticated) {
        const payload = await invoke("bootstrap", { refresh: false });
        syncDesktopPayload(payload);
        refreshServiceSnapshot();
      }
    } catch (error) {
      console.warn("[Slock Desktop] session sync failed", error);
    }
  };

  const scheduleSessionTokenSync = () => {
    window.clearTimeout(window.__slockDesktopSessionSyncDebounce);
    window.__slockDesktopSessionSyncDebounce = window.setTimeout(() => {
      syncSessionTokens();
    }, 100);
  };

  if (!window.__slockDesktopSessionSyncBound) {
    window.__slockDesktopSessionSyncBound = true;
    window.addEventListener("focus", scheduleSessionTokenSync);
    window.addEventListener("storage", scheduleSessionTokenSync);
    window.addEventListener("visibilitychange", scheduleSessionTokenSync);
    window.__slockDesktopSessionSyncTimer = window.setInterval(scheduleSessionTokenSync, 2000);
  }

  const getCurrentServerSlug = () => {
    const match = window.location.pathname.match(/^\/s\/([^/?#]+)/);
    if (!match?.[1]) return null;
    try {
      return decodeURIComponent(match[1]);
    } catch (_error) {
      return match[1];
    }
  };

  let routeServerSyncTimer = null;
  let routeServerSyncInFlight = false;
  const currentRouteServerIsKnown = (slug) =>
    !serviceSnapshot?.servers?.length || serviceSnapshot.servers.some((server) => server.slug === slug);
  const scheduleRouteServerSync = () => {
    window.clearTimeout(routeServerSyncTimer);
    routeServerSyncTimer = window.setTimeout(() => {
      void syncServiceServerFromRoute();
    }, 100);
  };
  const syncServiceServerFromRoute = async () => {
    const slug = getCurrentServerSlug();
    if (!slug || routeServerSyncInFlight || serviceSnapshot?.selectedServerSlug === slug) return;
    if (!currentRouteServerIsKnown(slug)) return;

    routeServerSyncInFlight = true;
    try {
      await loadServiceSnapshot("select_service_server", { selectedServerSlug: slug }, "service-status");
    } finally {
      routeServerSyncInFlight = false;
      if (getCurrentServerSlug() && serviceSnapshot?.selectedServerSlug !== getCurrentServerSlug()) {
        scheduleRouteServerSync();
      }
    }
  };
  const emitSlockRouteChanged = () => {
    window.dispatchEvent(new Event("slock-desktop-route-changed"));
  };

  const normalizeActionText = (value) =>
    value?.replace(/\s+/g, " ").trim().toLowerCase() || "";

  const isDaemonUpdateAction = (element) => {
    const label = normalizeActionText(
      [
        element.textContent,
        element.getAttribute("aria-label"),
        element.getAttribute("title"),
      ]
        .filter(Boolean)
        .join(" "),
    );
    if (!label.includes("update") && !label.includes("更新")) return false;

    const context = normalizeActionText(
      element.closest("[role='alert'], [class*='bg-brutal-'], [class*='border-2'], [class*='rounded'], [class*='shadow-']")?.textContent ||
        element.parentElement?.textContent ||
        "",
    );
    if (!context) return true;

    return /daemon|machine|computer|service|outdated|reconnect|offline|update|更新|服务|连接/.test(context);
  };

  const handleDaemonUpdateClick = async (event) => {
    const action =
      event.target instanceof Element
        ? event.target.closest("button, [role='button'], a")
        : null;
    if (!action || !isDaemonUpdateAction(action)) return;
    if (action.dataset.slockDesktopBusy === "true") return;
    if (!selectedServiceRunning()) return;

    try {
      const invoke = window.__TAURI__?.core?.invoke;
      if (typeof invoke !== "function") {
        throw new Error("Tauri invoke API is unavailable");
      }

      event.preventDefault();
      event.stopPropagation();
      action.dataset.slockDesktopBusy = "true";
      await invoke("update_service", {
        selectedServerSlug: getCurrentServerSlug(),
      });
    } catch (error) {
      console.warn("[Slock Desktop] daemon update bridge failed", error);
    } finally {
      delete action.dataset.slockDesktopBusy;
    }
  };

  window.__slockDesktopCloseTransientTitlebarPanels = closeTransientTitlebarPanels;
  syncSessionTokens();

  if (!window.__slockDesktopSettingsEscapeBound) {
    window.__slockDesktopSettingsEscapeBound = true;
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape") window.__slockDesktopCloseTransientTitlebarPanels?.();
    });
  }

  if (!window.__slockDesktopSettingsPointerBound) {
    window.__slockDesktopSettingsPointerBound = true;
    document.addEventListener("pointerdown", (event) => {
      const activeHost = document.getElementById(hostId);
      const path = event.composedPath ? event.composedPath() : [];
      const insideDesktopHost = activeHost && path.includes(activeHost);
      if (!insideDesktopHost) window.__slockDesktopCloseTransientTitlebarPanels?.();
    });
  }

  if (!window.__slockDesktopSessionSyncTimer) {
    window.__slockDesktopSessionSyncTimer = window.setInterval(syncSessionTokens, 4000);
  }

	  if (!window.__slockDesktopUpdateBridgeBound) {
	    window.__slockDesktopUpdateBridgeBound = true;
	    document.addEventListener("click", (event) => {
	      void handleDaemonUpdateClick(event);
	    }, true);
	  }

  if (!window.__slockDesktopRouteSyncBound) {
    window.__slockDesktopRouteSyncBound = true;
    const originalPushState = window.history.pushState;
    const originalReplaceState = window.history.replaceState;
    window.history.pushState = function pushState(...args) {
      const result = originalPushState.apply(this, args);
      emitSlockRouteChanged();
      return result;
    };
    window.history.replaceState = function replaceState(...args) {
      const result = originalReplaceState.apply(this, args);
      emitSlockRouteChanged();
      return result;
    };
    window.addEventListener("popstate", emitSlockRouteChanged);
    window.addEventListener("slock-desktop-route-changed", scheduleRouteServerSync);
  }
  scheduleRouteServerSync();

	  function closeTransientTitlebarPanels() {
    if (serviceLogViewer) {
      closeServiceLogViewer();
      return true;
    }
	    if (!titlebarThemeMenuOpen && !titlebarStyleMenuOpen && !releaseNotesOpen && !agentCardTarget) return false;
    if (titlebarThemeMenuOpen) {
      newThemeDraft = null;
      titlebarThemeWheelOpen = false;
    }
    titlebarThemeMenuOpen = false;
    titlebarStyleMenuOpen = false;
    releaseNotesOpen = false;
    agentCardTarget = null;
    render();
    return true;
  }

  syncWorkspaceChromeSafeArea();
  render();
  bindSlockMenuTranslator();

  const listenForDesktopUpdateChecks = async () => {
    const listen = window.__TAURI__?.event?.listen;
    if (typeof listen !== "function" || window.__slockDesktopUpdateCheckListenerReady) return;
    window.__slockDesktopUpdateCheckListenerReady = true;
    try {
      const unlisten = await listen("desktop_update_checked", (event) => {
        syncDesktopUpdateCheck(event?.payload);
      });
      window.__slockDesktopUpdateCheckUnlisten = unlisten;
    } catch (error) {
      window.__slockDesktopUpdateCheckListenerReady = false;
      console.warn("[Slock Desktop] update check listener failed", error);
    }
  };
  void listenForDesktopUpdateChecks();

  if (!window.__slockDesktopServicePrefetched) {
    window.__slockDesktopServicePrefetched = true;
    setTimeout(() => {
      if (serviceBusyAction) return;
      (async () => {
        if (!serviceSnapshot) {
          await loadServiceSnapshot("bootstrap", { refresh: false });
        }
        if (serviceBusyAction) return;
        refreshServiceSnapshot();
      })();
    }, 1500);
  }

  if (!window.__slockDesktopAgentsPrefetched) {
    window.__slockDesktopAgentsPrefetched = true;
    setTimeout(() => {
      fetchDashboardAgents();
    }, 2500);
  }

})();
"#;

#[cfg(test)]
mod tests {
    use super::settings_overlay_script;

    #[test]
    fn settings_overlay_translates_search_placeholder() {
        let script = settings_overlay_script("default", "system", "zh-CN", "zh-CN", &[]);

        assert!(script.contains("Search channels, DMs, messages..."));
        assert!(script.contains("Search channels, DMs, messages…"));
        assert!(script.contains("搜索频道、私信、消息..."));
        assert!(script.contains("Clear search"));
        assert!(script.contains("清除搜索"));
        assert!(script.contains("My messages"));
        assert!(script.contains("我的消息"));
        assert!(script.contains("Any time"));
        assert!(script.contains("任意时间"));
        assert!(script.contains("Search everything"));
        assert!(script.contains("搜索全部"));
        assert!(script.contains("Search channels, DMs, people, agents, and message history."));
        assert!(script.contains("搜索频道、私信、人员、Agent 和消息历史。"));
        assert!(script.contains("\"input[placeholder]\""));
        assert!(script.contains("const excludedForText = isExcludedTranslationTarget(element);"));
        assert!(script.contains("translateAttribute(element, \"placeholder\");"));
        assert!(script.contains("if (excludedForText) return;"));
    }

    #[test]
    fn settings_overlay_translates_search_empty_state_description_outside_main() {
        let script = settings_overlay_script("default", "system", "zh-CN", "zh-CN", &[]);

        assert!(script.contains("shouldTranslateSearchDescriptions"));
        assert!(script.contains("window.location.pathname.split(\"/\").includes(\"search\")"));
        assert!(script.contains("p, div, span, [class*='empty-state'], [class*='mt-1']"));
        assert!(script.contains("Search channels, DMs, people, agents, and message history."));
    }

    #[test]
    fn settings_overlay_stops_service_from_local_daemon_state() {
        let script = settings_overlay_script("default", "system", "zh-CN", "zh-CN", &[]);

        assert!(!script.contains("machineStatusCountsAsStarted"));
        assert!(!script.contains("runtimeRunning"));
        assert!(script.contains("const selectedRunning ="));
        assert!(script.contains("const serviceServerIsRunning = (service, serverSlug) =>"));
        assert!(script.contains("activeSlug === selectedSlug"));
        assert!(script.contains("const syncServiceServerFromRoute = async () =>"));
        assert!(script.contains(
            "await loadServiceSnapshot(\"select_service_server\", { selectedServerSlug: slug }, \"service-status\")"
        ));
        assert!(script.contains("window.history.pushState = function pushState(...args)"));
        assert!(!script.contains(
            "!service.activeServerSlug || service.activeServerSlug === selectedServerSlug"
        ));
        assert!(!script
            .contains("!service.activeServerSlug || service.activeServerSlug === selectedSlug"));
        assert!(script.contains("if (!selectedSlug)"));
        assert!(script.contains("data-titlebar-service"));
        assert!(!script.contains("data-service-action=\"toggle\""));
        assert!(!script.contains("data-service-action=\"start\""));
        assert!(!script.contains("data-service-action=\"stop\""));
        assert!(script.contains(
            "const command = selectedServiceRunning() ? \"stop_service\" : \"start_service\""
        ));
        assert!(script.contains(
            "const busy = selectedServiceRunning() ? \"service-stop\" : \"service-start\""
        ));
        assert!(
            script.contains("service.configured ? t(\"serviceIdle\") : t(\"serviceNotLinked\")")
        );
    }

    #[test]
    fn settings_overlay_exposes_titlebar_settings_controls() {
        let script = settings_overlay_script("default", "system", "zh-CN", "zh-CN", &[]);

        assert!(script.contains("data-titlebar-service"));
        assert!(script.contains("data-titlebar-log"));
        assert!(script.contains("data-titlebar-theme"));
        assert!(script.contains("data-titlebar-mode"));
        assert!(script.contains("data-titlebar-language"));
        assert!(script.contains("data-titlebar-update"));
        assert!(script.contains("titlebar-theme-button"));
        assert!(script.contains("titlebar-theme-menu"));
        assert!(script.contains("titlebar-theme-option"));
        assert!(script.contains("data-titlebar-theme-new"));
        assert!(script.contains("titlebarThemeSwatch"));
        assert!(script.contains("#ffd701"));
        assert!(script.contains("width: 100%;"));
        assert!(script.contains("optionIcon(mode.icon)"));
        assert!(script.contains("optionIcon(language.icon)"));
        assert!(script.contains("nextLanguageId()"));
        assert!(script.contains("M8 8h6"));
        assert!(script.contains("service-log-dialog"));
        assert!(script.contains("data-service-log-search"));
        assert!(script.contains("data-service-log-range"));
        assert!(script.contains("data-service-log-preset"));
        assert!(!script.contains("data-service-log-quick"));
        assert!(script.contains("serverLogCustomRange"));
        assert!(script.contains("serverLogQuick30s"));
        assert!(script.contains("openServiceLogViewer"));
        assert!(script.contains("scanLogMatchesInChunks"));
        assert!(script.contains("data-service-log-content"));
        assert!(script.contains("data-service-log-highlight"));
        assert!(!script.contains("getLogMatchSummary"));
        assert!(!script.contains("renderLogContentHtml"));
        assert!(!script.contains("matchIndex"));
        assert!(script.contains("titlebar-release-popover"));
        assert!(script.contains("data-titlebar-release-install"));
        assert!(script.contains("releaseNotesOpen = !releaseNotesOpen"));
        assert!(script.contains("titlebar-drag-strip"));
        assert!(script.contains("data-titlebar-drag"));
        assert!(script.contains("data-tauri-drag-region"));
        assert!(script.contains("start_window_drag"));
        assert!(script.contains("slock-desktop-titlebar-safe-area"));
        assert!(script.contains("data-slock-desktop-workspace-chrome"));
        assert!(
            script.contains("transform: translate3d(0, var(--slock-desktop-titlebar-height), 0)")
        );
        assert!(script.contains("data-titlebar-theme-wheel-toggle"));
        assert!(script.contains("data-titlebar-theme-wheel"));
        assert!(script.contains("accentFromWheelPointer"));
        assert!(script.contains("titlebar-accent-wheel-large"));
        assert!(script.contains("data-titlebar-theme-draft-hex"));
        assert!(script.contains("data-titlebar-theme-draft-rgb"));
        assert!(script.contains("data-titlebar-theme-create"));
        assert!(script.contains("data-titlebar-theme-delete"));
        assert!(script.contains("create_custom_theme"));
        assert!(script.contains("delete_custom_theme"));
        assert!(script.contains("newThemeDraft = makeThemeDraft();"));
        assert!(!script.contains("data-theme-new"));
        assert!(!script.contains("data-service-search"));
        assert!(!script.contains("data-update-action=\"check\""));
        assert!(!script.contains("__slockDesktopSettingsOpen"));
        assert!(!script.contains("className = \"panel\""));
        assert!(!script.contains("dock.append(panel, toolbar)"));
        assert!(script.contains("open_service_log"));
        assert!(script.contains("check_desktop_update"));
        assert!(script.contains("hydrateReleaseStateFromUpdateSnapshot"));
        assert!(script.contains("updateSnapshot?.latest"));
        assert!(script.contains("desktop_update_checked"));
        assert!(script.contains("syncDesktopUpdateCheck(event?.payload)"));
        assert!(!script.contains("__slockDesktopAutoUpdateChecked"));
        assert!(script.contains("install_desktop_update"));
        assert!(script.contains("if (!selectedServiceRunning()) return;"));
        assert!(!script.contains("class=\"launcher"));
    }
}
