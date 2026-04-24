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

pub fn agentation_script() -> &'static str {
    WORKSPACE_AGENTATION_SCRIPT
}

const WORKSPACE_AGENTATION_SCRIPT: &str = r#"
(() => {
  const rootId = "slock-desktop-agentation-root";
  if (window.__slockDesktopAgentationMounted) return;
  window.__slockDesktopAgentationMounted = true;

  const mount = document.getElementById(rootId) || document.createElement("div");
  mount.id = rootId;
  if (!mount.isConnected) document.body.appendChild(mount);

  Promise.all([
    import("https://esm.sh/react@19.2.5"),
    import("https://esm.sh/react-dom@19.2.5/client?deps=react@19.2.5"),
    import("https://esm.sh/agentation@3.0.2?deps=react@19.2.5,react-dom@19.2.5"),
  ])
    .then(([React, ReactDOM, AgentationModule]) => {
      if (window.__slockDesktopAgentationRoot) return;
      window.__slockDesktopAgentationRoot = ReactDOM.createRoot(mount);
      window.__slockDesktopAgentationRoot.render(React.createElement(AgentationModule.Agentation));
    })
    .catch((error) => {
      window.__slockDesktopAgentationMounted = false;
      console.warn("[Slock Desktop] Agentation workspace injection failed.", error);
    });
})();
"#;

const WORKSPACE_SETTINGS_SCRIPT: &str = r#"
(() => {
  const hostId = "slock-desktop-settings-host";
  const themes = __SLOCK_DESKTOP_THEMES__;
  const initialThemeId = __SLOCK_DESKTOP_ACTIVE_THEME__;
  const initialMode = __SLOCK_DESKTOP_ACTIVE_MODE__;
  const initialLanguage = __SLOCK_DESKTOP_ACTIVE_LANGUAGE__;
  const initialResolvedLanguage = __SLOCK_DESKTOP_RESOLVED_LANGUAGE__;
  const modes = [
    { id: "light", icon: "☼", key: "modeLight" },
    { id: "dark", icon: "◐", key: "modeDark" },
    { id: "system", icon: "◌", key: "modeSystem" },
  ];
  const languages = [
    { id: "en-US", shortKey: "languageEnglishShort", key: "languageEnglish" },
    { id: "zh-CN", shortKey: "languageChineseShort", key: "languageChinese" },
    { id: "system", shortKey: "languageSystemShort", key: "languageSystem" },
  ];
  const copy = {
    "en-US": {
      launcher: "Desktop Settings",
      eyebrow: "Slock Desktop",
      title: "Desktop Settings",
      settingsSections: "Desktop settings sections",
      description: "Appearance settings apply to this workspace window immediately and persist locally.",
      appearance: "Appearance",
      service: "Service",
      updates: "Updates",
      mode: "Mode",
      modeLight: "Light",
      modeDark: "Dark",
      modeSystem: "System",
      theme: "Theme",
      language: "Language",
      languageEnglish: "English",
      languageChinese: "Chinese",
      languageSystem: "System",
      languageEnglishShort: "EN",
      languageChineseShort: "中",
      languageSystemShort: "System",
      saved: "Saved in desktop config",
      themes: "themes",
      dragHint: "Drag to move",
      themeNames: {
        original: "Original",
      },
      themeSummaries: {
        original: "Keep the native Slock look with no desktop theme injection.",
      },
    },
    "zh-CN": {
      launcher: "桌面设置",
      eyebrow: "Slock 桌面端",
      title: "桌面设置",
      settingsSections: "桌面设置分区",
      description: "外观设置会立即应用到当前工作页窗口，并保存在本地。",
      appearance: "外观",
      service: "服务",
      updates: "更新",
      mode: "模式",
      modeLight: "亮色",
      modeDark: "暗黑",
      modeSystem: "系统",
      theme: "主题",
      language: "语言",
      languageEnglish: "英文",
      languageChinese: "中文",
      languageSystem: "系统",
      languageEnglishShort: "EN",
      languageChineseShort: "中",
      languageSystemShort: "跟随系统",
      saved: "已保存到桌面配置",
      themes: "个主题",
      dragHint: "拖动移动",
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
      inReview: "In review",
      inReviewTitle: "In Review",
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
      inReview: "待复核",
      inReviewTitle: "待复核",
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
  const themeVarNames = [
    "--desktop-canvas",
    "--desktop-surface",
    "--desktop-surface-secondary",
    "--desktop-line",
    "--desktop-text",
    "--desktop-muted",
    "--desktop-selection",
  ];
  const dockPositionKey = "slock-desktop-settings-position";
  let suppressLauncherClick = false;

  const clamp = (value, min, max) => Math.min(Math.max(value, min), max);
  const defaultDockPosition = () => ({
    x: Math.max(12, window.innerWidth - 68),
    y: Math.max(12, window.innerHeight - 68),
  });
  const sanitizeDockPosition = (position) => {
    const fallback = defaultDockPosition();
    const x = Number.isFinite(position?.x) ? position.x : fallback.x;
    const y = Number.isFinite(position?.y) ? position.y : fallback.y;
    return {
      x: clamp(x, 12, Math.max(12, window.innerWidth - 56)),
      y: clamp(y, 12, Math.max(12, window.innerHeight - 56)),
    };
  };
  const loadDockPosition = () => {
    try {
      return sanitizeDockPosition(JSON.parse(localStorage.getItem(dockPositionKey) || "null"));
    } catch {
      return sanitizeDockPosition(null);
    }
  };
  const saveDockPosition = (position) => {
    try {
      localStorage.setItem(dockPositionKey, JSON.stringify(sanitizeDockPosition(position)));
    } catch {
      // Local storage can be unavailable in hardened webview contexts.
    }
  };
  const applyDockPosition = (dock, position) => {
    const next = sanitizeDockPosition(position);
    dock.style.setProperty("--dock-x", `${next.x}px`);
    dock.style.setProperty("--dock-y", `${next.y}px`);
    dock.dataset.align = next.x < window.innerWidth / 2 ? "left" : "right";
    dock.dataset.vertical = next.y < window.innerHeight / 2 ? "top" : "bottom";
    return next;
  };

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
      "inReview",
      "inReviewTitle",
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
        if (isExcludedTranslationTarget(element)) return;

        translateAttribute(element, "aria-label");
        translateAttribute(element, "title");
        translateAttribute(element, "placeholder");

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
      z-index: 2147483647;
      width: 44px;
      height: 44px;
      transform: translate3d(var(--dock-x, calc(100vw - 68px)), var(--dock-y, calc(100vh - 68px)), 0);
      pointer-events: none;
    }

    button {
      font: inherit;
      touch-action: manipulation;
    }

    .launcher,
    .mode-option,
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
      width: 44px;
      height: 44px;
      min-height: 44px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      padding: 0;
      border: 1px solid var(--desktop-line);
      border-radius: var(--desktop-radius-md);
      background: var(--desktop-surface-secondary);
      color: var(--desktop-text);
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
      font-weight: 600;
      letter-spacing: -0.01em;
    }

    .launcher-icon {
      width: 20px;
      display: grid;
      gap: 4px;
    }

    .launcher-icon span {
      position: relative;
      height: 2px;
      border-radius: var(--desktop-radius-pill);
      background: currentColor;
      opacity: 0.82;
    }

    .launcher-icon span::after {
      content: "";
      position: absolute;
      top: 50%;
      width: 5px;
      height: 5px;
      border-radius: var(--desktop-radius-pill);
      background: var(--desktop-accent);
      box-shadow: 0 0 0 2px var(--desktop-selection);
      transform: translateY(-50%);
    }

    .launcher-icon span:nth-child(1)::after {
      left: 3px;
    }

    .launcher-icon span:nth-child(2)::after {
      right: 4px;
    }

    .launcher-icon span:nth-child(3)::after {
      left: 9px;
    }

    .panel {
      pointer-events: auto;
      position: absolute;
      right: 0;
      bottom: 54px;
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
      left: 0;
      right: auto;
    }

    .dock[data-vertical="top"] .panel {
      top: 54px;
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
      min-height: 38px;
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 8px;
      border-radius: var(--desktop-radius-md);
      color: var(--desktop-text);
      font-size: 13px;
      font-weight: 600;
    }

    .nav-item.active {
      background: var(--desktop-selection);
      box-shadow: none;
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
      border: 0;
      border-radius: var(--desktop-radius-sm);
      background: transparent;
      color: var(--desktop-text);
      font-size: 12px;
      font-weight: 600;
    }

    .language-option {
      min-width: 72px;
      min-height: 30px;
      padding: 0 10px;
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

    @media (hover: hover) {
      .launcher:hover {
        background: var(--desktop-hover);
      }

      .theme-option:hover {
        background: var(--desktop-hover);
      }

      .mode-option:hover {
        background: var(--desktop-hover);
      }

      .language-option:hover {
        background: var(--desktop-hover);
      }
    }

    .launcher:active,
    .language-option:active,
    .mode-option:active,
    .theme-option:active {
      transform: scale(0.97);
    }

    @media (max-width: 520px) {
      .dock {
        transform: translate3d(var(--dock-x, calc(100vw - 56px)), var(--dock-y, calc(100vh - 56px)), 0);
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
      .language-option,
      .mode-option,
      .theme-option,
      .panel {
        transition-duration: 1ms;
      }
    }
  `;

  const render = () => {
    syncHostTheme();
    shadow.innerHTML = "";

    const style = document.createElement("style");
    style.textContent = css;
    shadow.appendChild(style);

    const dock = document.createElement("div");
    dock.className = "dock";
    dock.dataset.open = String(open);
    applyDockPosition(dock, loadDockPosition());

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
      <div class="quick-controls">
        <div class="mode-list" role="radiogroup" aria-label="${t("mode")}"></div>
        <div class="language-list" role="radiogroup" aria-label="${t("language")}"></div>
      </div>
      <div class="settings-grid">
        <nav class="nav" aria-label="${t("settingsSections")}">
          <div class="nav-item active">${t("appearance")}</div>
          <div class="nav-item">${t("service")}</div>
          <div class="nav-item">${t("updates")}</div>
        </nav>
        <div class="content">
          <p class="setting-title">${t("theme")}</p>
          <div class="theme-list" role="radiogroup" aria-label="${t("theme")}"></div>
          <div class="status">
            <span>${t("saved")}</span>
            <span>${themes.length} ${t("themes")}</span>
          </div>
        </div>
      </div>
    `;

    const modeList = inner.querySelector(".mode-list");
    const list = inner.querySelector(".theme-list");
    const languageList = inner.querySelector(".language-list");

    modes.forEach((mode) => {
      const selected = mode.id === activeMode;
      const option = document.createElement("button");
      option.className = `mode-option${selected ? " active" : ""}`;
      option.type = "button";
      option.setAttribute("role", "radio");
      option.setAttribute("aria-checked", String(selected));
      option.title = t(mode.key);
      option.textContent = mode.icon;
      option.addEventListener("click", () => setMode(mode.id));
      modeList.appendChild(option);
    });

    languages.forEach((language) => {
      const selected = language.id === activeLanguage;
      const option = document.createElement("button");
      option.className = `language-option${selected ? " active" : ""}`;
      option.type = "button";
      option.setAttribute("role", "radio");
      option.setAttribute("aria-checked", String(selected));
      option.title = t(language.key);
      option.textContent = t(language.shortKey);
      option.addEventListener("click", () => setLanguage(language.id));
      languageList.appendChild(option);
    });

    themes.forEach((theme) => {
      const selected = theme.id === activeThemeId;
      const display = themeDisplay(theme);
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
      name.textContent = display.name;

      const summary = document.createElement("span");
      summary.className = "theme-summary";
      summary.textContent = display.summary;

      const check = document.createElement("span");
      check.className = "check";
      check.setAttribute("aria-hidden", "true");
      check.textContent = selected ? "✓" : "";

      copy.append(name, summary);
      option.append(swatch, copy, check);
      option.addEventListener("click", () => setTheme(theme.id));
      list.appendChild(option);
    });

    panel.appendChild(inner);

    const launcher = document.createElement("button");
    launcher.className = "launcher";
    launcher.type = "button";
    launcher.title = `${t("launcher")} · ${t("dragHint")}`;
    launcher.setAttribute("aria-label", t("launcher"));
    launcher.setAttribute("aria-expanded", String(open));
    launcher.innerHTML = `<span class="launcher-icon" aria-hidden="true"><span></span><span></span><span></span></span>`;
    launcher.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) return;
      const start = loadDockPosition();
      let current = start;
      let moved = false;
      const startX = event.clientX;
      const startY = event.clientY;
      launcher.setPointerCapture?.(event.pointerId);

      const move = (moveEvent) => {
        const deltaX = moveEvent.clientX - startX;
        const deltaY = moveEvent.clientY - startY;
        if (Math.abs(deltaX) + Math.abs(deltaY) > 5) {
          moved = true;
        }
        current = applyDockPosition(dock, {
          x: start.x + deltaX,
          y: start.y + deltaY,
        });
      };

      const end = () => {
        launcher.removeEventListener("pointermove", move);
        launcher.removeEventListener("pointerup", end);
        launcher.removeEventListener("pointercancel", end);
        launcher.releasePointerCapture?.(event.pointerId);
        if (moved) {
          saveDockPosition(current);
          suppressLauncherClick = true;
          setTimeout(() => {
            suppressLauncherClick = false;
          }, 0);
        }
      };

      launcher.addEventListener("pointermove", move);
      launcher.addEventListener("pointerup", end);
      launcher.addEventListener("pointercancel", end);
    });
    launcher.addEventListener("click", () => {
      if (suppressLauncherClick) return;
      open = window.__slockDesktopSettingsOpen === true;
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

  const setMode = async (mode) => {
    activeMode = mode;
    render();

    try {
      const invoke = window.__TAURI__?.core?.invoke;
      if (typeof invoke !== "function") {
        throw new Error("Tauri invoke API is unavailable");
      }
      await invoke("set_theme_mode", { themeMode: mode });
    } catch (error) {
      console.error("[Slock Desktop] theme mode update failed", error);
    }
  };

  const setLanguage = async (language) => {
    activeLanguage = language;
    render();
    translateSlockMenus();

    try {
      const invoke = window.__TAURI__?.core?.invoke;
      if (typeof invoke !== "function") {
        throw new Error("Tauri invoke API is unavailable");
      }
      await invoke("set_language", { language });
    } catch (error) {
      console.error("[Slock Desktop] language update failed", error);
    }
  };

  window.__slockDesktopSettingsClosePanel = closePanel;

  if (!window.__slockDesktopSettingsEscapeBound) {
    window.__slockDesktopSettingsEscapeBound = true;
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && window.__slockDesktopSettingsOpen) {
        window.__slockDesktopSettingsClosePanel?.();
      }
    });
  }

  if (!window.__slockDesktopSettingsPointerBound) {
    window.__slockDesktopSettingsPointerBound = true;
    document.addEventListener("pointerdown", (event) => {
      if (!window.__slockDesktopSettingsOpen) return;
      const activeHost = document.getElementById(hostId);
      const path = event.composedPath ? event.composedPath() : [];
      if (activeHost && path.includes(activeHost)) return;
      window.__slockDesktopSettingsClosePanel?.();
    });
  }

  function closePanel() {
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

  render();
  bindSlockMenuTranslator();
})();
"#;
