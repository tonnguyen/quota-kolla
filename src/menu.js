const { invoke } = window.__TAURI__?.core || {};
const { getCurrentWindow } = window.__TAURI__?.window || {};

// Progress bar color helper
function getBarColorClass(percentage) {
  if (percentage >= 85) return 'red';
  if (percentage >= 60) return 'yellow';
  return 'green';
}

// Format duration (milliseconds to human-readable)
function formatDuration(ms) {
  if (ms <= 0) return 'now';
  const mins = Math.floor(ms / 60000);
  if (mins < 60) return `${mins}m`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ${mins % 60}m`;
  const days = Math.floor(hrs / 24);
  return `${days}d ${hrs % 24}h`;
}

// Format reset time from ISO string
function formatReset(isoString) {
  if (!isoString || isoString === 'N/A') return 'N/A';
  const resetDate = new Date(isoString);
  if (isNaN(resetDate.getTime())) return 'N/A';
  const now = new Date();
  const diff = resetDate - now;
  return formatDuration(diff);
}

// Render a single provider section
function renderProvider(provider) {
  const section = document.createElement('div');
  section.className = 'provider-section';

  if (provider.error) {
    section.innerHTML = `
      <div class="provider-header">
        <span class="provider-name">${provider.label}</span>
      </div>
      <div class="error-message">
        <svg class="error-icon" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/>
        </svg>
        <span>${provider.error}</span>
      </div>
    `;
    return section;
  }

  // Build window rows
  const windows = [];

  if (provider.five_hour) {
    windows.push({
      label: '5h',
      ...provider.five_hour
    });
  }

  if (provider.seven_day) {
    const label = provider.provider === 'glm' ? '30d' : '7d';
    windows.push({
      label,
      ...provider.seven_day
    });
  }

  if (provider.seven_day_opus) {
    windows.push({
      label: 'Opus',
      ...provider.seven_day_opus
    });
  }

  if (provider.seven_day_sonnet) {
    windows.push({
      label: 'Son.',
      ...provider.seven_day_sonnet
    });
  }

  const windowRows = windows.map(w => {
    const colorClass = getBarColorClass(w.utilization);
    const percentage = Math.round(w.utilization);
    const resetTime = formatReset(w.resets_at);

    return `
      <div class="window-row">
        <span class="window-label">${w.label}</span>
        <div class="progress-bar">
          <div class="progress-fill ${colorClass}" style="width: ${percentage}%"></div>
        </div>
        <span class="percentage">${percentage}%</span>
        <span class="reset-time">${resetTime}</span>
      </div>
    `;
  }).join('');

  section.innerHTML = `
    <div class="provider-header">
      <span class="provider-name">${provider.label}</span>
      <span class="reset-label">Resets in</span>
    </div>
    ${windowRows}
  `;

  return section;
}

// Render all providers
function renderProviders(data) {
  const container = document.getElementById('providers');
  container.innerHTML = '';

  if (!data || data.length === 0) {
    container.innerHTML = '<div style="text-align:center;color:var(--text-muted);padding:20px;">No providers available</div>';
    return;
  }

  data.forEach(provider => {
    container.appendChild(renderProvider(provider));
  });
}

// Handle action button clicks
async function handleAction(action) {
  if (action === 'preferences') {
    await invoke('show_preferences');
    await invoke('hide_menu');
  } else if (action === 'quit') {
    await invoke('quit_app');
  }
}

// Setup event listeners
async function setupEventListeners() {
  // Listen for usage data from Rust via Tauri event system
  const { listen } = window.__TAURI__?.event || {};
  await listen('shown', () => {
    shownAt = Date.now();
    invoke('js_log', { msg: `shown event received, shownAt=${shownAt}` });
    // Pull data on shown in case usage-data event was emitted before listener registered
    invoke('get_menu_data').then(data => {
      invoke('js_log', { msg: `get_menu_data returned: ${data?.length ?? 0} providers` });
      renderProviders(data);
    });
  });
  await listen('usage-data', (event) => {
    invoke('js_log', { msg: `usage-data received: ${event.payload?.length ?? 0} providers` });
    renderProviders(event.payload);
  });

  // Action buttons
  document.querySelectorAll('.action-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      handleAction(btn.dataset.action);
    });
  });

  // Close on Escape key
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      invoke('hide_menu');
    }
  });

  // Close when window loses focus (guard against immediate close right after showing)
  if (getCurrentWindow) {
    getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      const msSinceShown = Date.now() - shownAt;
      invoke('js_log', { msg: `onFocusChanged: focused=${focused}, msSinceShown=${msSinceShown}` });
      if (!focused) {
        if (msSinceShown > 300) {
          invoke('js_log', { msg: 'hiding menu due to focus loss' });
          invoke('hide_menu');
        } else {
          invoke('js_log', { msg: 'ignoring focus-lost - window was just shown' });
        }
      } else {
        shownAt = Date.now();
      }
    });
  }

  // Log DOMContentLoaded to confirm JS is running
  invoke('js_log', { msg: 'menu.js setupEventListeners complete' });
}

// Track when window was shown to prevent immediate hide on focus events
let shownAt = 0;

// Initialize
document.addEventListener('DOMContentLoaded', () => {
  setupEventListeners();
  // Fetch data immediately on load — the webview loads lazily on first show,
  // so events emitted by Rust before page load are missed. Pulling via invoke
  // is reliable because Rust stores data before calling window.show().
  invoke('get_menu_data').then(data => {
    invoke('js_log', { msg: `DOMContentLoaded get_menu_data: ${data?.length ?? 0} providers` });
    if (data && data.length > 0) {
      renderProviders(data);
    }
  }).catch(e => {
    invoke('js_log', { msg: `DOMContentLoaded get_menu_data error: ${e}` });
  });
});
