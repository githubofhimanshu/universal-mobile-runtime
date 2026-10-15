import { UMRClient } from '../rpc-client';
import { DiagnosticEntry, DiagnosticReport } from '../types';

interface ParsedLogLine {
  priority: 'VERBOSE' | 'DEBUG' | 'INFO' | 'WARN' | 'ERROR' | 'FATAL';
  priorityAbbr: string;
  tag: string;
  body: string;
  tagColorClass: string;
  isTick: boolean;
  category: 'lifecycle' | 'input' | 'graphics' | 'storage' | 'system';
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

function getTagColorClass(tag: string): string {
  const lower = tag.toLowerCase();
  if (/input|key|pointer|touch|motion|nav/.test(lower)) return 'tag-purple';
  if (/activity|window|lifecycle|intent|am/.test(lower)) return 'tag-emerald';
  if (/surface|render|viewroot|frame|graphic|gl|choreograph/.test(lower)) return 'tag-sky';
  if (/storage|package|dex|install|file|pm/.test(lower)) return 'tag-amber';
  if (/audio|media|sound|video/.test(lower)) return 'tag-pink';
  if (/runtime|crash|error|fatal|fault/.test(lower)) return 'tag-rose';

  const colors = ['tag-purple', 'tag-emerald', 'tag-sky', 'tag-amber', 'tag-pink', 'tag-teal'];
  let hash = 0;
  for (let i = 0; i < tag.length; i++) hash = (hash << 5) - hash + tag.charCodeAt(i);
  return colors[Math.abs(hash) % colors.length];
}

function parseLogEntry(entry: DiagnosticEntry): ParsedLogLine {
  const rawMsg = entry.message || '';
  const fallbackTag = entry.source || entry.component || 'guest';
  const match = rawMsg.match(/^([VDIWEF])\/([a-zA-Z0-9_.$#\-]+):\s*(.*)$/s);

  let priority: ParsedLogLine['priority'] = 'INFO';
  let priorityAbbr = 'INF';
  let tag = fallbackTag;
  let body = rawMsg;

  if (match) {
    const pChar = match[1];
    tag = match[2];
    body = match[3];

    switch (pChar) {
      case 'V':
        priority = 'VERBOSE';
        priorityAbbr = 'VRB';
        break;
      case 'D':
        priority = 'DEBUG';
        priorityAbbr = 'DBG';
        break;
      case 'I':
        priority = 'INFO';
        priorityAbbr = 'INF';
        break;
      case 'W':
        priority = 'WARN';
        priorityAbbr = 'WRN';
        break;
      case 'E':
        priority = 'ERROR';
        priorityAbbr = 'ERR';
        break;
      case 'F':
        priority = 'FATAL';
        priorityAbbr = 'FTL';
        break;
    }
  } else {
    const normLevel = (entry.level || '').toUpperCase();
    if (normLevel.includes('ERR')) {
      priority = 'ERROR';
      priorityAbbr = 'ERR';
    } else if (normLevel.includes('WARN')) {
      priority = 'WARN';
      priorityAbbr = 'WRN';
    } else if (normLevel.includes('DEBUG')) {
      priority = 'DEBUG';
      priorityAbbr = 'DBG';
    } else if (normLevel.includes('VERB')) {
      priority = 'VERBOSE';
      priorityAbbr = 'VRB';
    }
  }

  const isTick = rawMsg.includes('Frame render tick:');
  const lowerTag = tag.toLowerCase();
  const lowerBody = body.toLowerCase();

  let category: ParsedLogLine['category'] = 'system';
  if (/input|key|pointer|touch|motion|nav|keycode/.test(lowerTag) || /input|key|pointer|touch|motion|nav|keycode/.test(lowerBody)) {
    category = 'input';
  } else if (/activity|window|lifecycle|intent|am/.test(lowerTag) || /start u0|intent/.test(lowerBody)) {
    category = 'lifecycle';
  } else if (/surface|render|viewroot|frame|choreograph|bufferqueue/.test(lowerTag) || isTick) {
    category = 'graphics';
  } else if (/storage|package|dex|install|file|pm/.test(lowerTag)) {
    category = 'storage';
  }

  return {
    priority,
    priorityAbbr,
    tag,
    body,
    tagColorClass: getTagColorClass(tag),
    isTick,
    category,
  };
}

export class DiagnosticsComponent {
  private container: HTMLElement;
  private client: UMRClient;
  private logs: DiagnosticEntry[] = [];
  private currentSessionId: string | null = null;
  private filterLevel: string = 'all';
  private filterCategory: string = 'all';
  private hideTicks: boolean = true;
  private searchQuery: string = '';
  private autoScroll: boolean = true;
  private isDrawerOpen: boolean = true;
  private isWideMode: boolean = false;
  private isWrapMode: boolean = false;

  constructor(container: HTMLElement, client: UMRClient) {
    this.container = container;
    this.client = client;
    this.render();
  }

  toggleDrawer() {
    this.isDrawerOpen = !this.isDrawerOpen;
    const panel = document.getElementById('diagnostics-panel');
    if (panel) {
      panel.classList.toggle('panel-collapsed', !this.isDrawerOpen);
    }
  }

  toggleWideMode() {
    this.isWideMode = !this.isWideMode;
    const panel = document.getElementById('diagnostics-panel');
    if (panel) {
      panel.classList.toggle('panel-wide', this.isWideMode);
    }
    const wideBtn = this.container.querySelector('#diag-wide-toggle') as HTMLButtonElement;
    if (wideBtn) {
      wideBtn.innerHTML = this.isWideMode ? '⤡ Compact' : '⤢ Expand';
      wideBtn.title = this.isWideMode ? 'Switch to standard drawer width' : 'Expand drawer for wide logcat viewing';
    }
  }

  async updateLogsForSession(sessionId: string) {
    this.currentSessionId = sessionId;
    try {
      const report = await this.client.getReport(sessionId);
      if (report && report.entries && report.entries.length !== this.logs.length) {
        this.logs = report.entries;
        this.renderLogs();
      }
    } catch (e) {
      // Ignored if session has no new logs yet
    }
  }

  private render() {
    this.container.innerHTML = `
      <div class="diagnostics-drawer">
        <div class="diag-header">
          <!-- Top Bar: Title & Telemetry Counters -->
          <div class="diag-header-top">
            <div class="diag-title-group">
              <span class="diag-pulse-live" title="Streaming live guest/runtime logs"></span>
              <h3>Live Diagnostics Stream</h3>
            </div>

            <div class="diag-telemetry-pills">
              <span class="telemetry-pill pill-total" id="telemetry-total" title="Total received logs">0 Total</span>
              <span class="telemetry-pill pill-warn" id="telemetry-warn" title="Warnings (Click to filter)" style="display: none;">0 Warn</span>
              <span class="telemetry-pill pill-error" id="telemetry-error" title="Errors (Click to filter)" style="display: none;">0 Err</span>
              <span class="telemetry-pill pill-sanitized" title="Hardware/Guest logs auto-sanitized for secrets">🔒 Sanitized</span>
            </div>

            <div class="diag-header-actions">
              <button class="btn btn-sm btn-outline diag-action-btn" id="diag-wide-toggle" title="Expand drawer for wide logcat viewing">
                ⤢ Expand
              </button>
              <button class="btn btn-sm btn-outline diag-action-btn" id="diag-copy-all-btn" title="Copy all visible logs to clipboard">
                📋 Copy All
              </button>
              <button class="btn btn-sm btn-outline diag-action-btn" id="diag-export-btn" title="Export Diagnostic Report as JSON">
                ⬇ Export
              </button>
              <button class="btn btn-sm btn-subtle diag-action-btn" id="diag-clear-btn" title="Clear view">
                ✕ Clear
              </button>
            </div>
          </div>

          <!-- Secondary Bar: Controls & Filters -->
          <div class="diag-controls-bar">
            <select class="select-filter-sm" id="diag-level-filter" title="Filter by Log Level">
              <option value="all">All Levels</option>
              <option value="verbose">VERBOSE</option>
              <option value="debug">DEBUG</option>
              <option value="info">INFO</option>
              <option value="warn">WARN</option>
              <option value="error">ERROR</option>
            </select>

            <div class="diag-search-box">
              <span class="diag-search-icon">🔍</span>
              <input 
                type="text" 
                class="input-search-sm" 
                id="diag-search" 
                placeholder="Filter logs or tags (e.g. KEYCODE, ViewRoot)..." 
              />
              <span class="diag-search-count" id="diag-search-count" style="display: none;"></span>
              <button class="diag-search-clear" id="diag-search-clear" style="display: none;" title="Clear search">✕</button>
            </div>

            <button class="btn btn-sm btn-outline" id="diag-wrap-toggle" title="Toggle line wrap (Horizontal scrolling)">
              ↔ No Wrap
            </button>
            <button class="btn btn-sm btn-outline" id="diag-autoscroll-toggle" title="Toggle automatic scrolling">
              ⚡ Auto-Scroll: ON
            </button>
          </div>

          <!-- Tertiary Bar: Quick Category Chips -->
          <div class="diag-category-bar">
            <span class="diag-cat-label">Categories:</span>
            <button class="diag-chip active" data-category="all">★ All</button>
            <button class="diag-chip" data-category="lifecycle">📱 Lifecycle</button>
            <button class="diag-chip" data-category="input">🎮 Input & Keys</button>
            <button class="diag-chip" data-category="graphics">🎨 Graphics</button>
            <button class="diag-chip" data-category="storage">💾 Storage</button>
            <button class="diag-chip chip-toggle active" id="diag-chip-hide-ticks" title="Hide repeating Frame render ticks">🚫 Hide Ticks</button>
          </div>
        </div>

        <div class="diag-log-container" id="diag-log-list">
          <div class="diag-empty-placeholder">
            <div class="diag-empty-icon">📡</div>
            <div class="diag-empty-title">Waiting for Live Session Diagnostics</div>
            <div class="diag-empty-desc">Launch an application from the library to stream real-time Android logcat, hardware key events, and runtime telemetry.</div>
          </div>
        </div>

        <button class="diag-resume-autoscroll" id="diag-resume-autoscroll" style="display: none;">
          ⬇ Jump to Bottom
        </button>
      </div>
    `;

    this.attachEventListeners();
  }

  private attachEventListeners() {
    const list = this.container.querySelector('#diag-log-list') as HTMLElement;
    const levelSelect = this.container.querySelector('#diag-level-filter') as HTMLSelectElement;
    const searchInput = this.container.querySelector('#diag-search') as HTMLInputElement;
    const searchClear = this.container.querySelector('#diag-search-clear') as HTMLElement;
    const clearBtn = this.container.querySelector('#diag-clear-btn') as HTMLElement;
    const exportBtn = this.container.querySelector('#diag-export-btn') as HTMLElement;
    const copyAllBtn = this.container.querySelector('#diag-copy-all-btn') as HTMLElement;
    const wideBtn = this.container.querySelector('#diag-wide-toggle') as HTMLElement;
    const autoScrollBtn = this.container.querySelector('#diag-autoscroll-toggle') as HTMLElement;
    const resumeBtn = this.container.querySelector('#diag-resume-autoscroll') as HTMLElement;
    const hideTicksChip = this.container.querySelector('#diag-chip-hide-ticks') as HTMLElement;
    const categoryChips = this.container.querySelectorAll('.diag-chip[data-category]');

    levelSelect?.addEventListener('change', () => {
      this.filterLevel = levelSelect.value;
      this.renderLogs();
    });

    searchInput?.addEventListener('input', () => {
      this.searchQuery = searchInput.value.toLowerCase().trim();
      if (searchClear) {
        searchClear.style.display = this.searchQuery ? 'block' : 'none';
      }
      this.renderLogs();
    });

    searchClear?.addEventListener('click', () => {
      if (searchInput) {
        searchInput.value = '';
        this.searchQuery = '';
        searchClear.style.display = 'none';
        this.renderLogs();
      }
    });

    categoryChips.forEach((chip) => {
      chip.addEventListener('click', () => {
        categoryChips.forEach((c) => c.classList.remove('active'));
        chip.classList.add('active');
        this.filterCategory = chip.getAttribute('data-category') || 'all';
        this.renderLogs();
      });
    });

    hideTicksChip?.addEventListener('click', () => {
      this.hideTicks = !this.hideTicks;
      hideTicksChip.classList.toggle('active', this.hideTicks);
      this.renderLogs();
    });

    clearBtn?.addEventListener('click', () => {
      this.logs = [];
      this.renderLogs();
    });

    exportBtn?.addEventListener('click', () => {
      this.exportReport();
    });

    copyAllBtn?.addEventListener('click', () => {
      this.copyAllLogs(copyAllBtn);
    });

    wideBtn?.addEventListener('click', () => {
      this.toggleWideMode();
    });

    // Telemetry click shortcuts
    const warnPill = this.container.querySelector('#telemetry-warn') as HTMLElement;
    warnPill?.addEventListener('click', () => {
      if (levelSelect) {
        levelSelect.value = this.filterLevel === 'warn' ? 'all' : 'warn';
        this.filterLevel = levelSelect.value;
        this.renderLogs();
      }
    });

    const errorPill = this.container.querySelector('#telemetry-error') as HTMLElement;
    errorPill?.addEventListener('click', () => {
      if (levelSelect) {
        levelSelect.value = this.filterLevel === 'error' ? 'all' : 'error';
        this.filterLevel = levelSelect.value;
        this.renderLogs();
      }
    });

    const wrapToggleBtn = this.container.querySelector('#diag-wrap-toggle') as HTMLElement;
    wrapToggleBtn?.addEventListener('click', () => {
      this.isWrapMode = !this.isWrapMode;
      list.classList.toggle('diag-wrap-mode', this.isWrapMode);
      if (wrapToggleBtn) {
        wrapToggleBtn.textContent = this.isWrapMode ? '↩ Wrapped' : '↔ No Wrap';
        wrapToggleBtn.title = this.isWrapMode
          ? 'Lines are wrapped onto multiple lines (Click for horizontal scrolling)'
          : 'Lines preserved on single line with horizontal scrolling (Click to wrap)';
      }
    });

    autoScrollBtn?.addEventListener('click', () => {
      this.autoScroll = !this.autoScroll;
      this.updateAutoScrollBtn();
      if (this.autoScroll && list) {
        list.scrollTop = list.scrollHeight;
        if (resumeBtn) resumeBtn.style.display = 'none';
      }
    });

    resumeBtn?.addEventListener('click', () => {
      this.autoScroll = true;
      this.updateAutoScrollBtn();
      if (list) {
        list.scrollTop = list.scrollHeight;
      }
      resumeBtn.style.display = 'none';
    });

    // Wheel event: explicitly track user wheel up vs down
    list?.addEventListener('wheel', (e: WheelEvent) => {
      if (e.deltaY < 0) {
        // User scrolled UP: pause auto-scroll immediately so incoming logs don't snap back
        this.autoScroll = false;
        this.updateAutoScrollBtn();
        if (resumeBtn) resumeBtn.style.display = 'flex';
      } else if (e.deltaY > 0) {
        // User scrolling DOWN: re-enable auto-scroll when reaching the bottom
        const distanceToBottom = list.scrollHeight - list.scrollTop - list.clientHeight;
        if (distanceToBottom <= 10) {
          this.autoScroll = true;
          this.updateAutoScrollBtn();
          if (resumeBtn) resumeBtn.style.display = 'none';
        }
      }
    }, { passive: true });

    // Scroll event: handles scrollbar drag, trackpad, and keyboard navigation
    list?.addEventListener('scroll', () => {
      const distanceToBottom = list.scrollHeight - list.scrollTop - list.clientHeight;
      if (distanceToBottom > 15) {
        if (this.autoScroll) {
          this.autoScroll = false;
          this.updateAutoScrollBtn();
        }
        if (resumeBtn) resumeBtn.style.display = 'flex';
      } else if (distanceToBottom <= 8) {
        if (!this.autoScroll) {
          this.autoScroll = true;
          this.updateAutoScrollBtn();
        }
        if (resumeBtn) resumeBtn.style.display = 'none';
      }
    });

    // Delegated click event for individual row copy buttons
    list?.addEventListener('click', (ev) => {
      const target = ev.target as HTMLElement;
      const copyBtn = target.closest('.diag-row-copy-btn') as HTMLButtonElement;
      if (copyBtn) {
        const text = copyBtn.getAttribute('data-copy-text') || '';
        if (text) {
          navigator.clipboard.writeText(text).then(() => {
            const originalText = copyBtn.innerHTML;
            copyBtn.innerHTML = '✓';
            copyBtn.classList.add('copied');
            setTimeout(() => {
              copyBtn.innerHTML = originalText;
              copyBtn.classList.remove('copied');
            }, 1200);
          });
        }
      }
    });
  }

  private updateAutoScrollBtn() {
    const btn = this.container.querySelector('#diag-autoscroll-toggle') as HTMLElement;
    if (btn) {
      btn.textContent = this.autoScroll ? '⚡ Auto-Scroll: ON' : '⏸ Auto-Scroll: OFF';
      btn.style.borderColor = this.autoScroll ? 'var(--primary)' : 'var(--border-color)';
      btn.style.color = this.autoScroll ? '#00e5ff' : 'var(--text-muted)';
    }
  }

  private renderLogs() {
    const list = this.container.querySelector('#diag-log-list') as HTMLElement;
    if (!list) return;

    // Update telemetry counts
    this.updateTelemetry();

    const previousScrollTop = list.scrollTop;
    const previousScrollLeft = list.scrollLeft;

    list.classList.toggle('diag-wrap-mode', this.isWrapMode);

    // Parse all entries
    const parsedEntries = this.logs.map((entry) => ({
      entry,
      parsed: parseLogEntry(entry),
    }));

    // Filter
    const filtered = parsedEntries.filter(({ entry, parsed }) => {
      // 1. Tick spam filter
      if (this.hideTicks && parsed.isTick) {
        return false;
      }

      // 2. Level filter
      if (this.filterLevel !== 'all') {
        if (parsed.priority.toLowerCase() !== this.filterLevel) {
          return false;
        }
      }

      // 3. Category filter
      if (this.filterCategory !== 'all') {
        if (parsed.category !== this.filterCategory) {
          return false;
        }
      }

      // 4. Search query
      if (this.searchQuery) {
        const q = this.searchQuery;
        const inTag = parsed.tag.toLowerCase().includes(q);
        const inBody = parsed.body.toLowerCase().includes(q);
        const inLevel = parsed.priority.toLowerCase().includes(q);
        if (!inTag && !inBody && !inLevel) {
          return false;
        }
      }

      return true;
    });

    // Update search count badge
    const searchCountEl = this.container.querySelector('#diag-search-count') as HTMLElement;
    if (searchCountEl) {
      if (this.searchQuery) {
        searchCountEl.textContent = `${filtered.length} matches`;
        searchCountEl.style.display = 'inline-block';
      } else {
        searchCountEl.style.display = 'none';
      }
    }

    if (this.logs.length === 0) {
      list.innerHTML = `
        <div class="diag-empty-placeholder">
          <div class="diag-empty-icon">📡</div>
          <div class="diag-empty-title">Waiting for Live Session Diagnostics</div>
          <div class="diag-empty-desc">Launch an application from the library to stream real-time Android logcat, hardware key events, and runtime telemetry.</div>
        </div>
      `;
      return;
    }

    if (filtered.length === 0) {
      list.innerHTML = `
        <div class="diag-empty-placeholder">
          <div class="diag-empty-icon">🔍</div>
          <div class="diag-empty-title">No logs match current filters</div>
          <div class="diag-empty-desc">Try resetting your search query or category filters.</div>
          <button class="btn btn-sm btn-outline diag-reset-btn" id="diag-btn-reset-filters" style="margin-top: 12px;">
            Reset All Filters
          </button>
        </div>
      `;
      const resetBtn = list.querySelector('#diag-btn-reset-filters');
      resetBtn?.addEventListener('click', () => {
        this.filterLevel = 'all';
        this.filterCategory = 'all';
        this.searchQuery = '';
        const levelSelect = this.container.querySelector('#diag-level-filter') as HTMLSelectElement;
        const searchInput = this.container.querySelector('#diag-search') as HTMLInputElement;
        const chips = this.container.querySelectorAll('.diag-chip[data-category]');
        if (levelSelect) levelSelect.value = 'all';
        if (searchInput) searchInput.value = '';
        chips.forEach((c) => c.classList.toggle('active', c.getAttribute('data-category') === 'all'));
        this.renderLogs();
      });
      return;
    }

    list.innerHTML = filtered
      .map(({ entry, parsed }) => {
        const time = new Date(entry.timestamp).toLocaleTimeString();
        const levelClass = `diag-level-${parsed.priority.toLowerCase()}`;
        const sanitizedMsg = this.highlightRedactions(parsed.body);
        const highlightedMsg = this.highlightSearch(sanitizedMsg);
        const highlightedTag = this.highlightSearch(escapeHtml(parsed.tag));

        // Format string for 1-click line copying
        const rawLineText = `[${time}] [${parsed.priority}] ${parsed.tag}: ${parsed.body}`;
        const escapedCopyText = escapeHtml(rawLineText);

        return `
          <div class="diag-log-row ${levelClass}-row">
            <span class="log-time" title="${entry.timestamp}">${time}</span>
            <span class="log-prio ${levelClass}" title="Priority: ${parsed.priority}">[${parsed.priorityAbbr}]</span>
            <span class="diag-tag-chip ${parsed.tagColorClass}" title="Component Tag">${highlightedTag}</span>
            <span class="diag-msg-body">${highlightedMsg}</span>
            <button class="diag-row-copy-btn" data-copy-text="${escapedCopyText}" title="Copy log line">
              📋
            </button>
          </div>
        `;
      })
      .join('');

    if (this.autoScroll) {
      list.scrollTop = list.scrollHeight;
    } else {
      list.scrollTop = previousScrollTop;
    }
    list.scrollLeft = previousScrollLeft;
  }

  private updateTelemetry() {
    const totalPill = this.container.querySelector('#telemetry-total') as HTMLElement;
    const warnPill = this.container.querySelector('#telemetry-warn') as HTMLElement;
    const errorPill = this.container.querySelector('#telemetry-error') as HTMLElement;

    if (totalPill) totalPill.textContent = `${this.logs.length} Total`;

    let warns = 0;
    let errors = 0;
    for (const log of this.logs) {
      const parsed = parseLogEntry(log);
      if (parsed.priority === 'WARN') warns++;
      if (parsed.priority === 'ERROR' || parsed.priority === 'FATAL') errors++;
    }

    if (warnPill) {
      if (warns > 0) {
        warnPill.textContent = `⚠️ ${warns} Warn`;
        warnPill.style.display = 'inline-flex';
      } else {
        warnPill.style.display = 'none';
      }
    }

    if (errorPill) {
      if (errors > 0) {
        errorPill.textContent = `🚨 ${errors} Err`;
        errorPill.style.display = 'inline-flex';
      } else {
        errorPill.style.display = 'none';
      }
    }
  }

  private highlightRedactions(msg: string): string {
    const escaped = escapeHtml(msg);
    return escaped.replace(
      /\[REDACTED\]/g,
      '<span class="badge-redacted" title="Sanitizer sanitized sensitive token">[REDACTED]</span>'
    );
  }

  private highlightSearch(html: string): string {
    if (!this.searchQuery) return html;
    const escapedQ = escapeHtml(this.searchQuery).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const regex = new RegExp(`(${escapedQ})`, 'gi');
    return html.replace(regex, '<mark class="diag-highlight">$1</mark>');
  }

  private copyAllLogs(btn: HTMLElement) {
    const lines = this.logs
      .filter((entry) => {
        const parsed = parseLogEntry(entry);
        if (this.hideTicks && parsed.isTick) return false;
        if (this.filterLevel !== 'all' && parsed.priority.toLowerCase() !== this.filterLevel) return false;
        if (this.filterCategory !== 'all' && parsed.category !== this.filterCategory) return false;
        return true;
      })
      .map((entry) => {
        const time = new Date(entry.timestamp).toLocaleTimeString();
        const parsed = parseLogEntry(entry);
        return `[${time}] [${parsed.priority}] ${parsed.tag}: ${parsed.body}`;
      });

    const textToCopy = lines.join('\n');
    navigator.clipboard.writeText(textToCopy).then(() => {
      const originalText = btn.innerHTML;
      btn.innerHTML = '✓ Copied!';
      btn.classList.add('btn-success');
      setTimeout(() => {
        btn.innerHTML = originalText;
        btn.classList.remove('btn-success');
      }, 1500);
    });
  }

  private exportReport() {
    const report: DiagnosticReport = {
      session_id: this.currentSessionId || 'offline',
      package_id: 'umr-session-diagnostic',
      created_at: new Date().toISOString(),
      entries: this.logs,
    };

    const blob = new Blob([JSON.stringify(report, null, 2)], {
      type: 'application/json',
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `umr-diagnostic-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }
}
