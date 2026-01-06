// Columbus Desktop App - Main JavaScript
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getVersion } from '@tauri-apps/api/app';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

// ==================== State ====================
let currentUser = null;
let products = [];
let selectedProductId = null;
let isScanning = false;
let isInitializing = true;

// Multi-instance state
let instances = [];
let activeInstanceId = null;
let instanceRenameId = null; // ID of instance being renamed

// Region/Auth state
let configuredRegions = ['local']; // Regions the user has set up
let selectedAuthRegion = 'local'; // Currently selected region in auth view
let platformAuthStatus = {}; // { "region:platform": true/false }

// Platform data
let platforms = [];
let PLATFORM_URLS = {};
let PLATFORM_NAMES = {};
let PLATFORM_LOGOS = {};

// Available countries (fetched from API)
let availableCountries = []; // Will be populated from fetch_proxy_config
let promptTargetRegions = []; // Unique regions from prompts for current product
const LOCAL_REGION = { code: 'local', name: 'Local (Your Location)', flag_emoji: '🏠' };

// Daily usage state
let dailyUsage = {
    current: 0,
    limit: 5,
    remaining: 5,
    isUnlimited: false,
    plan: 'free'
};
let currentProductPromptCount = 0; // Number of enabled prompts for current product

// Dashboard URL
const DASHBOARD_URL = 'https://columbus-aeo.com/dashboard';

// ==================== DOM Elements ====================
let loginView, mainView, scanningView, completeView, onboardingView, regionAuthView;
let settingsModal, addRegionModal, authRequiredModal;
let loginForm, loginError, loginBtn, googleLoginBtn, signupLink;
let userEmail, logoutBtn, productSelect;
let samplesPerPrompt, samplesWarning, scanBtn, scanInfo, dashboardLink;
let progressFill, progressText, phaseIndicator, cancelScanBtn;
let countdownDisplay, countdownSeconds;
let autoRunEnabled, scansPerDay, scansPerDayRow, timeWindowRow;
let timeWindowStart, timeWindowEnd;
let autostartEnabled;
let scheduleInfoSection, nextScanTime, scansCompleted, scansTotal;
let scanRunningSection, miniProgressFill, miniProgressText;

// ==================== Initialize ====================
async function init() {
    console.log('Initializing Columbus Desktop...');
    initDOMElements();
    setupEventListeners();
    setupInstanceSwitcher();
    await loadInstances();
    await loadPlatforms();
    await setupScanEventListeners();
    // Display app version
    await displayAppVersion();
    // checkAuthStatus will call hideLoadingScreen when fully ready
    await checkAuthStatus();
    // Check for updates in the background
    checkForUpdates();
}

async function displayAppVersion() {
    try {
        const version = await getVersion();
        const versionEl = document.getElementById('appVersion');
        if (versionEl) {
            versionEl.textContent = `Desktop v${version}`;
        }
    } catch (error) {
        console.error('Failed to get app version:', error);
    }
}

// ==================== Auto-Update ====================
async function checkForUpdates() {
    try {
        console.log('[Updater] Checking for updates...');
        console.warn('[Updater] Checking for updates...'); // Also warn so it's more visible
        const update = await check();

        if (update) {
            console.log(`[Updater] Update available: ${update.version}`);
            console.warn(`[Updater] Update available: ${update.version}`);
            showUpdateAvailable(update);
        } else {
            console.log('[Updater] No updates available');
            console.warn('[Updater] No updates available');
        }
    } catch (error) {
        console.log('[Updater] Update check failed:', error);
        console.warn('[Updater] Update check failed:', error);
    }
}

function showUpdateAvailable(update) {
    // Create update notification banner
    const banner = document.createElement('div');
    banner.id = 'updateBanner';
    banner.className = 'update-banner';
    banner.innerHTML = `
        <div class="update-banner-content">
            <span class="update-icon">🔄</span>
            <span class="update-text">Version ${update.version} is available!</span>
            <button id="installUpdateBtn" class="update-install-btn">Install & Restart</button>
            <button id="dismissUpdateBtn" class="update-dismiss-btn">&times;</button>
        </div>
    `;
    document.body.prepend(banner);

    // Install update handler
    document.getElementById('installUpdateBtn')?.addEventListener('click', async () => {
        const btn = document.getElementById('installUpdateBtn');
        if (btn) {
            btn.disabled = true;
            btn.textContent = 'Downloading...';
        }

        try {
            // Download and install the update
            let downloaded = 0;
            let contentLength = 0;
            await update.downloadAndInstall((progress) => {
                if (progress.event === 'Started' && btn) {
                    contentLength = progress.data.contentLength || 0;
                    btn.textContent = `Downloading... 0%`;
                } else if (progress.event === 'Progress' && btn) {
                    downloaded += progress.data.chunkLength;
                    if (contentLength > 0) {
                        const percent = Math.round((downloaded / contentLength) * 100);
                        btn.textContent = `Downloading... ${percent}%`;
                    }
                } else if (progress.event === 'Finished' && btn) {
                    btn.textContent = 'Restarting...';
                }
            });

            // Relaunch the app
            await relaunch();
        } catch (error) {
            console.error('Update failed:', error);
            if (btn) {
                btn.disabled = false;
                btn.textContent = 'Retry';
            }
            alert('Update failed: ' + error);
        }
    });

    // Dismiss handler
    document.getElementById('dismissUpdateBtn')?.addEventListener('click', () => {
        banner.remove();
    });
}

function showLoadingScreen() {
    const loadingScreen = document.getElementById('loadingScreen');
    if (loadingScreen) {
        loadingScreen.style.display = 'flex';
        loadingScreen.classList.remove('hidden');
    }
}

function hideLoadingScreen() {
    const loadingScreen = document.getElementById('loadingScreen');
    const mainContainer = document.getElementById('mainContainer');

    if (loadingScreen) {
        loadingScreen.classList.add('hidden');
        // Remove from DOM after transition
        setTimeout(() => {
            loadingScreen.style.display = 'none';
        }, 300);
    }

    if (mainContainer) {
        mainContainer.classList.remove('hidden');
    }
}

function initDOMElements() {
    // Views
    loginView = document.getElementById('loginView');
    mainView = document.getElementById('mainView');
    scanningView = document.getElementById('scanningView');
    completeView = document.getElementById('completeView');
    onboardingView = document.getElementById('onboardingView');
    regionAuthView = document.getElementById('regionAuthView');

    // Modals
    settingsModal = document.getElementById('settingsModal');
    addRegionModal = document.getElementById('addRegionModal');
    authRequiredModal = document.getElementById('authRequiredModal');

    // Login elements
    loginForm = document.getElementById('loginForm');
    loginError = document.getElementById('loginError');
    loginBtn = document.getElementById('loginBtn');
    googleLoginBtn = document.getElementById('googleLoginBtn');
    signupLink = document.getElementById('signupLink');

    // Main view elements
    userEmail = document.getElementById('userEmail');
    logoutBtn = document.getElementById('logoutBtn');
    productSelect = document.getElementById('productSelect');

    // Settings
    samplesPerPrompt = document.getElementById('samplesPerPrompt');
    samplesWarning = document.getElementById('samplesWarning');
    autoRunEnabled = document.getElementById('autoRunEnabled');
    scansPerDay = document.getElementById('scansPerDay');
    scansPerDayRow = document.getElementById('scansPerDayRow');
    timeWindowRow = document.getElementById('timeWindowRow');
    timeWindowStart = document.getElementById('timeWindowStart');
    timeWindowEnd = document.getElementById('timeWindowEnd');
    autostartEnabled = document.getElementById('autostartEnabled');

    // Schedule
    scheduleInfoSection = document.getElementById('scheduleInfoSection');
    nextScanTime = document.getElementById('nextScanTime');
    scansCompleted = document.getElementById('scansCompleted');
    scansTotal = document.getElementById('scansTotal');

    // Scan running indicator
    scanRunningSection = document.getElementById('scanRunningSection');
    miniProgressFill = document.getElementById('miniProgressFill');
    miniProgressText = document.getElementById('miniProgressText');

    // Scan controls
    scanBtn = document.getElementById('scanBtn');
    scanInfo = document.getElementById('scanInfo');
    dashboardLink = document.getElementById('dashboardLink');

    // Progress elements
    progressFill = document.getElementById('progressFill');
    progressText = document.getElementById('progressText');
    phaseIndicator = document.getElementById('phaseIndicator');
    cancelScanBtn = document.getElementById('cancelScanBtn');
    countdownDisplay = document.getElementById('countdownDisplay');
    countdownSeconds = document.getElementById('countdownSeconds');
}

function setupEventListeners() {
    // Login
    loginForm?.addEventListener('submit', handleLogin);
    googleLoginBtn?.addEventListener('click', handleGoogleLogin);
    signupLink?.addEventListener('click', (e) => {
        e.preventDefault();
        invoke('open_url_in_browser', { url: 'https://columbus-aeo.com/signup' });
    });

    // Main view
    logoutBtn?.addEventListener('click', handleLogout);
    productSelect?.addEventListener('change', handleProductChange);
    scanBtn?.addEventListener('click', handleStartScan);
    cancelScanBtn?.addEventListener('click', handleCancelScan);
    dashboardLink?.addEventListener('click', (e) => {
        e.preventDefault();
        invoke('open_url_in_browser', { url: DASHBOARD_URL });
    });

    // Settings
    document.getElementById('settingsBtn')?.addEventListener('click', () => {
        settingsModal.classList.remove('hidden');
    });
    document.getElementById('closeSettingsBtn')?.addEventListener('click', () => {
        settingsModal.classList.add('hidden');
    });
    document.getElementById('settingsModalOverlay')?.addEventListener('click', () => {
        settingsModal.classList.add('hidden');
    });
    document.getElementById('openRegionAuthBtn')?.addEventListener('click', () => {
        settingsModal.classList.add('hidden');
        showRegionAuthView();
    });

    // Manage Auth button on main view
    document.getElementById('manageAuthBtn')?.addEventListener('click', showRegionAuthView);

    // Auto-run settings
    autoRunEnabled?.addEventListener('change', handleAutoRunChange);
    scansPerDay?.addEventListener('change', saveProductConfig);
    timeWindowStart?.addEventListener('change', saveProductConfig);
    timeWindowEnd?.addEventListener('change', saveProductConfig);
    autostartEnabled?.addEventListener('change', handleAutostartChange);
    samplesPerPrompt?.addEventListener('change', () => {
        const val = parseInt(samplesPerPrompt.value) || 1;
        if (val > 3) {
            samplesWarning.classList.remove('hidden');
        } else {
            samplesWarning.classList.add('hidden');
        }
        saveProductConfig();
        // Update scan info to reflect new cost calculation
        updateScanButtonState();
    });

    // Onboarding
    document.getElementById('continueToAuthBtn')?.addEventListener('click', handleOnboardingContinue);

    // Region Auth View
    document.getElementById('backToMainBtn')?.addEventListener('click', () => {
        showView('main');
        updateAuthStatusGrid();
    });
    document.getElementById('addRegionBtn')?.addEventListener('click', showAddRegionModal);
    document.getElementById('openMagicLinkBtn')?.addEventListener('click', handleOpenMagicLink);

    // Add Region Modal
    document.getElementById('closeAddRegionBtn')?.addEventListener('click', () => {
        addRegionModal.classList.add('hidden');
    });
    document.getElementById('addRegionOverlay')?.addEventListener('click', () => {
        addRegionModal.classList.add('hidden');
    });

    // Auth Required Modal
    document.getElementById('dismissAuthRequiredBtn')?.addEventListener('click', () => {
        authRequiredModal.classList.add('hidden');
    });
    document.getElementById('authRequiredOverlay')?.addEventListener('click', () => {
        authRequiredModal.classList.add('hidden');
    });
    document.getElementById('goToAuthBtn')?.addEventListener('click', () => {
        authRequiredModal.classList.add('hidden');
        showRegionAuthView();
    });

    // Complete view
    document.getElementById('viewResultsBtn')?.addEventListener('click', () => {
        invoke('open_url_in_browser', { url: DASHBOARD_URL });
    });
    document.getElementById('newScanBtn')?.addEventListener('click', () => {
        showView('main');
    });

    // Message Modal
    document.getElementById('messageModalOverlay')?.addEventListener('click', hideMessageModal);
    document.getElementById('messageModalOkBtn')?.addEventListener('click', hideMessageModal);

    // Keyword Discovery
    document.getElementById('findKeywordsBtn')?.addEventListener('click', showKeywordModal);
    document.getElementById('closeKeywordModalBtn')?.addEventListener('click', hideKeywordModal);
    document.getElementById('keywordModalOverlay')?.addEventListener('click', hideKeywordModal);
    document.getElementById('cancelKeywordBtn')?.addEventListener('click', hideKeywordModal);
    document.getElementById('startKeywordBtn')?.addEventListener('click', handleStartKeywordDiscovery);
    document.getElementById('seedKeywordInput')?.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') handleStartKeywordDiscovery();
    });
}

// ==================== Message Modal ====================
function showMessageModal(message, title = 'Notice', type = 'warning') {
    const modal = document.getElementById('messageModal');
    const titleEl = document.getElementById('messageModalTitle');
    const textEl = document.getElementById('messageModalText');
    const iconEl = document.getElementById('messageModalIcon');

    if (titleEl) titleEl.textContent = title;
    if (textEl) textEl.textContent = message;

    // Update icon style based on type
    if (iconEl) {
        iconEl.className = 'message-modal-icon';
        if (type === 'error') {
            iconEl.classList.add('error');
            iconEl.innerHTML = `<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="15" y1="9" x2="9" y2="15"></line>
                <line x1="9" y1="9" x2="15" y2="15"></line>
            </svg>`;
        } else if (type === 'success') {
            iconEl.classList.add('success');
            iconEl.innerHTML = `<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
                <polyline points="22 4 12 14.01 9 11.01"></polyline>
            </svg>`;
        } else if (type === 'info') {
            iconEl.classList.add('info');
            iconEl.innerHTML = `<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="12" y1="16" x2="12" y2="12"></line>
                <line x1="12" y1="8" x2="12.01" y2="8"></line>
            </svg>`;
        } else {
            // Default warning icon
            iconEl.innerHTML = `<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="12" y1="8" x2="12" y2="12"></line>
                <line x1="12" y1="16" x2="12.01" y2="16"></line>
            </svg>`;
        }
    }

    if (modal) modal.classList.remove('hidden');
}

function hideMessageModal() {
    const modal = document.getElementById('messageModal');
    if (modal) modal.classList.add('hidden');
}

// ==================== Platform Loading ====================
async function loadPlatforms() {
    try {
        const platformsData = await invoke('get_ai_platforms', { forceRefresh: false });
        console.log('Loaded platforms:', platformsData);
        platforms = platformsData.map(p => p.id);
        PLATFORM_URLS = {};
        PLATFORM_NAMES = {};
        PLATFORM_LOGOS = {};
        platformsData.forEach(p => {
            PLATFORM_URLS[p.id] = p.website_url || '';
            PLATFORM_NAMES[p.id] = p.name || p.id;
            PLATFORM_LOGOS[p.id] = p.logo_url || '';
        });
        return platformsData;
    } catch (error) {
        console.error('Failed to load platforms:', error);
        platforms = ['chatgpt', 'claude', 'gemini', 'perplexity'];
        PLATFORM_URLS = {
            chatgpt: 'https://chat.openai.com',
            claude: 'https://claude.ai',
            gemini: 'https://gemini.google.com',
            perplexity: 'https://perplexity.ai'
        };
        PLATFORM_NAMES = {
            chatgpt: 'ChatGPT',
            claude: 'Claude',
            gemini: 'Gemini',
            perplexity: 'Perplexity'
        };
        PLATFORM_LOGOS = {};
        return null;
    }
}

// Helper function to render platform icon (logo image or fallback colored div)
function renderPlatformIcon(platform, cssClass = 'platform-icon') {
    const logoUrl = PLATFORM_LOGOS[platform];
    if (logoUrl) {
        return `<img src="${logoUrl}" alt="${platform}" class="${cssClass}" />`;
    }
    // Fallback to colored div if no logo
    return `<div class="${cssClass} ${platform}"></div>`;
}

// ==================== Country Loading ====================
async function loadAvailableCountries() {
    try {
        const countries = await invoke('fetch_proxy_config');
        console.log('Loaded available countries:', countries);
        availableCountries = countries || [];
        return countries;
    } catch (error) {
        // Free/Starter plans don't have geo-targeting - this is expected
        // Only "Local" region will be available
        if (error.toString().includes('paid plan') || error.toString().includes('Geo-targeting')) {
            console.log('Geo-targeting not available on current plan - using Local region only');
        } else {
            console.error('Failed to load countries:', error);
        }
        availableCountries = [];
        return [];
    }
}

function getAllRegions() {
    // Returns local + countries that are referenced in prompts
    // If no prompts have target_regions, only show local
    const allCountries = availableCountries.map(c => ({
        code: c.code.toLowerCase(),
        name: c.name,
        flag_emoji: c.flag_emoji || ''
    }));

    // Filter to only show countries that are in promptTargetRegions
    // Always include local, plus any countries that prompts target
    const filteredCountries = allCountries.filter(c =>
        promptTargetRegions.includes(c.code.toLowerCase())
    );

    return [LOCAL_REGION, ...filteredCountries];
}

async function loadPromptTargetRegions(productId) {
    try {
        const regions = await invoke('get_prompt_regions', { productId });
        console.log('Loaded prompt target regions:', regions);
        promptTargetRegions = regions || [];
        return regions;
    } catch (error) {
        // If the function doesn't exist or returns an error, default to empty
        // (which means only Local region will be shown)
        console.log('Could not load prompt regions, defaulting to Local only:', error.toString().substring(0, 100));
        promptTargetRegions = [];
        return [];
    }
}

function getRegionName(code) {
    if (code === 'local') return LOCAL_REGION.name;
    const country = availableCountries.find(c => c.code.toLowerCase() === code.toLowerCase());
    return country ? country.name : code.toUpperCase();
}

function getRegionFlag(code) {
    if (code === 'local') return LOCAL_REGION.flag_emoji;
    const country = availableCountries.find(c => c.code.toLowerCase() === code.toLowerCase());
    return country?.flag_emoji || '';
}

// ==================== View Management ====================
function showView(viewName) {
    [loginView, mainView, scanningView, completeView, onboardingView, regionAuthView].forEach(v => {
        v?.classList.add('hidden');
    });

    switch (viewName) {
        case 'login':
            loginView?.classList.remove('hidden');
            break;
        case 'main':
            mainView?.classList.remove('hidden');
            break;
        case 'scanning':
            scanningView?.classList.remove('hidden');
            break;
        case 'complete':
            completeView?.classList.remove('hidden');
            break;
        case 'onboarding':
            onboardingView?.classList.remove('hidden');
            break;
        case 'region-auth':
            regionAuthView?.classList.remove('hidden');
            break;
    }
}

// ==================== Authentication ====================
async function checkAuthStatus() {
    try {
        const status = await invoke('get_auth_status');
        console.log('Auth status:', status);

        if (status.authenticated && status.user) {
            currentUser = status.user;
            userEmail.textContent = currentUser.email;

            // Load data in correct order - some depend on others
            // First: load platforms, products, and settings in parallel
            await Promise.all([
                loadPlatforms(),
                loadProducts(),
                loadAutostartSetting()
            ]);

            // Second: load countries (fetch_proxy_config) - needed for regions
            await loadAvailableCountries();

            // Third: load regions (depends on availableCountries) and auth status (depends on regions)
            await loadConfiguredRegions();
            await loadPlatformAuthStatus();

            // Check if scan is running
            const scanRunning = await invoke('is_scan_running');
            if (scanRunning) {
                isScanning = true;
                renderPlatformProgressGrid();
                showView('scanning');
                const progress = await invoke('get_scan_progress');
                updateScanProgress(progress);
            } else if (configuredRegions.length === 1 && configuredRegions[0] === 'local' && !hasAnyAuth()) {
                // First time - show onboarding
                renderOnboardingRegions();
                showView('onboarding');
            } else {
                showView('main');
                updateAuthStatusGrid();
            }

            isInitializing = false;
            // All data loaded, hide loading screen
            hideLoadingScreen();
        } else {
            // Not authenticated, show login
            showView('login');
            hideLoadingScreen();
        }
    } catch (error) {
        console.error('Auth check failed:', error);
        showView('login');
        hideLoadingScreen();
    }
}

async function handleLogin(e) {
    e.preventDefault();
    const email = document.getElementById('email').value;
    const password = document.getElementById('password').value;

    loginBtn.disabled = true;
    loginError.classList.add('hidden');

    try {
        const result = await invoke('login', { email, password });
        console.log('Login result:', result);
        currentUser = result.user;
        // Show loading screen while loading user data
        showLoadingScreen();
        await checkAuthStatus();
    } catch (error) {
        console.error('Login error:', error);
        loginError.textContent = error;
        loginError.classList.remove('hidden');
    } finally {
        loginBtn.disabled = false;
    }
}

async function handleGoogleLogin() {
    googleLoginBtn.disabled = true;
    try {
        const user = await invoke('login_with_google');
        console.log('Google login successful:', user);
        // Show loading screen while loading user data
        showLoadingScreen();
        // Refresh the app state to show authenticated view
        await checkAuthStatus();
    } catch (error) {
        console.error('Google login error:', error);
        alert('Failed to start Google login: ' + error);
    } finally {
        googleLoginBtn.disabled = false;
    }
}

async function handleLogout() {
    try {
        await invoke('logout');
        currentUser = null;
        showView('login');
    } catch (error) {
        console.error('Logout error:', error);
    }
}

// ==================== Region & Auth Management ====================
async function loadConfiguredRegions() {
    try {
        // If user doesn't have geo-targeting access (free/starter plan),
        // only allow 'local' region regardless of what's stored
        if (availableCountries.length === 0) {
            console.log('No geo-targeting access - using Local region only');
            configuredRegions = ['local'];
            return;
        }

        // Get countries that have proxies configured (from API)
        const regions = await invoke('get_configured_proxy_countries');
        if (regions && regions.length > 0) {
            // Always include 'local' as an option, plus configured proxy countries
            // But filter out any regions that aren't in availableCountries
            const validRegions = regions.filter(r =>
                availableCountries.some(c => c.code.toLowerCase() === r.toLowerCase())
            );
            configuredRegions = ['local', ...validRegions];
        } else {
            configuredRegions = ['local'];
        }
        console.log('Configured regions:', configuredRegions);
    } catch (error) {
        console.error('Failed to load regions:', error);
        configuredRegions = ['local'];
    }
}

async function loadPlatformAuthStatus() {
    platformAuthStatus = {};
    for (const region of configuredRegions) {
        for (const platform of platforms) {
            try {
                const authInfo = await invoke('get_country_platform_auth', {
                    countryCode: region,
                    platform: platform
                });
                // authInfo is either null or { is_authenticated: bool, ... }
                // Handle both snake_case and camelCase just in case
                const isAuth = authInfo?.is_authenticated ?? authInfo?.isAuthenticated ?? false;
                platformAuthStatus[`${region}:${platform}`] = isAuth === true;
            } catch (e) {
                platformAuthStatus[`${region}:${platform}`] = false;
            }
        }
    }
    console.log('Platform auth status:', platformAuthStatus);
}

function hasAnyAuth() {
    return Object.values(platformAuthStatus).some(v => v === true);
}

function isPlatformAuthForRegion(region, platform) {
    return platformAuthStatus[`${region}:${platform}`] === true;
}

function getAuthCountForRegion(region) {
    return platforms.filter(p => isPlatformAuthForRegion(region, p)).length;
}

async function addRegion(regionCode) {
    if (!configuredRegions.includes(regionCode)) {
        configuredRegions.push(regionCode);
        // Initialize auth status for new region
        for (const platform of platforms) {
            platformAuthStatus[`${regionCode}:${platform}`] = false;
        }
        // Save to storage
        await invoke('set_country_platform_auth', {
            countryCode: regionCode,
            platform: platforms[0],
            authenticated: false
        });
    }
}

async function removeRegion(regionCode) {
    if (regionCode === 'local') return; // Can't remove local
    configuredRegions = configuredRegions.filter(r => r !== regionCode);
    // Remove from auth status
    for (const platform of platforms) {
        delete platformAuthStatus[`${regionCode}:${platform}`];
    }
}

// ==================== Daily Usage ====================
async function loadDailyUsage() {
    try {
        const usage = await invoke('check_daily_usage');
        dailyUsage = {
            current: usage.current || 0,
            limit: usage.limit || 5,
            remaining: usage.remaining || 0,
            isUnlimited: usage.isUnlimited || usage.is_unlimited || usage.limit === -1,
            plan: usage.plan || 'free'
        };
        console.log('Daily usage loaded:', dailyUsage);
    } catch (error) {
        console.log('Failed to load daily usage (may need to deploy edge function):', error);
        // Keep defaults
    }
}

// ==================== Auth Status Grid (Main View) ====================
function updateAuthStatusGrid() {
    const grid = document.getElementById('authStatusGrid');
    if (!grid) return;

    let html = '';
    for (const region of configuredRegions) {
        const authCount = getAuthCountForRegion(region);
        const flag = getRegionFlag(region);
        const name = region === 'local' ? 'Local' : region.toUpperCase();
        html += `
            <div class="auth-region-badge">
                <span class="region-name">${flag} ${name}</span>
                <div class="platform-dots">
                    ${platforms.map(p => `
                        <span class="platform-dot ${p} ${isPlatformAuthForRegion(region, p) ? 'authenticated' : ''}"
                              title="${capitalizeFirst(p)}: ${isPlatformAuthForRegion(region, p) ? 'Authenticated' : 'Not authenticated'}"></span>
                    `).join('')}
                </div>
            </div>
        `;
    }
    grid.innerHTML = html;
    updateScanButtonState();
}

// ==================== Region Auth View ====================
function showRegionAuthView() {
    selectedAuthRegion = configuredRegions[0] || 'local';
    showView('region-auth');
    renderRegionTabs();
    renderAuthPlatformsGrid();
}

function renderRegionTabs() {
    const tabsContainer = document.getElementById('regionTabs');
    if (!tabsContainer) return;

    let html = '';
    for (const region of configuredRegions) {
        const authCount = getAuthCountForRegion(region);
        const isActive = region === selectedAuthRegion;
        const flag = getRegionFlag(region);
        const name = region === 'local' ? 'Local' : region.toUpperCase();
        html += `
            <button class="region-tab ${isActive ? 'active' : ''}" data-region="${region}">
                <span>${flag} ${name}</span>
                <span class="auth-count">${authCount}/${platforms.length}</span>
                ${region !== 'local' ? `<button class="remove-region" data-region="${region}" title="Remove region">&times;</button>` : ''}
            </button>
        `;
    }
    tabsContainer.innerHTML = html;

    // Add click handlers
    tabsContainer.querySelectorAll('.region-tab').forEach(tab => {
        tab.addEventListener('click', (e) => {
            if (e.target.classList.contains('remove-region')) {
                e.stopPropagation();
                handleRemoveRegion(e.target.dataset.region);
                return;
            }
            selectedAuthRegion = tab.dataset.region;
            renderRegionTabs();
            renderAuthPlatformsGrid();
        });
    });
}

function renderAuthPlatformsGrid() {
    const grid = document.getElementById('authPlatformsGrid');
    if (!grid) return;

    let html = '';
    for (const platform of platforms) {
        const isAuth = isPlatformAuthForRegion(selectedAuthRegion, platform);
        const displayName = PLATFORM_NAMES[platform] || capitalizeFirst(platform);
        html += `
            <div class="auth-platform-card ${isAuth ? 'authenticated' : ''}">
                <div class="auth-platform-left">
                    ${renderPlatformIcon(platform, 'auth-platform-icon')}
                    <div class="auth-platform-info">
                        <span class="auth-platform-name">${displayName}</span>
                        <span class="auth-platform-status ${isAuth ? 'authenticated' : ''}">
                            ${isAuth ? 'Authenticated' : 'Not authenticated'}
                        </span>
                    </div>
                </div>
                <div class="auth-platform-right">
                    <button class="btn-auth ${isAuth ? 'authenticated' : ''}"
                            data-platform="${platform}"
                            data-region="${selectedAuthRegion}">
                        ${isAuth ? 'Re-auth' : 'Login'}
                    </button>
                    <button class="btn-toggle-auth ${isAuth ? 'is-auth' : ''}"
                            data-platform="${platform}"
                            data-region="${selectedAuthRegion}"
                            title="${isAuth ? 'Mark as not logged in' : 'Mark as logged in'}">
                        ${isAuth ? '✓' : '○'}
                    </button>
                </div>
            </div>
        `;
    }
    grid.innerHTML = html;

    // Add click handlers for login button
    grid.querySelectorAll('.btn-auth').forEach(btn => {
        btn.addEventListener('click', () => {
            handleAuthPlatform(btn.dataset.platform, btn.dataset.region);
        });
    });

    // Add click handlers for toggle button
    grid.querySelectorAll('.btn-toggle-auth').forEach(btn => {
        btn.addEventListener('click', () => {
            handleToggleAuth(btn.dataset.platform, btn.dataset.region);
        });
    });
}

async function handleAuthPlatform(platform, region) {
    console.log(`Opening auth for ${platform} in region ${region}`);
    try {
        // Open login webview for this platform/region (visible: true for manual auth)
        await invoke('open_country_login', {
            countryCode: region,
            platform: platform,
            visible: true
        });
    } catch (error) {
        console.error('Failed to open auth:', error);
        alert('Failed to open authentication: ' + error);
    }
}

async function handleToggleAuth(platform, region) {
    const isCurrentlyAuth = isPlatformAuthForRegion(region, platform);
    const newStatus = !isCurrentlyAuth;

    console.log(`Toggling auth for ${platform} in region ${region}: ${isCurrentlyAuth} -> ${newStatus}`);

    try {
        await invoke('set_platform_auth_status', {
            countryCode: region,
            platform: platform,
            authenticated: newStatus
        });

        // Update local state
        const key = `${region}:${platform}`;
        platformAuthStatus[key] = newStatus;

        // Re-render the grid
        renderAuthPlatformsGrid();
    } catch (error) {
        console.error('Failed to toggle auth status:', error);
        alert('Failed to update auth status: ' + error);
    }
}

async function handleRemoveRegion(regionCode) {
    if (regionCode === 'local') return;
    if (!confirm(`Remove ${regionCode.toUpperCase()} region? You'll need to re-authenticate platforms if you add it back.`)) {
        return;
    }
    await removeRegion(regionCode);
    if (selectedAuthRegion === regionCode) {
        selectedAuthRegion = configuredRegions[0] || 'local';
    }
    renderRegionTabs();
    renderAuthPlatformsGrid();
}

// ==================== Add Region Modal ====================
function showAddRegionModal() {
    const optionsContainer = document.getElementById('addRegionOptions');
    if (!optionsContainer) return;

    // Get regions not yet configured
    const allRegions = getAllRegions();
    const unconfiguredRegions = allRegions.filter(r => !configuredRegions.includes(r.code));

    if (unconfiguredRegions.length === 0) {
        optionsContainer.innerHTML = '<p style="color: #6b7280; text-align: center;">All available regions have been added.</p>';
    } else {
        let html = '';
        for (const region of unconfiguredRegions) {
            html += `
                <div class="add-region-option" data-region="${region.code}">
                    <span class="region-name">${region.flag_emoji} ${region.name}</span>
                    <span class="region-code">${region.code.toUpperCase()}</span>
                </div>
            `;
        }
        optionsContainer.innerHTML = html;

        // Add click handlers
        optionsContainer.querySelectorAll('.add-region-option').forEach(opt => {
            opt.addEventListener('click', async () => {
                await addRegion(opt.dataset.region);
                selectedAuthRegion = opt.dataset.region;
                addRegionModal.classList.add('hidden');
                renderRegionTabs();
                renderAuthPlatformsGrid();
            });
        });
    }

    addRegionModal.classList.remove('hidden');
}

// ==================== Magic Link ====================
async function handleOpenMagicLink() {
    const input = document.getElementById('magicLinkInput');
    const url = input?.value?.trim();
    if (!url) {
        alert('Please paste a URL first');
        return;
    }
    try {
        await invoke('open_magic_link', {
            countryCode: selectedAuthRegion,
            url: url
        });
        input.value = '';
    } catch (error) {
        console.error('Failed to open URL:', error);
        alert('Failed to open URL: ' + error);
    }
}

// ==================== Onboarding ====================
function renderOnboardingRegions() {
    const container = document.getElementById('onboardingRegions');
    if (!container) return;

    const allRegions = getAllRegions();

    let html = '';
    for (const region of allRegions) {
        const isLocal = region.code === 'local';
        html += `
            <label class="region-option">
                <input type="checkbox" value="${region.code}" ${isLocal ? 'checked' : ''}>
                <span class="region-name">${region.flag_emoji} ${region.name}</span>
            </label>
        `;
    }
    container.innerHTML = html;
}

async function handleOnboardingContinue() {
    // Get selected regions from onboarding
    const checkboxes = document.querySelectorAll('#onboardingRegions input[type="checkbox"]:checked');
    const selectedRegions = Array.from(checkboxes).map(cb => cb.value);

    if (selectedRegions.length === 0) {
        alert('Please select at least one region');
        return;
    }

    // Add selected regions
    for (const region of selectedRegions) {
        if (!configuredRegions.includes(region)) {
            await addRegion(region);
        }
    }

    // Go to region auth view
    showRegionAuthView();
}

// ==================== Products ====================
async function loadProducts() {
    try {
        const status = await invoke('get_status');
        products = status.products || [];
        const organizations = status.organizations || [];
        console.log('Loaded products:', products);
        console.log('Loaded organizations:', organizations);

        productSelect.innerHTML = '<option value="">Select a product...</option>';

        // Check if user has products from multiple organizations
        const uniqueOrgIds = [...new Set(products.map(p => p.organizationId))];
        const hasMultipleOrgs = uniqueOrgIds.length > 1;

        if (hasMultipleOrgs) {
            // Group products by organization using optgroups
            const productsByOrg = {};
            products.forEach(p => {
                const orgId = p.organizationId;
                if (!productsByOrg[orgId]) {
                    productsByOrg[orgId] = {
                        name: p.organizationName || 'Unknown',
                        products: []
                    };
                }
                productsByOrg[orgId].products.push(p);
            });

            // Create optgroups for each organization
            Object.entries(productsByOrg).forEach(([orgId, orgData]) => {
                const optgroup = document.createElement('optgroup');
                optgroup.label = orgData.name;
                orgData.products.forEach(p => {
                    const opt = document.createElement('option');
                    opt.value = p.id;
                    opt.textContent = p.name;
                    optgroup.appendChild(opt);
                });
                productSelect.appendChild(optgroup);
            });
        } else {
            // Single organization - simple flat list
            products.forEach(p => {
                const opt = document.createElement('option');
                opt.value = p.id;
                opt.textContent = p.name;
                productSelect.appendChild(opt);
            });
        }

        // Restore last selected product
        const lastProductId = await invoke('get_last_product_id');
        if (lastProductId && products.find(p => p.id === lastProductId)) {
            productSelect.value = lastProductId;
            selectedProductId = lastProductId;
            await loadProductConfig(lastProductId);
            // Load prompt target regions to filter available countries
            await loadPromptTargetRegions(lastProductId);
        }

        updateAuthStatusGrid();
        updateScanButtonState();
        updateKeywordButtonState();
    } catch (error) {
        console.error('Failed to load products:', error);
    }
}

async function handleProductChange() {
    selectedProductId = productSelect.value;
    if (selectedProductId) {
        await invoke('set_last_product_id', { productId: selectedProductId });
        await loadProductConfig(selectedProductId);
        // Load prompt target regions to filter available countries
        await loadPromptTargetRegions(selectedProductId);
        // Load daily usage and prompt count for scan cost preview
        await loadDailyUsage();
        await loadProductPromptCount(selectedProductId);
    }
    updateScanButtonState();
    updateKeywordButtonState();
}

async function loadProductPromptCount(productId) {
    try {
        // Get prompt data from extension-prompts endpoint via Rust command
        const promptData = await invoke('fetch_extension_prompts', { productId });
        currentProductPromptCount = promptData?.totalPrompts || promptData?.prompts?.length || 0;
        console.log('Product prompt count:', currentProductPromptCount);

        // Also update daily usage from the quota if available
        if (promptData?.quota) {
            dailyUsage = {
                current: promptData.quota.promptsUsedToday || 0,
                limit: promptData.quota.promptsPerDay || 5,
                remaining: promptData.quota.promptsRemaining ?? (promptData.quota.promptsPerDay - promptData.quota.promptsUsedToday),
                isUnlimited: promptData.quota.isUnlimited || promptData.quota.promptsPerDay === -1,
                plan: promptData.quota.plan || 'free'
            };
            console.log('Updated daily usage from prompts endpoint:', dailyUsage);
        }
    } catch (error) {
        console.log('Failed to load prompt count:', error);
        currentProductPromptCount = 0;
    }
}

async function loadProductConfig(productId) {
    try {
        const config = await invoke('get_product_config', { productId });
        console.log('Loaded product config:', config);

        samplesPerPrompt.value = config.samples_per_prompt || 1;
        autoRunEnabled.checked = config.auto_run_enabled !== false;
        scansPerDay.value = config.scans_per_day || 1;
        timeWindowStart.value = config.time_window_start ?? 9;
        timeWindowEnd.value = config.time_window_end ?? 17;

        updateAutoRunUI();
        loadScheduleInfo();

        // Auto-initialize platforms if empty (new product or fresh install)
        // This ensures auto-scan has platforms to work with
        if (!config.ready_platforms || config.ready_platforms.length === 0) {
            console.log('Product has no platforms configured, initializing with available platforms:', platforms);
            if (platforms.length > 0) {
                await saveProductConfig(true); // Force save even during initialization
            }
        }
    } catch (error) {
        console.error('Failed to load product config:', error);
    }
}

async function saveProductConfig(force = false) {
    if (!selectedProductId || (isInitializing && !force)) return;
    try {
        await invoke('set_product_config', {
            productId: selectedProductId,
            readyPlatforms: platforms, // All platforms are now "ready" if authenticated
            samplesPerPrompt: parseInt(samplesPerPrompt.value) || 1,
            autoRunEnabled: autoRunEnabled.checked,
            scansPerDay: parseInt(scansPerDay.value) || 1,
            timeWindowStart: parseInt(timeWindowStart.value),
            timeWindowEnd: parseInt(timeWindowEnd.value)
        });
        console.log('Saved product config with platforms:', platforms);
    } catch (error) {
        console.error('Failed to save product config:', error);
    }
}

// ==================== Auto-Run Settings ====================
function handleAutoRunChange() {
    updateAutoRunUI();
    saveProductConfig();
}

function updateAutoRunUI() {
    const enabled = autoRunEnabled.checked;
    scansPerDayRow.style.opacity = enabled ? '1' : '0.5';
    scansPerDayRow.style.pointerEvents = enabled ? 'auto' : 'none';
    timeWindowRow.style.opacity = enabled ? '1' : '0.5';
    timeWindowRow.style.pointerEvents = enabled ? 'auto' : 'none';

    if (enabled) {
        scheduleInfoSection.classList.remove('disabled');
    } else {
        scheduleInfoSection.classList.add('disabled');
    }
}

async function loadAutostartSetting() {
    try {
        const enabled = await invoke('get_autostart_enabled');
        autostartEnabled.checked = enabled;
    } catch (error) {
        console.error('Failed to load autostart setting:', error);
    }
}

async function handleAutostartChange() {
    try {
        await invoke('set_autostart_enabled', { enabled: autostartEnabled.checked });
    } catch (error) {
        console.error('Failed to save autostart setting:', error);
    }
}

async function loadScheduleInfo() {
    try {
        const info = await invoke('get_schedule_info', { productId: selectedProductId });
        console.log('[Schedule] Info:', info);

        // API returns next_scan_hour (0-23), not next_scan_time
        if (info.next_scan_hour !== null && info.next_scan_hour !== undefined) {
            // Format as HH:00
            const hour = info.next_scan_hour;
            const formattedHour = hour.toString().padStart(2, '0');
            nextScanTime.textContent = `${formattedHour}:00`;
        } else {
            nextScanTime.textContent = '--';
        }
        scansCompleted.textContent = info.scans_completed_today || 0;
        scansTotal.textContent = info.scans_total_today || 1;
    } catch (error) {
        console.error('Failed to load schedule info:', error);
    }
}

// ==================== Scan Functions ====================
function updateScanButtonState() {
    const hasProduct = !!selectedProductId;
    const hasAuth = hasAnyAuth();

    scanBtn.disabled = !hasProduct || !hasAuth;

    if (!hasProduct && !hasAuth) {
        scanInfo.textContent = 'Select a product and authenticate platforms';
    } else if (!hasProduct) {
        scanInfo.textContent = 'Select a product to start scanning';
    } else if (!hasAuth) {
        scanInfo.textContent = 'Authenticate at least one platform to scan';
    } else {
        const product = products.find(p => p.id === selectedProductId);

        // Calculate total prompts used accounting for samples per prompt
        const samples = parseInt(samplesPerPrompt?.value) || 1;
        const totalPromptsToUse = currentProductPromptCount * samples;

        // Show scan cost preview
        if (dailyUsage.isUnlimited) {
            if (samples > 1) {
                scanInfo.textContent = `Ready to scan ${product?.name || 'product'} (${currentProductPromptCount} prompts × ${samples} samples = ${totalPromptsToUse} tests)`;
            } else {
                scanInfo.textContent = `Ready to scan ${product?.name || 'product'} (${currentProductPromptCount} prompts)`;
            }
        } else if (currentProductPromptCount > 0) {
            const willUse = Math.min(totalPromptsToUse, dailyUsage.remaining);
            if (dailyUsage.remaining === 0) {
                scanInfo.innerHTML = `<span class="text-amber-600">Daily limit reached (${dailyUsage.current}/${dailyUsage.limit})</span>`;
                scanBtn.disabled = true;
            } else if (totalPromptsToUse > dailyUsage.remaining) {
                const promptsCanTest = Math.floor(dailyUsage.remaining / samples);
                if (samples > 1) {
                    scanInfo.innerHTML = `Will test ${promptsCanTest} of ${currentProductPromptCount} prompts (${samples}× samples) <span class="text-amber-600">(${dailyUsage.remaining} remaining today)</span>`;
                } else {
                    scanInfo.innerHTML = `Will test ${willUse} of ${currentProductPromptCount} prompts <span class="text-amber-600">(${dailyUsage.remaining} remaining today)</span>`;
                }
            } else {
                if (samples > 1) {
                    scanInfo.textContent = `This scan will use ${totalPromptsToUse} tests (${currentProductPromptCount} prompts × ${samples} samples) - ${dailyUsage.remaining} remaining today`;
                } else {
                    scanInfo.textContent = `This scan will use ${currentProductPromptCount} of your ${dailyUsage.remaining} remaining daily tests`;
                }
            }
        } else {
            scanInfo.textContent = `Ready to scan ${product?.name || 'product'}`;
        }
    }
}

async function handleStartScan() {
    if (!selectedProductId) return;
    if (!hasAnyAuth()) {
        showAuthRequiredModal([]);
        return;
    }

    // Refresh usage data and check quota before starting scan
    await loadProductPromptCount(selectedProductId);

    // Check if daily limit is reached (unless unlimited)
    if (!dailyUsage.isUnlimited && dailyUsage.remaining <= 0) {
        scanInfo.innerHTML = `<span class="text-amber-600">Daily limit reached (${dailyUsage.current}/${dailyUsage.limit}). Resets at midnight.</span>`;
        scanBtn.disabled = true;
        return;
    }

    // Check if there are any prompts to test
    if (currentProductPromptCount === 0) {
        scanInfo.innerHTML = `<span class="text-amber-600">No prompts configured for this product. Add prompts in the dashboard.</span>`;
        return;
    }

    // Check which platforms/regions are needed for this product's prompts
    try {
        const promptRegions = await invoke('get_prompt_target_regions', { productId: selectedProductId });
        const neededRegions = new Set();

        // Collect all regions needed
        for (const regions of Object.values(promptRegions)) {
            if (regions.length === 0) {
                neededRegions.add('local');
            } else {
                regions.forEach(r => neededRegions.add(r.toLowerCase()));
            }
        }

        // Check if we have auth for all needed regions
        const missingAuth = [];
        for (const region of neededRegions) {
            // Need at least one platform auth per region
            const hasAuthForRegion = platforms.some(p => isPlatformAuthForRegion(region, p));
            if (!hasAuthForRegion) {
                missingAuth.push({ region, platforms: platforms });
            }
        }

        if (missingAuth.length > 0) {
            showAuthRequiredModal(missingAuth);
            return;
        }

        // Get authenticated platforms for the scan
        const authPlatforms = platforms.filter(p =>
            configuredRegions.some(r => isPlatformAuthForRegion(r, p))
        );

        if (authPlatforms.length === 0) {
            showAuthRequiredModal([]);
            return;
        }

        // Start the scan
        scanBtn.disabled = true;
        isScanning = true;
        resetProgressUI();
        showView('scanning');

        await invoke('start_scan', {
            productId: selectedProductId,
            samplesPerPrompt: parseInt(samplesPerPrompt.value) || 1,
            platforms: authPlatforms
        });

        console.log('Scan started with platforms:', authPlatforms);
    } catch (error) {
        console.error('Start scan error:', error);
        showMessageModal('Failed to start scan: ' + error, 'Error', 'error');
        showView('main');
        scanBtn.disabled = false;
        isScanning = false;
    }
}

function showAuthRequiredModal(missingAuth) {
    const list = document.getElementById('authRequiredList');
    const message = document.getElementById('authRequiredMessage');

    if (missingAuth.length === 0) {
        message.textContent = 'No platforms are authenticated. Please set up authentication first.';
        list.innerHTML = '';
    } else {
        message.textContent = 'The following regions need at least one authenticated platform:';
        let html = '';
        for (const item of missingAuth) {
            html += `
                <div class="auth-required-item">
                    <span class="item-text">${item.region === 'local' ? 'Local' : item.region.toUpperCase()} - no authenticated platforms</span>
                </div>
            `;
        }
        list.innerHTML = html;
    }

    authRequiredModal.classList.remove('hidden');
}

async function handleCancelScan() {
    try {
        await invoke('cancel_scan');
        isScanning = false;
        showView('main');
        scanBtn.disabled = false;
    } catch (error) {
        console.error('Cancel scan error:', error);
    }
}

function renderPlatformProgressGrid() {
    const grid = document.getElementById('platformProgressGrid');
    if (!grid) return;

    let html = '';
    platforms.forEach(platform => {
        const displayName = PLATFORM_NAMES[platform] || capitalizeFirst(platform);
        html += `
            <div class="platform-progress-item" data-platform="${platform}">
                <div class="platform-progress-header">
                    ${renderPlatformIcon(platform, 'platform-progress-icon')}
                    <span class="platform-progress-name">${displayName}</span>
                    <span class="platform-progress-status" id="progress-${platform}-status">pending</span>
                </div>
                <div class="platform-progress-bar">
                    <div class="platform-progress-fill" id="progress-${platform}-fill" style="width: 0%"></div>
                </div>
                <span class="platform-progress-count" id="progress-${platform}-count">0/0</span>
            </div>
        `;
    });
    grid.innerHTML = html;
}

function resetProgressUI() {
    // Ensure the platform progress grid is rendered
    renderPlatformProgressGrid();

    progressFill.style.width = '0%';
    progressText.textContent = '0%';
    phaseIndicator.textContent = 'Initializing...';
    countdownDisplay.classList.add('hidden');
    countdownSeconds.textContent = '45';

    platforms.forEach(platform => {
        const statusEl = document.getElementById(`progress-${platform}-status`);
        const fillEl = document.getElementById(`progress-${platform}-fill`);
        const countEl = document.getElementById(`progress-${platform}-count`);

        if (statusEl) {
            statusEl.textContent = 'pending';
            statusEl.className = 'platform-progress-status pending';
        }
        if (fillEl) {
            fillEl.style.width = '0%';
            fillEl.classList.remove('complete');
        }
        if (countEl) {
            countEl.textContent = '0/0';
        }
    });
}

// ==================== Scan Events ====================
async function setupScanEventListeners() {
    await listen('scan:progress', (event) => {
        updateScanProgress(event.payload);
    });

    await listen('scan:complete', (event) => {
        handleScanComplete(event.payload);
    });

    await listen('scan:error', (event) => {
        handleScanError(event.payload);
    });

    await listen('scan:countdown', (event) => {
        updateCountdown(event.payload);
    });

    // Listen for auth state changes from webviews
    await listen('platform-auth-changed', async (event) => {
        const { region, platform, authenticated } = event.payload;
        platformAuthStatus[`${region}:${platform}`] = authenticated;
        renderAuthPlatformsGrid();
        updateAuthStatusGrid();
    });

    // Listen for OAuth login success - refresh the app state
    await listen('auth:success', async (event) => {
        console.log('OAuth login successful, refreshing app state...', event.payload);
        await checkAuthStatus();
    });
}

function getPhaseDisplayText(phase) {
    const phaseMap = {
        'initializing': 'Initializing...',
        'submitting': 'Submitting prompts...',
        'waiting': 'Waiting for responses...',
        'collecting': 'Collecting responses...',
        'complete': 'Complete',
        'cancelled': 'Cancelled'
    };
    return phaseMap[phase] || phase || 'Processing...';
}

function updateScanProgress(progress) {
    if (!progress) return;

    // Calculate progress based on both submissions and collections
    // Submission = 50% of the work, Collection = 50% of the work
    let totalSubmitted = 0;
    let totalCollected = 0;
    let totalTasks = 0;

    if (progress.platforms) {
        for (const [_, state] of Object.entries(progress.platforms)) {
            totalSubmitted += state.submitted || 0;
            totalCollected += state.collected || 0;
            totalTasks += state.total || 0;
        }
    }

    // Calculate percentage: submissions count as 0-50%, collections as 50-100%
    let percent = 0;
    if (totalTasks > 0) {
        const submissionProgress = (totalSubmitted / totalTasks) * 50;
        const collectionProgress = (totalCollected / totalTasks) * 50;
        percent = Math.round(submissionProgress + collectionProgress);
    }

    progressFill.style.width = `${percent}%`;
    progressText.textContent = `${percent}%`;
    phaseIndicator.textContent = getPhaseDisplayText(progress.phase);

    // Update mini progress in main view
    if (miniProgressFill) miniProgressFill.style.width = `${percent}%`;
    if (miniProgressText) miniProgressText.textContent = `${percent}%`;

    // Update countdown if present
    if (progress.countdownSeconds !== undefined && progress.countdownSeconds !== null) {
        updateCountdown(progress.countdownSeconds);
    } else {
        countdownDisplay?.classList.add('hidden');
    }

    // Update per-platform progress
    if (progress.platforms) {
        for (const [platform, state] of Object.entries(progress.platforms)) {
            const statusEl = document.getElementById(`progress-${platform}-status`);
            const fillEl = document.getElementById(`progress-${platform}-fill`);
            const countEl = document.getElementById(`progress-${platform}-count`);

            if (statusEl) {
                statusEl.textContent = state.status;
                statusEl.className = `platform-progress-status ${state.status}`;
            }
            if (fillEl) {
                const platformPercent = state.total > 0
                    ? Math.round(((state.submitted + state.collected) / (state.total * 2)) * 100)
                    : 0;
                fillEl.style.width = `${platformPercent}%`;
                if (state.status === 'complete') {
                    fillEl.classList.add('complete');
                }
            }
            if (countEl) {
                countEl.textContent = `${state.collected}/${state.total}`;
            }
        }
    }
}

function updateCountdown(seconds) {
    countdownDisplay.classList.remove('hidden');
    countdownSeconds.textContent = seconds;
}

function handleScanComplete(result) {
    console.log('Scan complete:', result);
    console.log('mention_rate raw:', result?.mention_rate, 'citation_rate raw:', result?.citation_rate);
    isScanning = false;
    scanBtn.disabled = false;

    const stats = document.getElementById('completeStats');
    if (stats && result) {
        stats.innerHTML = `
            <div class="stat-item">
                <div class="stat-value">${result.total_prompts || 0}</div>
                <div class="stat-label">Prompts Tested</div>
            </div>
            <div class="stat-item">
                <div class="stat-value">${Math.round(result.mention_rate || 0)}%</div>
                <div class="stat-label">Mention Rate</div>
            </div>
            <div class="stat-item">
                <div class="stat-value">${result.successful_prompts || 0}</div>
                <div class="stat-label">Successful</div>
            </div>
            <div class="stat-item">
                <div class="stat-value">${Math.round(result.citation_rate || 0)}%</div>
                <div class="stat-label">Citation Rate</div>
            </div>
        `;
    }

    showView('complete');
    loadScheduleInfo();
    updateScanRunningIndicator(false);
}

function handleScanError(error) {
    console.error('Scan error:', error);
    isScanning = false;
    scanBtn.disabled = false;
    const errorMsg = error.message || String(error);
    // Use info type for cancellation, error type for actual errors
    const isCancelled = errorMsg.toLowerCase().includes('cancel');
    showMessageModal(errorMsg, isCancelled ? 'Scan Cancelled' : 'Scan Error', isCancelled ? 'info' : 'error');
    showView('main');
    updateScanRunningIndicator(false);
}

function updateScanRunningIndicator(running, progress = null) {
    if (running) {
        scanRunningSection?.classList.remove('hidden');
        if (progress) {
            const percent = progress.total > 0
                ? Math.round((progress.current / progress.total) * 100)
                : 0;
            if (miniProgressFill) miniProgressFill.style.width = `${percent}%`;
            if (miniProgressText) miniProgressText.textContent = `${percent}%`;
        }
    } else {
        scanRunningSection?.classList.add('hidden');
    }
}

// ==================== Utilities ====================
function capitalizeFirst(str) {
    return str.charAt(0).toUpperCase() + str.slice(1);
}

// ==================== Multi-Instance Management ====================

// Load all instances from backend
async function loadInstances() {
    try {
        instances = await invoke('list_instances');
        const activeInstance = await invoke('get_active_instance');
        activeInstanceId = activeInstance?.id || null;
        console.log(`Loaded ${instances.length} instances, active: ${activeInstanceId}`);
        updateInstanceUI();
    } catch (e) {
        console.error('Failed to load instances:', e);
        // Instances might not exist yet (pre-migration)
        instances = [];
        activeInstanceId = null;
    }
}

// Set up instance switcher event listeners
function setupInstanceSwitcher() {
    const instanceBtn = document.getElementById('instanceBtn');
    const instanceDropdown = document.getElementById('instanceDropdown');
    const instanceSwitcher = document.getElementById('instanceSwitcher');
    const addInstanceBtn = document.getElementById('addInstanceBtn');

    // Toggle dropdown
    instanceBtn?.addEventListener('click', (e) => {
        e.stopPropagation();
        instanceDropdown?.classList.toggle('hidden');
        instanceSwitcher?.classList.toggle('open');
    });

    // Close dropdown when clicking outside
    document.addEventListener('click', () => {
        instanceDropdown?.classList.add('hidden');
        instanceSwitcher?.classList.remove('open');
    });

    // Add instance button
    addInstanceBtn?.addEventListener('click', async (e) => {
        e.stopPropagation();
        await createInstance();
        instanceDropdown?.classList.add('hidden');
        instanceSwitcher?.classList.remove('open');
    });

    // Instance rename modal
    const closeRenameBtn = document.getElementById('closeInstanceRenameBtn');
    const cancelRenameBtn = document.getElementById('cancelInstanceRenameBtn');
    const saveRenameBtn = document.getElementById('saveInstanceRenameBtn');
    const renameOverlay = document.getElementById('instanceRenameOverlay');

    closeRenameBtn?.addEventListener('click', closeInstanceRenameModal);
    cancelRenameBtn?.addEventListener('click', closeInstanceRenameModal);
    renameOverlay?.addEventListener('click', closeInstanceRenameModal);
    saveRenameBtn?.addEventListener('click', saveInstanceRename);

    // Handle Enter key in rename input
    document.getElementById('instanceNameInput')?.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            saveInstanceRename();
        }
    });
}

// Update instance UI (dropdown list and active instance display)
function updateInstanceUI() {
    const activeNameEl = document.getElementById('activeInstanceName');
    const instanceList = document.getElementById('instanceList');

    // Update active instance name in button
    const activeInstance = instances.find(i => i.id === activeInstanceId);
    if (activeNameEl) {
        activeNameEl.textContent = activeInstance?.name || 'Default';
    }

    // Update dropdown list
    if (instanceList) {
        instanceList.innerHTML = instances.map(instance => `
            <div class="instance-dropdown-item ${instance.id === activeInstanceId ? 'active' : ''}" data-instance-id="${instance.id}">
                ${instance.id === activeInstanceId ? `
                    <svg class="instance-check" width="16" height="16" viewBox="0 0 16 16" fill="none">
                        <path d="M3 8L6.5 11.5L13 5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                ` : '<span style="width: 16px;"></span>'}
                <span class="instance-name">${instance.name}</span>
                <div class="instance-actions">
                    <button type="button" class="instance-action-btn rename" title="Rename" data-action="rename" data-instance-id="${instance.id}">
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                            <path d="M10 2L12 4L5 11H3V9L10 2Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                    </button>
                    ${!instance.is_default ? `
                        <button type="button" class="instance-action-btn delete" title="Delete" data-action="delete" data-instance-id="${instance.id}">
                            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                                <path d="M3 4H11M5 4V3C5 2.45 5.45 2 6 2H8C8.55 2 9 2.45 9 3V4M10 4V11C10 11.55 9.55 12 9 12H5C4.45 12 4 11.55 4 11V4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                            </svg>
                        </button>
                    ` : ''}
                </div>
            </div>
        `).join('');

        // Add event listeners to instance items
        instanceList.querySelectorAll('.instance-dropdown-item').forEach(item => {
            // Click on item to switch
            item.addEventListener('click', async (e) => {
                // Don't switch if clicking action buttons
                if (e.target.closest('.instance-action-btn')) return;

                const instanceId = item.dataset.instanceId;
                if (instanceId && instanceId !== activeInstanceId) {
                    await switchInstance(instanceId);
                }
            });
        });

        // Add event listeners to action buttons
        instanceList.querySelectorAll('.instance-action-btn').forEach(btn => {
            btn.addEventListener('click', async (e) => {
                e.stopPropagation();
                const action = btn.dataset.action;
                const instanceId = btn.dataset.instanceId;

                if (action === 'rename') {
                    openInstanceRenameModal(instanceId);
                } else if (action === 'delete') {
                    await deleteInstance(instanceId);
                }
            });
        });
    }
}

// Switch to a different instance
async function switchInstance(instanceId) {
    try {
        await invoke('switch_instance', { instanceId });
        activeInstanceId = instanceId;
        console.log(`Switched to instance: ${instanceId}`);

        // Close dropdown
        document.getElementById('instanceDropdown')?.classList.add('hidden');
        document.getElementById('instanceSwitcher')?.classList.remove('open');

        // Reload instance-scoped data
        await loadPlatformAuthStatus();
        updateInstanceUI();
        updateAuthStatusGrid();
        renderAuthPlatformsGrid();

        // Show a brief notification
        const instance = instances.find(i => i.id === instanceId);
        console.log(`Active instance: ${instance?.name}`);
    } catch (e) {
        console.error('Failed to switch instance:', e);
        alert('Failed to switch instance: ' + (e.message || e));
    }
}

// Create a new instance
async function createInstance() {
    try {
        const newInstance = await invoke('create_instance', { name: null });
        instances.push(newInstance);
        console.log(`Created instance: ${newInstance.name} (${newInstance.id})`);

        // Switch to the new instance
        await switchInstance(newInstance.id);
    } catch (e) {
        console.error('Failed to create instance:', e);
        alert('Failed to create instance: ' + (e.message || e));
    }
}

// Delete an instance
async function deleteInstance(instanceId) {
    const instance = instances.find(i => i.id === instanceId);
    if (!instance) return;

    if (!confirm(`Delete instance "${instance.name}"?\n\nThis will remove all stored credentials and authentication data for this instance.`)) {
        return;
    }

    try {
        await invoke('delete_instance', { instanceId });
        instances = instances.filter(i => i.id !== instanceId);
        console.log(`Deleted instance: ${instanceId}`);

        // If deleted the active instance, switch to default
        if (instanceId === activeInstanceId) {
            const defaultInstance = instances.find(i => i.is_default);
            if (defaultInstance) {
                await switchInstance(defaultInstance.id);
            }
        }

        updateInstanceUI();
    } catch (e) {
        console.error('Failed to delete instance:', e);
        alert('Failed to delete instance: ' + (e.message || e));
    }
}

// Open instance rename modal
function openInstanceRenameModal(instanceId) {
    const instance = instances.find(i => i.id === instanceId);
    if (!instance) return;

    instanceRenameId = instanceId;
    const nameInput = document.getElementById('instanceNameInput');
    if (nameInput) {
        nameInput.value = instance.name;
    }

    document.getElementById('instanceRenameModal')?.classList.remove('hidden');

    // Focus input after modal opens
    setTimeout(() => nameInput?.focus(), 100);
}

// Close instance rename modal
function closeInstanceRenameModal() {
    instanceRenameId = null;
    document.getElementById('instanceRenameModal')?.classList.add('hidden');
}

// Save instance rename
async function saveInstanceRename() {
    if (!instanceRenameId) return;

    const nameInput = document.getElementById('instanceNameInput');
    const newName = nameInput?.value?.trim();

    if (!newName) {
        alert('Please enter a name for the instance.');
        return;
    }

    try {
        await invoke('rename_instance', { instanceId: instanceRenameId, newName });

        // Update local state
        const instance = instances.find(i => i.id === instanceRenameId);
        if (instance) {
            instance.name = newName;
        }

        console.log(`Renamed instance ${instanceRenameId} to: ${newName}`);
        closeInstanceRenameModal();
        updateInstanceUI();
    } catch (e) {
        console.error('Failed to rename instance:', e);
        alert('Failed to rename instance: ' + (e.message || e));
    }
}

// ==================== Keyword Discovery ====================
let isDiscoveringKeywords = false;

function showKeywordModal() {
    if (!selectedProductId) {
        showMessageModal('Please select a product first.', 'No Product Selected', 'warning');
        return;
    }
    document.getElementById('keywordModal')?.classList.remove('hidden');
    document.getElementById('seedKeywordInput')?.focus();
}

function hideKeywordModal(force = false) {
    if (isDiscoveringKeywords && !force) return; // Don't close while discovering (unless forced)
    document.getElementById('keywordModal')?.classList.add('hidden');
    document.getElementById('seedKeywordInput').value = '';
    document.getElementById('keywordProgress')?.classList.add('hidden');
}

async function handleStartKeywordDiscovery() {
    const seedKeywordInput = document.getElementById('seedKeywordInput');
    const seedKeyword = seedKeywordInput?.value?.trim();

    if (!seedKeyword) {
        hideKeywordModal(true);
        showMessageModal('Please enter a seed keyword.', 'Keyword Required', 'warning');
        return;
    }

    if (!selectedProductId) {
        hideKeywordModal(true);
        showMessageModal('Please select a product first.', 'No Product Selected', 'warning');
        return;
    }

    if (isDiscoveringKeywords) return;

    isDiscoveringKeywords = true;
    const startBtn = document.getElementById('startKeywordBtn');
    const cancelBtn = document.getElementById('cancelKeywordBtn');
    const progressSection = document.getElementById('keywordProgress');

    // Update UI
    if (startBtn) {
        startBtn.disabled = true;
        startBtn.querySelector('.btn-text').classList.add('hidden');
        startBtn.querySelector('.btn-loading').classList.remove('hidden');
    }
    if (cancelBtn) cancelBtn.disabled = true;
    if (progressSection) progressSection.classList.remove('hidden');

    try {
        console.log(`[Keyword Discovery] Starting with keyword: ${seedKeyword} for product: ${selectedProductId}`);

        const result = await invoke('start_paa_discovery', {
            productId: selectedProductId,
            seedKeyword: seedKeyword
        });

        console.log('[Keyword Discovery] Result:', result);

        // Always close the keyword modal first before showing any result modal
        hideKeywordModal(true);

        if (result.code === 'RATE_LIMIT_EXCEEDED') {
            showMessageModal(result.message || 'Rate limit exceeded. Please try again later.', 'Rate Limited', 'warning');
        } else if (result.code === 'GOOGLE_AUTH_REQUIRED') {
            showMessageModal(result.message || 'Please authenticate Google AI Overview first.', 'Authentication Required', 'warning');
        } else if (result.code === 'NO_PAA_FOUND') {
            showMessageModal(result.message || 'No "People Also Ask" section found. Try a different keyword.', 'No Results', 'info');
        } else if (!result.success && result.error) {
            showMessageModal(result.message || result.error || 'Failed to discover keywords.', 'Error', 'error');
        } else if (result.success) {
            showMessageModal(
                'The discovered questions will be analyzed and should be available in your Columbus Dashboard shortly.',
                'Discovery Complete',
                'success'
            );
        } else if (result.error) {
            showMessageModal(result.error, 'Discovery Failed', 'error');
        }
    } catch (e) {
        console.error('[Keyword Discovery] Error:', e);
        hideKeywordModal(true);
        showMessageModal(e.message || 'Failed to discover keywords', 'Error', 'error');
    } finally {
        isDiscoveringKeywords = false;

        // Reset UI
        if (startBtn) {
            startBtn.disabled = false;
            startBtn.querySelector('.btn-text').classList.remove('hidden');
            startBtn.querySelector('.btn-loading').classList.add('hidden');
        }
        if (cancelBtn) cancelBtn.disabled = false;
        if (progressSection) progressSection.classList.add('hidden');
    }
}

function updateKeywordProgress(progress) {
    const progressFill = document.getElementById('keywordProgressFill');
    const progressText = document.getElementById('keywordProgressText');
    const progressMessage = document.getElementById('keywordProgressMessage');

    if (progressFill) progressFill.style.width = `${progress.current}%`;
    if (progressText) progressText.textContent = `${progress.current}%`;
    if (progressMessage) progressMessage.textContent = progress.message || 'Processing...';
}

// Listen for PAA progress events
listen('paa:progress', (event) => {
    console.log('[Keyword Discovery] Progress:', event.payload);
    updateKeywordProgress(event.payload);
});

// Update keyword button state when product changes
function updateKeywordButtonState() {
    const btn = document.getElementById('findKeywordsBtn');
    if (btn) {
        btn.disabled = !selectedProductId || isScanning;
    }
}

// ==================== Initialize ====================
document.addEventListener('DOMContentLoaded', () => {
    console.log('DOMContentLoaded fired');
    init().catch(e => {
        console.error('Init error:', e);
    });
});

console.log('Main.js loaded');
