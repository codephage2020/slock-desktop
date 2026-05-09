pub fn agent_card_inject_script(server_slug: &str, resolved_language: &str) -> String {
    let server_slug =
        serde_json::to_string(server_slug).unwrap_or_else(|_| "\"\"".into());
    let resolved_language =
        serde_json::to_string(resolved_language).unwrap_or_else(|_| "\"en-US\"".into());
    AGENT_CARD_INJECT_SCRIPT
        .replace("__SLOCK_DESKTOP_AGENT_CARD_SERVER_SLUG__", &server_slug)
        .replace("__SLOCK_DESKTOP_AGENT_CARD_LOCALE__", &resolved_language)
}

const AGENT_CARD_INJECT_SCRIPT: &str = r##"
(function () {
  try {
    if (window.location.origin !== "https://app.slock.ai") return;

    const FALLBACK_SERVER_SLUG = __SLOCK_DESKTOP_AGENT_CARD_SERVER_SLUG__;
    const LOCALE = __SLOCK_DESKTOP_AGENT_CARD_LOCALE__;
    const INJECT_ATTR = "data-slock-desktop-agent-card-injected";
    const SLUG_REGEX = /^\/s\/([^/?#]+)/;

    const TRANSLATIONS = {
      "en-US": {
        description: "Description",
        noDescription: "No description",
        activity: "Recent Activity",
        noActivity: "No recent activity",
        online: "online",
        offline: "offline",
        justNow: "just now",
        minutesAgo: "%dm ago",
        hoursAgo: "%dh ago",
        daysAgo: "%dd ago",
        stop: "Stop",
        start: "Start",
        restart: "Quick Restart",
        actIdle: "Idle",
        actWorking: "Working",
        actThinking: "Thinking",
        actDisconnected: "Disconnected",
        actRunningCommand: "Running command",
        actSendingMessage: "Sending message",
        actOutput: "Output",
      },
      "zh-CN": {
        description: "描述",
        noDescription: "暂无描述",
        activity: "最近活动",
        noActivity: "暂无活动",
        online: "在线",
        offline: "离线",
        justNow: "刚刚",
        minutesAgo: "%d分钟前",
        hoursAgo: "%d小时前",
        daysAgo: "%d天前",
        stop: "停止",
        start: "启动",
        restart: "快速重启",
        actIdle: "空闲",
        actWorking: "工作中",
        actThinking: "思考中",
        actDisconnected: "已断开",
        actRunningCommand: "运行命令",
        actSendingMessage: "发送消息",
        actOutput: "输出",
      },
    };

    function t(key) {
      const dict = TRANSLATIONS[LOCALE] || TRANSLATIONS["en-US"];
      return dict[key] || key;
    }

    function formatRelativeTime(isoStr) {
      if (!isoStr) return "";
      const diff = Date.now() - new Date(isoStr).getTime();
      const mins = Math.floor(diff / 60000);
      if (mins < 1) return t("justNow");
      if (mins < 60) return t("minutesAgo").replace("%d", mins);
      const hours = Math.floor(mins / 60);
      if (hours < 24) return t("hoursAgo").replace("%d", hours);
      const days = Math.floor(hours / 24);
      return t("daysAgo").replace("%d", days);
    }

    // Format timestamp as HH:MM:SS for activity entries
    function formatTimeHMS(isoStr) {
      if (!isoStr) return "";
      try {
        var d = new Date(isoStr);
        if (isNaN(d.getTime())) return "";
        var hh = String(d.getHours()).padStart(2, "0");
        var mm = String(d.getMinutes()).padStart(2, "0");
        var ss = String(d.getSeconds()).padStart(2, "0");
        return hh + ":" + mm + ":" + ss;
      } catch (_) { return ""; }
    }

    // Map API activity kind to display { label, detail, dotColor }
    // API fields after unwrapEntry: kind, activity, detail, toolInput
    function mapActivityKind(entry) {
      var kind = entry.kind || "";
      var activity = entry.activity || "";
      var detail = entry.detail || "";
      var toolInput = entry.toolInput || "";

      // status entries: activity = connection state (online/disconnected),
      // detail = work state (Idle/Working/Thinking). Check detail first, then activity.
      // When kind is empty, only enter status branch if detail/activity matches a known status value.
      var knownStatuses = ["idle","online","working","thinking","disconnected","stopped"];
      var statusKey = (detail || activity || "").toLowerCase();
      if (kind === "status" || (!kind && knownStatuses.indexOf(statusKey) !== -1)) {
        var detailLower = detail.toLowerCase();
        var activityLower = activity.toLowerCase();
        // Check detail first (primary work state)
        if (detailLower === "idle") return { label: t("actIdle"), detail: "", dotColor: "#22c55e" };
        if (detailLower === "working") return { label: t("actWorking"), detail: "", dotColor: "#eab308" };
        if (detailLower === "thinking") return { label: t("actThinking"), detail: "", dotColor: "#eab308" };
        if (detailLower === "disconnected") return { label: t("actDisconnected"), detail: "", dotColor: "#ef4444" };
        // Fallback: detail empty, check activity field
        if (activityLower === "idle" || activityLower === "online") return { label: t("actIdle"), detail: "", dotColor: "#22c55e" };
        if (activityLower === "working") return { label: t("actWorking"), detail: "", dotColor: "#eab308" };
        if (activityLower === "thinking") return { label: t("actThinking"), detail: "", dotColor: "#eab308" };
        if (activityLower === "disconnected" || activityLower === "stopped") return { label: t("actDisconnected"), detail: "", dotColor: "#ef4444" };
        // Unknown status value: fall through to Output fallback
      }

      if (kind === "tool_use" || kind === "tool_start") {
        var input = toolInput || detail || "";
        return { label: t("actRunningCommand"), detail: input, dotColor: "#eab308" };
      }

      if (kind === "message") {
        return { label: t("actSendingMessage"), detail: detail || "", dotColor: "#eab308" };
      }

      if (kind === "output") {
        return { label: t("actOutput"), detail: detail || "", dotColor: "#3b82f6" };
      }

      // Fallback for any unknown kind: treat as Output to avoid raw labels
      var fallbackDetail = activity || detail || kind || "";
      return { label: t("actOutput"), detail: fallbackDetail, dotColor: "#3b82f6" };
    }

    // --- Tauri invoke ---
    function invoke(cmd, args) {
      const tauriInvoke =
        window.__TAURI__?.core?.invoke || window.__TAURI__?.invoke;
      if (!tauriInvoke) return Promise.reject(new Error("No Tauri invoke"));
      return tauriInvoke(cmd, args);
    }

    // --- Dynamic server slug ---
    function getCurrentServerSlug() {
      const match = window.location.pathname.match(SLUG_REGEX);
      if (!match) return FALLBACK_SERVER_SLUG || "";
      try { return decodeURIComponent(match[1]); } catch (_) { return match[1]; }
    }

    // --- Agent data cache (keyed by server slug) ---
    let cachedAgents = [];
    let cachedSlug = "";
    let lastFetchMs = 0;
    const CACHE_TTL = 30000;

    async function getAgents() {
      const slug = getCurrentServerSlug();
      if (!slug) return [];
      const now = Date.now();
      if (slug === cachedSlug && now - lastFetchMs < CACHE_TTL && cachedAgents.length) return cachedAgents;
      // Slug changed — clear stale cache before fetching
      if (slug !== cachedSlug) {
        cachedAgents = [];
        cachedSlug = slug;
        lastFetchMs = 0;
      }
      try {
        const data = await invoke("fetch_dashboard", { serverSlug: slug });
        cachedAgents = data.agents || [];
        cachedSlug = slug;
        lastFetchMs = Date.now();
      } catch (e) {
        console.warn("[Slock Desktop] agent card: fetch_dashboard failed", e);
      }
      return cachedAgents;
    }

    async function getAgentActivity(agentId) {
      const slug = getCurrentServerSlug();
      if (!slug || !agentId) return [];
      try {
        const result = await invoke("fetch_agent_activity", {
          serverSlug: slug,
          agentId: agentId,
        });
        console.log("[Slock Desktop] agent card: fetch_agent_activity returned", (result || []).length, "entries for", agentId);
        return result || [];
      } catch (e) {
        console.warn("[Slock Desktop] agent card: fetch_agent_activity failed", e);
        return [];
      }
    }

    // --- React fiber helpers ---
    function getReactFiber(el) {
      if (!el) return null;
      const key = Object.keys(el).find(
        (k) =>
          k.startsWith("__reactFiber$") ||
          k.startsWith("__reactInternalInstance$")
      );
      return key ? el[key] : null;
    }

    // UUID-like pattern for agent IDs
    const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

    // Check a single state object for agent ID (exact or broad match)
    function extractAgentId(obj) {
      if (!obj || typeof obj !== "object" || Array.isArray(obj)) return null;
      // Exact match: { type: "member-agent", id }
      if (obj.type === "member-agent" && obj.id) return obj.id;
      // Broad match: any object with type containing "agent" and a UUID id
      if (typeof obj.type === "string" && obj.type.indexOf("agent") !== -1
          && typeof obj.id === "string" && UUID_RE.test(obj.id)) return obj.id;
      // Check nested popover: { popover: { type: "member-agent", id } }
      if (obj.popover) {
        const pid = extractAgentId(obj.popover);
        if (pid) return pid;
      }
      return null;
    }

    // Walk a single fiber's state/props for agent ID
    function scanFiberForAgentId(fiber) {
      if (!fiber) return null;
      // Walk memoizedState linked list
      let state = fiber.memoizedState;
      for (let s = 0; s < 30 && state; s++) {
        const ms = state.memoizedState;
        const id = extractAgentId(ms);
        if (id) return id;
        // Check queue.lastRenderedState
        if (state.queue && state.queue.lastRenderedState) {
          const id2 = extractAgentId(state.queue.lastRenderedState);
          if (id2) return id2;
        }
        state = state.next;
      }
      // Check pendingProps
      if (fiber.pendingProps) {
        const id = extractAgentId(fiber.pendingProps);
        if (id) return id;
      }
      // Check memoizedProps
      if (fiber.memoizedProps) {
        const id = extractAgentId(fiber.memoizedProps);
        if (id) return id;
      }
      return null;
    }

    function getAgentIdFromFiber(cardEl) {
      try {
        let fiber = getReactFiber(cardEl);
        // Phase 1: Walk parent chain (return)
        for (let depth = 0; depth < 50 && fiber; depth++) {
          const id = scanFiberForAgentId(fiber);
          if (id) return id;
          // Also check alternate fiber (previous render)
          if (fiber.alternate) {
            const altId = scanFiberForAgentId(fiber.alternate);
            if (altId) return altId;
          }
          fiber = fiber.return;
        }
        // Phase 2: Walk child/sibling tree from root fiber (up to 60 nodes)
        fiber = getReactFiber(cardEl);
        if (fiber) {
          const queue = [fiber];
          let visited = 0;
          while (queue.length > 0 && visited < 60) {
            const f = queue.shift();
            visited++;
            const id = scanFiberForAgentId(f);
            if (id) return id;
            if (f.child) queue.push(f.child);
            if (f.sibling) queue.push(f.sibling);
          }
        }
      } catch (e) {
        console.warn("[Slock Desktop] agent card: fiber walk failed", e);
      }
      return null;
    }

    // --- Hover capture: track last-interacted agent trigger ---
    let lastHoveredAgentId = null;
    let lastHoveredTimestamp = 0;
    const HOVER_TTL = 5000; // 5 seconds

    function captureAgentFromTrigger(event) {
      try {
        // Walk up from event target to find an element with agent data in fiber
        let el = event.target;
        for (let i = 0; i < 10 && el; i++) {
          const fiber = getReactFiber(el);
          if (fiber) {
            // Walk parent chain briefly
            let f = fiber;
            for (let d = 0; d < 15 && f; d++) {
              const id = scanFiberForAgentId(f);
              if (id) {
                lastHoveredAgentId = id;
                lastHoveredTimestamp = Date.now();
                return;
              }
              f = f.return;
            }
          }
          el = el.parentElement;
        }
        // No agent ID found on this interaction — clear stale hover data
        // to prevent injecting old agent info into a non-agent card
        lastHoveredAgentId = null;
        lastHoveredTimestamp = 0;
      } catch (_) {}
    }

    // Clean up previous capture listeners before registering new ones
    // (script may be re-evaluated on theme/language changes)
    if (window.__slockDesktopAgentCardCaptureCleanup) {
      window.__slockDesktopAgentCardCaptureCleanup();
    }
    document.addEventListener("pointerover", captureAgentFromTrigger, true);
    document.addEventListener("click", captureAgentFromTrigger, true);
    window.__slockDesktopAgentCardCaptureCleanup = function () {
      document.removeEventListener("pointerover", captureAgentFromTrigger, true);
      document.removeEventListener("click", captureAgentFromTrigger, true);
    };

    function getHoveredAgentId() {
      if (lastHoveredAgentId && (Date.now() - lastHoveredTimestamp) < HOVER_TTL) {
        return lastHoveredAgentId;
      }
      return null;
    }

    // --- Name-based fallback: match card text against known agents ---
    function getAgentIdByName(cardEl, agents) {
      if (!agents || !agents.length) return null;
      const cardText = (cardEl.textContent || "").trim();
      if (!cardText) return null;
      // Try matching against agent names (prefer exact match on displayName or name)
      for (const agent of agents) {
        const dn = agent.displayName || "";
        const n = agent.name || "";
        if (dn && cardText.indexOf(dn) !== -1) return agent.id;
        if (n && cardText.indexOf(n) !== -1) return agent.id;
      }
      return null;
    }

    // --- Detect agent card (only hover popover, reject native page cards) ---
    function isAgentCard(el) {
      if (!el || !el.classList) return false;
      if (!el.classList.contains("card-brutal")) return false;
      // Reject: native page cards have layout classes like w-full, max-w-md, p-6
      if (el.classList.contains("w-full") || el.classList.contains("max-w-md") || el.classList.contains("p-6")) {
        console.log("[Slock Desktop] agent card: card-brutal skipped: native page layout class", el.className);
        return false;
      }
      // Positive allowlist: hover popover cards always have overflow-hidden
      if (!el.classList.contains("overflow-hidden")) {
        console.log("[Slock Desktop] agent card: card-brutal skipped: missing overflow-hidden", el.className);
        return false;
      }
      // Walk ancestors: reject if inside a dialog, modal, or Radix dialog content
      var parent = el.parentElement;
      var depth = 0;
      var ancestors = [];
      while (parent && depth < 30) {
        var tag = parent.tagName || "";
        var cls = (parent.className && typeof parent.className === "string") ? parent.className.substring(0, 60) : "";
        var role = (parent.getAttribute && parent.getAttribute("role")) || "";
        ancestors.push(tag + (cls ? "." + cls.split(" ")[0] : "") + (role ? "[role=" + role + "]" : ""));

        if (parent.getAttribute) {
          if (parent.getAttribute("role") === "dialog" || parent.getAttribute("aria-modal") === "true") {
            console.log("[Slock Desktop] agent card: card-brutal skipped: inside dialog/modal at depth", depth);
            return false;
          }
          if (parent.hasAttribute("data-radix-dialog-content")) {
            console.log("[Slock Desktop] agent card: card-brutal skipped: inside radix dialog at depth", depth);
            return false;
          }
        }
        parent = parent.parentElement;
        depth++;
      }
      console.log("[Slock Desktop] agent card: accepted card-brutal overflow-hidden, ancestors:", ancestors.slice(0, 10).join(" > "));
      return true;
    }

    // --- Build injected action buttons (neobrutalism btn-brutal style) ---
    var SVG_STOP = '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="6" width="12" height="12" rx="2"/></svg>';
    var SVG_START = '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><polygon points="8,5 20,12 8,19"/></svg>';
    var SVG_RESTART = '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 1 3 6.7"/><polyline points="3 22 3 16 9 16"/></svg>';

    function createBrutalButton(svg, label, onClick) {
      var btn = document.createElement("button");
      btn.type = "button";
      btn.title = label;
      btn.setAttribute("aria-label", label);
      btn.style.cssText =
        "display: inline-flex; align-items: center; gap: 4px; height: 28px; padding: 0 10px; border-radius: 8px; border: 1.5px solid currentColor; background: transparent; cursor: pointer; font-size: 11px; font-weight: 700; font-family: inherit; color: inherit; transition: box-shadow 0.15s, transform 0.15s;";
      btn.innerHTML = svg + '<span>' + label + '</span>';
      btn.addEventListener("mouseenter", function () {
        if (!btn.disabled) {
          btn.style.boxShadow = "2px 2px 0 currentColor";
          btn.style.transform = "translate(-1px, -1px)";
        }
      });
      btn.addEventListener("mouseleave", function () {
        btn.style.boxShadow = "none";
        btn.style.transform = "none";
      });
      btn.addEventListener("click", async function (e) {
        e.stopPropagation();
        btn.disabled = true;
        btn.style.opacity = "0.4";
        btn.style.cursor = "default";
        try {
          await onClick();
          lastFetchMs = 0;
        } catch (err) {
          console.warn("[Slock Desktop] agent card: action failed", err);
        }
        btn.disabled = false;
        btn.style.opacity = "1";
        btn.style.cursor = "pointer";
      });
      return btn;
    }

    function buildActionButtons(agent, serverSlug) {
      const container = document.createElement("div");
      container.setAttribute(INJECT_ATTR, "true");
      container.style.cssText =
        "padding: 8px 12px 4px; display: flex; justify-content: flex-end; gap: 6px;";

      const isOnline = agent.status !== "offline";

      // Start/Stop toggle
      var toggleBtn = createBrutalButton(
        isOnline ? SVG_STOP : SVG_START,
        isOnline ? t("stop") : t("start"),
        function () {
          return invoke(isOnline ? "stop_agent" : "start_agent", {
            serverSlug: serverSlug,
            agentId: agent.id,
          });
        }
      );
      container.appendChild(toggleBtn);

      // Quick Restart button (only when online) — stop → 1s → start
      if (isOnline) {
        var restartBtn = createBrutalButton(
          SVG_RESTART,
          t("restart"),
          async function () {
            await invoke("stop_agent", { serverSlug: serverSlug, agentId: agent.id });
            await new Promise(function (r) { setTimeout(r, 1000); });
            await invoke("start_agent", { serverSlug: serverSlug, agentId: agent.id });
          }
        );
        restartBtn.title = LOCALE === "zh-CN" ? "停止后立即启动此 Agent" : "Stop then start this agent";
        restartBtn.setAttribute("aria-label", LOCALE === "zh-CN" ? "快速重启：停止后立即启动此 Agent" : "Quick restart: stop then start this agent");
        container.appendChild(restartBtn);
      }

      return container;
    }

    // --- Build injected activity footer (bottom of card) ---
    // Unwrap nested entry: API returns { entry: { activity, detail, kind }, timestamp }
    // Merge strategy: start with raw, overlay inner fields where inner is non-empty
    function unwrapEntry(raw) {
      if (!raw) return raw;
      if (raw.entry && typeof raw.entry === "object") {
        var inner = raw.entry;
        var merged = {};
        // Copy all top-level fields first (except entry itself)
        for (var k in raw) {
          if (k !== "entry" && raw.hasOwnProperty(k)) merged[k] = raw[k];
        }
        // Overlay inner fields: inner wins when it has a truthy value
        for (var j in inner) {
          if (inner.hasOwnProperty(j)) {
            if (inner[j] || !merged[j]) merged[j] = inner[j];
          }
        }
        return merged;
      }
      return raw;
    }

    // Extract timestamp from an activity entry using fallback chain
    function getEntryTime(rawEntry) {
      var entry = unwrapEntry(rawEntry);
      // Handle numeric epoch ms timestamps
      var ts = entry.createdAt || entry.created_at || entry.timestamp || entry.updatedAt || entry.updated_at || "";
      if (typeof ts === "number" && ts > 0) {
        return new Date(ts).toISOString();
      }
      return ts;
    }

    // --- Activity cache (keyed by agent ID) ---
    var activityCache = {};
    var ACTIVITY_CACHE_TTL = 30000;

    function buildActivitySkeleton() {
      const container = document.createElement("div");
      container.setAttribute(INJECT_ATTR, "true");
      container.setAttribute("data-activity-placeholder", "true");
      container.style.cssText =
        "padding: 8px 12px; border-top: 1px solid rgba(0,0,0,0.06); margin-top: 6px;";

      const actTitle = document.createElement("p");
      actTitle.style.cssText =
        "margin: 0 0 3px; font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; color: #9ca3af;";
      actTitle.textContent = t("activity");
      container.appendChild(actTitle);

      const loading = document.createElement("p");
      loading.style.cssText = "margin: 0; font-size: 11px; color: #9ca3af;";
      loading.textContent = "...";
      container.appendChild(loading);

      return container;
    }

    function fillActivityContent(container, activity) {
      // Remove loading placeholder (keep title)
      while (container.children.length > 1) {
        container.removeChild(container.lastChild);
      }

      if (activity && activity.length > 0) {
        try {
          console.log("[Slock Desktop] agent card: activity entry[0] keys =", Object.keys(activity[0]).join(","), "values =", JSON.stringify(activity[0]).substring(0, 200));
        } catch (_) {}

        // Sort by timestamp descending (newest first)
        var sorted = activity.slice().sort(function (a, b) {
          var ta = getEntryTime(a);
          var tb = getEntryTime(b);
          var ma = ta ? new Date(ta).getTime() : 0;
          var mb = tb ? new Date(tb).getTime() : 0;
          return mb - ma;
        });

        const list = document.createElement("ul");
        list.style.cssText = "margin: 0; padding: 0; list-style: none;";

        const maxItems = Math.min(sorted.length, 5);
        for (let i = 0; i < maxItems; i++) {
          const rawEntry = sorted[i];
          const entry = unwrapEntry(rawEntry);
          const mapped = mapActivityKind(entry);
          const entryTime = getEntryTime(rawEntry);

          const li = document.createElement("li");
          li.style.cssText =
            "display: flex; align-items: baseline; gap: 4px; font-size: 11px; line-height: 1.6; white-space: nowrap;";

          // Time (HH:MM:SS)
          const timeSpan = document.createElement("span");
          timeSpan.style.cssText = "flex-shrink: 0; color: #9ca3af; font-size: 10px; font-variant-numeric: tabular-nums;";
          timeSpan.textContent = formatTimeHMS(entryTime);
          li.appendChild(timeSpan);

          // Dot (colored by kind)
          const dot = document.createElement("span");
          dot.style.cssText = "flex-shrink: 0; color: " + mapped.dotColor + "; font-size: 8px; line-height: 1;";
          dot.textContent = "\u25CF";
          li.appendChild(dot);

          // Label (bold)
          const labelSpan = document.createElement("span");
          labelSpan.style.cssText = "flex-shrink: 0; font-weight: 700; color: #374151;";
          labelSpan.textContent = mapped.label;
          li.appendChild(labelSpan);

          // Detail (gray, truncated single line)
          if (mapped.detail) {
            const detailSpan = document.createElement("span");
            detailSpan.style.cssText =
              "flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; color: #9ca3af;";
            // Truncate to single line: collapse newlines
            detailSpan.textContent = mapped.detail.replace(/[\r\n]+/g, " ").substring(0, 100);
            li.appendChild(detailSpan);
          }

          list.appendChild(li);
        }

        if (list.children.length === 0) {
          const noAct = document.createElement("p");
          noAct.style.cssText = "margin: 0; font-size: 11px; color: #9ca3af;";
          noAct.textContent = t("noActivity");
          container.appendChild(noAct);
        } else {
          container.appendChild(list);
        }
      } else {
        const noAct = document.createElement("p");
        noAct.style.cssText = "margin: 0; font-size: 11px; color: #9ca3af;";
        noAct.textContent = t("noActivity");
        container.appendChild(noAct);
      }
      container.removeAttribute("data-activity-placeholder");
    }

    // --- Build placeholder buttons (disabled, no agent data yet) ---
    function buildPlaceholderButtons() {
      const container = document.createElement("div");
      container.setAttribute(INJECT_ATTR, "true");
      container.setAttribute("data-buttons-placeholder", "true");
      container.style.cssText =
        "padding: 8px 12px 4px; display: flex; justify-content: flex-end; gap: 6px;";

      // Disabled placeholder buttons (no click handlers)
      var stopBtn = document.createElement("button");
      stopBtn.type = "button";
      stopBtn.disabled = true;
      stopBtn.style.cssText =
        "display: inline-flex; align-items: center; gap: 4px; height: 28px; padding: 0 10px; border-radius: 8px; border: 1.5px solid currentColor; background: transparent; cursor: default; font-size: 11px; font-weight: 700; font-family: inherit; color: inherit; opacity: 0.3;";
      stopBtn.innerHTML = SVG_STOP + '<span>...</span>';
      container.appendChild(stopBtn);

      return container;
    }

    // --- Injection logic (true sync-first: UI before any await) ---
    function injectAgentCard(cardEl) {
      if (cardEl.hasAttribute(INJECT_ATTR)) return;
      cardEl.setAttribute(INJECT_ATTR, "pending");
      console.log("[Slock Desktop] agent card: injection started", cardEl.className);

      // --- Immediate sync: render skeleton before any async work ---
      cardEl.style.width = "auto";
      cardEl.style.minWidth = "220px";
      cardEl.style.maxWidth = "320px";

      var placeholderButtons = buildPlaceholderButtons();
      cardEl.appendChild(placeholderButtons);

      var activityFooter = buildActivitySkeleton();
      cardEl.appendChild(activityFooter);

      cardEl.setAttribute(INJECT_ATTR, "loading");
      console.log("[Slock Desktop] agent card: skeleton rendered (sync)");

      // --- All async work happens after skeleton is visible ---
      resolveAndFill(cardEl, placeholderButtons, activityFooter);
    }

    async function resolveAndFill(cardEl, placeholderButtons, activityFooter) {
      try {
        // --- Step 1: Determine agent ID (prefer sync strategies) ---
        let agentId = getAgentIdFromFiber(cardEl);
        let idSource = "fiber-card";

        if (!agentId) {
          const prev = cardEl.previousElementSibling;
          if (prev) {
            agentId = getAgentIdFromFiber(prev);
            if (agentId) idSource = "fiber-sibling";
          }
        }

        if (!agentId && cardEl.parentElement) {
          agentId = getAgentIdFromFiber(cardEl.parentElement);
          if (agentId) idSource = "fiber-parent";
        }

        // Strategy 2: Hover-captured agent ID (sync check with cached agents)
        if (!agentId) {
          const hoverId = getHoveredAgentId();
          if (hoverId) {
            const agents = cachedAgents.length ? cachedAgents : [];
            let mismatch = false;
            if (agents.length) {
              const cardText = (cardEl.textContent || "").trim();
              for (const other of agents) {
                if (other.id === hoverId) continue;
                const odn = other.displayName || "";
                const on = other.name || "";
                if ((odn && cardText.indexOf(odn) !== -1) || (on && cardText.indexOf(on) !== -1)) {
                  mismatch = true;
                  break;
                }
              }
            }
            if (!mismatch) {
              agentId = hoverId;
              idSource = "hover-capture";
            }
          }
        }

        // Strategy 3: Name-match with cached agents (sync if cache warm)
        if (!agentId && cachedAgents.length) {
          agentId = getAgentIdByName(cardEl, cachedAgents);
          if (agentId) idSource = "name-match-cached";
        }

        // Strategy 4: Async fallback — fetch agents then name-match
        if (!agentId) {
          const agents = await getAgents();
          if (!document.body.contains(cardEl)) return;
          agentId = getAgentIdByName(cardEl, agents);
          if (agentId) idSource = "name-match";
        }

        if (!agentId) {
          console.warn("[Slock Desktop] agent card: could not determine agent ID, removing skeleton");
          if (document.body.contains(placeholderButtons)) placeholderButtons.remove();
          if (document.body.contains(activityFooter)) activityFooter.remove();
          cardEl.removeAttribute(INJECT_ATTR);
          return;
        }
        console.log("[Slock Desktop] agent card: found agent ID", agentId, "via", idSource);

        if (!document.body.contains(cardEl)) return;

        // --- Step 2: Get agent data and replace placeholder buttons ---
        let agents = cachedAgents.length ? cachedAgents : await getAgents();
        if (!document.body.contains(cardEl)) return;

        const agent = agents.find((a) => a.id === agentId);
        if (!agent) {
          console.warn("[Slock Desktop] agent card: agent not found", agentId);
          if (document.body.contains(placeholderButtons)) placeholderButtons.remove();
          if (document.body.contains(activityFooter)) activityFooter.remove();
          cardEl.removeAttribute(INJECT_ATTR);
          return;
        }

        console.log("[Slock Desktop] agent card: filling content for", agent.name);
        const serverSlug = getCurrentServerSlug();

        // Replace placeholder buttons with real action buttons
        const realButtons = buildActionButtons(agent, serverSlug);
        if (document.body.contains(placeholderButtons)) {
          placeholderButtons.replaceWith(realButtons);
        } else {
          // Placeholder gone (card restructured?), insert before activity
          if (document.body.contains(activityFooter)) {
            cardEl.insertBefore(realButtons, activityFooter);
          } else {
            cardEl.appendChild(realButtons);
          }
        }

        // --- Step 3: Fill activity (from cache or fetch) ---
        var cached = activityCache[agentId];
        if (cached && (Date.now() - cached.fetchedAt) < ACTIVITY_CACHE_TTL) {
          if (document.body.contains(activityFooter)) {
            fillActivityContent(activityFooter, cached.data);
          }
          cardEl.setAttribute(INJECT_ATTR, "done");
          return;
        }

        const activity = await getAgentActivity(agentId);
        activityCache[agentId] = { data: activity, fetchedAt: Date.now() };
        if (document.body.contains(cardEl) && document.body.contains(activityFooter)) {
          fillActivityContent(activityFooter, activity);
        }
        cardEl.setAttribute(INJECT_ATTR, "done");
      } catch (e) {
        console.warn("[Slock Desktop] agent card: resolveAndFill failed", e);
        if (document.body.contains(activityFooter)) {
          fillActivityContent(activityFooter, []);
        }
        cardEl.setAttribute(INJECT_ATTR, "done");
      }
    }

    // --- MutationObserver ---
    if (window.__slockDesktopAgentCardObserver) {
      window.__slockDesktopAgentCardObserver.disconnect();
    }

    window.__slockDesktopAgentCardObserver = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        for (const node of mutation.addedNodes) {
          if (!(node instanceof HTMLElement)) continue;
          // Check the node itself
          if (isAgentCard(node)) {
            console.log("[Slock Desktop] agent card: detected (direct)", node.className);
            void injectAgentCard(node);
            continue;
          }
          // Search descendants for agent cards (handles portal wrappers)
          const cards = node.querySelectorAll?.(".card-brutal");
          if (cards) {
            for (const el of cards) {
              if (el instanceof HTMLElement && isAgentCard(el)) {
                console.log("[Slock Desktop] agent card: detected (nested)", el.className);
                void injectAgentCard(el);
              }
            }
          }
        }
      }
    });

    window.__slockDesktopAgentCardObserver.observe(document.body, {
      childList: true,
      subtree: true,
    });

    // Pre-warm agents cache in background so first hover has data ready
    void getAgents();
  } catch (error) {
    console.warn("[Slock Desktop] agent card inject setup failed", error);
  }
})();
"##;

#[cfg(test)]
mod tests {
    use super::agent_card_inject_script;

    #[test]
    fn script_guards_workspace_origin() {
        let script = agent_card_inject_script("test-server", "en-US");
        assert!(script.contains(r#"window.location.origin !== "https://app.slock.ai""#));
    }

    #[test]
    fn script_injects_server_slug() {
        let script = agent_card_inject_script("my-server", "en-US");
        assert!(script.contains(r#""my-server""#));
    }

    #[test]
    fn script_injects_locale() {
        let script = agent_card_inject_script("test", "zh-CN");
        assert!(script.contains(r#""zh-CN""#));
    }

    #[test]
    fn script_uses_mutation_observer() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains("MutationObserver"));
        assert!(script.contains("card-brutal"));
    }

    #[test]
    fn script_calls_tauri_invoke() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains("fetch_dashboard"));
        assert!(script.contains("fetch_agent_activity"));
    }

    #[test]
    fn script_includes_chinese_translations() {
        let script = agent_card_inject_script("test", "zh-CN");
        assert!(script.contains("暂无描述"));
        assert!(script.contains("最近活动"));
        assert!(script.contains("停止"));
        assert!(script.contains("启动"));
    }

    #[test]
    fn script_detects_agent_cards_by_class_and_fiber() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains("card-brutal"));
        assert!(script.contains("getAgentIdFromFiber"));
    }

    #[test]
    fn script_observes_subtree() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains("subtree: true"));
    }

    #[test]
    fn script_uses_react_fiber_for_agent_id() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains("__reactFiber$"));
        assert!(script.contains("member-agent"));
    }

    #[test]
    fn script_has_hover_capture_fallback() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains("captureAgentFromTrigger"));
        assert!(script.contains("pointerover"));
        assert!(script.contains("lastHoveredAgentId"));
        assert!(script.contains("getHoveredAgentId"));
        // Verify listener cleanup on re-eval
        assert!(script.contains("__slockDesktopAgentCardCaptureCleanup"));
        assert!(script.contains("removeEventListener"));
        // Verify stale hover ID cleared when no agent found
        assert!(script.contains("lastHoveredAgentId = null"));
        // Verify hover ID verified against other agent names before use
        assert!(script.contains("mismatch"));
    }

    #[test]
    fn script_has_name_based_fallback() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains("getAgentIdByName"));
        assert!(script.contains("name-match"));
    }

    #[test]
    fn script_searches_child_and_sibling_fibers() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains("f.child"));
        assert!(script.contains("f.sibling"));
        assert!(script.contains("fiber.alternate"));
    }

    #[test]
    fn script_has_broad_agent_type_matching() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains("extractAgentId"));
        assert!(script.contains("UUID_RE"));
        assert!(script.contains("indexOf(\"agent\")"));
    }

    #[test]
    fn script_has_action_buttons() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains("buildActionButtons"));
        assert!(script.contains("stop_agent"));
        assert!(script.contains("start_agent"));
        assert!(script.contains("SVG_STOP"));
        assert!(script.contains("SVG_START"));
        assert!(script.contains("SVG_RESTART"));
        assert!(script.contains("aria-label"));
        // Restart SVG + translation key kept for future re-add (native flow)
        assert!(script.contains("restart"));
    }

    #[test]
    fn script_has_activity_two_phase() {
        let script = agent_card_inject_script("test", "en-US");
        // Phase 2: skeleton injected synchronously
        assert!(script.contains("buildActivitySkeleton"));
        assert!(script.contains("data-activity-placeholder"));
        // Phase 3: async fill replaces skeleton content
        assert!(script.contains("fillActivityContent"));
        // Activity cache (30s TTL)
        assert!(script.contains("activityCache"));
        assert!(script.contains("ACTIVITY_CACHE_TTL"));
        // Fallback chain for field names
        assert!(script.contains("getEntryTime"));
        // unwrapEntry for nested API response
        assert!(script.contains("unwrapEntry"));
        // Activity sorted by timestamp descending (newest first)
        assert!(script.contains(".sort("));
        // Kind-based mapping to label/detail/dotColor
        assert!(script.contains("mapActivityKind"));
        assert!(script.contains("dotColor"));
        // HH:MM:SS time format
        assert!(script.contains("formatTimeHMS"));
        assert!(script.contains("padStart"));
        // Activity kind labels
        assert!(script.contains("actIdle"));
        assert!(script.contains("actWorking"));
        assert!(script.contains("actThinking"));
        assert!(script.contains("actDisconnected"));
        assert!(script.contains("actRunningCommand"));
        assert!(script.contains("actSendingMessage"));
        assert!(script.contains("actOutput"));
    }

    #[test]
    fn script_limits_injection_to_popover() {
        let script = agent_card_inject_script("test", "en-US");
        // Positive allowlist: must require overflow-hidden
        assert!(script.contains("overflow-hidden"));
        assert!(script.contains("missing overflow-hidden"));
        // Must reject native page layout classes
        assert!(script.contains("w-full"));
        assert!(script.contains("max-w-md"));
        assert!(script.contains("p-6"));
        // Must reject dialogs/modals
        assert!(script.contains("role"));
        assert!(script.contains("aria-modal"));
        // Must reject Radix dialog content
        assert!(script.contains("data-radix-dialog-content"));
        // Must have skip diagnostic log
        assert!(script.contains("card-brutal skipped"));
        // Must log ancestors for diagnostics
        assert!(script.contains("ancestors:"));
    }

    #[test]
    fn script_has_true_sync_first_skeleton() {
        let script = agent_card_inject_script("test", "en-US");
        // Placeholder buttons rendered before any async work
        assert!(script.contains("buildPlaceholderButtons"));
        assert!(script.contains("data-buttons-placeholder"));
        // Async resolution in separate function
        assert!(script.contains("resolveAndFill"));
        // Placeholder replaced with real buttons after agent data available
        assert!(script.contains("replaceWith"));
    }
}
