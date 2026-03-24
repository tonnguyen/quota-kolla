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
  const numericValue = Number(isoString);
  const resetDate = Number.isFinite(numericValue) && `${numericValue}` === `${isoString}`
    ? new Date(numericValue > 10_000_000_000 ? numericValue : numericValue * 1000)
    : new Date(isoString);
  if (isNaN(resetDate.getTime())) return 'N/A';
  const now = new Date();
  const diff = resetDate - now;
  return formatDuration(diff);
}

function getUsageWindows(provider) {
  if (Array.isArray(provider.usage_windows) && provider.usage_windows.length > 0) {
    return provider.usage_windows;
  }

  const usageWindows = [];
  if (provider.five_hour) {
    usageWindows.push({ id: 'five_hour', label: '5h', ...provider.five_hour });
  }
  if (provider.seven_day) {
    usageWindows.push({
      id: 'seven_day',
      label: provider.provider === 'glm' ? '30d' : '7d',
      ...provider.seven_day,
    });
  }
  if (provider.seven_day_opus) {
    usageWindows.push({ id: 'seven_day_opus', label: 'Opus', ...provider.seven_day_opus });
  }
  if (provider.seven_day_sonnet) {
    usageWindows.push({ id: 'seven_day_sonnet', label: 'Sonnet', ...provider.seven_day_sonnet });
  }
  return usageWindows;
}

// Render a single provider section
function renderProvider(provider) {
  const section = document.createElement('div');
  section.className = 'provider-section';

  if (provider.error) {
    section.innerHTML = `
      <div class="error-message">
        <svg class="error-icon" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/>
        </svg>
        <span class="provider-name">${provider.label}: ${provider.error}</span>
      </div>
    `;
    return section;
  }

  const usageWindows = getUsageWindows(provider);
  section.innerHTML = usageWindows.map(window => `
    <div class="usage-line">
      <div class="usage-line-top">
        <span class="usage-label">${provider.label} ${window.label}</span>
        <span class="usage-value ${getBarColorClass(window.utilization)}">${Math.round(window.utilization)}%</span>
      </div>
      <div class="usage-reset">Reset in ${formatReset(window.resets_at)}</div>
    </div>
  `).join('');

  return section;
}

// Render all providers
function renderProviders(data) {
  const container = document.getElementById('providers');
  container.innerHTML = '';

  if (!data || data.length === 0) {
    // Don't show anything on empty data - wait for data to load
    return;
  }

  data.forEach(provider => {
    container.appendChild(renderProvider(provider));
  });
}

// Handle action button clicks
async function handleAction(action) {
  if (action === 'preferences') {
    await invoke('hide_menu');
    await invoke('show_preferences');
  } else if (action === 'quit') {
    await invoke('quit_app');
  }
}

// Setup event listeners
async function setupEventListeners() {
  // Listen for usage data from Rust via Tauri event system
  const { listen } = window.__TAURI__?.event || {};

  if (!listen) {
    return;
  }

  // Register listeners (don't await - they're persistent)
  listen('shown', () => {
    shownAt = Date.now();
    // Pull data on shown in case usage-data event was emitted before listener registered
    invoke('get_menu_data').then(data => {
      renderProviders(data);
    });
  }).catch(() => {});

  listen('usage-data', (event) => {
    renderProviders(event.payload);
  }).catch(() => {});

  // Menu items
  document.querySelectorAll('.menu-item').forEach(item => {
    item.addEventListener('click', () => {
      handleAction(item.dataset.action);
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
      if (!focused) {
        if (msSinceShown > 300) {
          invoke('hide_menu');
        }
      } else {
        shownAt = Date.now();
      }
    });
  }
}

// Track when window was shown to prevent immediate hide on focus events
let shownAt = 0;

// Initialize
document.addEventListener('DOMContentLoaded', () => {
  // Check if Tauri API is available
  if (!window.__TAURI__) {
    return;
  }

  setupEventListeners();

  // Fetch data immediately on load — the webview loads lazily on first show,
  // so events emitted by Rust before page load are missed. Pulling via invoke
  // is reliable because Rust stores data before calling window.show().
  invoke('get_menu_data').then(data => {
    if (data && data.length > 0) {
      renderProviders(data);
    }
  }).catch(() => {});
});
