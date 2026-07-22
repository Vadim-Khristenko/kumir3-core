#!/usr/bin/env bun
/**
 * Сборка статического сайта KITE из документов `arch/kite/`.
 *
 * Документы KITE — семантические HTML-фрагменты: они начинаются сразу с
 * `<link rel="stylesheet">` и `<article class="kite">`, чтобы их можно было
 * встраивать в чужую страницу. Браузер такой фрагмент показывает, но без
 * `<head>` у него нет ни charset, ни заголовка вкладки, ни описания для
 * поисковиков и мессенджеров.
 *
 * Скрипт оборачивает каждый фрагмент в полноценный документ, не трогая
 * исходники: фрагменты остаются фрагментами, сайт собирается отдельно.
 *
 * Запуск:  bun tools/kite-site.ts [--out <каталог>] [--base <префикс URL>]
 */

import { mkdir, readdir, rm, cp } from "node:fs/promises";
import { join, basename } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const SRC = join(ROOT, "arch", "kite");

const args = Bun.argv.slice(2);
const argOf = (name: string, fallback: string) => {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1]! : fallback;
};

const OUT = argOf("--out", join(ROOT, "target", "kite-site"));
/**
 * Префикс URL сайта — нужен только для абсолютных ссылок в sitemap.xml.
 * Имя хоста приводится к нижнему регистру: в CI оно подставляется из имени
 * владельца репозитория, а тот пишется с заглавных букв.
 */
const BASE = argOf("--base", "")
  .replace(/\/$/, "")
  .replace(/^(https?:\/\/)([^/]+)/i, (_m, scheme: string, host: string) => scheme + host.toLowerCase());

/** Иконка вкладки: воздушный змей (kite) в фирменном цвете, без внешних файлов. */
const FAVICON =
  "data:image/svg+xml," +
  encodeURIComponent(
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">' +
      '<path d="M16 2 4 14l12 12 12-12z" fill="#7aa2f7"/>' +
      '<path d="M16 2v24M4 14h24" stroke="#1a1b26" stroke-width="1.5"/>' +
      '<path d="M16 26l-3 4 3-1 3 1z" fill="#f7768e"/>' +
      "</svg>",
  );

const THEME_COLOR = "#1a1b26";
const decodeEntities = (s: string) =>
  s
    .replace(/&nbsp;/g, " ")
    .replace(/&laquo;/g, "«")
    .replace(/&raquo;/g, "»")
    .replace(/&mdash;/g, "—")
    .replace(/&ndash;/g, "–")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&amp;/g, "&");

const stripTags = (s: string) => decodeEntities(s.replace(/<[^>]*>/g, "")).replace(/\s+/g, " ").trim();
const escapeAttr = (s: string) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** Заголовок вкладки: `<h1>` документа плюс номер KITE, если он есть. */
function titleOf(html: string, file: string): string {
  const h1 = html.match(/<h1[^>]*>([\s\S]*?)<\/h1>/i);
  const heading = h1 ? stripTags(h1[1]!) : basename(file, ".html");
  const num = html.match(/data-kite="(\d+)"/);
  return num ? `KITE ${Number(num[1])} — ${heading}` : heading;
}

/** Описание для поисковиков: первый содержательный абзац документа. */
function descriptionOf(html: string): string {
  for (const m of html.matchAll(/<p[^>]*>([\s\S]*?)<\/p>/gi)) {
    const text = stripTags(m[1]!);
    if (text.length >= 40) return text.length > 300 ? `${text.slice(0, 297)}…` : text;
  }
  return "Стандарт языка программирования Kumir 3.";
}

/**
 * Дополняет `<head>` уже готовой страницы тем, чего в ней нет: иконкой,
 * описанием и цветом темы. Существующие теги не трогаются.
 */
function augmentHead(html: string): string {
  const additions: string[] = [];
  if (!/<link[^>]+rel="icon"/i.test(html)) additions.push(`<link rel="icon" href="${FAVICON}">`);
  if (!/<meta[^>]+name="theme-color"/i.test(html)) {
    additions.push(`<meta name="color-scheme" content="dark light">`);
    additions.push(`<meta name="theme-color" content="${THEME_COLOR}">`);
  }
  if (!/<meta[^>]+name="description"/i.test(html)) {
    const description = descriptionOf(html);
    additions.push(`<meta name="description" content="${escapeAttr(description)}">`);
    const title = html.match(/<title>([\s\S]*?)<\/title>/i);
    if (title) {
      additions.push(`<meta property="og:title" content="${escapeAttr(stripTags(title[1]!))}">`);
      additions.push(`<meta property="og:description" content="${escapeAttr(description)}">`);
    }
  }
  return additions.length ? html.replace(/<\/head>/i, `${additions.join("\n")}\n</head>`) : html;
}

/** Оборачивает фрагмент в полноценный документ; готовую страницу дополняет. */
function wrap(html: string, file: string): string {
  if (/^\s*<!DOCTYPE/i.test(html)) return augmentHead(html);

  const title = titleOf(html, file);
  const description = descriptionOf(html);
  // Ссылка на стиль переезжает в <head>: в теле она валидна, но задерживает отрисовку.
  const body = html.replace(/^\s*<link rel="stylesheet"[^>]*>\s*/i, "");

  return `<!DOCTYPE html>
<html lang="ru">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark light">
<meta name="theme-color" content="${THEME_COLOR}">
<title>${escapeAttr(title)}</title>
<meta name="description" content="${escapeAttr(description)}">
<meta property="og:type" content="article">
<meta property="og:title" content="${escapeAttr(title)}">
<meta property="og:description" content="${escapeAttr(description)}">
<link rel="icon" href="${FAVICON}">
<link rel="stylesheet" href="./assets/kite.css">
</head>
<body>
${body.trimStart()}
</body>
</html>
`;
}

/** Страница 404 в оформлении KITE — GitHub Pages отдаёт её для неизвестных путей. */
function notFoundPage(): string {
  return wrap(
    `<article class="kite" lang="ru">
  <header class="kite-header">
    <span class="kite-eyebrow">KITE · 404</span>
    <h1>Страница не найдена</h1>
    <p>Такого документа в серии KITE нет. Возможно, он ещё не написан или переехал —
       полный реестр всегда доступен на странице-оглавлении.</p>
  </header>
  <section class="kite-section">
    <p><a href="./index.html">Вернуться к реестру документов</a></p>
  </section>
</article>
<script src="./assets/kite.js" defer></script>
`,
    "404.html",
  );
}

await rm(OUT, { recursive: true, force: true });
await mkdir(OUT, { recursive: true });

const files = (await readdir(SRC)).filter((f) => f.endsWith(".html")).sort();
for (const file of files) {
  const html = await Bun.file(join(SRC, file)).text();
  await Bun.write(join(OUT, file), wrap(html, file));
}

await cp(join(SRC, "assets"), join(OUT, "assets"), { recursive: true });
await Bun.write(join(OUT, "404.html"), notFoundPage());
// .nojekyll: без него GitHub Pages прогоняет сайт через Jekyll и прячет файлы,
// начинающиеся с подчёркивания.
await Bun.write(join(OUT, ".nojekyll"), "");

if (BASE) {
  const urls = files
    .map((f) => `  <url><loc>${BASE}/${f === "index.html" ? "" : f}</loc></url>`)
    .join("\n");
  await Bun.write(
    join(OUT, "sitemap.xml"),
    `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}\n</urlset>\n`,
  );
  await Bun.write(join(OUT, "robots.txt"), `Sitemap: ${BASE}/sitemap.xml\n`);
}

console.log(`KITE: собрано ${files.length} документов → ${OUT}`);
