pub fn agent_env_import_script(resolved_language: &str) -> String {
    let resolved_language =
        serde_json::to_string(resolved_language).unwrap_or_else(|_| "\"en-US\"".into());
    AGENT_ENV_IMPORT_SCRIPT.replace("__SLOCK_DESKTOP_ENV_IMPORT_LOCALE__", &resolved_language)
}

const AGENT_ENV_IMPORT_SCRIPT: &str = r##"
(function () {
  try {
    if (window.location.origin !== "https://app.slock.ai") return;

    const BUTTON_ID = "slock-desktop-env-import-btn";
    const PANEL_ID = "slock-desktop-env-import-panel";
    const ADD_VARIABLE_TEXT = "Add Variable";

    const TRANSLATIONS = {
      "en-US": {
        importButton: "Import JSON",
        placeholder:
          'Paste a JSON object, e.g. {"ANTHROPIC_BASE_URL":"https://...","ANTHROPIC_MODEL":"..."}\nNested wrappers like {"env":{...}} are also accepted.\nOr KEY=VALUE per line.',
        replace: "Replace existing rows",
        cancel: "Cancel",
        import: "Import",
        importing: "Importing…",
        errEmpty: "Paste JSON or KEY=VALUE lines first.",
        errJson: "Invalid JSON: ",
        errTopObject: "JSON must be a top-level object.",
        errNoKeys: "No environment keys found in JSON.",
        errNoPairs: "No KEY=VALUE pairs detected.",
        errObjectValue: 'Value for "%k" must be a string, not an object or array.',
        errAddBtn: "Could not find the Add Variable control.",
        errContainer: "Could not locate the environment variables container.",
      },
      "zh-CN": {
        importButton: "导入 JSON",
        placeholder:
          '粘贴 JSON 对象，例如 {"ANTHROPIC_BASE_URL":"https://...","ANTHROPIC_MODEL":"..."}\n也支持嵌套结构，如 {"env":{...}}。\n或每行一个 KEY=VALUE。',
        replace: "替换现有变量",
        cancel: "取消",
        import: "导入",
        importing: "导入中…",
        errEmpty: "请先粘贴 JSON 或 KEY=VALUE。",
        errJson: "JSON 格式错误：",
        errTopObject: "JSON 顶层必须是对象。",
        errNoKeys: "JSON 中未找到环境变量键。",
        errNoPairs: "未识别到 KEY=VALUE 行。",
        errObjectValue: '键 "%k" 的值必须是字符串，不能是对象或数组。',
        errAddBtn: "找不到“添加变量”按钮。",
        errContainer: "找不到环境变量容器。",
      },
    };

    function tFor(locale) {
      return TRANSLATIONS[locale] || TRANSLATIONS["en-US"];
    }

    function currentLocale() {
      const value = window.__slockDesktopEnvImportLocale;
      return typeof value === "string" && TRANSLATIONS[value] ? value : "en-US";
    }

    window.__slockDesktopEnvImportLocale = __SLOCK_DESKTOP_ENV_IMPORT_LOCALE__;

    if (window.__slockDesktopEnvImportBound) {
      if (typeof window.__slockDesktopEnvImportRefresh === "function") {
        try {
          window.__slockDesktopEnvImportRefresh();
        } catch (error) {
          console.warn("[Slock Desktop] env import refresh failed", error);
        }
      }
      return;
    }
    window.__slockDesktopEnvImportBound = true;

    const ENV_WRAPPER_KEYS = [
      "env",
      "envVars",
      "env_vars",
      "environment",
      "variables",
      "vars",
    ];

    function findAddVariableButton() {
      const buttons = document.querySelectorAll('button[type="button"]');
      for (const btn of buttons) {
        if ((btn.textContent || "").trim() === ADD_VARIABLE_TEXT) return btn;
      }
      return null;
    }

    function setReactInputValue(input, value) {
      const proto =
        input.tagName === "TEXTAREA"
          ? HTMLTextAreaElement.prototype
          : HTMLInputElement.prototype;
      const setter = Object.getOwnPropertyDescriptor(proto, "value").set;
      setter.call(input, value);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    }

    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

    function listEnvRows(container, addBtn) {
      return Array.from(container.children).filter(
        (el) =>
          el !== addBtn &&
          el.querySelector &&
          el.querySelector('input[placeholder="KEY"]')
      );
    }

    async function applyEntries(entries, replace, t) {
      const addBtn = findAddVariableButton();
      if (!addBtn) throw new Error(t.errAddBtn);
      const container = addBtn.parentElement;
      if (!container) throw new Error(t.errContainer);

      if (replace) {
        const rows = listEnvRows(container, addBtn);
        for (const row of rows) {
          const delBtn = row.querySelector('button[type="button"]');
          if (delBtn) {
            delBtn.click();
            await sleep(0);
          }
        }
      }

      for (const [key, value] of entries) {
        addBtn.click();
        await sleep(16);
        const rows = listEnvRows(container, addBtn);
        const lastRow = rows[rows.length - 1];
        if (!lastRow) continue;
        const keyInput = lastRow.querySelector('input[placeholder="KEY"]');
        const valInput = lastRow.querySelector('input[placeholder="value"]');
        if (keyInput) setReactInputValue(keyInput, key);
        if (valInput) setReactInputValue(valInput, value);
      }
    }

    function unwrapEnvObject(parsed) {
      for (const key of ENV_WRAPPER_KEYS) {
        const value = parsed[key];
        if (value && typeof value === "object" && !Array.isArray(value)) {
          return value;
        }
      }
      return parsed;
    }

    function flattenEnvObject(obj, t) {
      const out = [];
      for (const [rawKey, rawValue] of Object.entries(obj)) {
        const key = String(rawKey).trim();
        if (!key) continue;
        if (rawValue == null) {
          out.push([key, ""]);
          continue;
        }
        if (typeof rawValue === "object") {
          throw new Error(t.errObjectValue.replace("%k", key));
        }
        out.push([key, String(rawValue)]);
      }
      return out;
    }

    function parseInput(text, t) {
      const trimmed = (text || "").trim();
      if (!trimmed) throw new Error(t.errEmpty);

      if (trimmed.startsWith("{")) {
        let parsed;
        try {
          parsed = JSON.parse(trimmed);
        } catch (err) {
          throw new Error(t.errJson + (err && err.message ? err.message : String(err)));
        }
        if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
          throw new Error(t.errTopObject);
        }
        const envObject = unwrapEnvObject(parsed);
        const entries = flattenEnvObject(envObject, t);
        if (entries.length === 0) throw new Error(t.errNoKeys);
        return entries;
      }

      const out = [];
      for (const raw of trimmed.split("\n")) {
        let line = raw.trim();
        if (!line || line.startsWith("#")) continue;
        if (line.startsWith("export ")) line = line.slice(7).trim();
        const eq = line.indexOf("=");
        if (eq < 0) continue;
        const key = line.slice(0, eq).trim();
        let value = line.slice(eq + 1).trim();
        if (
          (value.startsWith('"') && value.endsWith('"')) ||
          (value.startsWith("'") && value.endsWith("'"))
        ) {
          value = value.slice(1, -1);
        }
        if (key) out.push([key, value]);
      }
      if (out.length === 0) throw new Error(t.errNoPairs);
      return out;
    }

    function buildPanel() {
      const panel = document.createElement("div");
      panel.id = PANEL_ID;
      panel.style.display = "none";
      panel.style.marginTop = "12px";
      panel.className = "border-2 border-black bg-white p-3";

      const textarea = document.createElement("textarea");
      textarea.dataset.role = "text";
      textarea.spellcheck = false;
      textarea.rows = 6;
      textarea.className = "input-brutal w-full text-sm";
      textarea.style.fontFamily =
        "ui-monospace,SFMono-Regular,Menlo,Consolas,monospace";
      textarea.style.lineHeight = "1.4";
      textarea.style.resize = "vertical";
      textarea.style.minHeight = "120px";

      const replaceLabel = document.createElement("label");
      replaceLabel.className =
        "mt-2 flex items-center gap-2 text-sm font-bold text-black cursor-pointer select-none";
      const replaceCheckbox = document.createElement("input");
      replaceCheckbox.type = "checkbox";
      replaceCheckbox.dataset.role = "replace";
      replaceCheckbox.checked = true;
      const replaceText = document.createElement("span");
      replaceText.dataset.role = "replace-label";
      replaceLabel.appendChild(replaceCheckbox);
      replaceLabel.appendChild(replaceText);

      const errorEl = document.createElement("p");
      errorEl.dataset.role = "error";
      errorEl.style.display = "none";
      errorEl.className =
        "mt-2 border-2 border-black bg-brutal-orange/20 p-2 text-sm font-bold text-black";

      const actions = document.createElement("div");
      actions.className = "mt-3 flex justify-end gap-2";

      const cancelBtn = document.createElement("button");
      cancelBtn.type = "button";
      cancelBtn.dataset.role = "cancel";
      cancelBtn.className = "btn-brutal bg-white px-4 py-2 text-sm";

      const importBtn = document.createElement("button");
      importBtn.type = "button";
      importBtn.dataset.role = "import";
      importBtn.className = "btn-brutal bg-brutal-pink px-4 py-2 text-sm";

      actions.appendChild(cancelBtn);
      actions.appendChild(importBtn);

      panel.appendChild(textarea);
      panel.appendChild(replaceLabel);
      panel.appendChild(errorEl);
      panel.appendChild(actions);

      const showError = (msg) => {
        errorEl.textContent = msg;
        errorEl.style.display = "block";
      };
      const clearError = () => {
        errorEl.textContent = "";
        errorEl.style.display = "none";
      };

      cancelBtn.addEventListener("click", () => {
        panel.style.display = "none";
        clearError();
      });

      importBtn.addEventListener("click", async () => {
        const t = tFor(currentLocale());
        clearError();
        let entries;
        try {
          entries = parseInput(textarea.value, t);
        } catch (err) {
          showError(err && err.message ? err.message : String(err));
          return;
        }
        importBtn.disabled = true;
        importBtn.textContent = t.importing;
        try {
          await applyEntries(entries, !!replaceCheckbox.checked, t);
          textarea.value = "";
          panel.style.display = "none";
        } catch (err) {
          showError(err && err.message ? err.message : String(err));
        } finally {
          importBtn.disabled = false;
          importBtn.textContent = t.import;
        }
      });

      return panel;
    }

    function renderImportButtonContent(button) {
      const t = tFor(currentLocale());
      button.innerHTML =
        '<svg aria-hidden="true" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>';
      const label = document.createElement("span");
      label.textContent = t.importButton;
      button.appendChild(label);
    }

    function renderPanelLabels(panel) {
      const t = tFor(currentLocale());
      const textarea = panel.querySelector('[data-role="text"]');
      const replaceLabel = panel.querySelector('[data-role="replace-label"]');
      const cancelBtn = panel.querySelector('[data-role="cancel"]');
      const importBtn = panel.querySelector('[data-role="import"]');
      if (textarea) textarea.placeholder = t.placeholder;
      if (replaceLabel) replaceLabel.textContent = t.replace;
      if (cancelBtn) cancelBtn.textContent = t.cancel;
      if (importBtn && !importBtn.disabled) importBtn.textContent = t.import;
    }

    function injectImportButton() {
      const addBtn = findAddVariableButton();
      if (!addBtn) return;
      if (document.getElementById(BUTTON_ID)) return;
      const container = addBtn.parentElement;
      if (!container) return;

      const importBtn = document.createElement("button");
      importBtn.type = "button";
      importBtn.id = BUTTON_ID;
      importBtn.className = addBtn.className;
      importBtn.style.marginLeft = "12px";
      renderImportButtonContent(importBtn);

      addBtn.insertAdjacentElement("afterend", importBtn);

      const panel = buildPanel();
      importBtn.insertAdjacentElement("afterend", panel);
      renderPanelLabels(panel);

      importBtn.addEventListener("click", () => {
        const visible = panel.style.display !== "none";
        panel.style.display = visible ? "none" : "block";
        if (!visible) {
          const ta = panel.querySelector('[data-role="text"]');
          if (ta) ta.focus();
        }
      });
    }

    function refreshLocaleSensitiveDom() {
      const button = document.getElementById(BUTTON_ID);
      if (button) renderImportButtonContent(button);
      const panel = document.getElementById(PANEL_ID);
      if (panel) renderPanelLabels(panel);
    }
    window.__slockDesktopEnvImportRefresh = refreshLocaleSensitiveDom;

    const observer = new MutationObserver(() => {
      injectImportButton();
    });
    observer.observe(document.body, { childList: true, subtree: true });
    injectImportButton();
  } catch (error) {
    console.warn("[Slock Desktop] env import setup failed", error);
  }
})();
"##;

#[cfg(test)]
mod tests {
    use super::agent_env_import_script;

    #[test]
    fn script_guards_workspace_origin() {
        let script = agent_env_import_script("en-US");
        assert!(script.contains(r#"window.location.origin !== "https://app.slock.ai""#));
    }

    #[test]
    fn script_targets_add_variable_button() {
        let script = agent_env_import_script("en-US");
        assert!(script.contains("Add Variable"));
    }

    #[test]
    fn script_uses_react_aware_value_setter() {
        let script = agent_env_import_script("en-US");
        assert!(script.contains("HTMLInputElement.prototype"));
        assert!(script.contains(r#"new Event("input""#));
    }

    #[test]
    fn script_supports_json_and_dotenv_inputs() {
        let script = agent_env_import_script("en-US");
        assert!(script.contains("JSON.parse"));
        assert!(script.contains("KEY=VALUE"));
    }

    #[test]
    fn script_locates_rows_by_placeholder() {
        let script = agent_env_import_script("en-US");
        assert!(script.contains(r#"input[placeholder="KEY"]"#));
        assert!(script.contains(r#"input[placeholder="value"]"#));
    }

    #[test]
    fn script_unwraps_nested_env_wrappers() {
        let script = agent_env_import_script("en-US");
        assert!(script.contains("ENV_WRAPPER_KEYS"));
        for key in ["\"env\"", "\"envVars\"", "\"environment\"", "\"variables\""] {
            assert!(
                script.contains(key),
                "expected wrapper key {key} to be present"
            );
        }
    }

    #[test]
    fn script_uses_brutal_design_classes() {
        let script = agent_env_import_script("en-US");
        assert!(script.contains("input-brutal"));
        assert!(script.contains("btn-brutal"));
        assert!(script.contains("bg-brutal-pink"));
        assert!(script.contains("border-2 border-black"));
    }

    #[test]
    fn script_includes_chinese_translations() {
        let script = agent_env_import_script("zh-CN");
        assert!(script.contains("\"zh-CN\""));
        assert!(script.contains("导入 JSON"));
        assert!(script.contains("替换现有变量"));
    }

    #[test]
    fn script_is_locale_seeded_via_placeholder() {
        let script = agent_env_import_script("zh-CN");
        assert!(script.contains(r#"window.__slockDesktopEnvImportLocale = "zh-CN""#));
        let other = agent_env_import_script("en-US");
        assert!(other.contains(r#"window.__slockDesktopEnvImportLocale = "en-US""#));
    }

    #[test]
    fn script_supports_reactive_locale_refresh() {
        let script = agent_env_import_script("en-US");
        assert!(script.contains("__slockDesktopEnvImportRefresh"));
        assert!(script.contains("__slockDesktopEnvImportBound"));
    }

    #[test]
    fn script_rejects_object_values() {
        let script = agent_env_import_script("en-US");
        assert!(script.contains("errObjectValue"));
    }
}
