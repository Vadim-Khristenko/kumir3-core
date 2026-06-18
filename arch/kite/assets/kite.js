/* =============================================================================
 * KITE :: PROGRESSIVE ENHANCEMENT + THEME SYSTEM
 * =============================================================================
 * Modern ES6+ implementation.
 * Adds: heading IDs, auto TOC, smooth scroll, scrollspy, in-page search,
 * card filtering, keyboard accessibility, and floating theme switcher.
 * ========================================================================= */
(() => {
  "use strict";

  // --- HELPERS ---
  const slug = (text) =>
    text
      .toLowerCase()
      .trim()
      .replace(/[^\p{L}\p{N}]+/gu, "-")
      .replace(/^-+|-+$/g, "");

  const escapeRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

  const debounce = (fn, ms) => {
    let t;
    return (...args) => {
      clearTimeout(t);
      t = setTimeout(() => fn(...args), ms);
    };
  };

  const el = (tag, cls, html) => {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (html != null) e.innerHTML = html;
    return e;
  };

  // --- SEARCH HIGHLIGHTING ---
  const clearHighlights = (root) => {
    root.querySelectorAll("mark.kite-hit").forEach((m) => {
      m.replaceWith(document.createTextNode(m.textContent));
    });
    root.normalize();
  };

  const highlight = (root, query) => {
    if (!query) return;
    const rx = new RegExp(`(${escapeRe(query)})`, "gi");
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
      acceptNode: (n) => {
        if (!n.nodeValue.trim()) return NodeFilter.FILTER_REJECT;
        const p = n.parentNode;
        if (p && /^(MARK|SCRIPT|STYLE)$/.test(p.nodeName))
          return NodeFilter.FILTER_REJECT;
        return rx.test(n.nodeValue)
          ? NodeFilter.FILTER_ACCEPT
          : NodeFilter.FILTER_REJECT;
      },
    });

    const targets = [];
    let n;
    while ((n = walker.nextNode())) targets.push(n);

    targets.forEach((node) => {
      const frag = document.createDocumentFragment();
      const s = node.nodeValue;
      let last = 0;
      let m;
      rx.lastIndex = 0;

      while ((m = rx.exec(s))) {
        if (m.index > last)
          frag.appendChild(document.createTextNode(s.slice(last, m.index)));
        const mark = el("mark", "kite-hit");
        mark.textContent = m[0];
        frag.appendChild(mark);
        last = m.index + m[0].length;
        if (m.index === rx.lastIndex) rx.lastIndex++;
      }
      if (last < s.length)
        frag.appendChild(document.createTextNode(s.slice(last)));
      node.parentNode.replaceChild(frag, node);
    });
  };

  const makeSearch = (placeholder, onInput) => {
    const box = el("div", "kite-search");
    const input = el("input");
    input.type = "search";
    input.setAttribute("aria-label", placeholder);
    input.placeholder = placeholder;
    box.appendChild(input);
    input.addEventListener(
      "input",
      debounce(() => onInput(input.value.trim()), 150),
    );
    return box;
  };

  // --- SMOOTH SCROLL ---
  const enableSmoothScroll = () => {
    document.querySelectorAll('a[href^="#"]').forEach((anchor) => {
      anchor.addEventListener("click", function (e) {
        const targetId = this.getAttribute("href").slice(1);
        const targetEl = document.getElementById(targetId);
        if (targetEl) {
          e.preventDefault();
          targetEl.scrollIntoView({ behavior: "smooth", block: "start" });
          history.pushState(null, null, `#${targetId}`);
        }
      });
    });
  };

  // --- DOC ENHANCEMENT ---
  const enhanceDoc = (article) => {
    const sections = Array.from(article.querySelectorAll(".kite-section"));
    const headings = article.querySelectorAll(
      ".kite-section h2, .kite-section h3",
    );

    headings.forEach((h) => {
      if (!h.id)
        h.id =
          slug(h.textContent) || `s-${Math.random().toString(36).slice(2, 7)}`;
      const sec = h.closest(".kite-section");
      if (sec && !sec.id) sec.id = h.id;
    });

    const toc = article.querySelector("nav.kite-toc");
    let empty = null;

    if (toc) {
      if (toc.hasAttribute("data-auto")) {
        const ol = el("ol");
        article.querySelectorAll(".kite-section h2").forEach((h) => {
          const li = el("li");
          const a = el("a");
          a.href = `#${h.id}`;
          a.textContent = h.textContent;
          li.appendChild(a);
          ol.appendChild(li);
        });
        const oldList = toc.querySelector("ol, ul");
        oldList ? oldList.replaceWith(ol) : toc.appendChild(ol);
      }

      const search = makeSearch("Поиск по документу…", (q) => {
        clearHighlights(article);
        let anyVisible = false;
        sections.forEach((sec) => {
          if (sec.contains(toc)) return;
          const hit =
            !q || sec.textContent.toLowerCase().includes(q.toLowerCase());
          sec.classList.toggle("is-hidden", !hit);
          if (hit) {
            anyVisible = true;
            if (q) highlight(sec, q);
          }
        });
        if (empty) empty.classList.toggle("is-shown", !!q && !anyVisible);
      });

      const heading = toc.querySelector("h2");
      heading ? heading.after(search) : toc.prepend(search);

      empty = el("p", "kite-empty", "Ничего не найдено.");
      toc.after(empty);

      // Scrollspy
      const links = {};
      toc.querySelectorAll('a[href^="#"]').forEach((a) => {
        links[a.getAttribute("href").slice(1)] = a;
      });

      const spy = new IntersectionObserver(
        (entries) => {
          // Find the topmost intersecting entry
          const visible = entries.filter((e) => e.isIntersecting);
          if (visible.length && links[visible[0].target.id]) {
            Object.values(links).forEach((a) =>
              a.classList.remove("is-active"),
            );
            links[visible[0].target.id].classList.add("is-active");
          }
        },
        { rootMargin: "-10% 0px -70% 0px", threshold: 0 },
      );

      article
        .querySelectorAll(".kite-section[id]")
        .forEach((s) => spy.observe(s));
    }
  };

  // --- INDEX ENHANCEMENT ---
  const enhanceIndex = (container) => {
    const cards = Array.from(container.querySelectorAll(".kite-card"));
    const empty = el("p", "kite-empty", "Ничего не найдено.");
    const search = makeSearch("Поиск по KITE…", (q) => {
      let any = false;
      cards.forEach((c) => {
        const hit = !q || c.textContent.toLowerCase().includes(q.toLowerCase());
        c.classList.toggle("is-hidden", !hit);
        if (hit) any = true;
      });
      empty.classList.toggle("is-shown", !!q && !any);
    });
    container.before(search);
    container.after(empty);
  };

  // --- THEME SYSTEM ---
  const THEMES = [
    { id: "original",     label: "KITE Original",       accent: "#bf4e28", path: null },
    { id: "material1",    label: "Material Design 1",    accent: "#FF7043", path: "themes/kite-material1.css" },
    { id: "material2",    label: "Material Design 2",    accent: "#FFAB91", path: "themes/kite-material2.css" },
    { id: "material3",    label: "Material You (M3)",    accent: "#ffb59f", path: "themes/kite-material3.css" },
    { id: "shadcn",       label: "shadcn/ui",            accent: "#E4E4E7", path: "themes/kite-shadcn.css" },
    { id: "tokyo-night",  label: "Tokyo Night",          accent: "#7aa2f7", path: "themes/kite-tokyo-night.css" },
  ];

  const THEME_KEY = "kite-theme";
  const CUSTOM_URL_KEY = "kite-theme-custom-url";
  const LINK_ID = "kite-theme-override";

  const getAssetBase = () => {
    const link = document.querySelector('link[href*="kite.css"]');
    return link
      ? link.href.slice(0, link.href.lastIndexOf("/") + 1)
      : "./assets/";
  };

  const loadThemeHref = (href) => {
    let existing = document.getElementById(LINK_ID);
    if (!href) {
      if (existing) existing.remove();
      return;
    }
    if (!existing) {
      existing = document.createElement("link");
      existing.rel = "stylesheet";
      existing.id = LINK_ID;
      const after = document.querySelector('link[href*="kite.css"]');
      after && after.parentNode
        ? after.parentNode.insertBefore(existing, after.nextSibling)
        : document.head.appendChild(existing);
    }
    existing.href = href;
  };

  const applyTheme = (themeId, customUrl) => {
    const theme = THEMES.find((t) => t.id === themeId);
    const bar = document.getElementById("kite-theme-bar");

    if (bar) {
      bar.querySelectorAll(".kite-theme-swatch").forEach((sw) => {
        sw.classList.toggle("is-active", sw.dataset.themeId === themeId);
      });
      const customBtn = bar.querySelector(".kite-theme-custom-btn");
      if (customBtn)
        customBtn.classList.toggle("is-active", themeId === "custom");
    }

    if (themeId === "custom" && customUrl) {
      loadThemeHref(customUrl);
      try {
        localStorage.setItem(THEME_KEY, "custom");
        localStorage.setItem(CUSTOM_URL_KEY, customUrl);
      } catch (e) {}
    } else if (theme) {
      const href = theme.path ? getAssetBase() + theme.path : null;
      loadThemeHref(href);
      try {
        localStorage.setItem(THEME_KEY, themeId);
        localStorage.removeItem(CUSTOM_URL_KEY);
      } catch (e) {}
    }
  };

  const initThemes = () => {
    if (!document.querySelector("body > .kite")) return;

    const bar = el("div", "kite-theme-bar");
    bar.id = "kite-theme-bar";
    bar.setAttribute("aria-label", "Выбор темы");

    bar.appendChild(el("span", "kite-theme-label", "Тема"));
    bar.appendChild(el("span", "kite-theme-sep"));

    const swatchRow = el("div", "kite-theme-swatches");
    THEMES.forEach((theme) => {
      const btn = el("button", "kite-theme-swatch");
      btn.dataset.themeId = theme.id;
      btn.title = theme.label;
      btn.style.backgroundColor = theme.accent;
      btn.style.color = theme.accent;
      btn.setAttribute("aria-label", theme.label);
      btn.addEventListener("click", () => {
        panel.classList.remove("is-open");
        applyTheme(theme.id, null);
      });
      swatchRow.appendChild(btn);
    });
    bar.appendChild(swatchRow);
    bar.appendChild(el("span", "kite-theme-sep"));

    const customBtn = el("button", "kite-theme-custom-btn", "+");
    customBtn.title = "Своя тема (CSS URL)";
    customBtn.setAttribute("aria-label", "Загрузить свою тему");

    const panel = el("div", "kite-theme-panel");
    panel.appendChild(el("span", "kite-theme-panel-label", "URL своей темы"));

    const urlInput = el("input");
    urlInput.type = "url";
    urlInput.placeholder = "https://example.com/my-theme.css";
    try {
      urlInput.value = localStorage.getItem(CUSTOM_URL_KEY) || "";
    } catch (e) {}

    const actions = el("div", "kite-theme-panel-actions");
    const applyBtn = el("button", "kite-theme-panel-apply", "Применить");
    const clearBtn = el("button", "kite-theme-panel-clear", "Сбросить");

    applyBtn.addEventListener("click", () => {
      const url = urlInput.value.trim();
      if (url) {
        applyTheme("custom", url);
        panel.classList.remove("is-open");
      }
    });

    clearBtn.addEventListener("click", () => {
      urlInput.value = "";
      applyTheme("original", null);
      panel.classList.remove("is-open");
    });

    actions.append(applyBtn, clearBtn);
    panel.append(urlInput, actions);

    customBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      panel.classList.toggle("is-open");
    });

    // A11y & UX: Close panel on outside click or Escape key
    document.addEventListener("click", (e) => {
      if (!bar.contains(e.target) && !panel.contains(e.target)) {
        panel.classList.remove("is-open");
      }
    });
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape") panel.classList.remove("is-open");
    });

    bar.appendChild(customBtn);
    document.body.append(bar, panel);

    // Initial Load
    let savedId = "original";
    let savedUrl = null;
    try {
      savedId = localStorage.getItem(THEME_KEY) || "original";
      savedUrl = localStorage.getItem(CUSTOM_URL_KEY);
    } catch (e) {}

    applyTheme(savedId, savedUrl);
  };

  // --- BOOT ---
  const init = () => {
    document.querySelectorAll("article.kite").forEach(enhanceDoc);
    document.querySelectorAll("[data-kite-index]").forEach(enhanceIndex);
    enableSmoothScroll();
    initThemes();
  };

  document.readyState === "loading"
    ? document.addEventListener("DOMContentLoaded", init)
    : init();
})();
