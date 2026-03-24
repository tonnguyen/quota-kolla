// Utility functions for rendering

/**
 * Get the color class for a progress bar based on percentage
 * @param {number} percentage - Usage percentage (0-100)
 * @returns {string} - CSS class name for color
 */
function getBarColorClass(percentage) {
  if (percentage >= 90) return 'high';
  if (percentage >= 70) return 'medium';
  return 'low';
}

/**
 * Format duration in milliseconds to human-readable string
 * @param {number} ms - Duration in milliseconds
 * @returns {string} - Formatted duration (e.g., "2d 5h 30m")
 */
function formatDuration(ms) {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  const parts = [];
  if (days > 0) parts.push(`${days}d`);
  if (hours % 24 > 0) parts.push(`${hours % 24}h`);
  if (minutes % 60 > 0) parts.push(`${minutes % 60}m`);

  return parts.length > 0 ? parts.join(' ') : '0m';
}

/**
 * Format reset time from ISO string to human-readable string
 * @param {string} isoString - ISO 8601 date string
 * @returns {string} - Formatted date/time
 */
function formatReset(isoString) {
  const date = new Date(isoString);
  const now = new Date();
  const diff = date.getTime() - now.getTime();

  if (diff < 0) {
    return 'Resets soon';
  }

  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));

  if (days > 0) {
    return `Resets in ${days}d ${hours}h`;
  } else if (hours > 0) {
    return `Resets in ${hours}h`;
  } else {
    return 'Resets soon';
  }
}

/**
 * Render a single provider section
 * @param {Object} provider - Provider data object
 * @returns {HTMLElement} - Provider section element
 */
function renderProvider(provider) {
  const section = document.createElement('div');
  section.className = 'provider';

  // Calculate percentage
  const percentage = provider.limit > 0
    ? Math.round((provider.usage / provider.limit) * 100)
    : 0;

  const colorClass = getBarColorClass(percentage);

  // Provider header
  const header = document.createElement('div');
  header.className = 'provider-header';

  const icon = document.createElement('div');
  icon.className = 'provider-icon';
  icon.style.background = provider.color || '#666';

  const name = document.createElement('div');
  name.className = 'provider-name';
  name.textContent = provider.name;

  const usage = document.createElement('div');
  usage.className = 'provider-usage';
  usage.textContent = `${percentage}%`;

  header.appendChild(icon);
  header.appendChild(name);
  header.appendChild(usage);

  // Progress bar
  const progressBar = document.createElement('div');
  progressBar.className = 'progress-bar';

  const progressFill = document.createElement('div');
  progressFill.className = `progress-fill ${colorClass}`;
  progressFill.style.width = `${percentage}%`;

  progressBar.appendChild(progressFill);

  // Details
  const details = document.createElement('div');
  details.className = 'provider-details';

  const used = document.createElement('div');
  used.textContent = `${formatDuration(provider.usage)} used`;

  const total = document.createElement('div');
  total.textContent = `${formatDuration(provider.limit)} total`;

  details.appendChild(used);
  details.appendChild(total);

  // Reset time
  const reset = document.createElement('div');
  reset.className = 'provider-reset';
  reset.textContent = formatReset(provider.reset);

  section.appendChild(header);
  section.appendChild(progressBar);
  section.appendChild(details);
  section.appendChild(reset);

  return section;
}

/**
 * Render all providers
 * @param {Object} data - Data object with providers array
 */
function renderProviders(data) {
  const container = document.getElementById('providers');
  if (!container) return;

  container.innerHTML = '';

  if (!data.providers || data.providers.length === 0) {
    const empty = document.createElement('div');
    empty.className = 'provider';
    empty.textContent = 'No providers configured';
    container.appendChild(empty);
    return;
  }

  data.providers.forEach(provider => {
    const section = renderProvider(provider);
    container.appendChild(section);
  });
}

/**
 * Handle action button clicks
 * @param {string} action - Action type (preferences or quit)
 */
async function handleAction(action) {
  try {
    if (typeof window.__TAURI__ !== 'undefined') {
      const { invoke } = window.__TAURI__.core;

      switch (action) {
        case 'preferences':
          await invoke('show_preferences');
          await invoke('hide_menu');
          break;
        case 'quit':
          await invoke('quit_app');
          break;
        default:
          console.warn('Unknown action:', action);
      }
    } else {
      // Development fallback
      console.log('Action:', action);
    }
  } catch (error) {
    console.error('Error handling action:', action, error);
  }
}

/**
 * Setup event listeners
 */
function setupEventListeners() {
  // Action buttons
  document.querySelectorAll('.action-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const action = btn.getAttribute('data-action');
      if (action) {
        handleAction(action);
      }
    });
  });
}

/**
 * Initialize the menu
 */
async function init() {
  setupEventListeners();

  try {
    if (typeof window.__TAURI__ !== 'undefined') {
      const { invoke } = window.__TAURI__.core;

      // Fetch usage data from backend
      const data = await invoke('get_usage_data');
      renderProviders(data);

      // Set up refresh interval (every 30 seconds)
      setInterval(async () => {
        const updatedData = await invoke('get_usage_data');
        renderProviders(updatedData);
      }, 30000);
    } else {
      // Development fallback - render mock data
      const mockData = {
        providers: [
          {
            name: 'Claude',
            usage: 45000000,
            limit: 200000000,
            reset: new Date(Date.now() + 2 * 24 * 60 * 60 * 1000).toISOString(),
            color: '#7c4dff'
          },
          {
            name: 'ChatGPT',
            usage: 15000000,
            limit: 50000000,
            reset: new Date(Date.now() + 5 * 60 * 60 * 1000).toISOString(),
            color: '#00a8ff'
          }
        ]
      };
      renderProviders(mockData);
    }
  } catch (error) {
    console.error('Error initializing menu:', error);

    // Show error state
    const container = document.getElementById('providers');
    if (container) {
      container.innerHTML = '<div class="provider">Error loading data</div>';
    }
  }
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}
