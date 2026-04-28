use crate::theme;

pub fn settings_overlay_script(
    active_theme_id: &str,
    active_theme_mode: &str,
    active_language: &str,
    resolved_language: &str,
    themes: &[theme::ThemeMeta],
) -> String {
    let themes = serde_json::to_string(themes).unwrap_or_else(|_| "[]".into());
    let active_theme =
        serde_json::to_string(active_theme_id).unwrap_or_else(|_| "\"default\"".into());
    let active_mode =
        serde_json::to_string(active_theme_mode).unwrap_or_else(|_| "\"system\"".into());
    let active_language =
        serde_json::to_string(active_language).unwrap_or_else(|_| "\"system\"".into());
    let resolved_language =
        serde_json::to_string(resolved_language).unwrap_or_else(|_| "\"en-US\"".into());

    WORKSPACE_SETTINGS_SCRIPT
        .replace("__SLOCK_DESKTOP_THEMES__", &themes)
        .replace("__SLOCK_DESKTOP_ACTIVE_THEME__", &active_theme)
        .replace("__SLOCK_DESKTOP_ACTIVE_MODE__", &active_mode)
        .replace("__SLOCK_DESKTOP_ACTIVE_LANGUAGE__", &active_language)
        .replace("__SLOCK_DESKTOP_RESOLVED_LANGUAGE__", &resolved_language)
}

const WORKSPACE_SETTINGS_SCRIPT: &str = r#"
(() => {
  const hostId = "slock-desktop-settings-host";
  const themes = __SLOCK_DESKTOP_THEMES__;
  const initialThemeId = __SLOCK_DESKTOP_ACTIVE_THEME__;
  const initialMode = __SLOCK_DESKTOP_ACTIVE_MODE__;
  const initialLanguage = __SLOCK_DESKTOP_ACTIVE_LANGUAGE__;
  const initialResolvedLanguage = __SLOCK_DESKTOP_RESOLVED_LANGUAGE__;
  let activeSection = window.__slockDesktopSettingsSection || "appearance";
  let serviceSnapshot = window.__slockDesktopServiceSnapshot || null;
  let themeCatalog = window.__slockDesktopThemeCatalog || themes;
  let updateSnapshot = window.__slockDesktopUpdateSnapshot || null;
  let serviceQuery = window.__slockDesktopServiceQuery || "";
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
  let renamingThemeId = null;
  let renameDraft = "";
  let serviceBusyAction = null;
  let serviceError = null;
  let appearanceBusyAction = null;
  let updateBusyAction = null;
  let titlebarThemeMenuOpen = false;
  let releaseNotesOpen = false;
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
  const optionIcon = (type, className = "option-icon") => {
    if (type === "han") {
      return `<svg class="${className}" aria-hidden="true" viewBox="0 0 1024 1024" fill="currentColor"><path d="M555.231787 330.203429v-107.997284h-68.202727v108.038827H263.433935v273.457531H487.02906v210.976899h68.202727V603.70431h224.21827V330.203429H555.231787z m-68.202727 209.074952h-157.337694v-144.605675h157.335888v144.605675z m226.131053 0H555.195662v-144.605675h157.962645v144.605675z"></path></svg>`;
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
  const shellIcon = () =>
    `<svg class="option-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m4 8 4 4-4 4"></path><path d="M10 16h10"></path><rect width="20" height="16" x="2" y="4" rx="2"></rect></svg>`;
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
      themeNames: {
        original: "Original",
      },
      themeSummaries: {
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
      themeNames: {
        original: "原主题",
        default: "默认",
        light: "雾蓝",
        dark: "靛蓝",
        graphite: "石墨",
        crimson: "玫瑰",
        custom: "自定义",
      },
      themeSummaries: {
        original: "保持 Slock 原生外观，不注入桌面主题样式。",
        default: "适合日常桌面工作的克制绿色强调色。",
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
  let open = window.__slockDesktopSettingsOpen === true;
  let activeThemeId = initialThemeId;
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
      service.servers?.[0] ||
      null
    );
  };
  const serviceStatusText = () => {
    const service = serviceSnapshot;
    if (!service) return t("loadingService");
    const selected = selectedServiceServer();
    const selectedServerSlug = selected?.slug || service.selectedServerSlug || "";
    const selectedRunning =
      service.running &&
      selectedServerSlug &&
      (!service.activeServerSlug || service.activeServerSlug === selectedServerSlug);
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
    if (payload.updates) {
      updateSnapshot = payload.updates;
      window.__slockDesktopUpdateSnapshot = updateSnapshot;
      hydrateReleaseStateFromUpdateSnapshot();
    }
    if (payload.colorScheme) activeThemeId = payload.colorScheme;
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
      payload = await invokeDesktop("refresh_service_servers", {});
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
      syncAppearancePayload(payload);
    } catch (error) {
      console.warn("[Slock Desktop] custom theme create failed", error);
    } finally {
      appearanceBusyAction = null;
      render();
    }
  };
  const renameCustomTheme = async (id, name) => {
    const trimmed = String(name || "").trim();
    if (!trimmed) {
      renamingThemeId = null;
      renameDraft = "";
      render();
      return;
    }
    appearanceBusyAction = `theme-rename:${id}`;
    render();
    try {
      const payload = await invokeDesktop("rename_custom_theme", { id, name: trimmed });
      renamingThemeId = null;
      renameDraft = "";
      syncAppearancePayload(payload);
    } catch (error) {
      console.warn("[Slock Desktop] custom theme rename failed", error);
    } finally {
      appearanceBusyAction = null;
      render();
    }
  };
  const updateCustomThemeAccent = async (id, accent) => {
    appearanceBusyAction = `theme-accent:${id}`;
    render();
    try {
      const payload = await invokeDesktop("update_custom_theme_accent", { id, accent });
      syncAppearancePayload(payload);
    } catch (error) {
      console.warn("[Slock Desktop] custom theme accent update failed", error);
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
    return theme?.accent || "var(--desktop-accent)";
  };
  const selectedTheme = () =>
    themeCatalog.find((theme) => theme.id === activeThemeId) ||
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

    .titlebar-theme-wrap {
      position: relative;
      display: inline-grid;
      place-items: center;
    }

    .titlebar-theme-button {
      width: 30px;
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

    .titlebar-theme-trigger {
      display: inline-grid;
      place-items: center;
      width: 100%;
      height: 100%;
      pointer-events: none;
    }

    .titlebar-theme-icon {
      width: 14px;
      height: 14px;
      color: var(--theme-accent, var(--desktop-accent));
    }

    .titlebar-theme-swatch {
      position: absolute;
      right: 5px;
      bottom: 4px;
      width: 6px;
      height: 6px;
      border: 1px solid var(--desktop-surface);
      border-radius: var(--desktop-radius-pill);
      background: var(--theme-accent, var(--desktop-accent));
      box-shadow: 0 0 0 1px color-mix(in srgb, var(--theme-accent, var(--desktop-accent)) 34%, transparent);
    }

    .titlebar-theme-menu {
      position: absolute;
      top: 32px;
      right: 0;
      z-index: 4;
      display: grid;
      grid-template-columns: repeat(6, 24px);
      gap: 5px;
      padding: 6px;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-md);
      background: var(--desktop-surface);
      box-shadow: 0 10px 28px rgba(0, 0, 0, 0.12);
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

    .titlebar-theme-option.active {
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
      background: var(--desktop-surface-secondary);
      box-shadow: none;
      font-size: 16px;
      font-weight: 700;
      line-height: 1;
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
    }

    .theme-draft {
      grid-template-columns: 30px minmax(0, 1fr) auto auto;
      border-style: dashed;
      background: var(--desktop-surface-secondary);
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
	      gap: 6px;
	      flex: 0 0 auto;
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
      .titlebar-theme-option:hover,
      .titlebar-version:hover {
        color: var(--desktop-text);
        background: color-mix(in srgb, var(--desktop-surface) 76%, transparent);
        border-color: color-mix(in srgb, var(--desktop-text) 18%, var(--desktop-line));
        transform: translateY(-1px);
      }

	      .service-open-button:hover {
	        background: var(--desktop-accent-hover);
	      }
	    }

	    .language-option:active,
	    .mode-option:active,
	    .theme-option:active,
	    .nav-item:active,
		    .settings-icon-button:active,
        .theme-row:active,
        .theme-select:active,
        .service-row-wrap:active,
		    .service-row:active,
        .service-open-button:active,
        .titlebar-button:active,
        .titlebar-theme-button:active,
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
        .titlebar-theme-option,
	      .panel {
        transition-duration: 1ms;
      }
	    }
	  `;

  const navItem = (section, label) =>
    `<button class="nav-item${activeSection === section ? " active" : ""}" type="button" data-section="${section}">${label}</button>`;

  const isCustomTheme = (theme) => theme.id !== "original";
  const appearanceContent = () => {
    const themeRows = themeCatalog
      .map((theme) => {
        const selected = theme.id === activeThemeId || (theme.id === "original" && !activeThemeId);
        const display = themeDisplay(theme);
        const custom = isCustomTheme(theme);
        const renaming = renamingThemeId === theme.id;
        const deleting = appearanceBusyAction === `theme-delete:${theme.id}`;
        const swatchStyle = [
          `--theme-canvas:${escapeHtml(theme.canvas)}`,
          `--theme-surface:${escapeHtml(theme.surface)}`,
          `--theme-strong:${escapeHtml(theme.surfaceStrong)}`,
          `--theme-line:${escapeHtml(theme.line)}`,
          `--theme-accent:${escapeHtml(theme.accent)}`,
        ].join(";");
        return `
          <div class="theme-row${selected ? " active" : ""}${custom ? "" : " locked"}" data-theme-id="${escapeHtml(theme.id)}">
            <button
              class="theme-select"
              type="button"
              role="radio"
              aria-checked="${selected}"
              data-theme-select="${escapeHtml(theme.id)}"
              ${appearanceBusyAction ? "disabled" : ""}
            >
              <span class="swatch" style="${swatchStyle}" aria-hidden="true"><span></span><span></span><span></span></span>
              <span class="theme-copy">
                ${
                  renaming
                    ? `<input class="theme-name-input" data-theme-rename-input="${escapeHtml(theme.id)}" value="${escapeHtml(renameDraft)}" aria-label="${t("themeRename")}">`
                    : `<span class="theme-name">${escapeHtml(display.name)}</span><span class="theme-summary">${escapeHtml(custom ? theme.accent : display.summary)}</span>`
                }
              </span>
              <span class="check" aria-hidden="true">${selected ? "✓" : ""}</span>
            </button>
            ${
              custom
                ? `<div class="theme-actions">
                    <label class="accent-dot" style="--custom-accent:${escapeHtml(theme.accent)}" title="${t("themeAccent")}">
                      <span class="sr-only">${t("themeAccent")}</span>
                      <input type="color" value="${escapeHtml(theme.accent)}" data-theme-accent="${escapeHtml(theme.id)}" ${appearanceBusyAction ? "disabled" : ""}>
                    </label>
                    ${
                      renaming
                        ? `<button class="tiny-button" type="button" data-theme-rename-save="${escapeHtml(theme.id)}">${t("themeRenameSave")}</button>
                           <button class="tiny-button muted" type="button" data-theme-rename-cancel>${t("themeRenameCancel")}</button>`
                        : `<button class="settings-icon-button compact" type="button" data-theme-rename="${escapeHtml(theme.id)}" title="${t("themeRename")}" aria-label="${t("themeRename")}" ${appearanceBusyAction ? "disabled" : ""}>✎</button>
                           <button class="settings-icon-button danger compact" type="button" data-theme-delete="${escapeHtml(theme.id)}" title="${t("themeDelete")}" aria-label="${t("themeDelete")}" ${deleting ? "disabled" : ""}>${deleting ? actionIcon("refresh", true) : "×"}</button>`
                    }
                  </div>`
                : ""
            }
          </div>
        `;
      })
      .join("");
    const draft = newThemeDraft
      ? `<div class="theme-draft">
          <label class="accent-dot large" style="--custom-accent:${escapeHtml(newThemeDraft.accent)}" title="${t("themeAccent")}">
            <span class="sr-only">${t("themeAccent")}</span>
            <input type="color" value="${escapeHtml(newThemeDraft.accent)}" data-theme-draft-accent>
          </label>
          <input class="theme-name-input" data-theme-draft-name value="${escapeHtml(newThemeDraft.name)}" placeholder="${t("themeNamePlaceholder")}" aria-label="${t("themeNewLabel")}">
          <button class="tiny-button accent" type="button" data-theme-create ${appearanceBusyAction === "theme-create" ? "disabled" : ""}>${appearanceBusyAction === "theme-create" ? t("creatingTheme") : t("themeCreate")}</button>
          <button class="tiny-button muted" type="button" data-theme-draft-cancel>${t("themeRenameCancel")}</button>
        </div>`
      : "";

    return `
      <div class="content-head">
        <p class="setting-title">${t("theme")}</p>
        <button class="settings-icon-button positive" type="button" data-theme-new title="${t("themeNewLabel")}" aria-label="${t("themeNewLabel")}" ${newThemeDraft || appearanceBusyAction ? "disabled" : ""}>+</button>
      </div>
      <div class="theme-list" role="radiogroup" aria-label="${t("theme")}">
        ${themeRows || `<p class="service-empty">${t("themeEmptyHint")}</p>`}
        ${draft}
      </div>
      <div class="status">
        <span>${t("saved")}</span>
        <span>${themeCatalog.length} ${t("themes")}</span>
      </div>
    `;
  };

  const serviceContent = () => {
    const service = serviceSnapshot;
    const selected = selectedServiceServer();
    const busy = serviceBusyAction;
    const busyRefresh = busy === "service-refresh" || busy === "service-status" || busy === "service-load";
    const busyStart = busy === "service-start";
    const busyClose = busy === "service-stop";
    const busyOpen = busy === "service-open";
    const selectedSlug = selected?.slug || service?.selectedServerSlug || "";
    const serviceNote =
      serviceError ||
      service?.syncError ||
      service?.lastError ||
      (!service
        ? t("loadingService")
        : !service.authenticated
          ? t("serviceSignInHint")
          : service.servers.length === 0
            ? t("noServers")
            : t("serverSettingsDescription"));
    const servers = service?.servers || [];
    const normalizedQuery = serviceQuery.trim().toLowerCase();
    const visibleServers = normalizedQuery
      ? servers.filter((server) =>
          `${server.name} ${server.slug} ${server.machineName || ""}`.toLowerCase().includes(normalizedQuery),
        )
      : servers;
    const serverRows = servers
      .filter((server) => visibleServers.includes(server))
      .map((server) => {
        const selectedRow = server.slug === selectedSlug;
        const running = service?.running && server.slug === service.activeServerSlug;
        const status = running
          ? t("serviceRunning")
          : selectedRow
            ? service?.configured
              ? t("serviceIdle")
              : t("serviceNotLinked")
            : machineStatusLabel(server.machineStatus);
        const busySelect = busy === `service-select:${server.slug}`;
        const machineMeta = server.machineName
          ? `${t("machineStatus")}: ${escapeHtml(server.machineName)}`
          : `${t("machineStatus")}: ${escapeHtml(status)}`;
        return `
          <div class="service-row-wrap${selectedRow ? " active" : ""}${running ? " running" : ""}">
            <button
              class="service-row"
              type="button"
              data-service-action="select"
              data-server-slug="${escapeHtml(server.slug)}"
              aria-pressed="${selectedRow}"
              ${busy ? "disabled" : ""}
            >
              <span class="service-row-copy">
                <span class="service-row-name">${escapeHtml(server.name)}</span>
                <span class="service-row-meta">${escapeHtml(server.slug)} · ${machineMeta}</span>
              </span>
              <span class="service-chip${running ? " live" : ""}">${busySelect ? t("saving") : escapeHtml(status)}</span>
            </button>
            <button class="settings-icon-button compact service-log-button" type="button" data-service-action="log" data-server-slug="${escapeHtml(server.slug)}" title="${t("openServerLog")}" aria-label="${t("openServerLog")}: ${escapeHtml(server.name)}">${shellIcon()}</button>
          </div>
        `;
      })
      .join("");
    const emptyRows = servers.length > 0 && visibleServers.length === 0
      ? `<p class="service-empty">${t("noMatchingServers")}</p>`
      : `<p class="service-empty">${escapeHtml(serviceNote)}</p>`;

    return `
      <div class="service-panel">
        <div class="service-panel-head">
          <div>
            <p class="setting-title">${t("serverSettings")}</p>
            <p class="service-description">${escapeHtml(serviceNote)}</p>
          </div>
          <span class="service-count">${normalizedQuery ? `${visibleServers.length}/${servers.length}` : servers.length}</span>
          <div class="service-actions">
            <button class="settings-icon-button positive" type="button" data-service-action="start" title="${t("startService")}" aria-label="${t("startService")}" ${!selectedSlug || busy ? "disabled" : ""}>
              ${actionIcon("start", busyStart)}
            </button>
            <button class="settings-icon-button danger" type="button" data-service-action="stop" title="${t("closeServer")}" aria-label="${t("closeServer")}" ${busy ? "disabled" : ""}>
              ${actionIcon("stop", busyClose)}
            </button>
            <button class="settings-icon-button" type="button" data-service-action="refresh" title="${t("refreshServers")}" aria-label="${t("refreshServers")}" ${busy ? "disabled" : ""}>
              ${actionIcon("refresh", busyRefresh)}
            </button>
          </div>
        </div>
        <div class="service-facts">
          <span>${t("serverUrl")}</span>
          <strong>${escapeHtml(service?.serverUrl || "https://api.slock.ai")}</strong>
          <span>${t("serviceStatus")}</span>
          <strong>${escapeHtml(serviceStatusText())}</strong>
          <span>${t("selectedServer")}</span>
          <strong>${escapeHtml(selected?.name || selectedSlug || t("selectedServerPlaceholder"))}</strong>
        </div>
        ${
          servers.length > 0
            ? `<label class="server-search">
                <span aria-hidden="true">⌕</span>
                <span class="sr-only">${t("serverSearch")}</span>
                <input data-service-search value="${escapeHtml(serviceQuery)}" placeholder="${t("serverSearch")}" aria-label="${t("serverSearch")}">
              </label>`
            : ""
        }
        <div class="service-list" role="list" aria-label="${t("selectedServer")}">
          ${serverRows || emptyRows}
        </div>
        <button class="service-open-button" type="button" data-service-action="open" ${!selectedSlug || busy ? "disabled" : ""}>
          ${busyOpen ? t("openingServer") : t("openSelectedServer")}
        </button>
      </div>
    `;
  };
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
    return !!(
      service?.running &&
      selectedSlug &&
      (!service.activeServerSlug || service.activeServerSlug === selectedSlug)
    );
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
        const selected = theme.id === (activeThemeId || "original");
        const swatch = titlebarThemeSwatch(theme);
        return `
          <button
            class="titlebar-theme-option${selected ? " active" : ""}"
            type="button"
            data-titlebar-theme-option="${escapeHtml(theme.id)}"
            title="${escapeHtml(titlebarThemeLabel(theme))}"
            aria-label="${escapeHtml(titlebarThemeLabel(theme))}"
            style="--theme-accent:${escapeHtml(swatch)}"
          ><span class="titlebar-theme-option-swatch" aria-hidden="true"></span></button>
        `;
      })
      .join("");
    const releaseNote = escapeHtml(releaseNotesText());
    const releaseVersion = latestReleaseVersion();

    return `
      <div class="titlebar-drag-strip" data-titlebar-drag data-tauri-drag-region aria-hidden="true"></div>
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
        >${shellIcon()}</button>
        <div class="titlebar-theme-wrap" style="--theme-accent:${escapeHtml(themeAccent)}">
          <button class="titlebar-theme-button" type="button" data-titlebar-theme-toggle title="${escapeHtml(themeLabel)}" aria-label="${t("theme")}">
          <span class="titlebar-theme-trigger" aria-hidden="true">
            ${paletteIcon()}
            <span class="titlebar-theme-swatch"></span>
          </span>
          </button>
          ${titlebarThemeMenuOpen ? `<div class="titlebar-theme-menu" role="menu" aria-label="${t("theme")}" data-titlebar-theme-menu>
            ${themeOptions}
            <button class="titlebar-theme-option add" type="button" data-titlebar-theme-new title="${t("themeNewLabel")}" aria-label="${t("themeNewLabel")}">+</button>
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
              <button class="settings-icon-button compact" type="button" data-titlebar-release-close aria-label="${t("close")}">×</button>
            </div>
            <div class="titlebar-release-body">${releaseNote}</div>
            <div class="titlebar-release-actions">
              <button class="service-open-button" type="button" data-titlebar-release-install ${installing ? "disabled" : ""}>${installing ? t("installingUpdate") : t("installUpdate")}</button>
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
      activeSection = "service";
      open = true;
      window.__slockDesktopSettingsOpen = open;
      render();
      return;
    }
    const command = selectedServiceRunning() ? "stop_service" : "start_service";
    const busy = selectedServiceRunning() ? "service-stop" : "service-start";
    await loadServiceSnapshot(command, { selectedServerSlug: selectedSlug }, busy);
  };
  const openSelectedServiceLog = async () => {
    const service = serviceSnapshot;
    const selected = selectedServiceServer();
    const selectedSlug = selected?.slug || service?.selectedServerSlug || "";
    if (!selectedSlug) {
      serviceError = t("selectedServerPlaceholder");
      activeSection = "service";
      open = true;
      window.__slockDesktopSettingsOpen = open;
      render();
      return;
    }
    try {
      await invokeDesktop("open_service_log", { serverSlug: selectedSlug });
    } catch (error) {
      serviceError = error?.message || String(error);
      render();
    }
  };
  const updatesContent = () => {
    const currentVersion = updateSnapshot?.currentVersion || "";
    const latest = releaseState.latest;
    const updateAvailable = !!(latest?.available ?? latest?.updateAvailable);
    const latestVersion = latest?.version || latest?.tagName || "";
    const status = releaseState.error
      ? releaseState.error
      : latest
        ? updateAvailable
          ? t("updateAvailable")
          : t("upToDate")
        : t("notChecked");
    const checking = releaseState.loading || updateBusyAction === "release-check";
    const installing = releaseState.installing || updateBusyAction === "release-install";
    return `
      <div class="updates-panel">
        <div class="content-head">
          <p class="setting-title">${t("updatesTitle")}</p>
          <span class="service-chip${updateAvailable ? " live" : ""}">${escapeHtml(status)}</span>
        </div>
        <div class="service-facts">
          <span>${t("currentVersion")}</span>
          <strong>${escapeHtml(currentVersion || t("notChecked"))}</strong>
          ${
            latest
              ? `<span>${t("updateAvailable")}</span><strong>${escapeHtml(latestVersion || t("notChecked"))}</strong>`
              : `<span>${t("updateAvailable")}</span><strong>${t("notChecked")}</strong>`
          }
        </div>
        <div class="updates-actions">
          <button class="service-open-button secondary" type="button" data-update-action="check" ${checking || installing ? "disabled" : ""}>
            ${checking ? t("checkingUpdates") : t("checkUpdates")}
          </button>
          ${
            updateAvailable
              ? `<button class="service-open-button" type="button" data-update-action="install" ${installing || checking ? "disabled" : ""}>${installing ? t("installingUpdate") : t("installUpdate")}</button>`
              : ""
          }
        </div>
        <p class="service-description">${escapeHtml(latest?.body || latest?.name || releaseState.error || t("noReleaseNotes"))}</p>
      </div>
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
    dock.dataset.open = String(open);

    const panel = document.createElement("section");
    panel.className = "panel";
    panel.hidden = !open;
    panel.setAttribute("role", "dialog");
    panel.setAttribute("aria-label", t("title"));

    const inner = document.createElement("div");
    inner.className = "panel-inner";
    inner.innerHTML = `
      <div class="panel-head">
        <p class="eyebrow">${t("eyebrow")}</p>
        <h2>${t("title")}</h2>
        <p class="description">${t("description")}</p>
      </div>
      ${
        activeSection === "appearance"
          ? `<div class="quick-controls">
              <div class="mode-list" role="radiogroup" aria-label="${t("mode")}"></div>
              <div class="language-list" role="radiogroup" aria-label="${t("language")}"></div>
            </div>`
          : ""
      }
      <div class="settings-grid">
        <nav class="nav" aria-label="${t("settingsSections")}">
          ${navItem("appearance", t("appearance"))}
          ${navItem("service", t("service"))}
          ${navItem("updates", t("updates"))}
        </nav>
        <div class="content">${
          activeSection === "service"
            ? serviceContent()
            : activeSection === "updates"
              ? updatesContent()
              : appearanceContent()
        }</div>
      </div>
    `;

    const modeList = inner.querySelector(".mode-list");
    const languageList = inner.querySelector(".language-list");

    inner.querySelectorAll("[data-section]").forEach((item) => {
      item.addEventListener("click", () => {
        activeSection = item.dataset.section || "appearance";
        window.__slockDesktopSettingsSection = activeSection;
        render();
      });
    });

    modes.forEach((mode) => {
      if (!modeList) return;
      const selected = mode.id === activeMode;
      const option = document.createElement("button");
      option.className = `mode-option${selected ? " active" : ""}`;
      option.type = "button";
      option.setAttribute("role", "radio");
      option.setAttribute("aria-checked", String(selected));
      option.title = t(mode.key);
      option.innerHTML = `${optionIcon(mode.icon)}<span class="sr-only">${t(mode.key)}</span>`;
      option.addEventListener("click", () => setMode(mode.id));
      modeList.appendChild(option);
    });

    languages.forEach((language) => {
      if (!languageList) return;
      const selected = language.id === activeLanguage;
      const option = document.createElement("button");
      option.className = `language-option${selected ? " active" : ""}`;
      option.type = "button";
      option.setAttribute("role", "radio");
      option.setAttribute("aria-checked", String(selected));
      option.title = t(language.key);
      option.dataset.languageId = language.id;
      option.innerHTML = `${optionIcon(language.icon)}<span class="sr-only">${t(language.key)}</span>`;
      option.addEventListener("click", () => setLanguage(language.id));
      languageList.appendChild(option);
    });

    inner.querySelectorAll("[data-theme-select]").forEach((action) => {
      action.addEventListener("click", (event) => {
        if (event.target instanceof Element && event.target.closest("input")) return;
        const themeId = action.getAttribute("data-theme-select");
        if (themeId && renamingThemeId !== themeId) setTheme(themeId);
      });
    });
    inner.querySelector("[data-theme-new]")?.addEventListener("click", () => {
      newThemeDraft = { name: "", accent: '#10a37f' };
      render();
      queueMicrotask(() => shadow.querySelector("[data-theme-draft-name]")?.focus());
    });
    inner.querySelector("[data-theme-draft-accent]")?.addEventListener("change", (event) => {
      newThemeDraft = {
        ...(newThemeDraft || { name: "", accent: '#10a37f' }),
        accent: event.target.value,
      };
      render();
    });
    inner.querySelector("[data-theme-draft-name]")?.addEventListener("input", (event) => {
      newThemeDraft = {
        ...(newThemeDraft || { name: "", accent: '#10a37f' }),
        name: event.target.value,
      };
    });
    inner.querySelector("[data-theme-draft-name]")?.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        if (newThemeDraft) createCustomTheme(newThemeDraft.name, newThemeDraft.accent);
      }
      if (event.key === "Escape") {
        event.preventDefault();
        newThemeDraft = null;
        render();
      }
    });
    inner.querySelector("[data-theme-create]")?.addEventListener("click", () => {
      if (newThemeDraft) createCustomTheme(newThemeDraft.name, newThemeDraft.accent);
    });
    inner.querySelector("[data-theme-draft-cancel]")?.addEventListener("click", () => {
      newThemeDraft = null;
      render();
    });
    inner.querySelectorAll("[data-theme-accent]").forEach((input) => {
      input.addEventListener("change", (event) => {
        const themeId = input.getAttribute("data-theme-accent");
        if (themeId) updateCustomThemeAccent(themeId, event.target.value);
      });
    });
    inner.querySelectorAll("[data-theme-rename]").forEach((button) => {
      button.addEventListener("click", () => {
        const themeId = button.getAttribute("data-theme-rename");
        const theme = themeCatalog.find((item) => item.id === themeId);
        if (!theme) return;
        renamingThemeId = theme.id;
        renameDraft = theme.name || "";
        render();
        queueMicrotask(() => shadow.querySelector("[data-theme-rename-input]")?.focus());
      });
    });
    inner.querySelectorAll("[data-theme-rename-input]").forEach((input) => {
      input.addEventListener("input", (event) => {
        renameDraft = event.target.value;
      });
      input.addEventListener("keydown", (event) => {
        const themeId = input.getAttribute("data-theme-rename-input");
        if (event.key === "Enter" && themeId) {
          event.preventDefault();
          renameCustomTheme(themeId, renameDraft);
        }
        if (event.key === "Escape") {
          event.preventDefault();
          renamingThemeId = null;
          renameDraft = "";
          render();
        }
      });
    });
    inner.querySelectorAll("[data-theme-rename-save]").forEach((button) => {
      button.addEventListener("click", () => {
        const themeId = button.getAttribute("data-theme-rename-save");
        if (themeId) renameCustomTheme(themeId, renameDraft);
      });
    });
    inner.querySelector("[data-theme-rename-cancel]")?.addEventListener("click", () => {
      renamingThemeId = null;
      renameDraft = "";
      render();
    });
    inner.querySelectorAll("[data-theme-delete]").forEach((button) => {
      button.addEventListener("click", () => {
        const themeId = button.getAttribute("data-theme-delete");
        if (themeId) deleteCustomTheme(themeId);
      });
    });

    inner.querySelectorAll("[data-service-action]").forEach((action) => {
      action.addEventListener("click", () => {
        const serviceAction = action.dataset.serviceAction;
        const serverSlug = action.dataset.serverSlug;
        if (serviceAction === "refresh") {
          refreshServiceSnapshot();
        } else if (serviceAction === "start") {
          const selected = selectedServiceServer();
          const selectedServerSlug = selected?.slug || serviceSnapshot?.selectedServerSlug || "";
          if (!selectedServerSlug) {
            serviceError = t("selectedServerPlaceholder");
            render();
            return;
          }
          loadServiceSnapshot("start_service", { selectedServerSlug }, "service-start");
        } else if (serviceAction === "stop") {
          const selected = selectedServiceServer();
          const selectedServerSlug = selected?.slug || serviceSnapshot?.selectedServerSlug || "";
          if (!selectedServerSlug) {
            serviceError = t("serviceNotRunning");
            render();
            return;
          }
          loadServiceSnapshot("stop_service", { selectedServerSlug }, "service-stop");
        } else if (serviceAction === "select" && serverSlug) {
          loadServiceSnapshot(
            "select_service_server",
            { selectedServerSlug: serverSlug },
            `service-select:${serverSlug}`,
          );
        } else if (serviceAction === "open") {
          const selected = selectedServiceServer();
          const selectedServerSlug = selected?.slug || serviceSnapshot?.selectedServerSlug;
          if (selectedServerSlug) {
            loadServiceSnapshot("open_workspace", { selectedServerSlug }, "service-open");
          }
        } else if (serviceAction === "log" && serverSlug) {
          invokeDesktop("open_service_log", { serverSlug }).catch((error) => {
            serviceError = error?.message || String(error);
            render();
          });
        }
      });
    });
    inner.querySelector("[data-service-search]")?.addEventListener("input", (event) => {
      serviceQuery = event.target.value;
      window.__slockDesktopServiceQuery = serviceQuery;
      render();
      queueMicrotask(() => {
        const input = shadow.querySelector("[data-service-search]");
        if (input) {
          input.focus();
          input.setSelectionRange?.(serviceQuery.length, serviceQuery.length);
        }
      });
    });
    inner.querySelectorAll("[data-update-action]").forEach((action) => {
      action.addEventListener("click", () => {
        const updateAction = action.getAttribute("data-update-action");
        if (updateAction === "check") {
          checkDesktopRelease();
        } else if (updateAction === "install") {
          installDesktopRelease();
        }
      });
    });

    if (activeSection === "service" && !serviceSnapshot && !serviceBusyAction) {
      queueMicrotask(async () => {
        await loadServiceSnapshot("bootstrap", { refresh: false });
        if (serviceBusyAction) return;
        refreshServiceSnapshot();
      });
    }
    if (activeSection === "updates" && !updateSnapshot && !updateBusyAction) {
      updateBusyAction = "updates-load";
      queueMicrotask(async () => {
        await loadServiceSnapshot("bootstrap", { refresh: false }, null);
        updateBusyAction = null;
        render();
      });
    }

    panel.appendChild(inner);

    const toolbar = document.createElement("div");
    toolbar.className = "titlebar-tools";
    toolbar.innerHTML = titlebarToolsContent();
    toolbar.querySelector("[data-titlebar-drag]")?.addEventListener("mousedown", (event) => {
      if (event.button !== 0) return;
      event.preventDefault();
      void startWindowDrag();
    });
    toolbar.querySelector("[data-titlebar-service]")?.addEventListener("click", () => {
      void toggleSelectedService();
    });
    toolbar.querySelector("[data-titlebar-log]")?.addEventListener("click", () => {
      void openSelectedServiceLog();
    });
    toolbar.querySelector("[data-titlebar-theme-toggle]")?.addEventListener("click", () => {
      titlebarThemeMenuOpen = !titlebarThemeMenuOpen;
      releaseNotesOpen = false;
      render();
    });
    toolbar.querySelectorAll("[data-titlebar-theme-option]").forEach((button) => {
      button.addEventListener("click", () => {
        const themeId = button.getAttribute("data-titlebar-theme-option");
        titlebarThemeMenuOpen = false;
        if (themeId) setTheme(themeId);
      });
    });
    toolbar.querySelector("[data-titlebar-theme-new]")?.addEventListener("click", () => {
      titlebarThemeMenuOpen = false;
      activeSection = "appearance";
      window.__slockDesktopSettingsSection = activeSection;
      newThemeDraft = { name: "", accent: '#10a37f' };
      open = true;
      window.__slockDesktopSettingsOpen = open;
      render();
      queueMicrotask(() => shadow.querySelector("[data-theme-draft-name]")?.focus());
    });
    toolbar.querySelector("[data-titlebar-mode]")?.addEventListener("click", () => {
      titlebarThemeMenuOpen = false;
      releaseNotesOpen = false;
      setMode(nextModeId());
    });
    toolbar.querySelector("[data-titlebar-language]")?.addEventListener("click", () => {
      titlebarThemeMenuOpen = false;
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
        releaseNotesOpen = !releaseNotesOpen;
        render();
      } else {
        releaseNotesOpen = false;
        checkDesktopRelease();
      }
    });

    dock.append(panel, toolbar);
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
      await invoke("save_session_tokens", {
        accessToken,
        refreshToken,
      });
      window.__slockDesktopSessionSignature = nextSignature;
      if (wasUnauthenticated) {
        const payload = await invoke("bootstrap", { refresh: false });
        syncDesktopPayload(payload);
        if (activeSection === "service") {
          refreshServiceSnapshot();
        } else {
          render();
        }
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
    return match?.[1] || null;
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

  window.__slockDesktopSettingsClosePanel = closePanel;
  window.__slockDesktopCloseTransientTitlebarPanels = closeTransientTitlebarPanels;
  syncSessionTokens();

  if (!window.__slockDesktopSettingsEscapeBound) {
    window.__slockDesktopSettingsEscapeBound = true;
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && window.__slockDesktopCloseTransientTitlebarPanels?.()) {
        return;
      }
      if (event.key === "Escape" && window.__slockDesktopSettingsOpen) {
        window.__slockDesktopSettingsClosePanel?.();
      }
    });
  }

  if (!window.__slockDesktopSettingsPointerBound) {
    window.__slockDesktopSettingsPointerBound = true;
    document.addEventListener("pointerdown", (event) => {
      const activeHost = document.getElementById(hostId);
      const path = event.composedPath ? event.composedPath() : [];
      const insideDesktopHost = activeHost && path.includes(activeHost);
      if (!insideDesktopHost) window.__slockDesktopCloseTransientTitlebarPanels?.();
      if (!window.__slockDesktopSettingsOpen) return;
      if (insideDesktopHost) return;
      window.__slockDesktopSettingsClosePanel?.();
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

  function closePanel() {
    open = false;
    window.__slockDesktopSettingsOpen = false;
    const activeHost = document.getElementById(hostId);
    if (activeHost) {
      const activeDock = activeHost.shadowRoot?.querySelector(".dock");
      const activePanel = activeHost.shadowRoot?.querySelector(".panel");
      if (activeDock) activeDock.dataset.open = "false";
      if (activePanel) activePanel.hidden = true;
    }
  }

  function closeTransientTitlebarPanels() {
    if (!titlebarThemeMenuOpen && !releaseNotesOpen) return false;
    titlebarThemeMenuOpen = false;
    releaseNotesOpen = false;
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
        assert!(script.contains("if (!selectedServerSlug)"));
        assert!(script.contains("data-service-action=\"start\""));
        assert!(script.contains(
            "loadServiceSnapshot(\"start_service\", { selectedServerSlug }, \"service-start\")"
        ));
        assert!(script.contains("selectedRow"));
        assert!(script.contains("service?.configured"));
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
        assert!(script.contains("M10 16h10"));
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
        assert!(script.contains("data-theme-new"));
        assert!(script.contains("create_custom_theme"));
        assert!(script.contains("data-service-search"));
        assert!(script.contains("open_service_log"));
        assert!(script.contains("data-update-action=\"check\""));
        assert!(script.contains("check_desktop_update"));
        assert!(script.contains("hydrateReleaseStateFromUpdateSnapshot"));
        assert!(script.contains("updateSnapshot?.latest"));
        assert!(script.contains("desktop_update_checked"));
        assert!(script.contains("syncDesktopUpdateCheck(event?.payload)"));
        assert!(!script.contains("__slockDesktopAutoUpdateChecked"));
        assert!(script.contains("install_desktop_update"));
        assert!(!script.contains("class=\"launcher"));
    }
}
