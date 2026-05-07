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
        return await invoke("fetch_agent_activity", {
          serverSlug: slug,
          agentId: agentId,
        });
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

    function getAgentIdFromFiber(cardEl) {
      try {
        let fiber = getReactFiber(cardEl);
        for (let depth = 0; depth < 40 && fiber; depth++) {
          // Walk memoizedState linked list
          let state = fiber.memoizedState;
          for (let s = 0; s < 20 && state; s++) {
            const ms = state.memoizedState;
            if (ms && typeof ms === "object" && !Array.isArray(ms)) {
              if (ms.type === "member-agent" && ms.id) return ms.id;
            }
            // Check queue.lastRenderedState
            if (state.queue && state.queue.lastRenderedState) {
              const lrs = state.queue.lastRenderedState;
              if (
                lrs &&
                typeof lrs === "object" &&
                lrs.type === "member-agent" &&
                lrs.id
              )
                return lrs.id;
            }
            state = state.next;
          }
          // Check pendingProps
          if (fiber.pendingProps) {
            const p = fiber.pendingProps;
            if (p.type === "member-agent" && p.id) return p.id;
            if (p.popover && p.popover.type === "member-agent" && p.popover.id)
              return p.popover.id;
          }
          fiber = fiber.return;
        }
      } catch (e) {
        console.warn("[Slock Desktop] agent card: fiber walk failed", e);
      }
      return null;
    }

    // --- Detect agent card ---
    function isAgentCard(el) {
      if (!el || !el.classList) return false;
      if (!el.classList.contains("card-brutal")) return false;
      const buttons = el.querySelectorAll("button");
      for (const btn of buttons) {
        const text = (btn.textContent || "").trim().toLowerCase();
        if (text === "stop" || text === "start" || text === "reset") return true;
      }
      return false;
    }

    // --- Build injected content ---
    function buildInjectedHeader(agent, activity) {
      const container = document.createElement("div");
      container.setAttribute(INJECT_ATTR, "true");
      container.style.cssText =
        "padding: 8px 12px 4px; border-bottom: 1px solid rgba(0,0,0,0.08);";

      // Name + status row
      const header = document.createElement("div");
      header.style.cssText =
        "display: flex; align-items: center; gap: 6px; margin-bottom: 4px;";

      const dot = document.createElement("span");
      const isOnline = agent.status !== "offline";
      dot.style.cssText =
        "width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; background: " +
        (isOnline ? "#22c55e" : "#9ca3af") +
        ";";

      const name = document.createElement("span");
      name.style.cssText = "font-weight: 700; font-size: 13px; flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;";
      name.textContent = agent.displayName || agent.name;

      const status = document.createElement("span");
      status.style.cssText =
        "font-size: 11px; color: #6b7280; flex-shrink: 0;";
      status.textContent = isOnline ? t("online") : t("offline");

      header.appendChild(dot);
      header.appendChild(name);
      header.appendChild(status);
      container.appendChild(header);

      // Description
      if (agent.description) {
        const desc = document.createElement("p");
        desc.style.cssText =
          "margin: 0 0 6px; font-size: 12px; color: #6b7280; line-height: 1.4; word-break: break-word;";
        desc.textContent = agent.description;
        container.appendChild(desc);
      }

      // Activity
      if (activity && activity.length > 0) {
        const actTitle = document.createElement("p");
        actTitle.style.cssText =
          "margin: 0 0 3px; font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; color: #9ca3af;";
        actTitle.textContent = t("activity");
        container.appendChild(actTitle);

        const list = document.createElement("ul");
        list.style.cssText =
          "margin: 0 0 4px; padding: 0; list-style: none;";

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
          "margin: 2px 0 4px; font-size: 11px; color: #9ca3af;";
        noAct.textContent = t("noActivity");
        container.appendChild(noAct);
      }

      return container;
    }

    // --- Injection logic ---
    async function injectAgentCard(cardEl) {
      if (cardEl.hasAttribute(INJECT_ATTR)) return;
      cardEl.setAttribute(INJECT_ATTR, "pending");

      // Get agent ID from React fiber
      let agentId = getAgentIdFromFiber(cardEl);

      // Also try walking from the backdrop (sibling element)
      if (!agentId) {
        const prev = cardEl.previousElementSibling;
        if (prev) agentId = getAgentIdFromFiber(prev);
      }

      if (!agentId) {
        console.warn("[Slock Desktop] agent card: could not determine agent ID");
        cardEl.removeAttribute(INJECT_ATTR);
        return;
      }

      // Fetch agent info and activity in parallel
      const [agents, activity] = await Promise.all([
        getAgents(),
        getAgentActivity(agentId),
      ]);

      // Check card is still in DOM
      if (!document.body.contains(cardEl)) return;

      const agent = agents.find((a) => a.id === agentId);
      if (!agent) {
        cardEl.removeAttribute(INJECT_ATTR);
        return;
      }

      const header = buildInjectedHeader(agent, activity);
      cardEl.insertBefore(header, cardEl.firstChild);
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
          // Direct child of body — portal-rendered card
          if (isAgentCard(node)) {
            void injectAgentCard(node);
            return;
          }
          // The card might be inside a wrapper div
          const card = node.querySelector?.(".card-brutal");
          if (card && isAgentCard(card)) {
            void injectAgentCard(card);
            return;
          }
        }
      }
    });

    window.__slockDesktopAgentCardObserver.observe(document.body, {
      childList: true,
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
    }

    #[test]
    fn script_detects_agent_cards_by_buttons() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains(r#"text === "stop""#));
        assert!(script.contains(r#"text === "start""#));
        assert!(script.contains(r#"text === "reset""#));
    }

    #[test]
    fn script_uses_react_fiber_for_agent_id() {
        let script = agent_card_inject_script("test", "en-US");
        assert!(script.contains("__reactFiber$"));
        assert!(script.contains("member-agent"));
    }
}
