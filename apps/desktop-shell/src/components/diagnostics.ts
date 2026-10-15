import { UMRClient } from '../rpc-client';
import { DiagnosticEntry, DiagnosticReport } from '../types';

export class DiagnosticsComponent {
  private container: HTMLElement;
  private client: UMRClient;
  private logs: DiagnosticEntry[] = [];
  private currentSessionId: string | null = null;
  private filterLevel = 'all';
  private searchQuery = '';
  private autoScroll = true;
  private isDrawerOpen = true;

  constructor(container: HTMLElement, client: UMRClient) {
    this.container = container;
    this.client = client;
    this.render();
  }

  toggleDrawer() {
    this.isDrawerOpen = !this.isDrawerOpen;
    const drawer = this.container.querySelector('.diagnostics-drawer') as HTMLElement;
    if (drawer) {
      drawer.classList.toggle('drawer-collapsed', !this.isDrawerOpen);
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
          <div class="diag-title-group">
            <h3>Live Diagnostics Stream</h3>
            <span class="badge badge-success">🔒 Secrets Sanitized</span>
          </div>

          <div class="diag-controls">
            <select class="select-filter-sm" id="diag-level-filter">
              <option value="all">All Levels</option>
              <option value="lifecycle">Activity & Input (Hide Ticks)</option>
              <option value="debug">Debug / Verbose</option>
              <option value="info">Info</option>
              <option value="warn">Warnings</option>
              <option value="error">Errors</option>
            </select>
            <input 
              type="text" 
              class="input-search-sm" 
              id="diag-search" 
              placeholder="Search logs..." 
            />
            <button class="btn btn-sm btn-outline" id="diag-autoscroll-toggle" title="Toggle automatic scrolling">
              ⚡ Auto-Scroll: ON
            </button>
            <button class="btn btn-sm btn-outline" id="diag-export-btn" title="Export Diagnostic Report as JSON">
              ⬇ Export
            </button>
            <button class="btn btn-sm btn-subtle" id="diag-clear-btn" title="Clear view">
              ✕ Clear
            </button>
          </div>
        </div>

        <div class="diag-log-container" id="diag-log-list">
          <div class="diag-empty-placeholder">
            Waiting for session logs or system activity...
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
    const clearBtn = this.container.querySelector('#diag-clear-btn') as HTMLElement;
    const exportBtn = this.container.querySelector('#diag-export-btn') as HTMLElement;
    const autoScrollBtn = this.container.querySelector('#diag-autoscroll-toggle') as HTMLElement;
    const resumeBtn = this.container.querySelector('#diag-resume-autoscroll') as HTMLElement;

    levelSelect.addEventListener('change', () => {
      this.filterLevel = levelSelect.value;
      this.renderLogs();
    });

    searchInput.addEventListener('input', () => {
      this.searchQuery = searchInput.value.toLowerCase().trim();
      this.renderLogs();
    });

    clearBtn.addEventListener('click', () => {
      this.logs = [];
      this.renderLogs();
    });

    exportBtn.addEventListener('click', () => {
      this.exportReport();
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

    // Detect user manual scroll up/down
    list.addEventListener('scroll', () => {
      const isAtBottom = list.scrollHeight - list.scrollTop - list.clientHeight < 40;
      if (!isAtBottom) {
        if (this.autoScroll) {
          this.autoScroll = false;
          this.updateAutoScrollBtn();
        }
        if (resumeBtn) resumeBtn.style.display = 'flex';
      } else {
        if (!this.autoScroll) {
          this.autoScroll = true;
          this.updateAutoScrollBtn();
        }
        if (resumeBtn) resumeBtn.style.display = 'none';
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

    const previousScrollTop = list.scrollTop;

    const filtered = this.logs.filter((entry) => {
      const comp = entry.source || entry.component || 'guest';
      const isTick = entry.message.includes('Frame render tick:');

      let matchLevel = true;
      if (this.filterLevel === 'lifecycle') {
        matchLevel = !isTick;
      } else if (this.filterLevel !== 'all') {
        matchLevel = entry.level.toLowerCase() === this.filterLevel;
      }

      const matchQuery =
        !this.searchQuery ||
        comp.toLowerCase().includes(this.searchQuery) ||
        entry.message.toLowerCase().includes(this.searchQuery);

      return matchLevel && matchQuery;
    });

    if (filtered.length === 0) {
      list.innerHTML = `<div class="diag-empty-placeholder">No log events match filter</div>`;
      return;
    }

    list.innerHTML = filtered
      .map((entry) => {
        const time = new Date(entry.timestamp).toLocaleTimeString();
        const levelClass = `log-level-${entry.level.toLowerCase()}`;
        const sanitizedMsg = this.highlightRedactions(entry.message);
        const comp = entry.source || entry.component || 'guest';

        return `
          <div class="diag-log-row">
            <span class="log-time">${time}</span>
            <span class="log-level ${levelClass}">[${entry.level.toUpperCase()}]</span>
            <span class="log-comp">${comp}:</span>
            <span class="log-msg">${sanitizedMsg}</span>
          </div>
        `;
      })
      .join('');

    if (this.autoScroll) {
      list.scrollTop = list.scrollHeight;
    } else {
      list.scrollTop = previousScrollTop;
    }
  }

  private highlightRedactions(msg: string): string {
    return msg.replace(
      /\[REDACTED\]/g,
      '<span class="badge-redacted" title="Sanitizer sanitized sensitive value">[REDACTED]</span>'
    );
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

