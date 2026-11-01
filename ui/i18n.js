// StackerPulse — UI translation (RU → EN). Author: UBro-dev | License: MIT
//
// The markup is written in Russian. When English is selected, every text node,
// placeholder and title whose text matches a key below is swapped for its
// English value, and swapped back when Russian is selected again. Text that
// the app renders later is caught by a MutationObserver. Strings built with
// numbers or names are translated in app.js with L(ru, en) instead.
//
// Keys are the Russian text with whitespace collapsed to single spaces.

const RU_EN = {
  // Header and tabs
  "StackerPulse ⚡ — Командный центр и аналитика для Stacker News": "StackerPulse ⚡ — Command Center & Analytics for Stacker News",
  "Privacy-by-Design · Airgap: ВЫКЛ": "Privacy-by-Design · Airgap: OFF",
  "Tor: ВЫКЛ (Clearnet)": "Tor: OFF (Clearnet)",
  "Преломление:": "Refraction:",
  "Аналитика, охотник за баунти, экономика территорий и студия постов для Stacker News": "Analytics, bounty hunter, territory economics and post studio for Stacker News",
  "Радар & Закладки": "Radar & Bookmarks",
  "Охотник за Баунти": "Bounty Hunter",
  "Экономика Территорий & ROI": "Territory Economics & ROI",
  "Студия Постов & Очистка Ссылок": "Post Studio & Link Sanitizer",
  "Стакер-Аналитика & CSV": "Stacker Analytics & CSV",
  "GraphQL Песочница": "GraphQL Playground",
  "Rust Ядро (Win & Linux)": "Rust Core (Win & Linux)",
  "Готово": "Ready",
  "Разделы": "Sections",

  // Radar
  "Суммарный стак в выборке": "Total stacked in selection",
  "Средний Signal Score": "Average Signal Score",
  "Sats / коммент": "Sats / comment",
  "Отношение сатоши к шуму": "Sats-to-noise ratio",
  "Лучшая территория": "Top territory",
  "🔥 Лидер": "🔥 Leader",
  "Максимальная плотность сатоши": "Highest sats density",
  "Умные закладки": "Smart bookmarks",
  "🔖 Локальная база": "🔖 Local store",
  "С тегами, заметками и сортировкой": "With tags, notes and sorting",
  "🔥 Умная лента (24)": "🔥 Smart Feed (24)",
  "🔖 Менеджер закладок (": "🔖 Bookmark Manager (",
  "📥 Экспорт выборки (CSV)": "📥 Export selection (CSV)",
  "Поиск по заголовку / @nym / домену": "Search title / @nym / domain",
  "Территория (~sub)": "Territory (~sub)",
  "Все территории": "All territories",
  "Тип поста": "Post type",
  "Все типы": "All types",
  "🔗 Ссылка (Link)": "🔗 Link",
  "💬 Дискуссия": "💬 Discussion",
  "🎯 Баунти": "🎯 Bounty",
  "📊 Опрос (Poll)": "📊 Poll",
  "Сортировка": "Sort",
  "⚡ Больше всего сатоши": "⚡ Most sats",
  "💎 Высший Signal Score (Sats/Comment)": "💎 Highest Signal Score (Sats/Comment)",
  "🚀 Скорость стака (Sats/час)": "🚀 Stacking speed (Sats/hour)",
  "💬 Самые обсуждаемые": "💬 Most discussed",
  "Мин. порог:": "Min. threshold:",
  "Скрыть 0-sat / freebie": "Hide 0-sat / freebie",
  "🔖 Менеджер умных закладок (решение запроса GitHub Discussions: Reorder & Tag Bookmarks)": "🔖 Smart Bookmark Manager (answers the GitHub Discussions request: Reorder & Tag Bookmarks)",
  "На Stacker News нельзя менять порядок закладок, добавлять к ним личные заметки или теги. Здесь вы можете упорядочивать их кнопками ↑/↓ и экспортировать в JSON/CSV.": "Stacker News does not let you reorder bookmarks or add private notes and tags. Here you can order them with ↑/↓ and export to JSON/CSV.",
  "Все теги": "All tags",
  "💾 Экспорт закладок (JSON)": "💾 Export bookmarks (JSON)",
  "Напр. lightning, @nym, github...": "E.g. lightning, @nym, github...",

  // Bounties
  "🎯 Единый радар заработка сатоши (Community Bounties + GitHub PR Bounties)": "🎯 One radar for earning sats (Community Bounties + GitHub PR Bounties)",
  "Охотник за Баунти Stacker News": "Stacker News Bounty Hunter",
  "На Stacker News нет единой панели для отслеживания открытых баунти по всем территориям (": "Stacker News has no single place to track open bounties across all territories (",
  ") и официальных выплат за PR в репозиторий": ") and official payouts for PRs to the repository",
  "(от 20k до 1M sats). Фильтруйте задачи по статусу, сложности и ставке сатоши в час.": "(20k to 1M sats). Filter tasks by status, difficulty and sats per hour.",
  "Открытый пул баунти": "Open bounty pool",
  "Категория баунти": "Bounty category",
  "Все источники (Посты SN + GitHub PR)": "All sources (SN posts + GitHub PRs)",
  "⚡ Посты-баунти в сообществе SN": "⚡ Community bounty posts on SN",
  "🐙 Официальные PR-баунти репозитория SN": "🐙 Official SN repository PR bounties",
  "Статус выплаты": "Payout status",
  "🟢 Только открытые (Unpaid)": "🟢 Open only (Unpaid)",
  "Все (Открытые + Выплаченные)": "All (Open + Paid)",
  "✅ Выплаченные (Paid архив)": "✅ Paid (archive)",
  "Сложность": "Difficulty",
  "Любая сложность": "Any difficulty",
  "💰 По размеру награды (убыв.)": "💰 By reward (desc.)",
  "⚡ По выгодности (Sats / час оценки)": "⚡ By value (Sats / estimated hour)",
  "Фильтр ~территории...": "Filter ~territory...",

  // Territories
  "🧮 Калькулятор окупаемости территории": "🧮 Territory payback calculator",
  "Расчёт дохода основателя (~sub founder) по правилам экономики Stacker News": "Founder (~sub founder) income under Stacker News economics",
  "Тариф аренды территории (SN Billing)": "Territory billing plan (SN Billing)",
  "100,000 sats / месяц (Monthly)": "100,000 sats / month (Monthly)",
  "1,000,000 sats / год (≈ 83,333 sats/мес)": "1,000,000 sats / year (≈ 83,333 sats/mo)",
  "3,000,000 sats навсегда (Perpetual / Once)": "3,000,000 sats forever (Perpetual / Once)",
  "Цена поста (sats):": "Post cost (sats):",
  "Постов в день:": "Posts per day:",
  "Комментов в день (1 sat):": "Comments per day (1 sat):",
  "Суммарно zaps/день:": "Total zaps/day:",
  "Boost в день (sats):": "Boost per day (sats):",
  "Доля основателя:": "Founder share:",
  "Валовая выручка территории в месяц:": "Territory gross revenue per month:",
  "Выплата основателю (CST EOD):": "Founder payout (CST EOD):",
  "Отчисления в Daily Rewards Pool:": "Contribution to Daily Rewards Pool:",
  "Чистая прибыль / мес (Net ROI):": "Net profit / mo (Net ROI):",
  "Окупаемость: 0 дн.": "Payback: 0 days",
  "🗺️ Рейтинг территорий по мультипликатору стака (Sats / Post Cost)": "🗺️ Territories ranked by stacking multiplier (Sats / Post Cost)",
  "Сравнение стоимости входа, медианного стака и отдачи на 1 вложенный сатоши": "Entry cost, median stack and return per sat spent",
  "Территория": "Territory",
  "Основатель": "Founder",
  "Цена поста": "Post cost",
  "Медианный стак": "Median stack",
  "Мультипликатор": "Multiplier",
  "Лучшее окно (UTC)": "Best window (UTC)",
  "Нишевые технические территории (": "Niche technical territories (",
  ") дают в 2.4 раза более высокий мультипликатор на 1 вложенный сатоши благодаря низкой стоимости поста (10–50 sats) и целевым заперам.": ") give a 2.4× higher multiplier per sat spent thanks to low post costs (10–50 sats) and targeted zappers.",

  // Post studio
  "🛡️ Санитайзер ссылок & Анти-Дюп (Решение Issue #2664)": "🛡️ Link Sanitizer & Anti-Dupe (Issue #2664)",
  "Удаляет трекеры (": "Removes trackers (",
  ") и конвертирует": ") and converts",
  "в приватные зеркала (": "to private frontends (",
  "Только удалить UTM/трекеры": "Only remove UTM/trackers",
  "✨ Очистить URL": "✨ Sanitize URL",
  "✅ Дубликатов в ленте не найдено": "✅ No duplicates in the feed",
  "Копировать": "Copy",
  "Вставьте ссылку для очистки и проверки...": "Paste a link to clean and check...",
  "Свой инстанс Nitter/Redlib (необяз.)": "Custom Nitter/Redlib instance (optional)",
  "Публичные инстансы Nitter и Redlib часто отключаются. Укажите голое имя хоста, например nitter.example.org": "Public Nitter and Redlib instances often go offline. Enter a bare hostname, e.g. nitter.example.org",
  "✍️ Конструктор поста с авто-проверкой Оглавления (Auto-TOC)": "✍️ Post builder with Auto-TOC check",
  "Заголовок: 56 симв. (Оптимально)": "Title: 56 chars (Optimal)",
  "Заголовок поста (рекомендуется 40–80 символов без кликбейта)": "Post title (40–80 characters recommended, no clickbait)",
  "Целевая территория": "Target territory",
  "Текст поста (Markdown — используйте 2+ подзаголовка ##, чтобы Stacker News автоматически сгенерировал оглавление TOC)": "Post body (Markdown — use 2+ ## headings so Stacker News generates a table of contents)",
  "Boost (продвижение, кратно 1000 sats)": "Boost (promotion, multiples of 1000 sats)",
  "Запланировать слот (Discussion #3014)": "Schedule a slot (Discussion #3014)",
  "💾 В очередь": "💾 Add to queue",
  "📋 Копировать MD": "📋 Copy MD",
  "🔍 Аудит поста & Авто-Оглавление (SN Guide Check)": "🔍 Post audit & Auto-TOC (SN Guide Check)",
  "Итого: 50 sats": "Total: 50 sats",
  "🧼 OpSec Scrubber: Удалить пути ОС (/home/user, C:\\Users\\...) и скрыть секреты в черновике": "🧼 OpSec Scrubber: remove OS paths (/home/user, C:\\Users\\...) and hide secrets in the draft",
  "⏰ Тепловая карта активности заперов (UTC / Austin CST)": "⏰ Zapper activity heatmap (UTC / Austin CST)",
  "Пик: 13:00–17:00 UTC": "Peak: 13:00–17:00 UTC",
  "По данным обсуждения #3014 (Scheduled Posts), посты в ночные часы США получают на 68% меньше сатоши. Планируйте выпуск на утренний прайм-тайм Austin CST (08:00–12:00 CST / 13:00–17:00 UTC).": "According to discussion #3014 (Scheduled Posts), posts during US night hours get 68% fewer sats. Plan releases for the Austin CST morning prime time (08:00–12:00 CST / 13:00–17:00 UTC).",
  "Низкая": "Low",
  "ЕС утро": "EU morning",
  "Средняя": "Medium",
  "🔥 ПИК": "🔥 PEAK",
  "Высокая": "High",
  "Спад": "Decline",
  "📅 Очередь запланированных черновиков (": "📅 Scheduled draft queue (",
  "📆 Скачать .ICS напоминание": "📆 Download .ICS reminder",

  // Analytics
  "📊 Глубокая аналитика стакера, паттерны чаевых (Tipping Patterns) и экспорт CSV": "📊 In-depth stacker analytics, tipping patterns and CSV export",
  "Реализует запросы сообщества: выгрузка аналитики в CSV, медианный/средний зап и симулятор доли в ежедневном пуле наград (": "Covers community requests: analytics export to CSV, median/average zap and a daily rewards pool share simulator (",
  "📥 Скачать аналитику стакера (CSV)": "📥 Download stacker analytics (CSV)",
  "Всего собрано (Stacked)": "Total stacked",
  "Средний / Медианный стак на пост": "Average / median stack per post",
  "Запрос сообщества в ~meta": "Community request in ~meta",
  "Индекс кураторства (Early Zapper)": "Curation index (Early Zapper)",
  "Влияние на Daily Rewards": "Impact on Daily Rewards",
  "Любимые территории": "Favorite territories",
  "CC Donate: Включено": "CC Donate: On",
  "Динамика стака по месяцам (Посты vs Комментарии vs Rewards)": "Monthly stacking trend (Posts vs Comments vs Rewards)",
  "Распределение полученных сатоши по территориям": "Sats received by territory",
  "Авто-донат в Rewards включён": "Auto-donate to Rewards: on",
  "Принимает только Lightning Sats": "Accepts Lightning sats only",

  // GraphQL
  "⚡ Песочница публичного GraphQL API Stacker News (": "⚡ Stacker News public GraphQL API playground (",
  "Готовые шаблоны запросов для разработчиков ботов, виджетов, Nostr-мостов и мобильных клиентов. Запрос к": "Ready-made queries for developers of bots, widgets, Nostr bridges and mobile clients. Requests to",
  "идёт только через Rust-клиент (allowlist, Tor по желанию).": "go only through the Rust client (allowlist, optional Tor).",
  "🔥 Топ постов главной ленты (items)": "🔥 Top posts on the front page (items)",
  "💻 Посты территории ~devs": "💻 Posts in ~devs",
  "👤 Профиль стакера (@k00b)": "👤 Stacker profile (@k00b)",
  "🗺️ Список активных территорий (subs)": "🗺️ Active territories (subs)",
  "▶ Выполнить запрос": "▶ Run Query",
  "Копировать JSON": "Copy JSON",

  // SeedScan
  "Поиск утечек Seed-фраз на диске и защита от деанонимизации (SeedScan + OpSec Guard)": "Finding seed phrase leaks on disk and preventing deanonymization (SeedScan + OpSec Guard)",
  "Люди записывают сид-фразу в": "People write their seed phrase into",
  "«на минутку» и забывают её в": "\"just for a minute\" and forget it in",
  "или случайно вставляют": "or accidentally paste",
  "/ путь с именем пользователя ОС в лог на Stacker News.": "/ a path with their OS username into a log on Stacker News.",
  "SeedScan от UBro-dev": "SeedScan by UBro-dev",
  "проверяет контрольные суммы (BIP39, Base58Check, Bech32) и маскирует секреты (": "validates checksums (BIP39, Base58Check, Bech32) and masks secrets (",
  "🔍 Сканировать папки ПК (SeedScan)": "🔍 Scan PC folders (SeedScan)",
  "🔒 1. Никогда не трогает сеть": "🔒 1. Never touches the network",
  "Сканер работает на 100% локально. Детекторы": "The scanner runs 100% locally. The detectors",
  "и": "and",
  "не открывают сокетов и ничего не отправляют.": "open no sockets and send nothing.",
  "📖 2. Строго Read-Only": "📖 2. Strictly read-only",
  "Никогда не изменяет, не перемещает и не удаляет ваши файлы. Вы сами решаете, что делать с обнаруженной утечкой.": "Never changes, moves or deletes your files. What to do about a finding is your decision.",
  "🎭 3. Маскирование и валидация чек-сумм": "🎭 3. Masking and checksum validation",
  "Обычный текст из 12 английских слов игнорируется — срабатывает только валидная контрольная сумма, а вывод всегда скрыт звездочками.": "Ordinary text of 12 English words is ignored — only a valid checksum triggers a finding, and output is always masked with asterisks.",
  "📂 Сканер папок на диске (Windows & Linux)": "📂 Disk folder scanner (Windows & Linux)",
  "Готов (Offline)": "Ready (Offline)",
  "Целевые директории": "Target folders",
  "Лимит файла (МБ)": "File size limit (MB)",
  "Включать скрытые файлы (": "Include hidden files (",
  "Пропуск: node_modules, target, .git, бинарники": "Skipped: node_modules, target, .git, binaries",
  "🧪 Инспектор секретов, NWC и деанонимизации": "🧪 Secrets, NWC and deanonymization inspector",
  "Проверьте любой конфиг, лог или инвойс на утечку сид-фраз, ключей и метаданных ОС": "Check any config, log or invoice for leaked seed phrases, keys and OS metadata",
  "Тест-вектор BIP39/Nostr": "BIP39/Nostr test vector",
  "Проверить": "Check",
  "Вставьте текст, лог или конфиг для локальной проверки...": "Paste text, a log or a config to check locally...",
  "Укажите папки и нажмите «Сканировать».": "Choose folders and press Scan.",

  // Rust core
  "🦀 Нативное кроссплатформенное ядро Rust 2021 Edition · Tauri v2 · Tokio Async": "🦀 Native cross-platform core · Rust 2021 Edition · Tauri v2 · Tokio Async",
  "Архитектура Rust-приложения для Windows и Linux": "Rust app architecture for Windows and Linux",
  "Весь сетевой стек GraphQL (": "The whole GraphQL network stack (",
  "), очистка ссылок (": "), link sanitizing (",
  "crate) и математическая модель территорий выделены в нативные модули Rust. На": "crate) and the territory model live in native Rust modules. On",
  "используется системный композитор": "it uses the system compositor",
  "и установщики": "and installers",
  ", а на": ", and on",
  "— прозрачный композитинг": "— transparent compositing",
  "с пакетами": "with packages",
  "🧹 Сбросить локальный кэш (Zero-Trace)": "🧹 Wipe Local Cache (Zero-Trace)",
  "⚡ Проверить мост Rust IPC": "⚡ Test Rust IPC Bridge",
  "🪟 Сборка под Windows 10 / 11 (x86_64-pc-windows-msvc)": "🪟 Build for Windows 10 / 11 (x86_64-pc-windows-msvc)",
  "Включает аппаратное размытие": "Enables hardware blur",
  "через крейт": "via the crate",
  "и флаг": "and the flag",
  "🐧 Сборка под Linux (x86_64-unknown-linux-gnu)": "🐧 Build for Linux (x86_64-unknown-linux-gnu)",
  "Нативная RGBA-прозрачность окна для": "Native RGBA window transparency for",
  "и X11 на базе": "and X11, based on",
  "🧅 Маршрутизация через Tor (SOCKS5h) и скрытый сервис .onion": "🧅 Tor (SOCKS5h) Routing & Official .onion Hidden Service",
  "Перенаправляет все GraphQL-запросы Rust через": "Routes every Rust GraphQL request through",
  "без утечек DNS напрямую на официальное зеркало Stacker News Onion v3.": "with no DNS leaks, straight to the official Stacker News Onion v3 mirror.",
  "Tor SOCKS5h Активен": "Tor SOCKS5h Enabled",
  "Профиль прокси Tor (Remote DNS socks5h)": "Tor Proxy Profile (Remote DNS socks5h)",
  "Свой адрес SOCKS5h / Arti (Rust Tor)": "Custom SOCKS5h / Arti (Rust Tor) URI",
  "💎 Оценка потенциала SATS от сообщества": "💎 Community SATS Reward Potential Estimator",
  "ROI Прогноз": "ROI Forecast",
  "Расчёт на основе реальных выплат на Stacker News (": "Based on real payouts on Stacker News (",
  "Пост «Show SN» в ~devs и ~bitcoin (с гифкой и GitHub)": "\"Show SN\" Launch Post in ~devs & ~bitcoin (with GIF & GitHub)",
  "Доля из Daily Rewards Pool (топ-3 дня в ~devs)": "Daily Rewards Pool Share (Top-3 of the day in ~devs)",
  "PR в репозиторий SN: #2664 (xcancel) + CSV экспорт": "Upstream PRs to SN Repo: #2664 (xcancel) + CSV Export",
  "Грант сообщества (~Geyser_community / Nostr)": "Community Grant (~Geyser_community / Nostr Zaps)",
  "Ожидаемый диапазон наград:": "Estimated Sats Payout Range:",
  "Просмотр исходного модуля Rust:": "Rust module source viewer:",
  "📋 Копировать код модуля Rust": "📋 Copy Rust Module Code",
  "Privacy by Design: Блокировка всех внешних сокетов (Offline Airgap)": "Privacy by Design: block all outbound sockets (Offline Airgap)",
  "Изменить курс BTC для пересчёта всех сатоши в USD": "Change the BTC price used to convert sats to USD",

  // Bounty radar, territory model, User-Agent (round 4)
  "Обновить поиск": "Refresh search",
  "Мои задачи (в работе / PR)": "My tasks (in progress / PR)",
  "0 sats в работе": "0 sats in progress",
  "📝 Мои добавленные задачи": "📝 Tasks I added",
  "🎯 Только отслеживаемые мной": "🎯 Only ones I track",
  "Все (открытые и закрытые)": "All (open and closed)",
  "⚪ Закрытые / выплаченные": "⚪ Closed / paid",
  "Поиск по задачам (Enter или 🔍 — также искать на GitHub)": "Search tasks (Enter or 🔍 also searches GitHub)",
  "Искать": "Search",
  "🐙 Загрузить задачи GitHub": "🐙 Load GitHub issues",
  "➕ Своя задача": "➕ Own task",
  "📥 Трекер в CSV": "📥 Tracker to CSV",
  "Разрешить запросы к GitHub (api.github.com) — только по кнопке, без токена": "Allow requests to GitHub (api.github.com) — on button press only, no token",
  "⚠️ Tor выключен": "⚠️ Tor is off",
  "Запрос пойдёт к GitHub напрямую: GitHub (Microsoft) увидит ваш IP-адрес и время запроса. Включите Tor на вкладке «Rust Ядро», чтобы этого избежать.": "The request will go to GitHub directly: GitHub (Microsoft) will see your IP address and the time of the request. Turn on Tor in the \"Rust Core\" tab to avoid this.",
  "Отправить без Tor": "Send without Tor",
  "Отмена": "Cancel",
  "Название задачи": "Task title",
  "Ссылка (https://…)": "Link (https://…)",
  "Награда, sats": "Reward, sats",
  "Часов": "Hours",
  "Добавить": "Add",
  "Суммы GitHub — по таблице README Stacker News: метка difficulty (20k / 100k / 250k / 500k / 1M sats) × priority (×0.5 / ×1.5 / ×2 / ×3). Это верхняя граница: за каждую существенную правку после ревью снимается 10%, выплата на усмотрение SN. Задачи без метки difficulty показаны без суммы. Трекер хранится только на этом компьютере.": "GitHub amounts follow the Stacker News README table: difficulty label (20k / 100k / 250k / 500k / 1M sats) × priority (×0.5 / ×1.5 / ×2 / ×3). This is an upper bound: each substantial change requested in review takes off 10%, and payouts are at SN's discretion. Issues without a difficulty label are shown without an amount. The tracker is stored only on this computer.",
  "Доля основателя: плата за посты и бусты · запы": "Founder share: post and boost fees · zaps",
  "50,000 sats / месяц": "50,000 sats / month",
  "500,000 sats / год": "500,000 sats / year",
  "3,000,000 sats разово (навсегда)": "3,000,000 sats once (forever)",
  "Комментов в день:": "Comments per day:",
  "Цена комментария (sats):": "Comment cost (sats):",
  "⚙️ Параметры модели (по FAQ Stacker News)": "⚙️ Model settings (from the Stacker News FAQ)",
  "Доля основателя от платы за посты, комменты и бусты, %": "Founder share of post, comment and boost fees, %",
  "Доля основателя от запов в территории, %": "Founder share of zaps in the territory, %",
  "Цена в месяц, sats": "Monthly price, sats",
  "Цена в год, sats": "Yearly price, sats",
  "Разовая цена, sats": "One-time price, sats",
  "↺ Значения из FAQ": "↺ FAQ values",
  "По умолчанию — FAQ в репозитории Stacker News, сверено 25.10.2026: основатель получает 70% платы за посты и бусты и 21% запов; территория стоит 50k sats в месяц, 500k в год или 3M разово. При запуске территорий в 2023 году было 50% и 100k / 1M / 3M — ставки меняются, сверяйте на stacker.news/faq.": "Defaults come from the FAQ in the Stacker News repository, checked 2026-10-25: the founder gets 70% of post and boost fees and 21% of zaps; a territory costs 50k sats a month, 500k a year or 3M once. When territories launched in 2023 it was 50% and 100k / 1M / 3M — rates change, so check stacker.news/faq.",
  "Плата за посты, комменты и бусты / мес:": "Post, comment and boost fees / mo:",
  "Запы в территории / мес:": "Zaps in the territory / mo:",
  "Доход основателя / мес:": "Founder income / mo:",
  "Плата, которая не идёт основателю (SN / пул наград):": "Fees not paid to the founder (SN / rewards pool):",
  "Не отправлять User-Agent в Stacker News. Если снять галочку, отправляется UA как у Tor Browser (Firefox ESR). GitHub без UA не отвечает, поэтому ему UA отправляется всегда.": "Don't send a User-Agent to Stacker News. If unchecked, a Tor Browser UA (Firefox ESR) is sent. GitHub refuses requests without a UA, so it always gets one.",

  // v3.0: live data, territories, stacker stats, settings
  "Лента, баунти, экономика территорий, SeedScan и студия постов для Stacker News": "Feed, bounties, territory economics, SeedScan and a post studio for Stacker News",
  "Обновить с SN": "Refresh from SN",
  "Экономика территорий": "Territory Economics",
  "Статистика стакера": "Stacker Stats",
  "Настройки и приватность": "Settings & Privacy",
  "🔥 Умная лента (0)": "🔥 Smart Feed (0)",
  "Сортировка на стороне SN": "Sorting done by SN",
  "🔥 Горячее": "🔥 Hot",
  "🆕 Новое": "🆕 New",
  "🏆 Топ": "🏆 Top",
  "Период для «Топ»": "Period for \"Top\"",
  "за день": "day",
  "за неделю": "week",
  "за месяц": "month",
  "за год": "year",
  "за всё время": "all time",
  "⬇ Загрузить с SN": "⬇ Load from SN",
  "Не загружено": "Not loaded",
  "⚡ Баунти SN": "⚡ SN bounties",
  "SN: не загружено.": "SN: not loaded.",
  "GitHub: не загружено.": "GitHub: not loaded.",
  "🗺️ Реальная экономика территорий": "🗺️ Real territory economics",
  "Данные самого Stacker News (запрос topSubs): сколько основатель фактически получил за период, сколько назаппали авторам, сколько вышло постов — и окупает ли территория свой тариф. Доход приводится к 30 дням, чтобы сравнивать с ценой тарифа.": "Data from Stacker News itself (the topSubs query): what the founder actually earned in the period, how much authors were zapped, how many posts appeared — and whether the territory pays for its plan. Revenue is scaled to 30 days to compare with the plan price.",
  "Тарифы SN: 50k sats в месяц, 500k в год или 3M разово. Основатель получает 21% запов и по умолчанию 70% платы за посты и бусты; эту долю можно менять для каждой территории, поэтому здесь берётся только фактический доход.": "SN plans: 50k sats a month, 500k a year or 3M once. The founder gets 21% of zaps and by default 70% of post and boost fees; that share can be changed per territory, so only actual revenue is used here.",
  "Не загружено.": "Not loaded.",
  "Территорий": "Territories",
  "Доход всех основателей за период": "All founders' revenue in the period",
  "Окупают свой тариф": "Pay for their plan",
  "Медианный доход / 30 дней": "Median revenue / 30 days",
  "Доход основателя и цена тарифа, 30 дней (топ-12)": "Founder revenue vs plan price, 30 days (top 12)",
  "Фильтр по ~территории или @основателю...": "Filter by ~territory or @founder...",
  "💰 По доходу основателя": "💰 By founder revenue",
  "📈 По чистому итогу / 30 дней": "📈 By net result / 30 days",
  "⚡ По запам": "⚡ By zaps",
  "📝 По числу постов": "📝 By number of posts",
  "🏷️ По цене поста (дешевле)": "🏷️ By post price (cheapest)",
  "Только окупающиеся": "Only those that pay off",
  "Пост / коммент": "Post / comment",
  "Тариф": "Plan",
  "Доход за период": "Revenue in period",
  "Доход / 30 дн.": "Revenue / 30 d",
  "Тариф / 30 дн.": "Plan / 30 d",
  "Итог / 30 дн.": "Net / 30 d",
  "Запы": "Zaps",
  "Постов": "Posts",
  "Заголовок поста": "Post title",
  "## Раздел Текст поста в Markdown…": "## Section\nPost text in Markdown…",
  "📊 Статистика стакера": "📊 Stacker stats",
  "Публичная статистика любого пользователя Stacker News за день, неделю, месяц, год и всё время: сколько собрал и потратил, сколько опубликовал, серии. Если человек скрыл статистику в настройках SN, приложение так и покажет.": "Public stats of any Stacker News user for a day, week, month, year and all time: sats stacked and spent, items posted, streaks. If someone hides their stats in SN settings, the app says so.",
  "@ник": "@nym",
  "🔍 Загрузить с SN": "🔍 Load from SN",
  "Введите ник и нажмите «Загрузить».": "Enter a nym and press Load.",
  "Собрано за всё время": "Stacked, all time",
  "Собрано за 30 дней / потрачено": "Stacked in 30 days / spent",
  "Постов и комментариев (всего)": "Posts and comments (total)",
  "Серия / рекорд": "Streak / best",
  "Собрано и потрачено по периодам": "Stacked and spent by period",
  "⚙️ Настройки и приватность": "⚙️ Settings & privacy",
  "Все сетевые запросы делает Rust-клиент: только stacker.news и его официальный .onion (плюс api.github.com, если вы его разрешили на вкладке баунти), проверка на каждом редиректе, Tor через socks5h, режим Airgap. Интерфейс сам в сеть не ходит.": "All network requests are made by the Rust client: only stacker.news and its official .onion (plus api.github.com if you allowed it on the bounty tab), checked on every redirect, Tor via socks5h, Airgap mode. The interface itself never goes online.",

  "Курс BTC из Stacker News; нужен, чтобы показывать суммы в долларах": "BTC price from Stacker News; used to show amounts in dollars",
  "Сколько сатоши SN раздаст сегодня самым полезным авторам и зап-перам": "How many sats SN will pay out today to the most valuable posters and zappers",
  "Награды SN сегодня:": "SN rewards today:",
  "🔎 Найти Tor": "🔎 Find Tor",
  "Проверяет порты 9150 (Tor Browser) и 9050 (служба tor) на этом компьютере.": "Checks ports 9150 (Tor Browser) and 9050 (tor service) on this computer.",
  "Если .onion не отвечает, повторить запрос к stacker.news через тот же Tor. IP остаётся скрытым; SN увидит выходной узел Tor вместо onion-цепочки.": "If the .onion does not answer, retry via stacker.news through the same Tor. Your IP stays hidden; SN sees a Tor exit node instead of an onion circuit.",
  "Состояние .onion станет известно после первой загрузки через Tor.": "The .onion status will be known after the first load over Tor.",
  "↻ Попробовать .onion снова": "↻ Try the .onion again",
  "THIRD-PARTY.md (лицензии всех зависимостей)": "THIRD-PARTY.md (licences of every dependency)",
  "DISCLAIMER.md (отказ от гарантий и границы применения)": "DISCLAIMER.md (warranty disclaimer and scope of use)",
  // Strings set by app.js without numbers
  "Tor: ВКЛ (.onion SOCKS5h)": "Tor: ON (.onion SOCKS5h)",
  "Маршрут: Tor SOCKS5h (.onion v3)": "Route: Tor SOCKS5h (.onion v3)"
};

(function () {
  const norm = s => s.replace(/\s+/g, ' ').trim();
  const SKIP = new Set(['SCRIPT', 'STYLE', 'TEXTAREA', 'PRE', 'CODE']);
  const ATTRS = ['placeholder', 'title', 'aria-label'];
  // Remember the Russian original of each node/attribute we change.
  const origText = new WeakMap();
  const origAttr = new WeakMap();

  function lang() {
    return document.documentElement.lang === 'en' ? 'en' : 'ru';
  }

  function textNode(node, l) {
    if (l === 'en') {
      const key = norm(node.nodeValue);
      const en = RU_EN[key];
      if (en === undefined) return;
      if (!origText.has(node)) origText.set(node, node.nodeValue);
      const m = node.nodeValue.match(/^(\s*)[\s\S]*?(\s*)$/);
      node.nodeValue = m[1] + en + m[2];
    } else if (origText.has(node)) {
      node.nodeValue = origText.get(node);
      origText.delete(node);
    }
  }

  function element(el, l) {
    for (const a of ATTRS) {
      if (!el.hasAttribute(a)) continue;
      let saved = origAttr.get(el);
      if (l === 'en') {
        const en = RU_EN[norm(el.getAttribute(a))];
        if (en === undefined) continue;
        if (!saved) { saved = {}; origAttr.set(el, saved); }
        if (!(a in saved)) saved[a] = el.getAttribute(a);
        el.setAttribute(a, en);
      } else if (saved && a in saved) {
        el.setAttribute(a, saved[a]);
        delete saved[a];
      }
    }
  }

  function walk(root, l) {
    if (root.nodeType === Node.TEXT_NODE) {
      if (!root.parentElement || !SKIP.has(root.parentElement.tagName)) textNode(root, l);
      return;
    }
    if (root.nodeType !== Node.ELEMENT_NODE || SKIP.has(root.tagName)) return;
    element(root, l);
    const tw = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, {
      acceptNode(n) {
        if (n.nodeType === Node.ELEMENT_NODE) {
          return SKIP.has(n.tagName) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_SKIP;
        }
        return NodeFilter.FILTER_ACCEPT;
      }
    });
    root.querySelectorAll('[placeholder],[title],[aria-label]').forEach(el => element(el, l));
    let n;
    while ((n = tw.nextNode())) textNode(n, l);
  }

  window.translateStaticUi = function (l) {
    walk(document.body, l === 'en' ? 'en' : 'ru');
  };

  // Content rendered later (feed cards, toasts, badges) is translated as it
  // appears. Only childList is observed, so our own nodeValue writes do not
  // retrigger the observer.
  const obs = new MutationObserver(muts => {
    if (lang() !== 'en') return;
    for (const m of muts) m.addedNodes.forEach(n => walk(n, 'en'));
  });
  document.addEventListener('DOMContentLoaded', () => {
    obs.observe(document.body, { childList: true, subtree: true });
  });
})();
