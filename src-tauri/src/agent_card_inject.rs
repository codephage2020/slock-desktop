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
        restart: "Restart",
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
        restart: "重启",
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

    // --- Detect agent card ---
    function isAgentCard(el) {
      if (!el || !el.classList) return false;
      // Detect by card-brutal class (web app's card component)
      return el.classList.contains("card-brutal");
    }

    // --- Build injected action buttons (compact icon group in card header) ---
    // SVG icons for actions (20px display, 24px viewBox)
    var SVG_STOP = '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="6" width="12" height="12" rx="2"/></svg>';
    var SVG_START = '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><polygon points="8,5 20,12 8,19"/></svg>';
    var SVG_RESTART = '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M21 2v6h-6"/><path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M3 22v-6h6"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/></svg>';

    function buildActionButtons(agent, serverSlug) {
      const container = document.createElement("div");
      container.setAttribute(INJECT_ATTR, "true");
      container.style.cssText =
        "position: absolute; top: 8px; right: 8px; display: flex; gap: 4px; z-index: 10;";

      const isOnline = agent.status !== "offline";

      function makeIconBtn(svgHtml, titleText, color) {
        var btn = document.createElement("button");
        btn.style.cssText =
          "display: inline-flex; align-items: center; justify-content: center; width: 20px; height: 20px; border-radius: 6px; border: none; background: transparent; cursor: pointer; padding: 0; color: " + color + "; transition: background 0.15s;";
        btn.innerHTML = svgHtml;
        btn.title = titleText;
        btn.setAttribute("aria-label", titleText);
        btn.addEventListener("mouseenter", function () { if (!btn.disabled) btn.style.background = "rgba(0,0,0,0.06)"; });
        btn.addEventListener("mouseleave", function () { btn.style.background = "transparent"; });
        return btn;
      }

      // Start/Stop button
      var toggleBtn = makeIconBtn(
        isOnline ? SVG_STOP : SVG_START,
        isOnline ? t("stop") : t("start"),
        isOnline ? "#dc2626" : "#16a34a"
      );
      toggleBtn.addEventListener("click", async function (e) {
        e.stopPropagation();
        toggleBtn.disabled = true;
        toggleBtn.style.opacity = "0.3";
        toggleBtn.style.cursor = "default";
        try {
          await invoke(isOnline ? "stop_agent" : "start_agent", {
            serverSlug: serverSlug,
            agentId: agent.id,
          });
          lastFetchMs = 0;
        } catch (err) {
          console.warn("[Slock Desktop] agent card: action failed", err);
        }
        toggleBtn.disabled = false;
        toggleBtn.style.opacity = "1";
        toggleBtn.style.cursor = "pointer";
      });

      // Restart button
      var restartBtn = makeIconBtn(
        SVG_RESTART,
        t("restart"),
        "#2563eb"
      );
      if (!isOnline) {
        restartBtn.disabled = true;
        restartBtn.style.opacity = "0.3";
        restartBtn.style.cursor = "default";
      }
      restartBtn.addEventListener("click", async function (e) {
        e.stopPropagation();
        if (!isOnline) return;
        restartBtn.disabled = true;
        restartBtn.style.opacity = "0.3";
        restartBtn.style.cursor = "default";
        try {
          await invoke("stop_agent", { serverSlug: serverSlug, agentId: agent.id });
          await new Promise(function (r) { setTimeout(r, 1000); });
          await invoke("start_agent", { serverSlug: serverSlug, agentId: agent.id });
          lastFetchMs = 0;
        } catch (err) {
          console.warn("[Slock Desktop] agent card: restart failed", err);
        }
        restartBtn.disabled = false;
        restartBtn.style.opacity = "1";
        restartBtn.style.cursor = "pointer";
      });

      container.appendChild(toggleBtn);
      container.appendChild(restartBtn);
      return container;
    }

    // --- Build injected activity footer (bottom of card) ---
    function buildActivityFooter(activity) {
      const container = document.createElement("div");
      container.setAttribute(INJECT_ATTR, "true");
      container.style.cssText =
        "padding: 8px 12px; border-top: 1px solid rgba(0,0,0,0.06); margin-top: 6px;";

      if (activity && activity.length > 0) {
        const actTitle = document.createElement("p");
        actTitle.style.cssText =
          "margin: 0 0 3px; font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; color: #9ca3af;";
        actTitle.textContent = t("activity");
        container.appendChild(actTitle);

        const list = document.createElement("ul");
        list.style.cssText =
          "margin: 0; padding: 0; list-style: none;";

        const maxItems = Math.min(activity.length, 5);
        for (let i = 0; i < maxItems; i++) {
          const entry = activity[i];
          const li = document.createElement("li");
          li.style.cssText =
            "display: flex; justify-content: space-between; gap: 6px; font-size: 11px; line-height: 1.5;";

          const text = document.createElement("span");
          text.style.cssText =
            "flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #374151;";
          text.textContent = entry.activity;

          const time = document.createElement("span");
          time.style.cssText = "flex-shrink: 0; color: #9ca3af; font-size: 10px;";
          time.textContent = formatRelativeTime(entry.createdAt);

          li.appendChild(text);
          li.appendChild(time);
          list.appendChild(li);
        }
        container.appendChild(list);
      } else {
        const noAct = document.createElement("p");
        noAct.style.cssText =
          "margin: 0; font-size: 11px; color: #9ca3af;";
        noAct.textContent = t("noActivity");
        container.appendChild(noAct);
      }

      return container;
    }

    // --- Injection logic ---
    async function injectAgentCard(cardEl) {
      if (cardEl.hasAttribute(INJECT_ATTR)) return;
      cardEl.setAttribute(INJECT_ATTR, "pending");
      console.log("[Slock Desktop] agent card: injection started", cardEl.className);

      // Strategy 1: Get agent ID from React fiber (card + siblings + parent)
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

      // Strategy 2: Use hover-captured agent ID (from pointerover/click on avatar)
      // Trust the capture if TTL is valid, but reject if card text contains
      // a DIFFERENT known agent's name (cross-card mismatch protection).
      // Note: original card may not contain the agent's name at all (e.g. only
      // action buttons), so we can't require name presence — only reject on
      // explicit mismatch with another agent.
      if (!agentId) {
        const hoverId = getHoveredAgentId();
        if (hoverId) {
          const agents = await getAgents();
          const hoverAgent = agents.find((a) => a.id === hoverId);
          if (hoverAgent) {
            const cardText = (cardEl.textContent || "").trim();
            // Check if card text contains a DIFFERENT agent's name
            let mismatch = false;
            for (const other of agents) {
              if (other.id === hoverId) continue;
              const odn = other.displayName || "";
              const on = other.name || "";
              if ((odn && cardText.indexOf(odn) !== -1) || (on && cardText.indexOf(on) !== -1)) {
                mismatch = true;
                console.log("[Slock Desktop] agent card: hover ID rejected (card matches different agent)", hoverId, other.id);
                break;
              }
            }
            if (!mismatch) {
              agentId = hoverId;
              idSource = "hover-capture";
            }
          }
        }
      }

      // Strategy 3: Name-based fallback — match card text against known agents
      if (!agentId) {
        const agents = await getAgents();
        agentId = getAgentIdByName(cardEl, agents);
        if (agentId) idSource = "name-match";
      }

      if (!agentId) {
        console.warn("[Slock Desktop] agent card: could not determine agent ID (all strategies failed)");
        // Log card info for debugging
        try {
          console.log("[Slock Desktop] agent card debug: textContent =", (cardEl.textContent || "").substring(0, 100));
          console.log("[Slock Desktop] agent card debug: innerHTML length =", (cardEl.innerHTML || "").length);
          console.log("[Slock Desktop] agent card debug: children =", cardEl.children.length);
        } catch (_) {}
        cardEl.removeAttribute(INJECT_ATTR);
        return;
      }
      console.log("[Slock Desktop] agent card: found agent ID", agentId, "via", idSource);

      // Fetch agent info first (populates cached_servers), then activity
      const agents = await getAgents();
      console.log("[Slock Desktop] agent card: fetched", agents.length, "agents");
      const activity = await getAgentActivity(agentId);

      // Check card is still in DOM
      if (!document.body.contains(cardEl)) {
        console.log("[Slock Desktop] agent card: card removed from DOM during fetch");
        return;
      }

      const agent = agents.find((a) => a.id === agentId);
      if (!agent) {
        console.warn("[Slock Desktop] agent card: agent not found in dashboard data", agentId);
        cardEl.removeAttribute(INJECT_ATTR);
        return;
      }

      console.log("[Slock Desktop] agent card: injecting content for", agent.name);
      const serverSlug = getCurrentServerSlug();

      // Ensure card has position context for absolute-positioned buttons
      const cardPosition = window.getComputedStyle(cardEl).position;
      if (cardPosition === "static") {
        cardEl.style.position = "relative";
      }

      // Insert action buttons (absolute positioned top-right)
      const buttons = buildActionButtons(agent, serverSlug);
      cardEl.appendChild(buttons);

      // Append activity footer at bottom of card
      const footer = buildActivityFooter(activity);
      cardEl.appendChild(footer);

      cardEl.setAttribute(INJECT_ATTR, "done");

      // Widen card to fit content
      cardEl.style.width = "auto";
      cardEl.style.minWidth = "220px";
      cardEl.style.maxWidth = "320px";
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
        assert!(script.contains("重启"));
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
        assert!(script.contains("hover ID rejected (card matches different agent)"));
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
        assert!(script.contains("SVG_RESTART"));
        assert!(script.contains("SVG_STOP"));
        assert!(script.contains("SVG_START"));
        assert!(script.contains("aria-label"));
    }

    #[test]
    fn script_has_activity_footer() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains("buildActivityFooter"));
        assert!(script.contains("appendChild(footer)"));
    }
}
