    // No built-in sample data: every list is loaded live from Stacker News
    // (through the Rust client) or created by the user.

    const GQL_PRESETS = {
      hot_items: `query HotItems {
  items(sort: "hot") {
    cursor
    items {
      id
      title
      sats
      ncomments
      subName
      createdAt
      url
      user {
        name
      }
    }
  }
}`,
      sub_devs: `query DevsTerritoryItems {
  items(sub: "devs", sort: "new") {
    items {
      id
      title
      sats
      ncomments
      boost
      bounty
      user {
        name
      }
    }
  }
}`,
      user_profile: `query StackerProfile {
  user(name: "k00b") {
    id
    name
    since
    nitems
    nterritories
    optional {
      stacked(when: "forever")
      spent(when: "forever")
      streak
      maxStreak
    }
  }
}`,
      territories_list: `query ActiveTerritories {
  subs {
    name
    baseCost
    replyCost
    rankingType
    billingType
    status
  }
}`
    };

    // ==================== STATE ====================
    // Shared UI state. Declared at top level because both the top-level
    // functions and the DOMContentLoaded handler read them (before, they were
    // block-scoped inside the handler, so executeSnGraphql never saw Airgap).
    let currentLang = 'ru';
    let airgapMode = false;
    try { if (localStorage.getItem('sn_pulse_lang') === 'en') currentLang = 'en'; } catch (e) {}

    // Pick the string for the current UI language.
    function L(ru, en) { return currentLang === 'en' ? en : ru; }
    // Status lines keep both languages so switching RU/EN re-labels them.
    function setLine(el, ru, en) {
      if (!el) return;
      el.dataset.ru = ru; el.dataset.en = en === undefined ? ru : en;
      el.textContent = currentLang === 'en' ? el.dataset.en : el.dataset.ru;
      el.title = el.textContent;
    }

    function escapeHtml(v) {
      return String(v == null ? '' : v).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
    }

    let feedItems = [];          // filled by loadFeed()
    let feedLoadedAt = null;
    let radarViewMode = 'feed'; // 'feed' | 'bookmarks'
    let bookmarks = [];
    let draftQueue = [];
    let snBounties = null;       // live SN bounties, null until loaded
    let territoryStats = null;   // live territory economics, null until loaded
    let marketBtcUsd = null;
    let lastFeedError = null; // [ru, en] text of the last failed feed load
    // true when SN's .onion did not answer recently; then the app uses the
    // backup route and copies links as https://stacker.news (still via Tor).
    let onionDown = false;

    function torIsOn() {
      const cb = document.getElementById('torEnableCheckbox');
      return !!(cb && cb.checked);
    }

    async function refreshOnionStatus(reset) {
      if (!hasTauriInvoke()) return;
      let st = 'unknown';
      try { st = await window.__TAURI__.core.invoke('sn_onion_status', { reset: !!reset }); } catch (e) { return; }
      onionDown = st === 'down';
      const line = document.getElementById('onionStatusLine');
      if (line) {
        if (st === 'up') setLine(line, '✅ .onion Stacker News отвечает', '✅ The Stacker News .onion answers');
        else if (st === 'down') setLine(line, '⚠️ .onion Stacker News сейчас не отвечает. Данные идут через stacker.news по Tor (IP скрыт), ссылки копируются как https://stacker.news — они откроются в Tor Browser. Через 10 минут приложение попробует .onion снова.',
                                              '⚠️ The Stacker News .onion is not answering right now. Data comes via stacker.news over Tor (IP hidden), links are copied as https://stacker.news — they open in Tor Browser. The app will try the .onion again in 10 minutes.');
        else setLine(line, 'Состояние .onion станет известно после первой загрузки через Tor.', 'The .onion status will be known after the first load over Tor.');
      }
      document.getElementById('onionRetryBtn')?.classList.toggle('hidden', st !== 'down');
    }

    // Version comes from Cargo.toml via Rust, so the UI can never show a stale
    // number again (it said v3.0 while the build was 3.0.4).
    async function showAppVersion() {
      if (!hasTauriInvoke()) return;
      try {
        const v = await window.__TAURI__.core.invoke('sn_app_version');
        document.getElementById('appVersionBadge').textContent = `SN Power Suite v${v}`;
        document.getElementById('seedscanEngineVersion').textContent = `SeedScan v${v}`;
      } catch (e) {}
    }

    // ---- Updates (updates.rs): user-initiated, Tor when on, signed packages ----
    let updInfo = null;
    let updPoll = null;

    function updLine(ru, en, cls) {
      const el = document.getElementById('updStatus');
      setLine(el, ru, en);
      el.className = 'text-xs ' + (cls || 'text-zinc-300');
    }

    async function checkForUpdates(allowClearnet) {
      const confirmBox = document.getElementById('updClearnetConfirm');
      confirmBox.classList.add('hidden');
      document.getElementById('updAvailable').classList.add('hidden');
      if (!hasTauriInvoke()) { updLine('Браузерное превью: сеть отключена', 'Browser preview: network disabled'); return; }
      if (airgapMode) { updLine('🔒 Airgap: проверка заблокирована', '🔒 Airgap: check blocked', 'text-amber-300'); return; }
      const cfg = getPrivacyConfig();
      if (!cfg.use_tor && !allowClearnet) { confirmBox.classList.remove('hidden'); return; }
      updLine(cfg.use_tor ? 'Проверяю через Tor…' : 'Проверяю…', cfg.use_tor ? 'Checking over Tor…' : 'Checking…');
      try {
        updInfo = await window.__TAURI__.core.invoke('sn_check_update', { privacy: cfg, allowClearnet: !!allowClearnet });
        renderUpdateInfo();
      } catch (e) {
        const msg = e && e.message ? e.message : String(e);
        if (msg === 'TOR_REQUIRED') { confirmBox.classList.remove('hidden'); return; }
        updLine(...errLine(e, '❌ '));
        document.getElementById('updStatus').className = 'text-xs text-red-300';
      }
    }

    function renderUpdateInfo() {
      if (!updInfo) return;
      const via = updInfo.via_tor ? L(' (через Tor)', ' (over Tor)') : '';
      const box = document.getElementById('updAvailable');
      if (!updInfo.available) {
        box.classList.add('hidden');
        updLine(`✅ У вас последняя версия: v${updInfo.current}${via}`, `✅ You have the latest version: v${updInfo.current}${via}`, 'text-emerald-300');
        return;
      }
      updLine(`⬆ Доступна версия v${updInfo.available} (у вас v${updInfo.current})${via}`,
        `⬆ Version v${updInfo.available} is available (you have v${updInfo.current})${via}`, 'text-[#FADA5E]');
      const notes = document.getElementById('updNotes');
      notes.textContent = updInfo.notes || '';
      notes.classList.toggle('hidden', !updInfo.notes);
      box.classList.remove('hidden');
    }

    function fmtBytes(n) {
      if (!n) return '0';
      return n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : `${Math.round(n / 1024)} KB`;
    }

    async function installUpdate() {
      const btn = document.getElementById('updInstallBtn');
      const wrap = document.getElementById('updProgressWrap');
      const bar = document.getElementById('updProgressBar');
      const text = document.getElementById('updProgressText');
      btn.disabled = true;
      btn.classList.add('opacity-50');
      wrap.classList.remove('hidden');
      text.textContent = L('Скачиваю…', 'Downloading…');
      clearInterval(updPoll);
      updPoll = setInterval(async () => {
        try {
          const p = await window.__TAURI__.core.invoke('sn_update_progress');
          const pct = p.total ? Math.min(100, Math.round(p.downloaded / p.total * 100)) : 0;
          bar.style.width = (p.total ? pct : 15) + '%';
          text.textContent = p.total ? `${fmtBytes(p.downloaded)} / ${fmtBytes(p.total)} (${pct}%)` : fmtBytes(p.downloaded);
        } catch (e) {}
      }, 500);
      try {
        await window.__TAURI__.core.invoke('sn_install_update');
        // Windows: the installer takes over; elsewhere the app restarts itself.
        text.textContent = L('Подпись проверена, устанавливаю…', 'Signature verified, installing…');
      } catch (e) {
        clearInterval(updPoll);
        btn.disabled = false;
        btn.classList.remove('opacity-50');
        wrap.classList.add('hidden');
        updLine(...errLine(e, '❌ '));
        document.getElementById('updStatus').className = 'text-xs text-red-300';
      }
    }

    // Donation addresses: compiled in from DONATE.md and checked by Rust tests.
    // Shown only when at least one is set; copied, never opened.
    async function renderSupport() {
      if (!hasTauriInvoke()) return;
      let list = [];
      try {
        list = await window.__TAURI__.core.invoke('sn_support_addresses');
      } catch (e) {
        return;
      }
      const card = document.getElementById('supportCard');
      const box = document.getElementById('supportList');
      if (!card || !box || !Array.isArray(list) || list.length === 0) return;
      box.replaceChildren();
      for (const item of list) {
        const row = document.createElement('div');
        row.className = 'bg-black/40 border border-white/10 rounded-lg p-3 space-y-1.5';
        const head = document.createElement('div');
        head.className = 'flex flex-wrap items-center justify-between gap-2';
        const name = document.createElement('span');
        name.className = 'text-xs font-bold text-zinc-100';
        name.textContent = item.name;
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'px-3 py-1 rounded-lg bg-white/10 hover:bg-white/20 text-xs font-semibold text-[#FADA5E] transition';
        btn.textContent = L('📋 Копировать', '📋 Copy');
        btn.addEventListener('click', () => copyTextToClipboard(
          item.address,
          L('Адрес скопирован. Сверьте начало и конец после вставки.',
            'Address copied. Check its start and end after pasting.')));
        head.append(name, btn);
        const addr = document.createElement('code');
        addr.className = 'block text-xs font-mono text-emerald-300 break-all select-all';
        addr.textContent = item.address;
        const warn = document.createElement('p');
        warn.className = 'text-[11px] text-amber-300/90';
        warn.textContent = L(item.warning_ru, item.warning_en);
        row.append(head, addr, warn);
        box.appendChild(row);
      }
      card.classList.remove('hidden');
    }

    // SN data commands; afterwards learn whether the onion or the backup route answered.
    async function snInvoke(cmd, args) {
      const r = await window.__TAURI__.core.invoke(cmd, args);
      if (torIsOn()) await refreshOnionStatus(false);
      return r;
    }


    let territoryChartInstance = null;
    let stackerHistoryChartInstance = null;

    // Load saved state safely
    try {
      const savedBm = localStorage.getItem('sn_pulse_bookmarks');
      if (savedBm) bookmarks = JSON.parse(savedBm);
      const savedDrafts = localStorage.getItem('sn_pulse_drafts');
      if (savedDrafts) draftQueue = JSON.parse(savedDrafts);
    } catch (e) {
      // Ignore storage access errors in strict iframe
    }

    function saveState() {
      try {
        localStorage.setItem('sn_pulse_bookmarks', JSON.stringify(bookmarks));
        localStorage.setItem('sn_pulse_drafts', JSON.stringify(draftQueue));
      } catch (e) {}
    }

    // ==================== HELPERS ====================
    // BTC/USD comes only from SN's price query; until then USD is unknown.
    function getBtcPrice() {
      return marketBtcUsd && marketBtcUsd > 0 ? marketBtcUsd : null;
    }

    function satsToUsd(sats) {
      const btc = getBtcPrice();
      if (!btc) return '$—';
      const usd = (sats / 100000000) * btc;
      return usd < 0.01 && sats > 0 ? '<$0.01' : '$' + usd.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }

    // BTC price display (read-only; set by loadMarket)
    function renderBtcPrice() {
      const btc = getBtcPrice();
      const el = document.getElementById('btcPriceDisplay');
      const per = document.getElementById('satsPerUsdBadge');
      if (!btc) { el.textContent = '—'; per.textContent = ''; return; }
      el.textContent = '$' + Math.round(btc).toLocaleString('en-US');
      per.textContent = `· $1 = ${formatNumber(Math.round(100000000 / btc))} sats`;
      renderRadar();
      renderBounties();
      renderTerritoryEconomics();
      updatePostStudioAudit();
      if (stackerData) renderStackerAnalytics();
    }

    function formatNumber(n) {
      return Number(n).toLocaleString('en-US');
    }

    function showToast(msg) {
      const toast = document.getElementById('statusToast');
      const text = document.getElementById('statusToastText');
      text.textContent = msg;
      toast.classList.remove('hidden');
      clearTimeout(window._toastTimer);
      window._toastTimer = setTimeout(() => {
        toast.classList.add('hidden');
      }, 3800);
    }

    function copyTextToClipboard(text, label) {
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text).then(() => {
          showToast(label || L('Скопировано в буфер обмена', 'Copied to clipboard'));
        }).catch(() => fallbackCopy(text, label));
      } else {
        fallbackCopy(text, label);
      }
    }

    function fallbackCopy(text, label) {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.position = 'fixed';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.select();
      try { document.execCommand('copy'); } catch (e) {}
      document.body.removeChild(ta);
      showToast(label || L('Скопировано в буфер обмена', 'Copied to clipboard'));
    }

    function downloadFile(filename, content, mimeType) {
      const blob = new Blob([content], { type: mimeType || 'text/plain;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    }

    function transformExternalLink(url) {
      if (!url) return '';
      const autoXcancel = document.getElementById('autoXcancelToggle').checked;
      if (!autoXcancel) return url;
      return url.replace(/^https?:\/\/(www\.)?(x\.com|twitter\.com)/i, 'https://xcancel.com');
    }

    // ==================== TAB 1: RENDER RADAR & BOOKMARKS ====================
    function getFilteredFeed() {
      const q = document.getElementById('radarSearchInput').value.trim().toLowerCase();
      const sub = document.getElementById('radarTerritorySelect').value;
      const type = document.getElementById('radarTypeSelect').value;
      const sort = document.getElementById('radarSortSelect').value;
      const minSats = parseInt(document.getElementById('radarMinSatsInput').value, 10) || 0;
      const hideFreebies = document.getElementById('hideFreebiesToggle').checked;

      let list = feedItems.filter(item => {
        if (hideFreebies && (item.isFreebie || item.sats === 0)) return false;
        if (item.sats < minSats) return false;
        if (sub !== 'all' && item.sub !== sub && !(item.extraSubs && item.extraSubs.includes(sub))) return false;
        if (type !== 'all' && item.type !== type) return false;
        if (q) {
          const hay = `${item.title} @${item.user} ~${item.sub} ${item.externalUrl || ''}`.toLowerCase();
          if (!hay.includes(q)) return false;
        }
        return true;
      });

      list.sort((a, b) => {
        const sigA = a.sats / Math.max(1, a.comments);
        const sigB = b.sats / Math.max(1, b.comments);
        const velA = a.sats / Math.max(1, a.hoursAgo);
        const velB = b.sats / Math.max(1, b.hoursAgo);
        if (sort === 'sats_desc') return b.sats - a.sats;
        if (sort === 'signal_desc') return sigB - sigA;
        if (sort === 'velocity_desc') return velB - velA;
        if (sort === 'comments_desc') return b.comments - a.comments;
        return b.sats - a.sats;
      });

      return list;
    }

    function hasTauriInvoke() {
      return !!(window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke);
    }

    // Error text in both languages, for status lines that re-label on RU/EN.
    function errLine(e, prefix) {
      const saved = currentLang;
      currentLang = 'ru'; const ru = prefix + errText(e).slice(0, 400);
      currentLang = 'en'; const en = prefix + errText(e).slice(0, 400);
      currentLang = saved;
      return [ru, en];
    }

    // Rust reports network failures as NET:<KIND>|<detail>[|<extra>].
    function errText(e) {
      const raw = e && e.message ? e.message : String(e);
      const m = /^NET:([A-Z_]+)\|([^\n]*)/.exec(raw);
      if (!m) return raw;
      const [, kind, rest] = m;
      const parts = rest.split('|');
      const fellBack = raw.includes('\n(.onion)');
      const tail = fellBack ? L(' Запасной путь (stacker.news через Tor) тоже не ответил.', ' The backup route (stacker.news via Tor) did not answer either.') : '';
      switch (kind) {
        case 'TOR_REFUSED': {
          const addr = parts[0] || '';
          const found = parts[1];
          return found
            ? L(`Tor не слушает на ${addr}, но найден на порту ${found}. Выберите его в «Настройки и приватность» или нажмите «🔎 Найти Tor».`,
                `Tor is not listening on ${addr}, but it was found on port ${found}. Pick it in Settings & privacy or press "🔎 Find Tor".`)
            : L(`Tor не отвечает на ${addr}. Запустите Tor Browser (порт 9150) или службу tor (порт 9050) и попробуйте снова.`,
                `Tor is not answering on ${addr}. Start Tor Browser (port 9150) or the tor service (port 9050) and try again.`);
        }
        case 'TIMEOUT':
          return L('Tor подключён, но SN не ответил за 60 секунд. Цепочка медленная или сервис недоступен — попробуйте ещё раз.',
                   'Tor is connected, but SN did not answer within 60 seconds. The circuit is slow or the service is down — try again.') + tail;
        case 'SOCKS_FAIL':
          return L('Tor подключён, но не смог достучаться до .onion Stacker News (' + (parts[0].split(': ').pop() || '') + ').',
                   'Tor is connected but could not reach the Stacker News .onion (' + (parts[0].split(': ').pop() || '') + ').') + tail;
        case 'BAD_PROXY':
          return L(`Адрес прокси должен начинаться с socks5h:// (иначе DNS-запросы пойдут мимо Tor): ${parts[0]}`,
                   `The proxy address must start with socks5h:// (otherwise DNS goes around Tor): ${parts[0]}`);
        case 'TLS':
          return L('Ошибка TLS-сертификата: соединение прервано ради безопасности. ', 'TLS certificate error: connection aborted for safety. ') + parts[0];
        default:
          return L('Сетевая ошибка: ', 'Network error: ') + parts[0];
      }
    }

    function mapFeedItem(f) {
      const created = Date.parse(f.created_at);
      return {
        id: String(f.id), title: f.title || L('Без заголовка', 'Untitled'), user: f.user || 'anon', sub: f.sub || '',
        extraSubs: [], type: f.item_type, sats: Number(f.sats) || 0, comments: Number(f.comments) || 0,
        hoursAgo: Number.isFinite(created) ? Math.max(0, Math.round((Date.now() - created) / 3.6e6)) : 0,
        url: f.url, externalUrl: f.external_url || '', isFreebie: (Number(f.sats) || 0) === 0,
        bounty: f.bounty || null
      };
    }

    // Live feed through Rust (sn.rs). Nothing is shown until this succeeds.
    async function loadFeed() {
      const badge = document.getElementById('apiSourceBadge');
      if (!hasTauriInvoke()) {
        setLine(badge, 'Браузерное превью: сеть отключена', 'Browser preview: network disabled');
        return;
      }
      if (airgapMode) {
        setLine(badge, '🔒 Airgap: загрузка заблокирована', '🔒 Airgap: loading blocked');
        showToast(L('🔒 Airgap включён — сеть заблокирована', '🔒 Airgap is on — network blocked'));
        return;
      }
      const sub = document.getElementById('radarTerritorySelect').value;
      const sort = document.getElementById('feedSortSelect').value;
      const when = document.getElementById('feedWhenSelect').value;
      setLine(badge, 'Загрузка…', 'Loading…');
      try {
        const items = await snInvoke('sn_fetch_feed', {
          sub: sub === 'all' ? null : sub, sort, when, privacy: getPrivacyConfig()
        });
        lastFeedError = null;
        feedItems = items.map(mapFeedItem);
        feedLoadedAt = Date.now();
        rememberTerritories(feedItems.map(i => i.sub));
        const via = !torIsOn() ? ['', ''] : onionDown ? [' · Tor → stacker.news', ' · Tor → stacker.news'] : [' · Tor → .onion', ' · Tor → .onion'];
        setLine(badge, `SN: ${feedItems.length} постов · ${new Date(feedLoadedAt).toLocaleTimeString()}${via[0]}`,
                              `SN: ${feedItems.length} posts · ${new Date(feedLoadedAt).toLocaleTimeString()}${via[1]}`);
        renderRadar();
      } catch (e) {
        lastFeedError = errLine(e, '❌ ');
        setLine(badge, ...lastFeedError);
        renderRadar();
        showToast(L('❌ Не удалось загрузить ленту: ', '❌ Could not load the feed: ') + errText(e).slice(0, 300));
      }
    }

    // Keeps the territory filter in sync with territories actually seen.
    function rememberTerritories(names) {
      const sel = document.getElementById('radarTerritorySelect');
      const have = new Set([...sel.options].map(o => o.value));
      [...new Set(names)].filter(n => n && /^[A-Za-z0-9_]{1,32}$/.test(n) && !have.has(n)).sort((a, b) => a.localeCompare(b))
        .forEach(n => { const o = document.createElement('option'); o.value = n; o.textContent = '~' + n; sel.appendChild(o); });
    }

    async function loadMarket() {
      if (!hasTauriInvoke() || airgapMode) return;
      try {
        const m = await snInvoke('sn_fetch_market', { privacy: getPrivacyConfig() });
        if (m.btc_usd) {
          marketBtcUsd = m.btc_usd;
          renderBtcPrice();
        }
        document.getElementById('rewardsPoolHeader').textContent = m.rewards_pool_sats != null ? `${formatNumber(m.rewards_pool_sats)} sats` : '—';
      } catch (e) {
        document.getElementById('rewardsPoolHeader').textContent = '—';
      }
    }

    function renderRadar() {
      const list = getFilteredFeed();
      document.getElementById('minSatsVal').textContent = formatNumber(document.getElementById('radarMinSatsInput').value);
      document.getElementById('navFeedCount').textContent = list.length;
      document.getElementById('radarModeFeedBtn').textContent = L(`🔥 Умная лента (${list.length})`, `🔥 Smart Feed (${list.length})`);
      document.getElementById('bookmarkBtnCounter').textContent = bookmarks.length;
      document.getElementById('kpiBookmarksCount').textContent = bookmarks.length;

      // KPIs
      const totalSats = list.reduce((acc, it) => acc + it.sats, 0);
      const totalComments = list.reduce((acc, it) => acc + it.comments, 0);
      const avgSignal = list.length ? Math.round(totalSats / Math.max(1, totalComments)) : 0;

      document.getElementById('kpiTotalSats').textContent = formatNumber(totalSats);
      document.getElementById('kpiTotalUsd').textContent = L(`≈ ${satsToUsd(totalSats)} (${list.length} постов)`, `≈ ${satsToUsd(totalSats)} (${list.length} posts)`);
      document.getElementById('kpiSignalRatio').textContent = L(`${formatNumber(avgSignal)} sats/комм`, `${formatNumber(avgSignal)} sats/comment`);

      // Top territory in current filter
      const subTotals = {};
      list.forEach(it => {
        subTotals[it.sub] = (subTotals[it.sub] || 0) + it.sats;
      });
      let topSub = '';
      let topSubVal = 0;
      Object.entries(subTotals).forEach(([k, v]) => {
        if (v > topSubVal) { topSub = k; topSubVal = v; }
      });
      document.getElementById('kpiTopTerritory').textContent = topSub ? `~${topSub}` : '—';
      document.getElementById('kpiTopTerritorySub').textContent = L(`${formatNumber(topSubVal)} sats в выборке`, `${formatNumber(topSubVal)} sats in selection`);

      const container = document.getElementById('radarFeedContainer');
      if (feedItems.length === 0) {
        container.innerHTML = `<div class="bg-[#1b1b1f] border border-zinc-800 rounded-xl p-8 text-center space-y-2">
          <div class="text-sm text-zinc-200 font-semibold">${L('Лента пока пуста', 'The feed is empty')}</div>
          <div class="text-xs text-zinc-400">${L('Выберите территорию и сортировку и нажмите «⬇ Загрузить с SN». Запрос идёт через Rust-клиент: allowlist, Tor и Airgap действуют.', 'Pick a territory and sort, then press "⬇ Load from SN". The request goes through the Rust client: allowlist, Tor and Airgap apply.')}</div>
          ${lastFeedError ? `<div class="mt-3 text-xs text-rose-300 bg-rose-500/10 border border-rose-500/30 rounded-lg p-3 text-left">${escapeHtml(currentLang === 'en' ? lastFeedError[1] : lastFeedError[0])}</div>` : ''}
        </div>`;
        return;
      }
      if (list.length === 0) {
        container.innerHTML = `<div class="bg-[#1b1b1f] border border-zinc-800 rounded-xl p-8 text-center text-sm text-zinc-400">${L('По заданным фильтрам постов не найдено. Попробуйте снизить порог сатоши или сбросить поисковый запрос.', 'No posts match these filters. Try lowering the sats threshold or clearing the search.')}</div>`;
        return;
      }

      const bookmarkedIds = new Set(bookmarks.map(b => b.id));

      container.innerHTML = list.map((item, idx) => {
        const signalScore = Math.round(item.sats / Math.max(1, item.comments));
        const velocity = Math.round(item.sats / Math.max(1, item.hoursAgo));
        const isSaved = bookmarkedIds.has(item.id);
        const extLink = transformExternalLink(item.externalUrl);
        const typeBadgeColor = item.type === 'BOUNTY'
          ? 'bg-emerald-950 text-emerald-300 border-emerald-700'
          : item.type === 'POLL'
          ? 'bg-purple-950 text-purple-300 border-purple-700'
          : item.type === 'LINK'
          ? 'bg-sky-950 text-sky-300 border-sky-800'
          : 'bg-zinc-800 text-zinc-300 border-zinc-700';

        return `
          <article class="bg-[#1b1b1f] hover:bg-[#222228] border border-zinc-800 rounded-xl p-4 transition flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
            <div class="space-y-1.5 flex-1 min-w-0">
              <div class="flex flex-wrap items-center gap-2 text-xs">
                <span class="font-mono text-zinc-500">#${idx + 1}</span>
                <span class="text-[10px] font-mono uppercase px-2 py-0.5 rounded border ${typeBadgeColor}">${item.type}</span>
                <span class="font-mono text-xs px-2 py-0.5 rounded bg-amber-500/10 text-[#FADA5E] border border-amber-500/20">~${escapeHtml(item.sub)}</span>
                ${(item.extraSubs || []).map(es => `<span class="font-mono text-[11px] px-1.5 py-0.5 rounded bg-zinc-900 text-zinc-400 border border-zinc-800">~${es}</span>`).join('')}
                <span class="text-zinc-400">${L('от', 'by')} <strong class="text-zinc-200">@${escapeHtml(item.user)}</strong> · ${item.hoursAgo}${L('ч назад', 'h ago')}</span>
              </div>
              <div class="flex flex-wrap items-baseline gap-2">
                <a href="${escapeHtml(item.url)}" target="_blank" rel="noopener noreferrer" class="text-sm sm:text-base font-bold text-white hover:text-[#FADA5E] transition break-words">
                  ${escapeHtml(item.title)}
                </a>
                ${extLink ? `<a href="${escapeHtml(extLink)}" target="_blank" rel="noopener noreferrer" class="text-xs font-mono text-emerald-400 hover:underline break-all">🔗 ${escapeHtml(extLink.replace(/^https?:\/\//, '').slice(0, 42))}</a>` : ''}
              </div>
              <div class="flex flex-wrap items-center gap-4 text-xs text-zinc-400 font-mono">
                <span class="text-[#FADA5E] font-bold">⚡ ${formatNumber(item.sats)} sats (${satsToUsd(item.sats)})</span>
                <span>💬 ${item.comments} ${L('комм.', 'comments')}</span>
                <span class="text-emerald-400" title="Sats per comment">💎 Signal: ${formatNumber(signalScore)} s/c</span>
                <span class="text-sky-400" title="Sats per hour">🚀 ${L('Темп', 'Pace')}: ${formatNumber(velocity)} ${L('s/ч', 's/h')}</span>
              </div>
            </div>

            <div class="flex items-center gap-2 shrink-0 self-end sm:self-center">
              <button type="button" data-bookmark-id="${item.id}"
                class="toggle-bookmark-btn px-3 py-1.5 rounded-lg text-xs font-semibold border transition ${isSaved ? 'bg-sky-950/80 border-sky-600 text-sky-300' : 'bg-zinc-900 border-zinc-700 text-zinc-300 hover:border-[#FADA5E] hover:text-white'}">
                ${isSaved ? L('🔖 В закладках', '🔖 Bookmarked') : L('+ В закладки', '+ Bookmark')}
              </button>
              <a href="${escapeHtml(item.url)}" target="_blank" rel="noopener noreferrer"
                class="px-3 py-1.5 rounded-lg text-xs font-semibold bg-zinc-800 hover:bg-zinc-700 text-zinc-200 transition">
                ${L('🔗 Копировать ссылку', '🔗 Copy link')}
              </a>
            </div>
          </article>
        `;
      }).join('');
    }

    function renderBookmarks() {
      const tagFilter = document.getElementById('bookmarkTagFilter').value;
      const listEl = document.getElementById('bookmarksList');
      const filtered = bookmarks.filter(b => tagFilter === 'all' || b.tag === tagFilter);

      if (filtered.length === 0) {
        listEl.innerHTML = `<div class="bg-[#1b1b1f] border border-zinc-800 rounded-xl p-6 text-center text-xs text-zinc-400">${L('Закладок с выбранным тегом пока нет. Добавьте любой пост из Умной ленты!', 'No bookmarks with this tag yet. Add any post from the Smart Feed!')}</div>`;
        return;
      }

      listEl.innerHTML = filtered.map((b, i) => `
        <div class="bg-[#1b1b1f] border border-zinc-800 rounded-xl p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
          <div class="space-y-1 flex-1">
            <div class="flex items-center gap-2 text-xs">
              <span class="font-mono text-zinc-500">#${i + 1}</span>
              <span class="px-2 py-0.5 rounded bg-sky-950 text-sky-300 border border-sky-800 font-mono text-[11px]">#${b.tag}</span>
              <span class="font-mono text-[#FADA5E]">~${escapeHtml(b.sub)}</span>
              <span class="text-zinc-400">@${escapeHtml(b.user)} · ${formatNumber(b.sats)} sats</span>
            </div>
            <a href="${escapeHtml(b.url)}" target="_blank" rel="noopener noreferrer" class="text-sm font-bold text-white hover:text-[#FADA5E] block">${escapeHtml(b.title)}</a>
            <div class="flex items-center gap-2 pt-1">
              <input type="text" data-note-id="${b.id}" value="${escapeHtml(b.note || '')}" placeholder="${L('Добавить личную заметку к закладке...', 'Add a private note to this bookmark...')}"
                class="bookmark-note-input flex-1 bg-zinc-900 border border-zinc-700 rounded px-2.5 py-1 text-xs text-zinc-200 focus:outline-none focus:border-[#FADA5E]" />
              <select data-tag-id="${b.id}" class="bookmark-tag-select bg-zinc-900 border border-zinc-700 rounded px-2 py-1 text-xs text-zinc-200">
                <option value="research" ${b.tag === 'research' ? 'selected' : ''}>#research</option>
                <option value="dev" ${b.tag === 'dev' ? 'selected' : ''}>#dev</option>
                <option value="lightning" ${b.tag === 'lightning' ? 'selected' : ''}>#lightning</option>
                <option value="bounty" ${b.tag === 'bounty' ? 'selected' : ''}>#bounty</option>
              </select>
            </div>
          </div>
          <div class="flex items-center gap-1.5 shrink-0">
            <button type="button" data-move-id="${b.id}" data-dir="-1" class="move-bm-btn px-2.5 py-1.5 rounded bg-zinc-800 hover:bg-zinc-700 text-xs" title="${L('Поднять выше', 'Move up')}">↑</button>
            <button type="button" data-move-id="${b.id}" data-dir="1" class="move-bm-btn px-2.5 py-1.5 rounded bg-zinc-800 hover:bg-zinc-700 text-xs" title="${L('Опустить ниже', 'Move down')}">↓</button>
            <button type="button" data-del-id="${b.id}" class="del-bm-btn px-2.5 py-1.5 rounded bg-red-950/60 hover:bg-red-900/80 text-red-300 border border-red-800/50 text-xs">${L('Удалить', 'Delete')}</button>
          </div>
        </div>
      `).join('');
    }

    // ==================== TAB 2: RENDER BOUNTY HUNTER ====================
    // ---------- Shared privacy settings for every Rust network call ----------
    function getPrivacyConfig() {
      const torOn = !!(document.getElementById('torEnableCheckbox') && document.getElementById('torEnableCheckbox').checked);
      const uaEl = document.getElementById('stripUaCheckbox');
      return {
        use_tor: torOn,
        socks5_proxy: document.getElementById('torCustomProxyInput').value,
        use_onion_endpoint: torOn,
        strip_user_agent: uaEl ? uaEl.checked : true,
        onion_fallback: !document.getElementById('onionFallbackCheckbox') || document.getElementById('onionFallbackCheckbox').checked,
        airgap_mode: airgapMode === true
      };
    }

    // ---------- Local storage helpers (never throw) ----------
    function loadJson(key, fallback) {
      try { const v = localStorage.getItem(key); return v ? JSON.parse(v) : fallback; } catch (e) { return fallback; }
    }
    function saveJson(key, value) {
      try { localStorage.setItem(key, JSON.stringify(value)); } catch (e) {}
    }

    // ---------- Bounty sources: live SN bounties, GitHub issues, own entries ----------
    // GitHub issues come from Rust (github.rs). Amounts follow the Stacker News
    // README table: difficulty label × priority multiplier, an upper bound.
    // Issues without a difficulty label have no amount; nothing is guessed.
    let ghLive = loadJson('sn_pulse_gh_cache', null);          // { ts, viaTor, issues }
    let bountyTracker = loadJson('sn_pulse_bounty_tracker', {}); // { id: { status, hours } }
    let customBounties = loadJson('sn_pulse_custom_bounties', []);
    let ghPendingQuery = null;

    const GH_DIFF_GROUP = {
      'difficulty:good-first-issue': 'Easy', 'difficulty:easy': 'Easy',
      'difficulty:medium': 'Medium', 'difficulty:medium-hard': 'Hard', 'difficulty:hard': 'Hard'
    };

    function ghIssueToBounty(i) {
      return {
        id: 'gh-' + i.number,
        title: `${i.title} (#${i.number})`,
        source: 'GITHUB_PR',
        territory: 'github/stacker.news',
        rewardSats: i.award_sats || 0,
        awardKnown: i.award_sats != null,
        awardNote: i.difficulty ? `${i.difficulty}${i.priority ? ' × ' + i.priority : ''}` : '',
        estHours: i.est_hours || 0,
        difficulty: GH_DIFF_GROUP[i.difficulty] || '—',
        status: i.state === 'closed' ? 'CLOSED' : 'OPEN',
        author: 'stackernews/stacker.news',
        url: i.url,
        since: i.updated_at || '', sinceKind: 'updated',
        tags: (i.labels || []).filter(l => !/^difficulty:|^priority:/.test(l)).slice(0, 3),
        live: true
      };
    }

    function allBounties() {
      const sn = (snBounties || []).map(f => ({
        id: 'sn-' + f.id, title: f.title, source: 'SN_POST', territory: f.sub ? '~' + f.sub : 'SN',
        rewardSats: f.bounty || 0, awardKnown: !!f.bounty, awardNote: '', estHours: 0, difficulty: '—',
        status: 'OPEN', author: '@' + (f.user || 'anon'), url: f.url, tags: [],
        since: f.created_at || '', sinceKind: 'created'
      }));
      const gh = ghLive && Array.isArray(ghLive.issues) ? ghLive.issues.map(ghIssueToBounty) : [];
      const own = customBounties.map(c => ({ ...c, source: 'CUSTOM', awardKnown: c.rewardSats > 0, author: L('добавлено вами', 'added by you'), territory: L('свой трекер', 'own tracker'), tags: [] }));
      return [...sn, ...gh, ...own].map(b => {
        const t = bountyTracker[b.id] || {};
        const hours = Number(t.hours) > 0 ? Number(t.hours) : b.estHours;
        return { ...b, estHours: hours, myStatus: t.status || 'none' };
      });
    }

    const MY_STATUS = {
      none: () => L('— не отслеживаю', '— not tracking'),
      in_progress: () => L('🔥 В работе', '🔥 In progress'),
      pr_sent: () => L('📤 PR / отклик отправлен', '📤 PR / claim sent'),
      paid: () => L('✅ Выплачено мне', '✅ Paid to me')
    };

    function minutesAgo(ts) {
      const m = Math.max(0, Math.round((Date.now() - ts) / 60000));
      if (m < 1) return L('только что', 'just now');
      if (m < 60) return L(`${m} мин назад`, `${m} min ago`);
      const h = Math.round(m / 60);
      return h < 48 ? L(`${h} ч назад`, `${h} h ago`) : L(`${Math.round(h / 24)} дн назад`, `${Math.round(h / 24)} days ago`);
    }

    function renderGhStatus(extra) {
      const el = document.getElementById('ghStatusLine');
      if (!el) return;
      const allowed = document.getElementById('ghAllowToggle').checked;
      let txt;
      if (ghLive && ghLive.issues) {
        txt = L(`GitHub: ${ghLive.issues.length} задач, обновлено ${minutesAgo(ghLive.ts)} (${ghLive.viaTor ? 'через Tor' : 'без Tor'})`,
                `GitHub: ${ghLive.issues.length} issues, updated ${minutesAgo(ghLive.ts)} (${ghLive.viaTor ? 'via Tor' : 'without Tor'})`);
        if (ghLive.rateRemaining != null) txt += L(` · осталось запросов: ${ghLive.rateRemaining}`, ` · requests left: ${ghLive.rateRemaining}`);
      } else {
        txt = L('GitHub: не загружено. Список загружается только по кнопке.', 'GitHub: not loaded. The list loads only when you press the button.');
      }
      if (!allowed) txt += L(' · запросы к GitHub выключены', ' · GitHub requests are off');
      if (extra) txt += ' · ' + extra;
      el.textContent = txt;
      document.getElementById('ghFetchBtn').disabled = !allowed;
      document.getElementById('ghFetchBtn').classList.toggle('opacity-50', !allowed);
    }

    async function loadSnBounties() {
      const line = document.getElementById('snBountyStatusLine');
      if (!hasTauriInvoke()) { setLine(line, 'SN: в браузерном превью сеть отключена', 'SN: network is disabled in the browser preview'); return; }
      if (airgapMode) { setLine(line, 'SN: заблокировано Airgap', 'SN: blocked by Airgap'); return; }
      setLine(line, 'SN: загрузка…', 'SN: loading…');
      try {
        snBounties = await snInvoke('sn_fetch_sn_bounties', { privacy: getPrivacyConfig() });
        setLine(line, `SN: ${snBounties.length} активных баунти · ${new Date().toLocaleTimeString()}`,
                             `SN: ${snBounties.length} active bounties · ${new Date().toLocaleTimeString()}`);
        renderBounties();
      } catch (e) {
        setLine(line, ...errLine(e, '❌ SN: '));
      }
    }

    async function fetchGithubIssues(allowClearnet) {
      const q = ghPendingQuery != null ? ghPendingQuery : (document.getElementById('bountySearchInput').value || '');
      const statusVal = document.getElementById('bountyStatusFilter').value;
      const state = statusVal === 'PAID' ? 'closed' : statusVal === 'all' ? 'all' : 'open';
      const confirmBox = document.getElementById('ghClearnetConfirm');
      confirmBox.classList.add('hidden');
      if (airgapMode) { renderGhStatus(L('заблокировано Airgap', 'blocked by Airgap')); return; }
      if (!(window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke)) {
        renderGhStatus(L('в браузерном превью сеть отключена', 'network is disabled in the browser preview'));
        return;
      }
      const cfg = getPrivacyConfig();
      if (!cfg.use_tor && !allowClearnet) {
        ghPendingQuery = q;
        confirmBox.classList.remove('hidden');
        return;
      }
      renderGhStatus(L('загрузка…', 'loading…'));
      try {
        const res = await window.__TAURI__.core.invoke('sn_fetch_github_issues', {
          query: q || null, state, privacy: cfg, allowClearnet: !!allowClearnet
        });
        ghLive = { ts: Date.now(), viaTor: res.via_tor, rateRemaining: res.rate_limit_remaining, issues: res.issues };
        saveJson('sn_pulse_gh_cache', ghLive);
        renderBounties();
        renderGhStatus();
        showToast(L(`🐙 Загружено задач GitHub: ${res.issues.length}`, `🐙 GitHub issues loaded: ${res.issues.length}`));
        ghPendingQuery = null;
      } catch (e) {
        const msg = e && e.message ? e.message : String(e);
        if (msg === 'TOR_REQUIRED') { ghPendingQuery = q; confirmBox.classList.remove('hidden'); return; }
        ghPendingQuery = null;
        renderGhStatus('❌ ' + msg);
      }
    }

    function exportTrackerCsv() {
      const rows = allBounties().filter(b => b.myStatus !== 'none' || b.source === 'CUSTOM');
      const esc = v => `"${String(v == null ? '' : v).replace(/"/g, '""')}"`;
      const csv = ['id,title,source,my_status,reward_sats,award_known,est_hours,url']
        .concat(rows.map(b => [b.id, b.title, b.source, b.myStatus, b.rewardSats, b.awardKnown, b.estHours, b.url].map(esc).join(',')))
        .join('\n');
      downloadFile('stackerpulse_bounty_tracker.csv', csv, 'text/csv;charset=utf-8');
      showToast(L(`Трекер выгружен в CSV (${rows.length})`, `Tracker exported to CSV (${rows.length})`));
    }

    function addCustomBounty() {
      const title = document.getElementById('newBountyTitle').value.trim();
      const url = document.getElementById('newBountyUrl').value.trim();
      const sats = Math.max(0, parseInt(document.getElementById('newBountySats').value, 10) || 0);
      const hours = Math.max(0, parseFloat(document.getElementById('newBountyHours').value) || 0);
      if (!title) { showToast(L('Укажите название задачи', 'Enter a task title')); return; }
      if (url && !/^https?:\/\//i.test(url)) { showToast(L('Ссылка должна начинаться с https://', 'The link must start with https://')); return; }
      const id = 'own-' + Date.now();
      customBounties.unshift({ id, title: title.slice(0, 200), url: url.slice(0, 500), rewardSats: sats, estHours: hours || 1, difficulty: '—', status: 'OPEN' });
      bountyTracker[id] = { status: 'in_progress' };
      saveJson('sn_pulse_custom_bounties', customBounties);
      saveJson('sn_pulse_bounty_tracker', bountyTracker);
      ['newBountyTitle', 'newBountyUrl', 'newBountySats', 'newBountyHours'].forEach(i => { document.getElementById(i).value = ''; });
      document.getElementById('addBountyForm').classList.add('hidden');
      renderBounties();
      showToast(L('Задача добавлена в трекер', 'Task added to the tracker'));
    }

    function bountyAgeDays(b) {
      const t = Date.parse(b.since || '');
      return isNaN(t) ? 0 : Math.max(0, (Date.now() - t) / 86400000);
    }

    function bountyAgeLabel(b) {
      if (!b.since || isNaN(Date.parse(b.since))) return '';
      const d = Math.floor(bountyAgeDays(b));
      const n = d < 1 ? L('сегодня', 'today')
        : d < 31 ? L(`${d} дн. назад`, `${d}d ago`)
        : d < 365 ? L(`${Math.floor(d / 30)} мес. назад`, `${Math.floor(d / 30)}mo ago`)
        : L(`${Math.floor(d / 365)} г. ${Math.floor((d % 365) / 30)} мес. назад`, `${Math.floor(d / 365)}y ${Math.floor((d % 365) / 30)}mo ago`);
      const what = b.sinceKind === 'updated' ? L('активность', 'activity') : L('опубликовано', 'posted');
      const stale = d > 180;
      return `<span class="text-[11px] font-mono ${stale ? 'text-amber-400' : 'text-zinc-500'}" title="${escapeHtml(b.since)}">🕒 ${what}: ${n}${stale ? L(' · возможно, заброшено', ' · may be abandoned') : ''}</span>`;
    }

    function renderBounties() {
      const source = document.getElementById('bountySourceFilter').value;
      const status = document.getElementById('bountyStatusFilter').value;
      const diff = document.getElementById('bountyDifficultyFilter').value;
      const sort = document.getElementById('bountySortFilter').value;
      const text = (document.getElementById('bountySearchInput').value || '').trim().toLowerCase();
      const ageSel = document.getElementById('bountyAgeFilter').value;
      const maxAgeDays = ageSel === 'all' ? Infinity : Number(ageSel);
      const items = allBounties();

      const openTotal = items.filter(b => b.status === 'OPEN' && b.awardKnown && b.source !== 'CUSTOM').reduce((acc, b) => acc + b.rewardSats, 0);
      document.getElementById('bountyTotalOpenSats').textContent = `${formatNumber(openTotal)} sats`;
      document.getElementById('bountyTotalOpenUsd').textContent = `≈ ${satsToUsd(openTotal)}`;
      const mine = items.filter(b => b.myStatus === 'in_progress' || b.myStatus === 'pr_sent');
      document.getElementById('bountyMyActiveCount').textContent = mine.length;
      document.getElementById('bountyMyActiveSats').textContent = L(`${formatNumber(mine.reduce((a, b) => a + (b.awardKnown ? b.rewardSats : 0), 0))} sats в работе`, `${formatNumber(mine.reduce((a, b) => a + (b.awardKnown ? b.rewardSats : 0), 0))} sats in progress`);

      let filtered = items.filter(b => {
        if (source === 'MINE') { if (b.myStatus === 'none') return false; }
        else if (source !== 'all' && b.source !== source) return false;
        if (status === 'OPEN' && b.status !== 'OPEN') return false;
        if (status === 'PAID' && b.status === 'OPEN') return false;
        if (diff !== 'all' && b.difficulty !== diff) return false;
        if (text && !(`${b.title} ${(b.tags || []).join(' ')}`.toLowerCase().includes(text))) return false;
        // Stale bounties: SN shows them as open for years after the author left.
        // Your own tasks are never hidden by age.
        if (b.source !== 'CUSTOM' && bountyAgeDays(b) > maxAgeDays) return false;
        return true;
      });

      const rate = b => (b.awardKnown && b.estHours > 0 ? b.rewardSats / b.estHours : -1);
      filtered.sort((a, b) => sort === 'hourly_desc' ? rate(b) - rate(a) : (b.awardKnown ? b.rewardSats : -1) - (a.awardKnown ? a.rewardSats : -1));

      document.getElementById('navBountyCount').textContent = filtered.length;

      const grid = document.getElementById('bountyCardsGrid');
      if (filtered.length === 0) {
        const nothing = !snBounties && !(ghLive && ghLive.issues) && customBounties.length === 0;
        grid.innerHTML = `<div class="bg-[#1b1b1f] border border-zinc-800 rounded-xl p-6 text-center text-xs text-zinc-400 md:col-span-2">${nothing
          ? L('Пока ничего не загружено. Нажмите «⚡ Баунти SN» или «🐙 Загрузить задачи GitHub», либо добавьте свою задачу.', 'Nothing loaded yet. Press "⚡ SN bounties" or "🐙 Load GitHub issues", or add your own task.')
          : L('Задач по этим фильтрам нет. Попробуйте «Свежесть: любые».', 'No tasks match these filters. Try "Freshness: any".')}</div>`;
        return;
      }
      grid.innerHTML = filtered.map(b => {
        const hourlySats = b.awardKnown && b.estHours > 0 ? Math.round(b.rewardSats / b.estHours) : null;
        const closed = b.status !== 'OPEN';
        const srcBadge = b.source === 'GITHUB_PR'
          ? `<span class="text-[11px] font-mono px-2 py-0.5 rounded bg-purple-950 text-purple-300 border border-purple-800">🐙 GitHub PR</span>`
          : b.source === 'CUSTOM'
            ? `<span class="text-[11px] font-mono px-2 py-0.5 rounded bg-sky-950 text-sky-300 border border-sky-800">📝 ${L('Своя задача', 'Own task')}</span>`
            : `<span class="text-[11px] font-mono px-2 py-0.5 rounded bg-amber-500/10 text-[#FADA5E] border border-amber-500/30">⚡ SN bounty</span>`;
        const statusBadge = b.status === 'OPEN' ? L('🟢 ОТКРЫТО', '🟢 OPEN') : b.status === 'CLOSED' ? L('⚪ ЗАКРЫТО', '⚪ CLOSED') : L('✅ ВЫПЛАЧЕНО', '✅ PAID');
        const reward = b.awardKnown
          ? `<div class="text-lg font-extrabold font-mono text-[#FADA5E]">⚡ ${b.source === 'GITHUB_PR' ? L('до ', 'up to ') : ''}${formatNumber(b.rewardSats)} sats <span class="text-xs text-emerald-400">(${satsToUsd(b.rewardSats)})</span></div>
             ${b.awardNote ? `<div class="text-[11px] font-mono text-zinc-500">${escapeHtml(b.awardNote)}</div>` : ''}
             ${hourlySats != null ? `<div class="text-[11px] font-mono text-zinc-400">${L('Расчётная ставка', 'Estimated rate')}: ~${formatNumber(hourlySats)} ${L('sats/час', 'sats/hour')}</div>` : ''}`
          : `<div class="text-sm font-bold font-mono text-zinc-400">${L('Сумма не определена', 'No amount set')}</div>
             <div class="text-[11px] font-mono text-zinc-500">${b.source === 'GITHUB_PR' ? L('у задачи нет метки difficulty', 'the issue has no difficulty label') : L('сумма не указана', 'no amount given')}</div>`;
        return `
          <div class="bg-[#1b1b1f] border ${closed ? 'border-zinc-800 opacity-75' : 'border-zinc-700/80 hover:border-[#FADA5E]'} rounded-xl p-5 flex flex-col justify-between gap-4 transition">
            <div class="space-y-2">
              <div class="flex flex-wrap items-center justify-between gap-2">
                <div class="flex items-center gap-2">${srcBadge}<span class="text-xs font-mono text-zinc-400">${escapeHtml(b.territory)}</span></div>
                <span class="text-[11px] font-mono px-2 py-0.5 rounded ${closed ? 'bg-zinc-800 text-zinc-400' : 'bg-emerald-950 text-emerald-300 border border-emerald-700'}">${statusBadge}</span>
              </div>
              <h3 class="text-sm sm:text-base font-bold text-white">${escapeHtml(b.title)}</h3>
              ${bountyAgeLabel(b)}
              <p class="text-xs text-zinc-400">${L('Автор / Куратор', 'Author / Curator')}: <strong class="text-zinc-200">${escapeHtml(b.author)}</strong> · ${L('Сложность', 'Difficulty')}: <strong class="text-amber-300">${escapeHtml(b.difficulty)}</strong></p>
              <div class="flex flex-wrap gap-1.5 pt-1">
                ${(b.tags || []).map(t => `<span class="text-[11px] font-mono px-2 py-0.5 rounded bg-zinc-900 text-zinc-300 border border-zinc-800">${escapeHtml(t)}</span>`).join('')}
              </div>
              <div class="flex flex-wrap items-center gap-2 pt-1 text-xs">
                <select data-track-id="${escapeHtml(b.id)}" class="bounty-track-select bg-zinc-900 border border-zinc-700 rounded px-2 py-1 text-xs text-zinc-200">
                  ${Object.keys(MY_STATUS).map(k => `<option value="${k}" ${b.myStatus === k ? 'selected' : ''}>${MY_STATUS[k]()}</option>`).join('')}
                </select>
                <label class="flex items-center gap-1 text-zinc-400">${L('часов', 'hours')}:
                  <input type="number" min="0" step="0.5" data-hours-id="${escapeHtml(b.id)}" value="${b.estHours || ''}" class="bounty-hours-input w-16 bg-zinc-900 border border-zinc-700 rounded px-2 py-1 text-xs text-zinc-200" />
                </label>
                ${b.source === 'CUSTOM' ? `<button type="button" data-del-bounty="${escapeHtml(b.id)}" class="del-bounty-btn px-2 py-1 rounded bg-zinc-800 hover:bg-red-950 text-zinc-400 hover:text-red-300">✕</button>` : ''}
              </div>
            </div>
            <div class="border-t border-zinc-800 pt-3 flex items-center justify-between gap-3">
              <div>${reward}</div>
              ${b.url ? `<a href="${escapeHtml(b.url)}" target="_blank" rel="noopener noreferrer"
                class="px-3.5 py-2 rounded-lg bg-zinc-800 hover:bg-[#FADA5E] hover:text-black text-zinc-100 font-semibold text-xs transition shrink-0">
                ${L('🔗 Копировать ссылку', '🔗 Copy link')}
              </a>` : ''}
            </div>
          </div>
        `;
      }).join('');
    }

    // ==================== TAB 3: TERRITORY ECONOMICS (LIVE) ====================
    // Real numbers from SN's topSubs, computed in Rust (sn.rs): what each
    // founder actually earned, scaled to 30 days, against the plan price.

    async function loadTerritories() {
      const line = document.getElementById('terrStatusLine');
      if (!hasTauriInvoke()) { setLine(line, 'Браузерное превью: сеть отключена', 'Browser preview: network disabled'); return; }
      if (airgapMode) { setLine(line, '🔒 Airgap: загрузка заблокирована', '🔒 Airgap: loading blocked'); return; }
      const period = document.getElementById('terrPeriodSelect').value;
      setLine(line, 'Загрузка…', 'Loading…');
      try {
        territoryStats = await snInvoke('sn_fetch_territories', { period, privacy: getPrivacyConfig() });
        territoryStats.period = period;
        rememberTerritories(territoryStats.map(t => t.name));
        syncDraftTerritories(territoryStats);
        fillTrendNames();
        setLine(line, `SN: ${territoryStats.length} территорий · ${new Date().toLocaleTimeString()}`,
                             `SN: ${territoryStats.length} territories · ${new Date().toLocaleTimeString()}`);
        renderTerritoryEconomics();
      } catch (e) {
        setLine(line, ...errLine(e, '❌ '));
      }
    }

    // Post Studio: real post prices once territories are known.
    function syncDraftTerritories(list) {
      const sel = document.getElementById('draftTerritorySelect');
      const current = sel.value;
      const active = list.filter(t => t.status !== 'STOPPED').sort((a, b) => a.name.localeCompare(b.name));
      if (!active.length) return;
      sel.innerHTML = active.map(t => `<option value="${escapeHtml(t.name)}" data-cost="${Number(t.post_cost) || 0}">~${escapeHtml(t.name)} (${formatNumber(t.post_cost)} sats)</option>`).join('');
      if ([...sel.options].some(o => o.value === current)) sel.value = current;
      updatePostStudioAudit();
    }

    function planLabel(t) {
      const cost = formatNumber(t.billing_cost);
      if (t.billing_type === 'MONTHLY') return L(`${cost} / мес`, `${cost} / mo`);
      if (t.billing_type === 'YEARLY') return L(`${cost} / год`, `${cost} / yr`);
      if (t.billing_type === 'ONCE') return L(`${cost} разово`, `${cost} once`);
      return escapeHtml(t.billing_type || '—');
    }

    function verdict(t) {
      if (t.billing_type === 'ONCE') {
        return t.months_to_recoup != null
          ? `<span class="text-sky-300">${L(`окупится за ~${t.months_to_recoup} мес`, `pays back in ~${t.months_to_recoup} mo`)}</span>`
          : `<span class="text-zinc-500">${L('нет дохода', 'no revenue')}</span>`;
      }
      const sign = t.net_30d >= 0 ? '+' : '';
      return `<span class="${t.net_30d >= 0 ? 'text-emerald-400' : 'text-red-400'} font-bold">${sign}${formatNumber(t.net_30d)}</span>`;
    }

    function renderTerritoryEconomics() {
      const tbody = document.getElementById('territoryTableBody');
      const set = (id, v) => { document.getElementById(id).textContent = v; };
      if (!territoryStats) {
        tbody.innerHTML = `<tr><td colspan="10" class="py-6 text-center text-zinc-400 font-sans">${L('Нажмите «⬇ Загрузить с SN», чтобы получить реальные цифры.', 'Press "⬇ Load from SN" to get the real numbers.')}</td></tr>`;
        ['terrKpiCount', 'terrKpiRevenue', 'terrKpiCovering', 'terrKpiMedian'].forEach(id => set(id, '—'));
        if (territoryChartInstance) { territoryChartInstance.destroy(); territoryChartInstance = null; }
        return;
      }
      const all = territoryStats;
      const recurring = all.filter(t => t.covers_cost !== null && t.covers_cost !== undefined);
      const covering = recurring.filter(t => t.covers_cost);
      const sorted30 = all.map(t => t.revenue_30d).sort((a, b) => a - b);
      const median = sorted30.length ? sorted30[Math.floor(sorted30.length / 2)] : 0;
      set('terrKpiCount', formatNumber(all.length));
      set('terrKpiRevenue', `${formatNumber(all.reduce((a, t) => a + t.revenue, 0))} sats`);
      set('terrKpiCovering', `${covering.length} / ${recurring.length}`);
      set('terrKpiMedian', `${formatNumber(median)} sats`);

      const q = document.getElementById('terrTableSearch').value.trim().toLowerCase().replace(/^[~@]/, '');
      const only = document.getElementById('terrOnlyCovering').checked;
      const sort = document.getElementById('terrSortSelect').value;
      const key = { revenue: t => -t.revenue, net: t => -t.net_30d, stacked: t => -t.stacked, items: t => -t.items, post_cost: t => t.post_cost }[sort] || (t => -t.revenue);
      const rows = all
        .filter(t => !q || t.name.toLowerCase().includes(q) || (t.founder || '').toLowerCase().includes(q))
        .filter(t => !only || t.covers_cost === true || (t.billing_type === 'ONCE' && t.months_to_recoup != null))
        .sort((a, b) => key(a) - key(b));

      tbody.innerHTML = rows.length ? rows.map(t => `
        <tr class="hover:bg-zinc-900/60 transition">
          <td class="py-2 px-2 whitespace-nowrap"><a href="${escapeHtml(t.url)}" class="font-bold text-[#FADA5E] hover:underline">~${escapeHtml(t.name)}</a>
            <button type="button" data-trends="${escapeHtml(t.name)}" title="${L('Динамика территории', 'Territory trends')}" class="ml-1 px-1 rounded hover:bg-white/10">📈</button></td>
          <td class="py-2 px-2 text-zinc-300">@${escapeHtml(t.founder || '—')}</td>
          <td class="py-2 px-2 text-right text-zinc-300">${formatNumber(t.post_cost)} / ${formatNumber(t.reply_cost)}</td>
          <td class="py-2 px-2 text-zinc-300">${planLabel(t)}</td>
          <td class="py-2 px-2 text-right text-white">${formatNumber(t.revenue)}</td>
          <td class="py-2 px-2 text-right text-white">${formatNumber(t.revenue_30d)}</td>
          <td class="py-2 px-2 text-right text-zinc-400">${t.billing_type === 'ONCE' ? '—' : formatNumber(t.cost_30d)}</td>
          <td class="py-2 px-2 text-right">${verdict(t)}</td>
          <td class="py-2 px-2 text-right text-zinc-300">${formatNumber(t.stacked)}</td>
          <td class="py-2 px-2 text-right text-zinc-300">${formatNumber(t.items)}</td>
        </tr>`).join('')
        : `<tr><td colspan="10" class="py-6 text-center text-zinc-400 font-sans">${L('Нет территорий по этому фильтру.', 'No territories match this filter.')}</td></tr>`;

      // Chart: the 12 biggest earners, revenue vs plan price per 30 days
      const top = [...all].sort((a, b) => b.revenue_30d - a.revenue_30d).slice(0, 12);
      const ctx = document.getElementById('territoryRoiChart').getContext('2d');
      if (territoryChartInstance) territoryChartInstance.destroy();
      territoryChartInstance = new Chart(ctx, {
        type: 'bar',
        data: {
          labels: top.map(t => '~' + t.name),
          datasets: [
            { label: L('Доход основателя / 30 дн.', 'Founder revenue / 30 days'), data: top.map(t => t.revenue_30d),
              backgroundColor: 'rgba(250, 218, 94, 0.75)', borderRadius: 4 },
            { label: L('Цена тарифа / 30 дн.', 'Plan price / 30 days'), data: top.map(t => t.cost_30d),
              backgroundColor: 'rgba(113, 113, 122, 0.6)', borderRadius: 4 }
          ]
        },
        options: {
          responsive: true, maintainAspectRatio: false,
          plugins: { legend: { labels: { color: '#d4d4d8', font: { size: 11 } } } },
          scales: { x: { ticks: { color: '#a1a1aa' }, grid: { color: '#27272a' } },
                    y: { ticks: { color: '#a1a1aa' }, grid: { color: '#27272a' } } }
        }
      });
    }

    // ---- Territory trends (sn.rs: fetch_trends, SN's growth charts) ----
    let terrTrends = null;
    let terrTrendsChartInstance = null;
    const TREND_KIND_LABELS = {
      ZAP: ['запы авторам', 'zaps to authors'],
      TERRITORY_REVENUE: ['доход основателя', 'founder revenue'],
      REWARD: ['ежедневные награды', 'daily rewards'],
      BOUNTY_PAYMENT: ['выплаты баунти', 'bounty payouts'],
      INVITE_GIFT: ['подарки по приглашениям', 'invite gifts'],
    };

    function fillTrendNames() {
      const list = document.getElementById('terrTrendsNames');
      if (!list || !territoryStats) return;
      list.innerHTML = territoryStats.map(t => `<option value="~${escapeHtml(t.name)}"></option>`).join('');
    }

    async function loadTerritoryTrends(name) {
      const line = document.getElementById('terrTrendsStatus');
      const input = document.getElementById('terrTrendsSub');
      if (name) input.value = '~' + name;
      const sub = input.value.trim().replace(/^~/, '');
      if (!/^[A-Za-z0-9_]{1,32}$/.test(sub)) { setLine(line, 'Укажите территорию, например ~bitcoin', 'Enter a territory, e.g. ~bitcoin'); return; }
      if (!hasTauriInvoke()) { setLine(line, 'Браузерное превью: сеть отключена', 'Browser preview: network disabled'); return; }
      if (airgapMode) { setLine(line, '🔒 Airgap: загрузка заблокирована', '🔒 Airgap: loading blocked'); return; }
      const period = document.getElementById('terrTrendsPeriod').value;
      setLine(line, `Загрузка ~${sub}…`, `Loading ~${sub}…`);
      try {
        terrTrends = await snInvoke('sn_fetch_territory_trends', { sub, period, privacy: getPrivacyConfig() });
        setLine(line, `SN: ~${terrTrends.sub} · ${new Date().toLocaleTimeString()}`, `SN: ~${terrTrends.sub} · ${new Date().toLocaleTimeString()}`);
        renderTerritoryTrends();
      } catch (e) {
        terrTrends = null;
        renderTerritoryTrends();
        setLine(line, ...errLine(e, '❌ '));
      }
    }

    function trendTimeLabel(iso, granularity) {
      const d = new Date(iso);
      if (isNaN(d)) return iso;
      const loc = currentLang === 'en' ? 'en-GB' : 'ru-RU';
      if (granularity === 'hour') return d.toLocaleTimeString(loc, { hour: '2-digit', minute: '2-digit' });
      if (granularity === 'month') return d.toLocaleDateString(loc, { month: 'short', year: 'numeric' });
      return d.toLocaleDateString(loc, { day: '2-digit', month: 'short' });
    }

    function renderTerritoryTrends() {
      const set = (id, v, cls) => { const el = document.getElementById(id); el.textContent = v; if (cls) el.className = cls; };
      const kinds = document.getElementById('terrTrendsKinds');
      if (terrTrendsChartInstance) { terrTrendsChartInstance.destroy(); terrTrendsChartInstance = null; }
      if (!terrTrends) {
        ['terrTrendsEarned', 'terrTrendsSpent', 'terrTrendsItems'].forEach(id => set(id, '—'));
        set('terrTrendsChange', '—', 'text-xl font-extrabold font-mono text-zinc-300');
        kinds.replaceChildren();
        return;
      }
      const t = terrTrends;
      set('terrTrendsEarned', `${formatNumber(t.total_stacked)} sats`);
      set('terrTrendsSpent', `${formatNumber(t.total_spent)} sats`);
      set('terrTrendsItems', formatNumber(t.total_items));
      if (t.change_pct === null || t.change_pct === undefined) {
        set('terrTrendsChange', L('нет данных', 'no data'), 'text-xl font-extrabold font-mono text-zinc-500');
      } else {
        const up = t.change_pct >= 0;
        set('terrTrendsChange', `${up ? '▲ +' : '▼ '}${t.change_pct}%`,
          `text-xl font-extrabold font-mono ${up ? 'text-emerald-400' : 'text-red-400'}`);
      }

      const total = t.stacked_by_kind.reduce((a, k) => a + k.sats, 0) || 1;
      kinds.replaceChildren(...t.stacked_by_kind.map(k => {
        const span = document.createElement('span');
        span.className = 'px-2 py-1 rounded-lg bg-zinc-900 border border-zinc-800 text-zinc-300';
        const label = TREND_KIND_LABELS[k.kind] ? L(...TREND_KIND_LABELS[k.kind]) : k.kind;
        span.textContent = `${label}: ${formatNumber(k.sats)} (${Math.round(k.sats / total * 100)}%)`;
        return span;
      }));
      if (!t.stacked_by_kind.length) {
        const span = document.createElement('span');
        span.className = 'text-zinc-500';
        span.textContent = L('за этот период заработка не было', 'nothing was earned in this period');
        kinds.appendChild(span);
      }

      const ctx = document.getElementById('terrTrendsChart').getContext('2d');
      terrTrendsChartInstance = new Chart(ctx, {
        data: {
          labels: t.times.map(x => trendTimeLabel(x, t.granularity)),
          datasets: [
            { type: 'line', label: L('Заработали, sats', 'Earned, sats'), data: t.stacked, yAxisID: 'sats',
              borderColor: '#FADA5E', backgroundColor: 'rgba(250, 218, 94, 0.15)', fill: true, tension: 0.25, pointRadius: 0 },
            { type: 'line', label: L('Потратили, sats', 'Spent, sats'), data: t.spent, yAxisID: 'sats',
              borderColor: '#38bdf8', backgroundColor: 'transparent', tension: 0.25, pointRadius: 0 },
            { type: 'bar', label: L('Постов и комментариев', 'Posts and comments'), data: t.items, yAxisID: 'count',
              backgroundColor: 'rgba(161, 161, 170, 0.35)', borderRadius: 3 },
            { type: 'line', label: L('Зарабатывали (людей)', 'Earners (people)'), data: t.stackers, yAxisID: 'count',
              borderColor: '#34d399', backgroundColor: 'transparent', borderDash: [4, 3], tension: 0.25, pointRadius: 0 }
          ]
        },
        options: {
          responsive: true, maintainAspectRatio: false, interaction: { mode: 'index', intersect: false },
          plugins: { legend: { labels: { color: '#d4d4d8', font: { size: 11 } } } },
          scales: {
            x: { ticks: { color: '#a1a1aa', maxRotation: 0, autoSkip: true, maxTicksLimit: 12 }, grid: { color: '#27272a' } },
            sats: { position: 'left', beginAtZero: true, ticks: { color: '#a1a1aa' }, grid: { color: '#27272a' } },
            count: { position: 'right', beginAtZero: true, ticks: { color: '#a1a1aa', precision: 0 }, grid: { drawOnChartArea: false } }
          }
        }
      });
    }

    // ==================== TAB 4: POST STUDIO & LINK SANITIZER ====================
    // JS mirror of sanitizer.rs, used only in the browser preview. Inside the
    // app the Rust sanitizer is the source of truth.
    const GLOBAL_TRACKERS_JS = ['utm_source','utm_medium','utm_campaign','utm_term','utm_content','utm_id','fbclid','gclid','dclid','msclkid','mc_eid','_bhiiv','bhcl_id'];
    function hostIs(host, list) { return list.includes(host); }
    function hostScopedTrackersJS(host) {
      if (hostIs(host, ['x.com','www.x.com','twitter.com','www.twitter.com','mobile.twitter.com'])) return ['s','t','ref_src','ref_url'];
      if (hostIs(host, ['youtube.com','www.youtube.com','m.youtube.com','youtu.be','music.youtube.com'])) return ['si','feature','pp'];
      if (hostIs(host, ['open.spotify.com','spotify.com'])) return ['si'];
      if (hostIs(host, ['instagram.com','www.instagram.com'])) return ['igsh','igshid'];
      if (hostIs(host, ['github.com','www.github.com'])) return ['ref','ref_type'];
      return [];
    }
    function sanitizeUrlJS(raw, mode, instance) {
      const u = new URL(raw);
      const host = u.hostname.toLowerCase();
      const trackers = GLOBAL_TRACKERS_JS.concat(hostScopedTrackersJS(host));
      [...u.searchParams.keys()].forEach(k => {
        if (trackers.some(t => t.toLowerCase() === k.toLowerCase())) u.searchParams.delete(k);
      });
      const isX = hostIs(host, ['x.com','www.x.com','twitter.com','www.twitter.com']);
      const isReddit = hostIs(host, ['reddit.com','www.reddit.com','old.reddit.com']);
      const isMedium = host === 'medium.com' || host.endsWith('.medium.com');
      if (mode === 'xcancel' && isX) u.hostname = 'xcancel.com';
      else if (mode === 'nitter' && isX) u.hostname = instance || 'nitter.net';
      else if (mode === 'redlib' && isReddit) u.hostname = instance || 'redlib.catsarch.com';
      else if (mode === 'scribe' && isMedium) u.hostname = 'scribe.rip';
      let out = u.toString();
      if ([...u.searchParams.keys()].length === 0) out = out.replace(/\?$/, '');
      return out;
    }

    async function sanitizeUrlNow() {
      const raw = document.getElementById('rawUrlInput').value.trim();
      const mode = document.getElementById('privacyFrontendSelect').value;
      const instEl = document.getElementById('privacyFrontendInstance');
      const instance = instEl ? instEl.value.trim() : '';
      if (!raw) {
        document.getElementById('sanitizedUrlOutput').textContent = L('Введите URL выше', 'Enter a URL above');
        return;
      }

      let cleaned = raw;
      if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
        try {
          const res = await window.__TAURI__.core.invoke('sn_sanitize_url', { rawUrl: raw, mode, instance: instance || null });
          cleaned = res.sanitized;
        } catch (e) {
          document.getElementById('sanitizedUrlOutput').textContent = '❌ ' + (e && e.message ? e.message : String(e));
          return;
        }
      } else {
        try {
          cleaned = sanitizeUrlJS(raw, mode, /^[a-z0-9.-]+\.[a-z]{2,}$/i.test(instance) ? instance.toLowerCase() : '');
        } catch (e) {
          document.getElementById('sanitizedUrlOutput').textContent = L('❌ Некорректный URL', '❌ Invalid URL');
          return;
        }
      }

      document.getElementById('sanitizedUrlOutput').textContent = cleaned;

      // Check against existing feed URLs for dupe warning
      const norm = cleaned.replace(/^https?:\/\/[^\/]+/i, '').toLowerCase();
      const dupe = feedItems.find(it => it.externalUrl && it.externalUrl.toLowerCase().includes(norm) && norm.length > 6);
      const badge = document.getElementById('dupeCheckBadge');
      if (dupe) {
        badge.className = 'text-[11px] px-2 py-0.5 rounded bg-amber-950 text-amber-300 border border-amber-700';
        setLine(badge, `⚠️ Возможный дубль: #${dupe.id} (@${dupe.user})`, `⚠️ Possible duplicate: #${dupe.id} (@${dupe.user})`);
      } else {
        badge.className = 'text-[11px] px-2 py-0.5 rounded bg-emerald-950 text-emerald-300 border border-emerald-800';
        setLine(badge, '✅ Дубликатов в ленте не найдено', '✅ No duplicates in the feed');
      }
    }

    function updatePostStudioAudit() {
      const title = document.getElementById('draftTitleInput').value;
      const body = document.getElementById('draftBodyInput').value;
      const sel = document.getElementById('draftTerritorySelect');
      const costAttr = sel.options[sel.selectedIndex] ? sel.options[sel.selectedIndex].getAttribute('data-cost') : '';
      const baseCost = costAttr === '' || costAttr == null ? null : parseInt(costAttr, 10);
      const boost = parseInt(document.getElementById('draftBoostInput').value, 10) || 0;
      const totalCost = (baseCost || 0) + boost;

      document.getElementById('charCountBadge').textContent = L(`Заголовок: ${title.length} симв. (${title.length >= 35 && title.length <= 85 ? 'Оптимально' : 'Рекомендуется 35–85'})`, `Title: ${title.length} chars (${title.length >= 35 && title.length <= 85 ? 'Optimal' : 'Recommended 35–85'})`);
      document.getElementById('postTotalCostBadge').textContent = baseCost == null
        ? L(`Цена поста неизвестна — загрузите территории${boost ? ` (+ boost ${formatNumber(boost)})` : ''}`, `Post cost unknown — load territories${boost ? ` (+ boost ${formatNumber(boost)})` : ''}`)
        : L(`Итого: ${formatNumber(totalCost)} sats (${satsToUsd(totalCost)})`, `Total: ${formatNumber(totalCost)} sats (${satsToUsd(totalCost)})`);

      const headings = body.split('\n').filter(line => /^#{1,3}\s+.+/.test(line.trim()));
      const tocBox = document.getElementById('tocPreviewBox');

      if (headings.length >= 2) {
        tocBox.innerHTML = `
          <div class="text-emerald-400 font-semibold">${L(`✅ Авто-Оглавление Stacker News активируется (подзаголовков: ${headings.length}):`, `✅ Stacker News Auto-TOC will activate (${headings.length} headings):`)}</div>
          <ul class="list-disc list-inside text-zinc-300 font-mono space-y-1 pt-1">
            ${headings.map(h => `<li>${escapeHtml(h.replace(/^#{1,3}\s+/, ''))}</li>`).join('')}
          </ul>
        `;
      } else {
        tocBox.innerHTML = `
          <div class="text-amber-300 font-semibold">${L('ℹ️ Авто-Оглавление (TOC) пока не активно', 'ℹ️ Auto-TOC is not active yet')}</div>
          <p class="text-zinc-400">${L('Согласно', 'Per the')} <a href="https://stacker.news/guide" target="_blank" class="underline text-[#FADA5E]">Stacker News Content Guidelines</a>${L(', добавьте минимум 2 подзаголовка', ', add at least 2 headings')} (<code class="text-white">## ${L('Раздел', 'Section')}</code>)${L(', чтобы SN автоматически сгенерировал навигационное оглавление.', ' so SN generates a table of contents automatically.')}</p>
        `;
      }
    }

    function renderDraftQueue() {
      document.getElementById('draftQueueCount').textContent = draftQueue.length;
      const list = document.getElementById('draftQueueList');
      if (draftQueue.length === 0) {
        list.innerHTML = `<div class="text-xs text-zinc-500 py-3 text-center">${L('Очередь черновиков пуста.', 'The draft queue is empty.')}</div>`;
        return;
      }
      list.innerHTML = draftQueue.map(d => `
        <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-3 flex items-center justify-between gap-2 text-xs">
          <div class="min-w-0">
            <div class="font-bold text-white truncate">${escapeHtml(d.title)}</div>
            <div class="font-mono text-[11px] text-zinc-400 mt-0.5">
              <span class="text-[#FADA5E]">~${d.territory}</span> · 🕒 ${d.scheduledFor.replace('T', ' ')} · ⚡ ${formatNumber(d.cost + d.boost)} sats · TOC: ${d.tocCount}
            </div>
          </div>
          <button type="button" data-del-draft="${d.id}" class="del-draft-btn px-2 py-1 rounded bg-zinc-800 hover:bg-red-950 text-zinc-400 hover:text-red-300 shrink-0">✕</button>
        </div>
      `).join('');
    }

    // ==================== TAB 5: STACKER ANALYTICS ====================
    // Public stats of one stacker for every period, from SN through Rust.
    const STACKER_PERIODS = ['day', 'week', 'month', 'year', 'forever'];
    let stackerData = null; // { name, url, byPeriod: { period: stats } }

    async function loadStacker() {
      const line = document.getElementById('stackerStatusLine');
      const raw = document.getElementById('stackerNameInput').value.trim().replace(/^@/, '');
      if (!/^[A-Za-z0-9_]{1,32}$/.test(raw)) { setLine(line, 'Ник: только латиница, цифры и _', 'Nym: letters, digits and _ only'); return; }
      if (!hasTauriInvoke()) { setLine(line, 'Браузерное превью: сеть отключена', 'Browser preview: network disabled'); return; }
      if (airgapMode) { setLine(line, '🔒 Airgap: загрузка заблокирована', '🔒 Airgap: loading blocked'); return; }
      setLine(line, 'Загрузка…', 'Loading…');
      try {
        const results = await Promise.all(STACKER_PERIODS.map(period =>
          snInvoke('sn_fetch_stacker', { name: raw, period, privacy: getPrivacyConfig() })));
        const byPeriod = {};
        STACKER_PERIODS.forEach((p, i) => { byPeriod[p] = results[i]; });
        stackerData = { name: results[0].name, url: results[0].url, byPeriod };
        setLine(line, `@${stackerData.name} · загружено ${new Date().toLocaleTimeString()}`, `@${stackerData.name} · loaded ${new Date().toLocaleTimeString()}`);
        renderStackerAnalytics();
      } catch (e) {
        stackerData = null; renderStackerAnalytics();
        setLine(line, ...errLine(e, '❌ '));
      }
    }

    function renderStackerAnalytics() {
      const set = (id, v) => { document.getElementById(id).textContent = v; };
      if (!stackerData) {
        ['profTotalStacked', 'profMonth', 'profItems', 'profStreak'].forEach(id => set(id, '—'));
        ['profTotalStackedUsd', 'profTerritories', 'profMeta'].forEach(id => set(id, ''));
        if (stackerHistoryChartInstance) { stackerHistoryChartInstance.destroy(); stackerHistoryChartInstance = null; }
        return;
      }
      const f = stackerData.byPeriod.forever, m = stackerData.byPeriod.month;
      const hidden = f.stacked == null;
      const fmt = v => v == null ? L('скрыто', 'hidden') : `${formatNumber(v)} sats`;
      set('profTotalStacked', fmt(f.stacked));
      set('profTotalStackedUsd', f.stacked != null ? `≈ ${satsToUsd(f.stacked)}` : L('пользователь скрыл статистику', 'the stacker hides their stats'));
      set('profMonth', hidden ? L('скрыто', 'hidden') : `${formatNumber(m.stacked || 0)} / ${formatNumber(m.spent || 0)}`);
      set('profItems', formatNumber(f.items));
      set('profTerritories', L(`территорий: ${f.territories}`, `territories: ${f.territories}`));
      set('profStreak', f.streak == null && f.max_streak == null ? '—' : `${f.streak ?? '—'} / ${f.max_streak ?? '—'}`);
      const since = f.member_since ? new Date(f.member_since).toLocaleDateString() : '—';
      set('profMeta', [L(`с ${since}`, `since ${since}`), f.is_contributor ? L('контрибьютор SN', 'SN contributor') : '',
                       f.has_recv_wallet ? L('принимает sats на кошелёк', 'receives sats to a wallet') : ''].filter(Boolean).join(' · '));

      const labels = STACKER_PERIODS.map(p => ({ day: L('день', 'day'), week: L('неделя', 'week'), month: L('месяц', 'month'), year: L('год', 'year'), forever: L('всё время', 'all time') })[p]);
      const ctx = document.getElementById('stackerHistoryChart').getContext('2d');
      if (stackerHistoryChartInstance) stackerHistoryChartInstance.destroy();
      stackerHistoryChartInstance = new Chart(ctx, {
        type: 'bar',
        data: {
          labels,
          datasets: [
            { label: L('Собрано, sats', 'Stacked, sats'), data: STACKER_PERIODS.map(p => stackerData.byPeriod[p].stacked || 0), backgroundColor: 'rgba(250, 218, 94, 0.75)', borderRadius: 4 },
            { label: L('Потрачено, sats', 'Spent, sats'), data: STACKER_PERIODS.map(p => stackerData.byPeriod[p].spent || 0), backgroundColor: 'rgba(56, 189, 248, 0.6)', borderRadius: 4 }
          ]
        },
        options: {
          responsive: true, maintainAspectRatio: false,
          plugins: { legend: { labels: { color: '#e4e4e7' } } },
          scales: { x: { ticks: { color: '#a1a1aa' }, grid: { color: '#27272a' } },
                    y: { type: 'logarithmic', ticks: { color: '#a1a1aa' }, grid: { color: '#27272a' } } }
        }
      });
    }

    // ==================== TAB 6: GRAPHQL PLAYGROUND & LIVE SYNC ====================
    async function executeSnGraphql(queryText) {
      const outEl = document.getElementById('gqlResponseOutput');
      if (airgapMode) {
        outEl.textContent = JSON.stringify({
          status: "BLOCKED_BY_AIRGAP_LOCK",
          author: "UBro-dev Privacy-by-Design Firewall",
          reason: "100% Offline Airgap Mode is enabled. Zero network packets were sent."
        }, null, 2);
        showToast(L('🔒 Запрос заблокирован файрволом Airgap Mode (100% Оффлайн)!', '🔒 Request blocked by the Airgap Mode firewall (100% offline)!'));
        return;
      }
      outEl.textContent = L('// Выполнение POST запроса через Rust-клиент ...', '// Sending POST request via the Rust client ...');
      const useTor = document.getElementById('torEnableCheckbox') && document.getElementById('torEnableCheckbox').checked;
      const targetEndpoint = useTor
        ? 'http://snsnsnya6h3ot563f3p566wuhfoklkg5f62hokdlaqzcaub3gf4xlxyd.onion/api/graphql'
        : 'https://stacker.news/api/graphql';
      // Network requests go only through the Rust client: it enforces the host
      // allowlist on every redirect, routes through Tor (socks5h) and honours
      // Airgap mode. A plain browser has none of that, so the preview does not
      // send anything at all.
      if (!(window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke)) {
        outEl.textContent = JSON.stringify({
          status: "NETWORK_DISABLED_IN_BROWSER_PREVIEW",
          reason: L('Запросы к Stacker News выполняются только в приложении — через Rust-клиент с allowlist и Tor. Из обычного браузера ничего не отправлено.', 'Requests to Stacker News run only inside the app, through the Rust client with the allowlist and Tor. Nothing was sent from this browser.'),
          endpoint_in_app: targetEndpoint
        }, null, 2);
        showToast(L('Браузерное превью: сеть отключена. Запустите приложение.', 'Browser preview: network is disabled. Run the app.'));
        return;
      }
      let data;
      try {
        data = await window.__TAURI__.core.invoke('sn_execute_graphql', {
          query: queryText,
          variables: {},
          privacy: getPrivacyConfig()
        });
      } catch (err) {
        outEl.textContent = JSON.stringify({
          status: "REQUEST_FAILED",
          endpoint: targetEndpoint,
          error: errText(err),
          raw: err && err.message ? err.message : String(err)
        }, null, 2);
        showToast(L('❌ Запрос GraphQL не выполнен — подробности в панели ответа', '❌ GraphQL request failed — see the response panel'));
        return;
      }
      outEl.textContent = JSON.stringify(data, null, 2);

      showToast(L('Ответ получен через нативный Rust-клиент!', 'Response received via the native Rust client!'));
    }

    // ==================== EVENT LISTENERS ====================
    window.addEventListener('DOMContentLoaded', () => {
      // Tab switching
      document.querySelectorAll('.nav-tab').forEach(btn => {
        btn.addEventListener('click', () => {
          const target = btn.getAttribute('data-tab');
          document.querySelectorAll('.nav-tab').forEach(b => {
            b.classList.remove('tab-active');
            b.classList.add('text-zinc-400');
          });
          btn.classList.add('tab-active');
          btn.classList.remove('text-zinc-400');

          document.querySelectorAll('.tab-panel').forEach(p => p.classList.add('hidden'));
          document.getElementById(`tab-${target}`).classList.remove('hidden');

          if (target === 'territories') renderTerritoryEconomics();
          if (target === 'analytics') renderStackerAnalytics();
        });
      });


      // Toast close
      document.getElementById('closeToastBtn').addEventListener('click', () => {
        document.getElementById('statusToast').classList.add('hidden');
      });

      // Live GraphQL Sync Button
      // Header "Sync": feed with the current settings + BTC price and rewards pool
      document.getElementById('syncLiveApiBtn').addEventListener('click', () => { loadFeed(); loadMarket(); });
      let marketLoaded = false;
      document.getElementById('feedLoadBtn').addEventListener('click', () => { loadFeed(); if (!marketLoaded) { marketLoaded = true; loadMarket(); } });
      const feedWhen = document.getElementById('feedWhenSelect');
      const syncWhen = () => { feedWhen.disabled = document.getElementById('feedSortSelect').value !== 'top'; feedWhen.classList.toggle('opacity-50', feedWhen.disabled); };
      document.getElementById('feedSortSelect').addEventListener('change', syncWhen);
      syncWhen();

      // Radar controls
      ['radarSearchInput', 'radarTerritorySelect', 'radarTypeSelect', 'radarSortSelect', 'radarMinSatsInput', 'hideFreebiesToggle', 'autoXcancelToggle'].forEach(id => {
        document.getElementById(id).addEventListener('input', renderRadar);
        document.getElementById(id).addEventListener('change', renderRadar);
      });

      document.getElementById('radarModeFeedBtn').addEventListener('click', () => {
        radarViewMode = 'feed';
        document.getElementById('radarFeedContainer').classList.remove('hidden');
        document.getElementById('radarBookmarksContainer').classList.add('hidden');
        document.getElementById('radarModeFeedBtn').className = 'px-3 py-1.5 rounded-lg text-xs font-semibold bg-[#FADA5E] text-black transition';
        document.getElementById('radarModeBookmarksBtn').className = 'px-3 py-1.5 rounded-lg text-xs font-semibold bg-zinc-800 text-zinc-300 hover:text-white transition';
      });

      document.getElementById('radarModeBookmarksBtn').addEventListener('click', () => {
        radarViewMode = 'bookmarks';
        document.getElementById('radarFeedContainer').classList.add('hidden');
        document.getElementById('radarBookmarksContainer').classList.remove('hidden');
        document.getElementById('radarModeBookmarksBtn').className = 'px-3 py-1.5 rounded-lg text-xs font-semibold bg-[#FADA5E] text-black transition';
        document.getElementById('radarModeFeedBtn').className = 'px-3 py-1.5 rounded-lg text-xs font-semibold bg-zinc-800 text-zinc-300 hover:text-white transition';
        renderBookmarks();
      });

      // Delegate bookmark toggle in Feed
      document.getElementById('radarFeedContainer').addEventListener('click', e => {
        const btn = e.target.closest('.toggle-bookmark-btn');
        if (!btn) return;
        const id = btn.getAttribute('data-bookmark-id');
        const existingIdx = bookmarks.findIndex(b => b.id === id);
        if (existingIdx >= 0) {
          bookmarks.splice(existingIdx, 1);
          showToast(L('Удалено из закладок', 'Removed from bookmarks'));
        } else {
          const item = feedItems.find(it => it.id === id);
          if (item) {
            bookmarks.unshift({
              id: item.id,
              title: item.title,
              user: item.user,
              sub: item.sub,
              sats: item.sats,
              tag: item.type === 'BOUNTY' ? 'bounty' : item.sub === 'devs' ? 'dev' : 'research',
              note: '',
              url: item.url
            });
            showToast(L('Пост добавлен в Умные закладки!', 'Post added to Smart Bookmarks!'));
          }
        }
        saveState();
        renderRadar();
        renderBookmarks();
      });

      // Delegate Bookmark Manager actions (reorder, delete, edit note/tag)
      document.getElementById('bookmarkTagFilter').addEventListener('change', renderBookmarks);

      document.getElementById('bookmarksList').addEventListener('click', e => {
        const moveBtn = e.target.closest('.move-bm-btn');
        if (moveBtn) {
          const id = moveBtn.getAttribute('data-move-id');
          const dir = parseInt(moveBtn.getAttribute('data-dir'), 10);
          const idx = bookmarks.findIndex(b => b.id === id);
          const targetIdx = idx + dir;
          if (idx >= 0 && targetIdx >= 0 && targetIdx < bookmarks.length) {
            const temp = bookmarks[idx];
            bookmarks[idx] = bookmarks[targetIdx];
            bookmarks[targetIdx] = temp;
            saveState();
            renderBookmarks();
          }
          return;
        }
        const delBtn = e.target.closest('.del-bm-btn');
        if (delBtn) {
          const id = delBtn.getAttribute('data-del-id');
          bookmarks = bookmarks.filter(b => b.id !== id);
          saveState();
          renderRadar();
          renderBookmarks();
          showToast(L('Закладка удалена', 'Bookmark deleted'));
        }
      });

      document.getElementById('bookmarksList').addEventListener('input', e => {
        if (e.target.classList.contains('bookmark-note-input')) {
          const id = e.target.getAttribute('data-note-id');
          const bm = bookmarks.find(b => b.id === id);
          if (bm) { bm.note = e.target.value; saveState(); }
        }
      });

      document.getElementById('bookmarksList').addEventListener('change', e => {
        if (e.target.classList.contains('bookmark-tag-select')) {
          const id = e.target.getAttribute('data-tag-id');
          const bm = bookmarks.find(b => b.id === id);
          if (bm) { bm.tag = e.target.value; saveState(); renderBookmarks(); }
        }
      });

      // Export Feed CSV & Bookmarks JSON
      document.getElementById('exportFeedCsvBtn').addEventListener('click', () => {
        const list = getFilteredFeed();
        const header = 'id,title,user,territory,type,sats,comments,signal_score,url\n';
        const rows = list.map(it => `${it.id},"${it.title.replace(/"/g, '""')}",${it.user},${it.sub},${it.type},${it.sats},${it.comments},${Math.round(it.sats / Math.max(1, it.comments))},${it.url}`).join('\n');
        downloadFile('stacker_news_feed_export.csv', header + rows, 'text/csv;charset=utf-8');
        showToast(L('CSV-таблица ленты успешно скачана!', 'Feed CSV downloaded!'));
      });

      document.getElementById('exportBookmarksJsonBtn').addEventListener('click', () => {
        downloadFile('stacker_news_bookmarks.json', JSON.stringify(bookmarks, null, 2), 'application/json');
        showToast(L('Закладки экспортированы в JSON!', 'Bookmarks exported to JSON!'));
      });

      // Bounty filters
      ['bountySourceFilter', 'bountyStatusFilter', 'bountyDifficultyFilter', 'bountySortFilter', 'bountyAgeFilter'].forEach(id => {
        document.getElementById(id).addEventListener('change', renderBounties);
      });

      // Bounty search, GitHub radar and personal tracker
      document.getElementById('bountySearchInput').addEventListener('input', renderBounties);
      const ghSearch = () => {
        renderBounties();
        if (document.getElementById('ghAllowToggle').checked) fetchGithubIssues(false);
      };
      document.getElementById('bountySearchBtn').addEventListener('click', ghSearch);
      document.getElementById('bountySearchInput').addEventListener('keydown', e => { if (e.key === 'Enter') ghSearch(); });
      document.getElementById('ghFetchBtn').addEventListener('click', () => { ghPendingQuery = null; fetchGithubIssues(false); });
      document.getElementById('snBountyFetchBtn').addEventListener('click', loadSnBounties);
      document.getElementById('ghClearnetYes').addEventListener('click', () => fetchGithubIssues(true));
      document.getElementById('ghClearnetNo').addEventListener('click', () => {
        ghPendingQuery = null;
        document.getElementById('ghClearnetConfirm').classList.add('hidden');
      });
      const ghAllow = document.getElementById('ghAllowToggle');
      ghAllow.checked = loadJson('sn_pulse_gh_allowed', false) === true;
      ghAllow.addEventListener('change', () => {
        saveJson('sn_pulse_gh_allowed', ghAllow.checked);
        if (!ghAllow.checked) document.getElementById('ghClearnetConfirm').classList.add('hidden');
        renderGhStatus();
      });
      document.getElementById('toggleAddBountyBtn').addEventListener('click', () => document.getElementById('addBountyForm').classList.toggle('hidden'));
      document.getElementById('addBountyBtn').addEventListener('click', addCustomBounty);
      document.getElementById('exportTrackerCsvBtn').addEventListener('click', exportTrackerCsv);
      const grid = document.getElementById('bountyCardsGrid');
      grid.addEventListener('change', e => {
        const sel = e.target.closest('.bounty-track-select');
        const hrs = e.target.closest('.bounty-hours-input');
        if (sel) {
          const id = sel.getAttribute('data-track-id');
          bountyTracker[id] = { ...(bountyTracker[id] || {}), status: sel.value };
          if (sel.value === 'none' && !bountyTracker[id].hours) delete bountyTracker[id];
        } else if (hrs) {
          const id = hrs.getAttribute('data-hours-id');
          const v = parseFloat(hrs.value);
          bountyTracker[id] = { ...(bountyTracker[id] || {}), hours: v > 0 ? Math.min(v, 10000) : undefined };
        } else return;
        saveJson('sn_pulse_bounty_tracker', bountyTracker);
        renderBounties();
      });
      grid.addEventListener('click', e => {
        const del = e.target.closest('.del-bounty-btn');
        if (!del) return;
        const id = del.getAttribute('data-del-bounty');
        customBounties = customBounties.filter(c => c.id !== id);
        delete bountyTracker[id];
        saveJson('sn_pulse_custom_bounties', customBounties);
        saveJson('sn_pulse_bounty_tracker', bountyTracker);
        renderBounties();
      });
      renderGhStatus();
      setInterval(() => { if (ghLive) renderGhStatus(); }, 60000);

      const uaBox = document.getElementById('stripUaCheckbox');
      uaBox.checked = loadJson('sn_pulse_strip_ua', true) !== false;
      uaBox.addEventListener('change', () => saveJson('sn_pulse_strip_ua', uaBox.checked));

      // Radar search: refresh button and Enter
      document.getElementById('radarSearchRefreshBtn').addEventListener('click', renderRadar);
      document.getElementById('radarSearchInput').addEventListener('keydown', e => { if (e.key === 'Enter') renderRadar(); });

      // Territory ROI listeners
      document.getElementById('terrLoadBtn').addEventListener('click', loadTerritories);
      document.getElementById('updCheckBtn').addEventListener('click', () => checkForUpdates(false));
      document.getElementById('updClearnetYes').addEventListener('click', () => checkForUpdates(true));
      document.getElementById('updClearnetNo').addEventListener('click', () => {
        document.getElementById('updClearnetConfirm').classList.add('hidden');
        updLine('Проверка отменена.', 'Check cancelled.');
      });
      document.getElementById('updInstallBtn').addEventListener('click', installUpdate);
      document.getElementById('terrTrendsLoadBtn').addEventListener('click', () => loadTerritoryTrends());
      document.getElementById('terrTrendsSub').addEventListener('keydown', e => { if (e.key === 'Enter') loadTerritoryTrends(); });
      document.getElementById('terrTrendsPeriod').addEventListener('change', () => { if (terrTrends) loadTerritoryTrends(); });
      document.getElementById('territoryTableBody').addEventListener('click', e => {
        const b = e.target.closest && e.target.closest('button[data-trends]');
        if (!b) return;
        loadTerritoryTrends(b.dataset.trends);
        document.getElementById('terrTrendsPanel').scrollIntoView({ behavior: 'smooth', block: 'start' });
      });
      ['terrTableSearch', 'terrSortSelect', 'terrOnlyCovering'].forEach(id => {
        document.getElementById(id).addEventListener('input', renderTerritoryEconomics);
        document.getElementById(id).addEventListener('change', renderTerritoryEconomics);
      });

      // Post Studio & URL Sanitizer listeners
      document.getElementById('sanitizeUrlBtn').addEventListener('click', sanitizeUrlNow);
      document.getElementById('rawUrlInput').addEventListener('input', sanitizeUrlNow);
      document.getElementById('privacyFrontendSelect').addEventListener('change', sanitizeUrlNow);

      document.getElementById('copyCleanUrlBtn').addEventListener('click', () => {
        const url = document.getElementById('sanitizedUrlOutput').textContent;
        copyTextToClipboard(url, L('Очищенный URL скопирован!', 'Clean URL copied!'));
      });

      ['draftTitleInput', 'draftTerritorySelect', 'draftBodyInput', 'draftBoostInput'].forEach(id => {
        document.getElementById(id).addEventListener('input', updatePostStudioAudit);
        document.getElementById(id).addEventListener('change', updatePostStudioAudit);
      });

      document.getElementById('saveDraftBtn').addEventListener('click', () => {
        const title = document.getElementById('draftTitleInput').value.trim() || L('Черновик без названия', 'Untitled draft');
        const sel = document.getElementById('draftTerritorySelect');
        const territory = sel.value;
        const cost = parseInt(sel.options[sel.selectedIndex].getAttribute('data-cost'), 10) || 50;
        const boost = parseInt(document.getElementById('draftBoostInput').value, 10) || 0;
        const scheduledFor = document.getElementById('draftScheduleTime').value || '2026-10-26T14:00';
        const body = document.getElementById('draftBodyInput').value;
        const tocCount = body.split('\n').filter(line => /^#{1,3}\s+.+/.test(line.trim())).length;

        draftQueue.unshift({
          id: 'd-' + Date.now(),
          title,
          territory,
          cost,
          boost,
          scheduledFor,
          tocCount
        });
        saveState();
        renderDraftQueue();
        showToast(L('Черновик добавлен в очередь расписания!', 'Draft added to the schedule queue!'));
      });

      document.getElementById('copyMarkdownBtn').addEventListener('click', () => {
        const title = document.getElementById('draftTitleInput').value;
        const cleanUrl = document.getElementById('sanitizedUrlOutput').textContent;
        const body = document.getElementById('draftBodyInput').value;
        copyTextToClipboard(`# ${title}\nURL: ${cleanUrl}\n\n${body}`, L('Markdown поста скопирован!', 'Post Markdown copied!'));
      });

      document.getElementById('draftQueueList').addEventListener('click', e => {
        const del = e.target.closest('.del-draft-btn');
        if (!del) return;
        const id = del.getAttribute('data-del-draft');
        draftQueue = draftQueue.filter(d => d.id !== id);
        saveState();
        renderDraftQueue();
        showToast(L('Черновик удалён из очереди', 'Draft removed from the queue'));
      });

      document.getElementById('exportIcsBtn').addEventListener('click', () => {
        const icsLines = [
          'BEGIN:VCALENDAR',
          'VERSION:2.0',
          'PRODID:-//StackerPulse//SN Scheduler//RU',
          ...draftQueue.map(d => [
            'BEGIN:VEVENT',
            L(`SUMMARY:Опубликовать на Stacker News (~${d.territory}): ${d.title}`, `SUMMARY:Publish on Stacker News (~${d.territory}): ${d.title}`),
            `DTSTART:${d.scheduledFor.replace(/[-:]/g, '')}00Z`,
            L(`DESCRIPTION:Стоимость: ${d.cost + d.boost} sats. Проверено через StackerPulse.`, `DESCRIPTION:Cost: ${d.cost + d.boost} sats. Checked with StackerPulse.`),
            'END:VEVENT'
          ].join('\n')),
          'END:VCALENDAR'
        ].join('\n');
        downloadFile('stacker_news_schedule.ics', icsLines, 'text/calendar;charset=utf-8');
        showToast(L('Файл календаря .ICS скачан!', 'Calendar .ICS file downloaded!'));
      });

      // Analytics Tab listeners
      document.getElementById('stackerLoadBtn').addEventListener('click', loadStacker);
      document.getElementById('stackerNameInput').addEventListener('keydown', e => { if (e.key === 'Enter') loadStacker(); });
      document.getElementById('downloadAnalyticsCsvBtn').addEventListener('click', () => {
        if (!stackerData) { showToast(L('Сначала загрузите стакера', 'Load a stacker first')); return; }
        const rows = ['nym,period,stacked_sats,spent_sats,items,territories'].concat(
          STACKER_PERIODS.map(p => { const d = stackerData.byPeriod[p]; return [stackerData.name, p, d.stacked ?? '', d.spent ?? '', d.items, d.territories].join(','); }));
        downloadFile(`stacker_${stackerData.name}.csv`, rows.join('\n'), 'text/csv;charset=utf-8');
        showToast(L(`CSV для @${stackerData.name} скачан`, `CSV for @${stackerData.name} downloaded`));
      });

      // GraphQL Playground listeners
      const gqlPreset = document.getElementById('gqlPresetSelect');
      const gqlTextarea = document.getElementById('gqlQueryTextarea');
      gqlTextarea.value = GQL_PRESETS.hot_items;

      gqlPreset.addEventListener('change', () => {
        gqlTextarea.value = GQL_PRESETS[gqlPreset.value] || GQL_PRESETS.hot_items;
      });

      document.getElementById('runGqlQueryBtn').addEventListener('click', () => {
        executeSnGraphql(gqlTextarea.value);
      });

      document.getElementById('copyGqlResponseBtn').addEventListener('click', () => {
        copyTextToClipboard(document.getElementById('gqlResponseOutput').textContent, L('JSON-ответ скопирован!', 'JSON response copied!'));
      });


      // ==================== RUST CORE & LIQUID GLASS CONTROLS ====================

      const rustFileSelect = document.getElementById('rustFileSelect');
      const rustCodeViewer = document.getElementById('rustCodeViewer');
      // Sources are the real files, embedded into the binary at compile time with
      // include_str! (see sn_get_rust_source in lib.rs), so what is shown here
      // can never drift from the code that actually runs.
      async function loadRustSource(key) {
        const tauri = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
        if (!tauri) {
          rustCodeViewer.textContent =
            L('// Исходный код показывается только в приложении StackerPulse.\n// В браузерном превью смотрите файлы в папке src-tauri/src/',
              '// Source code is shown only inside the StackerPulse app.\n// In the browser preview, see the files in src-tauri/src/');
          return;
        }
        try {
          rustCodeViewer.textContent = await window.__TAURI__.core.invoke('sn_get_rust_source', { name: key });
        } catch (e) {
          rustCodeViewer.textContent = L('// Не удалось загрузить исходник: ', '// Could not load the source: ') + e;
        }
      }

      rustFileSelect.addEventListener('change', () => loadRustSource(rustFileSelect.value));
      loadRustSource(rustFileSelect.value);

      document.getElementById('copyRustSourceBtn').addEventListener('click', () => {
        copyTextToClipboard(rustCodeViewer.textContent, L('Исходный код модуля Rust скопирован!', 'Rust module source copied!'));
      });


      // ==================== BILINGUAL (EN / RU), TOR SOCKS5h & SATS ESTIMATOR ====================
      const SN_ONION_HOST = 'http://snsnsnya6h3ot563f3p566wuhfoklkg5f62hokdlaqzcaub3gf4xlxyd.onion';
      const SN_CLEAR_HOST = 'https://stacker.news';

      // Texts set by id. Everything else in the markup is translated by i18n.js.
      const I18N_MAP = {
        ru: {
          appSubtitleText: 'Лента, баунти, экономика территорий, SeedScan и студия постов для Stacker News',
          syncBtnLabel: 'Обновить с SN',
          exportFeedCsvBtn: '📥 Экспорт выборки (CSV)',
          exportBookmarksJsonBtn: '💾 Экспорт закладок (JSON)',
          sanitizeUrlBtn: '✨ Очистить URL',
          copyCleanUrlBtn: 'Копировать',
          saveDraftBtn: '💾 В очередь',
          copyMarkdownBtn: '📋 Копировать MD',
          exportIcsBtn: '📆 Скачать .ICS напоминание',
          downloadAnalyticsCsvBtn: '📥 CSV',
          runGqlQueryBtn: '▶ Выполнить запрос',
          copyGqlResponseBtn: 'Копировать JSON',
          wipeLocalDataBtn: '🧹 Сбросить локальный кэш (Zero-Trace)',
          copyRustSourceBtn: '📋 Копировать код модуля Rust',
          torPanelTitle: '🧅 Маршрутизация через Tor (SOCKS5h) и скрытый сервис .onion',
          torToggleCheckboxLabel: 'Tor SOCKS5h Активен',
          torProxyPresetLabel: 'Профиль прокси Tor (Remote DNS socks5h)',
          torCustomProxyLabel: 'Свой адрес SOCKS5h / Arti (Rust Tor)',
          tabs: {
            radar: 'Радар & Закладки',
            bounties: 'Охотник за Баунти',
            territories: 'Экономика территорий',
            studio: 'Студия Постов & Очистка Ссылок',
            analytics: 'Статистика стакера',
            graphql: 'GraphQL Песочница',
            seedscan: 'SeedScan & OpSec',
            rustcore: 'Настройки и приватность'
          }
        },
        en: {
          appSubtitleText: 'Feed, bounties, territory economics, SeedScan and a post studio for Stacker News',
          syncBtnLabel: 'Refresh from SN',
          exportFeedCsvBtn: '📥 Export Feed (CSV)',
          exportBookmarksJsonBtn: '💾 Export Bookmarks (JSON)',
          sanitizeUrlBtn: '✨ Sanitize URL',
          copyCleanUrlBtn: 'Copy',
          saveDraftBtn: '💾 Queue Draft',
          copyMarkdownBtn: '📋 Copy MD',
          exportIcsBtn: '📆 Download .ICS Reminder',
          downloadAnalyticsCsvBtn: '📥 CSV',
          runGqlQueryBtn: '▶ Run Query',
          copyGqlResponseBtn: 'Copy JSON',
          wipeLocalDataBtn: '🧹 Wipe Local Cache (Zero-Trace)',
          copyRustSourceBtn: '📋 Copy Rust Module Code',
          torPanelTitle: '🧅 Tor (SOCKS5h) Routing & Official .onion Hidden Service',
          torToggleCheckboxLabel: 'Tor SOCKS5h Enabled',
          torProxyPresetLabel: 'Tor Proxy Profile (Remote DNS socks5h)',
          torCustomProxyLabel: 'Custom SOCKS5h / Arti (Rust Tor) URI',
          tabs: {
            radar: 'Radar & Bookmarks',
            bounties: 'Bounty Hunter',
            territories: 'Territory Economics',
            studio: 'Post Studio & Link Sanitizer',
            analytics: 'Stacker Stats',
            graphql: 'GraphQL Playground',
            seedscan: 'SeedScan & OpSec',
            rustcore: 'Settings & Privacy'
          }
        }
      };

      function applyLanguage(lang) {
        currentLang = lang;
        document.documentElement.lang = lang;
        const dict = I18N_MAP[lang] || I18N_MAP.ru;

        const langRuBtn = document.getElementById('langRuBtn');
        const langEnBtn = document.getElementById('langEnBtn');
        if (lang === 'en') {
          langEnBtn.className = 'px-2 py-0.5 rounded text-[11px] font-bold bg-[#FADA5E] text-black transition';
          langRuBtn.className = 'px-2 py-0.5 rounded text-[11px] font-bold text-zinc-300 hover:text-white transition';
        } else {
          langRuBtn.className = 'px-2 py-0.5 rounded text-[11px] font-bold bg-[#FADA5E] text-black transition';
          langEnBtn.className = 'px-2 py-0.5 rounded text-[11px] font-bold text-zinc-300 hover:text-white transition';
        }

        Object.keys(dict).filter(k => k !== 'tabs').forEach(id => {
          const el = document.getElementById(id);
          if (el && dict[id]) el.textContent = dict[id];
        });

        // Update nav tab titles while keeping icons and counters intact
        document.querySelectorAll('.nav-tab').forEach(btn => {
          const key = btn.getAttribute('data-tab');
          const spans = btn.querySelectorAll('span');
          if (spans.length >= 2 && dict.tabs[key]) {
            spans[1].textContent = dict.tabs[key];
          }
        });

        updateTorRoutingState(false);
        renderRadar();

        // Re-render everything that builds text in JS, then swap static text.
        [renderBookmarks, renderBounties, renderTerritoryEconomics, updatePostStudioAudit,
         renderDraftQueue, renderStackerAnalytics, sanitizeUrlNow, renderLiveSeedscanInspector,
         checkDraftOpsecGuard, renderGhStatus, renderSupport, renderTerritoryTrends, renderUpdateInfo].forEach(fn => { try { fn(); } catch (e) {} });
        const gqlOut = document.getElementById('gqlResponseOutput');
        if (gqlOut && gqlOut.textContent.includes('"ready": true')) {
          gqlOut.textContent = JSON.stringify({ hint: L('Нажмите «Выполнить запрос», чтобы отправить запрос через Rust-клиент', 'Press "Run Query" to send the request through the Rust client'), ready: true }, null, 2);
        }
        const diskOut = document.getElementById('seedscanDiskOutput');
        if (diskOut && /^(Укажите папки и нажмите «Сканировать»\.|Choose folders and press Scan\.)$/.test(diskOut.textContent)) {
          diskOut.textContent = L('Укажите папки и нажмите «Сканировать».', 'Choose folders and press Scan.');
        }
        if (window.translateStaticUi) window.translateStaticUi(lang);
        try { localStorage.setItem('sn_pulse_lang', lang); } catch (e) {}
        document.querySelectorAll('[data-ru][data-en]').forEach(el => { el.textContent = lang === 'en' ? el.dataset.en : el.dataset.ru; });
      }

      function updateTorRoutingState(notify) {
        const torCheck = document.getElementById('torEnableCheckbox');
        const isTor = torCheck && torCheck.checked;
        const quickLabel = document.getElementById('torQuickStatusLabel');
        const quickBtn = document.getElementById('torQuickToggleBtn');
        const epDisplay = document.getElementById('activeEndpointDisplay');
        const proxyUri = document.getElementById('torCustomProxyInput').value.trim() || 'socks5h://127.0.0.1:9050';

        // Rewrite item URLs between clearnet and official SN .onion service
        const useOnion = isTor && !onionDown;
        feedItems.forEach(it => {
          it.url = useOnion ? it.url.replace(SN_CLEAR_HOST, SN_ONION_HOST) : it.url.replace(SN_ONION_HOST, SN_CLEAR_HOST);
        });
        bookmarks.forEach(bm => {
          bm.url = useOnion ? bm.url.replace(SN_CLEAR_HOST, SN_ONION_HOST) : bm.url.replace(SN_ONION_HOST, SN_CLEAR_HOST);
        });

        if (isTor) {
          quickLabel.textContent = currentLang === 'en' ? 'Tor: ON (.onion SOCKS5h)' : 'Tor: ВКЛ (.onion SOCKS5h)';
          quickBtn.className = 'px-2.5 py-0.5 rounded-full bg-emerald-500/25 text-emerald-200 border border-emerald-400/60 transition flex items-center gap-1 font-bold';
          epDisplay.textContent = `${proxyUri} ➔ ${SN_ONION_HOST}/api/graphql`;
        } else {
          quickLabel.textContent = currentLang === 'en' ? 'Tor: OFF (Clearnet)' : 'Tor: ВЫКЛ (Clearnet)';
          quickBtn.className = 'px-2.5 py-0.5 rounded-full bg-purple-500/15 hover:bg-purple-500/30 text-purple-200 border border-purple-400/40 transition flex items-center gap-1';
          epDisplay.textContent = 'https://stacker.news/api/graphql (Direct TLS 1.3 · No-Referrer)';
        }

        renderRadar();
        renderBookmarks();
        if (notify) {
          showToast(isTor
            ? (currentLang === 'en' ? '🧅 Tor SOCKS5h & .onion v3 routing enabled!' : '🧅 Трафик перенаправлен через Tor SOCKS5h на .onion зеркало Stacker News!')
            : (currentLang === 'en' ? 'Direct Clearnet TLS mode enabled' : 'Включён прямой режим Clearnet TLS без телеметрии'));
        }
      }

      document.getElementById('langRuBtn').addEventListener('click', () => applyLanguage('ru'));
      document.getElementById('langEnBtn').addEventListener('click', () => applyLanguage('en'));

      document.getElementById('torQuickToggleBtn').addEventListener('click', () => {
        const cb = document.getElementById('torEnableCheckbox');
        cb.checked = !cb.checked;
        updateTorRoutingState(true);
        if (cb.checked) detectTor(true);
      });

      // Looks for Tor on 127.0.0.1:9150 (Tor Browser) and :9050 (tor service).
      // Only opens a local TCP connection; nothing is sent.
      async function detectTor(quiet) {
        const line = document.getElementById('torDetectStatus');
        if (!hasTauriInvoke()) { setLine(line, 'Браузерное превью: проверка недоступна', 'Browser preview: check unavailable'); return; }
        setLine(line, 'Ищу Tor…', 'Looking for Tor…');
        let ports = [];
        try { ports = await window.__TAURI__.core.invoke('sn_detect_tor'); } catch (e) { setLine(line, ...errLine(e, '❌ ')); return; }
        const input = document.getElementById('torCustomProxyInput');
        const cur = (/:(\d+)\s*$/.exec(input.value.trim()) || [])[1];
        if (!ports.length) {
          setLine(line, '❌ Tor не найден на портах 9150 и 9050. Запустите Tor Browser и дождитесь подключения.',
                        '❌ Tor not found on ports 9150 or 9050. Start Tor Browser and wait until it connects.');
          if (!quiet) showToast(L('🧅 Tor не найден: запустите Tor Browser', '🧅 Tor not found: start Tor Browser'));
          return;
        }
        if (cur && ports.includes(Number(cur))) {
          setLine(line, `✅ Tor отвечает на порту ${cur}`, `✅ Tor answers on port ${cur}`);
          return;
        }
        const port = ports[0];
        input.value = `socks5h://127.0.0.1:${port}`;
        const preset = document.getElementById('torProxyPresetSelect');
        if ([...preset.options].some(o => o.value === input.value)) preset.value = input.value;
        saveJson('sn_pulse_tor_proxy', input.value);
        updateTorRoutingState(false);
        setLine(line, `✅ Tor найден на порту ${port}${port === 9150 ? ' (Tor Browser)' : ' (служба tor)'} — переключено`,
                      `✅ Tor found on port ${port}${port === 9150 ? ' (Tor Browser)' : ' (tor service)'} — switched`);
        if (!quiet) showToast(L(`🧅 Tor найден на порту ${port}, адрес прокси обновлён`, `🧅 Tor found on port ${port}, proxy address updated`));
      }

      (function restoreTorSettings() {
        const saved = loadJson('sn_pulse_tor_proxy', null);
        if (typeof saved === 'string' && /^socks5h:\/\/[\w.\-]+:\d{1,5}$/.test(saved)) {
          document.getElementById('torCustomProxyInput').value = saved;
          const preset = document.getElementById('torProxyPresetSelect');
          if ([...preset.options].some(o => o.value === saved)) preset.value = saved;
        }
        const fb = document.getElementById('onionFallbackCheckbox');
        if (fb) fb.checked = loadJson('sn_pulse_onion_fallback', true) !== false;
      })();

      document.getElementById('torEnableCheckbox').addEventListener('change', () => {
        updateTorRoutingState(true);
        if (document.getElementById('torEnableCheckbox').checked) detectTor(true);
      });
      document.getElementById('torDetectBtn').addEventListener('click', () => detectTor(false));
      document.getElementById('onionRetryBtn').addEventListener('click', async () => {
        await refreshOnionStatus(true);
        showToast(L('🧅 Следующая загрузка снова попробует .onion', '🧅 The next load will try the .onion again'));
      });
      document.getElementById('onionFallbackCheckbox').addEventListener('change', e => saveJson('sn_pulse_onion_fallback', e.target.checked));
      document.getElementById('torProxyPresetSelect').addEventListener('change', e => {
        document.getElementById('torCustomProxyInput').value = e.target.value;
        saveJson('sn_pulse_tor_proxy', e.target.value);
        updateTorRoutingState(true);
      });
      document.getElementById('torCustomProxyInput').addEventListener('input', e => {
        if (/^socks5h:\/\/[\w.\-]+:\d{1,5}$/.test(e.target.value.trim())) saveJson('sn_pulse_tor_proxy', e.target.value.trim());
        updateTorRoutingState(false);
      });

      document.getElementById('wipeLocalDataBtn').addEventListener('click', () => {
        try {
          localStorage.removeItem('sn_pulse_bookmarks');
          localStorage.removeItem('sn_pulse_drafts');
          localStorage.removeItem('sn_pulse_lang');
          ['sn_pulse_gh_cache', 'sn_pulse_bounty_tracker', 'sn_pulse_custom_bounties', 'sn_pulse_gh_allowed', 'sn_pulse_strip_ua', 'sn_pulse_tor_proxy', 'sn_pulse_onion_fallback'].forEach(k => localStorage.removeItem(k));
        } catch (e) {}
        bookmarks = []; draftQueue = []; bountyTracker = {}; customBounties = []; ghLive = null;
        feedItems = []; snBounties = null; territoryStats = null; stackerData = null; lastFeedError = null;
        setLine(document.getElementById('apiSourceBadge'), 'Не загружено', 'Not loaded');
        setLine(document.getElementById('snBountyStatusLine'), 'SN: не загружено.', 'SN: not loaded.');
        setLine(document.getElementById('terrStatusLine'), 'Не загружено.', 'Not loaded.');
        setLine(document.getElementById('stackerStatusLine'), 'Введите ник и нажмите «Загрузить».', 'Enter a nym and press Load.');
        renderRadar(); renderBookmarks(); renderBounties(); renderDraftQueue(); renderTerritoryEconomics(); renderStackerAnalytics();
        showToast(L('🧹 Локальные данные удалены: закладки, черновики, трекер, кэш GitHub.', '🧹 Local data wiped: bookmarks, drafts, tracker, GitHub cache.'));
      });



      // ==================== UBRO-DEV SEEDSCAN, OPSEC GUARD & PRIVACY BY DESIGN ====================



      function maskSecret(str, edge) {
        if (!str || str.length <= edge * 2 + 4) return '**********';
        return str.slice(0, edge) + '**********' + str.slice(-edge);
      }

      // Same rules as detect.rs: the NWC secret sits at the end of the URI,
      // so an edge-preserving mask would print part of it. Redact the whole
      // value instead, and never show the username a path leak warns about.
      function maskNwc() {
        return 'nostr+walletconnect://<redacted>?...&secret=****************';
      }
      const NON_PERSONAL_USERS = new Set(['public','default','user','username','you','yourname','your_name','me','name']);

      function hasTauriInvoke() {
        return !!(window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke);
      }

      // Inside the app the Rust detector is the source of truth (checksummed
      // BIP39, base58check, bech32). The JS version below is only a rough
      // stand-in for the browser preview.
      // Rust sends English labels; show them in the interface language.
      // level: 'secret' (red), 'warn' (amber: name/IP/possible seed), 'info' (test data).
      const FINDING_TEXT = {
        Bip39Mnemonic:      ['secret', 'Seed-фраза BIP39', 'Кто прочитает этот текст, получит доступ к кошельку. Перенесите фразу офлайн (бумага, аппаратный кошелёк) и удалите файл.'],
        PossibleMnemonic:   ['secret', 'Похоже на seed-фразу (контрольная сумма не сходится)', 'Слова из словаря BIP39 подряд, но контрольная сумма не совпадает. Если это ваша фраза с опечаткой или без одного слова — её всё равно можно подобрать. Относитесь к ней как к настоящей.'],
        ExtendedPrivateKey: ['secret', 'Расширенный приватный ключ (xprv)', 'Из xprv выводятся все ключи кошелька. Это то же самое, что seed-фраза.'],
        WifPrivateKey:      ['secret', 'Приватный ключ WIF', 'Ключ одного адреса. Переведите средства на новый адрес и удалите файл.'],
        NostrSecretKey:     ['secret', 'Секретный ключ Nostr (nsec)', 'nsec — это ваша личность в Nostr: с ним можно писать от вашего имени. Смените ключ, если файл куда-то попадал.'],
        SolanaKeypairFile:  ['secret', 'Файл ключа Solana', 'Незашифрованный приватный ключ (проверено: публичная половина совпадает с секретной). Держите вне облака и git.'],
        SolanaPrivateKey:   ['secret', 'Приватный ключ Solana (base58)', 'Ключ в формате экспорта кошелька. Переведите средства на новый кошелёк и удалите его.'],
        NwcWalletSecret:    ['secret', 'Секрет Lightning NWC', 'С такой ссылкой можно тратить из вашего Lightning-кошелька. Немедленно отзовите подключение в Alby/LNbits.'],
        OsUserPathLeak:     ['warn',   'Путь с именем пользователя ОС', 'Путь вида C:\\Users\\<имя>\\ выдаёт ваше настоящее имя пользователя, если опубликовать лог.'],
        EmailAddress:       ['warn',   'Адрес e-mail', 'Почта может связать текст с вами. Уберите её перед публикацией.'],
        PublicIpAddress:    ['warn',   'Публичный IP-адрес', 'IP может выдать ваше местоположение или провайдера. Уберите его из лога перед публикацией.'],
      };
      function findingText(f) {
        const row = FINDING_TEXT[f.kind];
        if (f.test_vector) {
          return {
            level: 'info',
            label: L(`${row ? row[1] : f.label} — публичный тестовый пример`, f.label),
            advice: L('Известный пример из спецификации или документации, а не чей-то ключ. Средства, отправленные на него, боты забирают за секунды.', f.advice),
          };
        }
        if (!row) return { level: 'secret', label: f.label, advice: f.advice };
        return { level: row[0], label: L(row[1], f.label), advice: L(row[2], f.advice) };
      }

      async function scanText(text) {
        if (hasTauriInvoke()) {
          try {
            const res = await window.__TAURI__.core.invoke('sn_scan_text_secrets', { content: text || '' });
            return res.map(f => { const t = findingText(f); return { line: f.line, kind: t.label, masked: f.masked, advice: t.advice, level: t.level }; });
          } catch (e) {
            return [{ line: 0, kind: L('Сканер Rust недоступен', 'Rust scanner unavailable'), masked: '—', advice: 'IPC error: ' + (e && e.message ? e.message : String(e)) }];
          }
        }
        return scanTextForSecretsJS(text);
      }

      function scanTextForSecretsJS(text) {
        const findings = [];
        const lines = (text || '').split('\n');

        // Whole-file Solana 64-byte keypair array check
        const trimmed = (text || '').trim();
        if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
          try {
            const arr = JSON.parse(trimmed);
            if (Array.isArray(arr) && arr.length === 64 && arr.every(n => Number.isInteger(n) && n >= 0 && n <= 255)) {
              findings.push({
                line: 0,
                kind: 'Solana keypair file',
                masked: '[64-byte secret key array]',
                advice: 'Unencrypted Solana keypair file. Keep out of synced folders and git.'
              });
            }
          } catch (e) {}
        }

        lines.forEach((line, idx) => {
          const lineno = idx + 1;
          if (line.length > 20000) return;

          // 1. BIP39 seed phrase detection (12/15/18/21/24 words)
          const words = (line.toLowerCase().match(/[a-z]{3,8}/g) || []);
          const validLengths = [24, 21, 18, 15, 12];
          for (const len of validLengths) {
            if (words.length >= len) {
              // Check known BIP39 patterns or repeated test vectors / seed keywords
              const w0 = words[0];
              const wLast = words[len - 1];
              const bip39SampleWords = new Set(['abandon','ability','able','about','above','absent','absorb','abstract','absurd','abuse','access','accident','account','accuse','achieve','acid','acoustic','acquire','across','act','action','actor','actress','actual','adapt','add','addict','address','adjust','admit','adult','advance','advice','aerobic','affair','afford','afraid','again','age','agent','agree','ahead','aim','air','airport','aisle','alarm','album','alcohol','alert','alien','all','alley','allow','almost','alone','alpha','already','also','alter','always','amateur','amazing','among','amount','amused','analyst','anchor','ancient','anger','angle','angry','animal','ankle','announce','annual','another','answer','antenna','antique','anxiety','any','apart','apology','appear','apple','approve','april','arch','arctic','area','arena','argue','arm','armed','armor','army','around','arrange','arrest','arrive','arrow','art','artefact','artist','artwork','ask','aspect','assault','asset','assist','assume','asthma','athlete','atom','attack','attend','attitude','attract','auction','audit','august','aunt','author','auto','autumn','average','avocado','avoid','awake','aware','away','awesome','awful','awkward','axis','baby','bachelor','bacon','badge','bag','balance','balcony','ball','bamboo','banana','banner','bar','barely','bargain','barrel','base','basic','basket','battle','beach','bean','beauty','because','become','beef','before','begin','behave','behind','believe','below','belt','bench','benefit','best','betray','better','between','beyond','bicycle','bid','bike','bind','biology','bird','birth','bitter','black','blade','blame','blanket','blast','bleak','bless','blind','blood','blossom','blouse','blue','blur','blush','board','boat','body','boil','bomb','bone','bonus','book','boost','border','boring','borrow','boss','bottom','bounce','box','boy','bracket','brain','brand','brass','brave','bread','breeze','brick','bridge','brief','bright','bring','brisk','broccoli','broken','bronze','broom','brother','brown','brush','bubble','buddy','budget','buffalo','build','bulb','bulk','bullet','bundle','bunker','burden','burger','burst','bus','business','busy','butter','buyer','buzz','zoo','wrong','worth','world','word','wood','wonder','woman','wolf','witness','wish','wise','wire','winter','winner','wing','wine','window','wind','win','will','wild','wife','wide','width','widget','whisper','whip','where','wheat','wheel','whale','west','wet','whale','what']);
              const bipHits = words.slice(0, len).filter(w => bip39SampleWords.has(w)).length;
              if (bipHits >= Math.floor(len * 0.75)) {
                findings.push({
                  line: lineno,
                  kind: 'BIP39 seed phrase',
                  masked: `${w0} ***** ${wLast} (${len} words)`,
                  advice: 'Anyone who reads this controls the wallet. Move offline (hardware wallet/paper) and delete.'
                });
                break;
              }
            }
          }

          // 2. Extended private keys (xprv / yprv / zprv / tprv)
          const xprvMatches = line.match(/\b(xprv|yprv|zprv|tprv|uprv|vprv)[1-9A-HJ-NP-Za-km-z]{105,108}\b/g) || [];
          xprvMatches.forEach(m => {
            findings.push({
              line: lineno,
              kind: 'Extended private key (xprv/zprv)',
              masked: maskSecret(m, 6),
              advice: 'An xprv derives every child key of the wallet. Treat it like the seed phrase itself.'
            });
          });

          // 3. WIF private keys (5..., K..., L...)
          const wifMatches = line.match(/\b[5KL9c][1-9A-HJ-NP-Za-km-z]{50,51}\b/g) || [];
          wifMatches.forEach(m => {
            findings.push({
              line: lineno,
              kind: 'WIF private key',
              masked: maskSecret(m, 6),
              advice: 'Single-address private key. Sweep funds to a new address immediately.'
            });
          });

          // 4. Nostr secret key (nsec1...)
          const nsecMatches = line.match(/\bnsec1[02-9ac-hj-np-z]{50,70}\b/g) || [];
          nsecMatches.forEach(m => {
            findings.push({
              line: lineno,
              kind: 'Nostr secret key (nsec)',
              masked: maskSecret(m, 8),
              advice: 'Anyone holding your nsec can sign Nostr events and login as you. Rotate identity if exposed.'
            });
          });

          // 5. Lightning Nostr Wallet Connect URI with secret=
          const nwcMatches = line.match(/nostr\+walletconnect:\/\/[^\s]+secret=[0-9a-fA-F]{16,64}/g) || [];
          nwcMatches.forEach(m => {
            findings.push({
              line: lineno,
              kind: 'Lightning NWC Wallet Secret',
              masked: maskNwc(),
              advice: 'NWC URI contains a spending secret! Revoke this connection in Alby Hub / LNbits immediately.'
            });
          });

          // 6. OS Username Path Leak (Deanonymization hazard)
          const pathRe = /(\/home\/|C:\\Users\\|\/Users\/)([a-zA-Z0-9._-]+)([\/\\])/g;
          let pm;
          while ((pm = pathRe.exec(line)) !== null) {
            if (NON_PERSONAL_USERS.has(pm[2].toLowerCase())) continue;
            findings.push({
              line: lineno,
              kind: 'OS Username Path Leak (Deanonymization)',
              masked: pm[1] + '<user>' + pm[3],
              advice: 'Local OS home directory path reveals your system username. Scrub before posting logs to Stacker News.'
            });
          }
        });

        return findings;
      }

      let liveScanSeq = 0;
      async function renderLiveSeedscanInspector() {
        const text = document.getElementById('seedscanLiveTextarea').value;
        const seq = ++liveScanSeq;
        const findings = await scanText(text);
        if (seq !== liveScanSeq) return; // a newer keystroke already rendered
        const listEl = document.getElementById('seedscanLiveFindingsList');

        if (findings.length === 0) {
          listEl.innerHTML = `<div class="bg-emerald-950/40 border border-emerald-500/30 rounded-xl p-3 text-xs text-emerald-300 font-mono">${L('✅ Секретов, seed-фраз и деанонимизирующих путей не обнаружено (0 findings).', '✅ No secrets, seed phrases or deanonymizing paths found (0 findings).')}</div>`;
          return;
        }

        const STYLE = {
          secret: ['bg-red-950/40 border-red-500/40', 'text-red-300', '⚠️'],
          warn: ['bg-amber-950/40 border-amber-500/40', 'text-amber-300', '👤'],
          info: ['bg-zinc-900 border-zinc-600/60', 'text-zinc-300', '🧪'],
        };
        listEl.innerHTML = findings.map(f => { const st = STYLE[f.level] || STYLE.secret; return `
          <div class="${st[0]} border rounded-xl p-3 space-y-1 text-xs">
            <div class="flex items-center justify-between gap-2 font-mono">
              <span class="font-bold ${st[1]}">${st[2]} ${escapeHtml(f.kind)} ${f.line > 0 ? L(`(строка ${f.line})`, `(line ${f.line})`) : L('(весь текст)', '(whole text)')}</span>
              <span class="px-2 py-0.5 rounded bg-black/50 text-[#FADA5E] border border-white/10">${escapeHtml(f.masked)}</span>
            </div>
            <p class="text-zinc-300 text-[11px]">➔ ${escapeHtml(f.advice)}</p>
          </div>
        `; }).join('');
      }

      let draftScanSeq = 0;
      async function checkDraftOpsecGuard() {
        const draftEl = document.getElementById('draftBodyInput');
        const guardBox = document.getElementById('draftOpsecGuardBox');
        if (!draftEl || !guardBox) return;

        const seq = ++draftScanSeq;
        const findings = await scanText(draftEl.value);
        if (seq !== draftScanSeq) return;
        if (findings.length === 0) {
          guardBox.className = 'bg-zinc-900 border border-emerald-500/30 rounded-lg p-3 space-y-1 text-xs';
          guardBox.innerHTML = `<div class="text-emerald-400 font-semibold">🛡️ SeedScan Pre-Publish OpSec Guard (by UBro-dev): ${L('Чисто', 'Clean')}</div><p class="text-zinc-400 text-[11px]">${L('В тексте поста не найдено утечек BIP39 сид-фраз, ключей nsec/xprv/NWC и домашних путей ОС.', 'No BIP39 seed phrases, nsec/xprv/NWC keys or OS home paths found in the post.')}</p>`;
        } else {
          guardBox.className = 'bg-red-950/60 border border-red-500/60 rounded-lg p-3 space-y-1.5 text-xs';
          guardBox.innerHTML = `
            <div class="text-red-300 font-bold">${L(`🚨 ВНИМАНИЕ: Обнаружена утечка секрета или деанонимизация в черновике (${findings.length})!`, `🚨 WARNING: secret leak or deanonymization found in the draft (${findings.length})!`)}</div>
            <ul class="list-disc list-inside font-mono text-[11px] text-amber-200 space-y-0.5">
              ${findings.map(f => `<li>Line ${f.line}: <strong>${escapeHtml(f.kind)}</strong> — ${escapeHtml(f.masked)}</li>`).join('')}
            </ul>
          `;
        }
      }

      let draftGuardTimer = null;
      document.getElementById('draftBodyInput').addEventListener('input', () => { clearTimeout(draftGuardTimer); draftGuardTimer = setTimeout(checkDraftOpsecGuard, 250); });

      document.getElementById('scrubDraftPathsBtn').addEventListener('click', () => {
        const draftEl = document.getElementById('draftBodyInput');
        let txt = draftEl.value;
        txt = txt.replace(/\/home\/[a-zA-Z0-9._-]+\//g, '/home/<redacted>/');
        txt = txt.replace(/\/Users\/[a-zA-Z0-9._-]+\//g, '/Users/<redacted>/');
        txt = txt.replace(/C:\\Users\\[a-zA-Z0-9._-]+\\/g, 'C:\\Users\\<redacted>\\');
        txt = txt.replace(/nsec1[02-9ac-hj-np-z]{50,70}/g, 'nsec1<REDACTED_BY_SEEDSCAN>');
        txt = txt.replace(/nostr\+walletconnect:\/\/[^\s]+/g, 'nostr+walletconnect://<REDACTED_NWC_SECRET>');
        draftEl.value = txt;
        checkDraftOpsecGuard();
        showToast(L('🧼 Черновик очищен от деанонимизирующих путей и ключей!', '🧼 Draft scrubbed by SeedScan OpSec Guard!'));
      });

      // Same layout as seedscan-cli, in the interface language.
      function formatDiskReport(rep) {
        const out = [];
        out.push(L('SeedScan — офлайн, только чтение. С этого компьютера ничего не уходит.', 'SeedScan — offline, read-only. Nothing leaves this machine.'), '');
        (rep.roots || []).forEach(r => out.push(L('  папка: ', '  folder: ') + r));
        (rep.missing || []).forEach(r => out.push(L('  ⚠ нет такой папки, пропущена: ', '  ⚠ no such folder, skipped: ') + r));
        out.push('');
        if (!(rep.roots || []).length) {
          out.push(L('Ни одной из указанных папок не существует — проверять было нечего.',
                     'None of these folders exist — there was nothing to scan.'));
          out.push(L('Пример для Windows: C:\\Users\\<имя>\\Documents, ~/Downloads, ~/OneDrive',
                     'Windows example: C:\\Users\\<name>\\Documents, ~/Downloads, ~/OneDrive'));
          return out.join('\n');
        }
        out.push(L(`Проверено файлов: ${formatNumber(rep.scanned_files)}.`, `Scanned ${formatNumber(rep.scanned_files)} files.`));
        if (!rep.hits.length) {
          out.push('', L('Секретов в открытом виде не найдено. Архивы, PDF и офисные файлы не проверяются.',
                         'No plain-text secrets found. Archives, PDFs and office files are not checked.'));
          return out.join('\n');
        }
        // Three groups: real secrets first, then your username in files,
        // then what is almost certainly test data (spec examples, tests/ folders).
        const groups = { secret: [], name: [], test: [] };
        rep.hits.forEach(h => {
          h.findings.forEach(f => {
            const t = findingText(f);
            const g = (f.test_vector || h.test_folder) ? 'test' : (t.level === 'secret' ? 'secret' : 'name');
            groups[g].push({ h, f, t });
          });
        });
        const section = (title, items) => {
          if (!items.length) return;
          out.push('', '====================================================================', title, '====================================================================');
          let lastPath = null;
          const advices = [];
          items.forEach(({ h, f, t }) => {
            if (h.path !== lastPath) {
              if (advices.length) { [...new Set(advices)].forEach(a => out.push('    -> ' + a)); advices.length = 0; }
              out.push('', h.path); lastPath = h.path;
            }
            out.push(`  ${f.line ? L('строка ', 'line ') + String(f.line).padEnd(5) : L('весь файл ', 'whole file')}  ${t.label} — ${f.masked}`);
            advices.push(t.advice);
          });
          [...new Set(advices)].forEach(a => out.push('    -> ' + a));
        };
        out.push('', L(`Секреты: ${groups.secret.length} · ваше имя в файлах: ${groups.name.length} · тестовые данные: ${groups.test.length}`,
                       `Secrets: ${groups.secret.length} · your username in files: ${groups.name.length} · test data: ${groups.test.length}`));
        section(L('⚠️ СЕКРЕТЫ — требуют внимания', '⚠️ SECRETS — need attention'), groups.secret);
        section(L('👤 ВАШЕ ИМЯ ПОЛЬЗОВАТЕЛЯ В ФАЙЛАХ — важно, только если будете их публиковать', '👤 YOUR USERNAME IN FILES — matters only if you publish them'), groups.name);
        section(L('🧪 СКОРЕЕ ВСЕГО ТЕСТОВЫЕ ДАННЫЕ — примеры из спецификаций и папки tests/fixtures', '🧪 MOST LIKELY TEST DATA — spec examples and tests/fixtures folders'), groups.test);
        out.push('', '====================================================================', '',
          L('Секреты показаны замаскированными. SeedScan ничего не копировал, не перемещал и не отправлял.',
            'Secrets are shown masked. SeedScan did not copy, move or send anything.'),
          L('Если файл когда-либо был в облаке, бэкапе, чате или git — считайте секрет скомпрометированным и переведите средства.',
            'If a file was ever in a synced folder, a backup, a chat or git — assume the secret is compromised and move the funds.'));
        return out.join('\n');
      }

      document.getElementById('runDiskSeedscanBtn').addEventListener('click', async () => {
        const pathsRaw = document.getElementById('seedscanPathsInput').value;
        const maxMb = parseInt(document.getElementById('seedscanMaxSizeInput').value, 10) || 8;
        const hidden = document.getElementById('seedscanHiddenToggle').checked;
        const outPre = document.getElementById('seedscanDiskOutput');

        if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
          const btn = document.getElementById('runDiskSeedscanBtn');
          btn.disabled = true; btn.classList.add('opacity-50');
          outPre.textContent = L('Сканирую… Большие папки могут занять минуту. Файлы только читаются, в сеть ничего не уходит.',
                                 'Scanning… Large folders can take a minute. Files are only read; nothing goes online.');
          try {
            const paths = pathsRaw.split(',').map(s => s.trim()).filter(Boolean);
            const rep = await window.__TAURI__.core.invoke('sn_run_seedscan_disk', { paths, maxSizeMb: maxMb, hidden });
            outPre.textContent = formatDiskReport(rep);
            const realSecrets = rep.hits.filter(h => !h.test_folder).flatMap(h => h.findings).filter(f => !f.test_vector && findingText(f).level === 'secret').length;
            showToast(realSecrets
              ? L(`⚠️ SeedScan: найдено секретов: ${realSecrets}`, `⚠️ SeedScan: ${realSecrets} secret(s) found`)
              : L('✅ SeedScan: настоящих секретов не найдено', '✅ SeedScan: no real secrets found'));
          } catch (e) {
            outPre.textContent = L('Сканирование не выполнено:\n', 'Scan failed:\n') + (e && e.message ? e.message : String(e));
          } finally {
            btn.disabled = false; btn.classList.remove('opacity-50');
          }
          return;
        }

        outPre.textContent = L('DEMO — браузерное превью, диск НЕ сканировался. Пример вывода:', 'DEMO — browser preview, the disk was NOT scanned. Sample output:') + `

seedscan (by UBro-dev) — offline, read-only. Nothing leaves this machine.
  scanning ${pathsRaw} (max_size_mb=${maxMb}, hidden=${hidden})

Scanned 4,812 files. Found 3 item(s) in 3 file(s).
====================================================================

~/Documents/notes/wallet-backup.txt
  line 4     BIP39 seed phrase — abandon ***** about (12 words)
    -> Anyone who reads this file controls the wallet. Move it offline
       (paper or a hardware wallet) and delete the file.

~/Downloads/alby_nwc_connection.txt
  line 1     Lightning NWC secret URI — nostr+walletconnect://<redacted>?...&secret=****************
    -> A nostr+walletconnect:// URI with a secret parameter allows spending
       from your Lightning wallet. Revoke this connection in Alby/LNbits immediately.

~/Downloads/project/id.json
  whole file  Solana keypair file — [64-byte secret key array]
    -> A Solana keypair file is an unencrypted private key. Keep it out of
       synced or shared folders, and never commit it.
====================================================================
Secrets are shown masked. seedscan did not copy, move or send anything.`;
        showToast(currentLang === 'en' ? '🔍 SeedScan disk audit complete (Offline Read-Only)' : '🔍 Локальный аудит SeedScan завершён (Offline Read-Only)');
      });

      document.getElementById('runLiveTextScanBtn').addEventListener('click', renderLiveSeedscanInspector);
      // Scan after a short pause in typing, not on every keystroke.
      let liveScanTimer = null;
      document.getElementById('seedscanLiveTextarea').addEventListener('input', () => {
        clearTimeout(liveScanTimer);
        liveScanTimer = setTimeout(renderLiveSeedscanInspector, 250);
      });
      document.getElementById('loadSampleLeakBtn').addEventListener('click', () => {
        document.getElementById('seedscanLiveTextarea').value = `my seed backup: abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about
nostr key: nsec1vl029mgpspedva04g90vltkh6fvh240zqtv9k0t9af8935ke9laqsnlfe5
nwc: nostr+walletconnect://b889ff5b1513b641e2a139f661a661364979c5beee91842f8f0ef42ab558e9d4?relay=wss://relay.damus.io&secret=71a8c14c1407c113601079c4302dab36460f0ccd0ad506f1f2dc73b5100e4f3c
crash dump in C:\Users\UBro_Admin\AppData\Local\lnd.log`;
        renderLiveSeedscanInspector();
      });

      // Airgap Mode Toggle (100% Offline Lock)
      document.getElementById('airgapQuickToggleBtn').addEventListener('click', () => {
        airgapMode = !airgapMode;
        const badge = document.getElementById('zeroTelemetryBadge');
        const btn = document.getElementById('airgapQuickToggleBtn');
        if (airgapMode) {
          badge.textContent = currentLang === 'en' ? '🔒 AIRGAP: ON (100% Offline)' : '🔒 AIRGAP: ВКЛ (100% Оффлайн)';
          btn.className = 'px-2.5 py-0.5 rounded-full bg-amber-500/25 text-[#FADA5E] border border-[#FADA5E] transition flex items-center gap-1 font-bold';
          showToast(currentLang === 'en'
            ? '🔒 Airgap Mode ENABLED: All outbound network sockets are physically blocked.'
            : '🔒 Режим Airgap ВКЛЮЧЁН: Все исходящие сетевые запросы полностью заблокированы.');
        } else {
          badge.textContent = currentLang === 'en' ? '🛡️ Privacy-by-Design · Airgap: OFF' : '🛡️ Privacy-by-Design · Airgap: ВЫКЛ';
          btn.className = 'px-2.5 py-0.5 rounded-full bg-emerald-500/15 hover:bg-emerald-500/25 text-emerald-300 border border-emerald-400/30 transition flex items-center gap-1';
          showToast(currentLang === 'en'
            ? '🛡️ Airgap OFF: Strict Allowlist (stacker.news / .onion only) active.'
            : '🛡️ Airgap ВЫКЛ: Активен строгий белый список (только stacker.news и .onion).');
        }
      });

      // Initialize SeedScan outputs. Inside the app the disk scan runs only
      // when the user presses the button; the preview shows the demo sample.
      if (!(window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke)) {
        document.getElementById('runDiskSeedscanBtn').click();
      } else {
        document.getElementById('seedscanDiskOutput').textContent = L('Укажите папки и нажмите «Сканировать».', 'Choose folders and press Scan.');
      }
      renderLiveSeedscanInspector();
      checkDraftOpsecGuard();
      showAppVersion();
      renderSupport();

      // Initial renders
      renderRadar();
      renderBookmarks();
      renderBounties();
      sanitizeUrlNow();
      updatePostStudioAudit();
      renderDraftQueue();
      document.getElementById('gqlResponseOutput').textContent = JSON.stringify({
        hint: L('Нажмите «Выполнить запрос», чтобы отправить запрос через Rust-клиент', 'Press "Run Query" to send the request through the Rust client'),
        ready: true
      }, null, 2);

      // Links are copied, never opened. Opening them in the system browser
      // would go around Tor; with Tor on, Stacker News links are copied as
      // the .onion address. Paste them into Tor Browser.
      function copyLink(e) {
        const a = e.target.closest && e.target.closest('a[href]');
        if (!a) return;
        const href = a.getAttribute('href') || '';
        if (!/^https?:\/\//i.test(href)) return;
        e.preventDefault();
        if (e.type === 'auxclick' && e.button !== 1) return;
        let url = href;
        const torOn = document.getElementById('torEnableCheckbox') && document.getElementById('torEnableCheckbox').checked;
        const toOnion = torOn && !onionDown;
        if (toOnion) url = url.replace(/^https:\/\/stacker\.news(?=\/|$)/i, SN_ONION_HOST);
        else url = url.replace(SN_ONION_HOST, SN_CLEAR_HOST);
        copyTextToClipboard(url, toOnion
          ? L('🔗 Ссылка скопирована (.onion) — вставьте её в Tor Browser', '🔗 Link copied (.onion) — paste it into Tor Browser')
          : torOn
          ? L('🔗 Ссылка скопирована — вставьте её в Tor Browser (.onion сейчас не отвечает)', '🔗 Link copied — paste it into Tor Browser (the .onion is not answering right now)')
          : L('🔗 Ссылка скопирована — вставьте её в браузер', '🔗 Link copied — paste it into your browser'));
      }
      document.addEventListener('click', copyLink);
      document.addEventListener('auxclick', copyLink);

      if (currentLang === 'en') applyLanguage('en');
    });
  
