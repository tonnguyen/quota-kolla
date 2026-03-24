// Preferences window functionality

let currentConfig = null;

/**
 * Load current configuration from backend
 */
async function loadConfig() {
  try {
    if (typeof window.__TAURI__ !== 'undefined') {
      const { invoke } = window.__TAURI__.core;
      currentConfig = await invoke('get_preferences');
    } else {
      // Development fallback
      currentConfig = {
        version: 1,
        providers: {
          claude: { visible: true, mode: 'bar' },
          ccs: { visible: true, mode: 'bar' },
          codex: { visible: true, mode: 'bar' }
        }
      };
    }
    updateUI();
  } catch (error) {
    console.error('Failed to load config:', error);
  }
}

/**
 * Update UI elements to match current config
 */
function updateUI() {
  if (!currentConfig || !currentConfig.providers) return;

  const providers = ['claude', 'ccs', 'codex'];

  providers.forEach(providerId => {
    const config = currentConfig.providers[providerId];
    if (!config) return;

    const checkbox = document.getElementById(`${providerId}-enabled`);
    const select = document.getElementById(`${providerId}-mode`);

    if (checkbox) {
      checkbox.checked = config.visible;
    }
    if (select) {
      select.value = config.mode || 'bar';
    }
  });
}

/**
 * Gather current UI state into config object
 */
function getConfigFromUI() {
  const providers = {};
  const providerIds = ['claude', 'ccs', 'codex'];

  providerIds.forEach(providerId => {
    const checkbox = document.getElementById(`${providerId}-enabled`);
    const select = document.getElementById(`${providerId}-mode`);

    if (checkbox && select) {
      providers[providerId] = {
        visible: checkbox.checked,
        mode: select.value
      };
    }
  });

  return {
    version: 1,
    providers
  };
}

/**
 * Save configuration to backend
 */
async function saveConfig() {
  try {
    const newConfig = getConfigFromUI();

    if (typeof window.__TAURI__ !== 'undefined') {
      const { invoke } = window.__TAURI__.core;
      await invoke('save_preferences', { config: newConfig });
    } else {
      // Development fallback
      console.log('Would save config:', newConfig);
    }

    currentConfig = newConfig;
    closeWindow();
  } catch (error) {
    console.error('Failed to save config:', error);
    alert('Failed to save preferences. Please try again.');
  }
}

/**
 * Close the preferences window
 */
function closeWindow() {
  if (typeof window.__TAURI__ !== 'undefined') {
    const { getCurrentWindow } = window.__TAURI__.window;
    const window = getCurrentWindow();
    window.close();
  } else {
    // Development fallback
    console.log('Would close window');
  }
}

/**
 * Setup event listeners
 */
function setupEventListeners() {
  // Save button
  const saveBtn = document.querySelector('.btn-save');
  if (saveBtn) {
    saveBtn.addEventListener('click', saveConfig);
  }

  // Cancel button
  const cancelBtn = document.querySelector('.btn-cancel');
  if (cancelBtn) {
    cancelBtn.addEventListener('click', closeWindow);
  }

  // Close on ESC key
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      closeWindow();
    }
  });

  // Close on focus loss (optional - macOS style)
  if (typeof window.__TAURI__ !== 'undefined') {
    const { getCurrentWindow } = window.__TAURI__.window;
    const window = getCurrentWindow();
    window.onFocusChanged(({ payload: focused }) => {
      if (!focused) {
        // Uncomment to enable auto-close on focus loss
        // closeWindow();
      }
    });
  }
}

/**
 * Initialize preferences window
 */
async function init() {
  setupEventListeners();
  await loadConfig();
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}
