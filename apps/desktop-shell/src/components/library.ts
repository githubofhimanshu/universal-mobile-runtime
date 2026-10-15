import { UMRClient } from '../rpc-client';
import { ExecutionMode, NegotiationResult, PackageMetadata } from '../types';

export interface AppEntry {
  id: string;
  name: string;
  packageName: string;
  version: string;
  format: 'apk' | 'ipa';
  filePath: string;
  executionMode: ExecutionMode;
  executionModeLabel: string;
  detailsSummary: string;
  iconText: string;
  isCompatible: boolean;
}

export class LibraryComponent {
  private container: HTMLElement;
  private client: UMRClient;
  private apps: AppEntry[] = [];
  private onLaunchApp?: (app: AppEntry, sessionId: string) => void;
  private onShowErrorModal?: (title: string, error: any) => void;
  private onShowInspectModal?: (
    app: AppEntry,
    neg: NegotiationResult,
    meta: PackageMetadata,
    onLaunch?: (app: AppEntry) => void
  ) => void;

  constructor(container: HTMLElement, client: UMRClient) {
    this.container = container;
    this.client = client;
    this.initDefaultCatalog();
    this.render();
  }

  setCallbacks(
    onLaunch: (app: AppEntry, sessionId: string) => void,
    onShowError: (title: string, error: any) => void,
    onInspect: (
      app: AppEntry,
      neg: NegotiationResult,
      meta: PackageMetadata,
      onLaunch?: (app: AppEntry) => void
    ) => void
  ) {
    this.onLaunchApp = onLaunch;
    this.onShowErrorModal = onShowError;
    this.onShowInspectModal = onInspect;
  }

  private initDefaultCatalog() {
    this.apps = [
      {
        id: 'synthetic-hello',
        name: 'Synthetic Hello App',
        packageName: 'org.umr.synthetic.hello',
        version: '1.0.0',
        format: 'apk',
        filePath: 'tests/fixtures/synthetic-app.apk',
        executionMode: 'local_native',
        executionModeLabel: 'Local (Native)',
        detailsSummary: 'Android 14 (API 21..34) • x86_64, Arm64',
        iconText: '🤖',
        isCompatible: true,
      },
      {
        id: 'ios-sim-app',
        name: 'iOS Simulator Showcase',
        packageName: 'com.example.simapp',
        version: '2.1.0',
        format: 'ipa',
        filePath: 'tests/fixtures/valid-sim.ipa',
        executionMode: 'remote_stream',
        executionModeLabel: 'Remote / Device Stream',
        detailsSummary: 'iOS 16+ • Simulator SDK / Tethered Stream',
        iconText: '🍎',
        isCompatible: true,
      },
      {
        id: 'fairplay-encrypted',
        name: 'FairPlay Encrypted Game',
        packageName: 'com.example.storegame',
        version: '1.0.0',
        format: 'ipa',
        filePath: 'tests/fixtures/encrypted-mock.ipa',
        executionMode: 'unsupported',
        executionModeLabel: 'Unsupported (DRM)',
        detailsSummary: 'FairPlay cryptid=1 • Rejection enforced',
        iconText: '🔒',
        isCompatible: false,
      },
      {
        id: 'malformed-slip',
        name: 'Path Traversal Exploit',
        packageName: 'com.malicious.slip',
        version: '0.0.1',
        format: 'apk',
        filePath: 'tests/fixtures/malformed-slip.apk',
        executionMode: 'unsupported',
        executionModeLabel: 'Security Violation',
        detailsSummary: 'Zip Slip path traversal blocked',
        iconText: '⚠️',
        isCompatible: false,
      },
    ];
  }

  addApp(app: AppEntry) {
    this.apps.unshift(app);
    this.render();
  }

  private render() {
    this.container.innerHTML = `
      <div class="library-header">
        <div class="library-title-group">
          <h2>Application Catalog</h2>
          <span class="catalog-count">${this.apps.length} applications</span>
        </div>
        <div class="library-filter-group">
          <input type="text" class="input-search" id="lib-search" placeholder="Search applications..." />
          <select class="select-filter" id="lib-filter-platform">
            <option value="all">All Formats</option>
            <option value="apk">Android (.apk)</option>
            <option value="ipa">iOS (.ipa)</option>
          </select>
        </div>
      </div>

      <div class="app-grid" id="app-grid">
        ${this.apps.map((app) => this.renderAppCard(app)).join('')}
      </div>
    `;

    this.attachEventListeners();
  }

  private renderAppCard(app: AppEntry): string {
    const badgeClass = this.getBadgeClass(app.executionMode);

    return `
      <div class="app-card ${!app.isCompatible ? 'card-warning' : ''}" data-app-id="${app.id}">
        <div class="app-card-header">
          <div class="app-icon">${app.iconText}</div>
          <div class="app-meta">
            <div class="app-name">${app.name}</div>
            <div class="app-pkg">${app.packageName}</div>
          </div>
          <span class="badge ${badgeClass}">${app.executionModeLabel}</span>
        </div>

        <div class="app-card-body">
          <p class="app-summary">${app.detailsSummary}</p>
          <div class="app-path-tag" title="${app.filePath}">${app.filePath}</div>
        </div>

        <div class="app-card-footer">
          <button class="btn btn-sm btn-primary launch-btn" data-app-id="${app.id}">
            ▶ Launch
          </button>
          <button class="btn btn-sm btn-outline inspect-btn" data-app-id="${app.id}">
            🔍 Inspect
          </button>
        </div>
      </div>
    `;
  }

  private getBadgeClass(mode: ExecutionMode): string {
    switch (mode) {
      case 'local_native':
        return 'badge-success';
      case 'local_virtualized':
        return 'badge-info';
      case 'remote_stream':
        return 'badge-warning';
      case 'local_hle':
        return 'badge-primary';
      case 'local_translated':
        return 'badge-secondary';
      case 'unsupported':
      default:
        return 'badge-danger';
    }
  }

  private attachEventListeners() {
    // Search & filter
    const searchInput = this.container.querySelector('#lib-search') as HTMLInputElement;
    const filterSelect = this.container.querySelector('#lib-filter-platform') as HTMLSelectElement;

    const filterHandler = () => {
      const q = searchInput.value.toLowerCase().trim();
      const platform = filterSelect.value;

      const cards = this.container.querySelectorAll('.app-card') as NodeListOf<HTMLElement>;
      cards.forEach((card) => {
        const appId = card.dataset.appId;
        const app = this.apps.find((a) => a.id === appId);
        if (!app) return;

        const matchesQuery =
          app.name.toLowerCase().includes(q) || app.packageName.toLowerCase().includes(q);
        const matchesPlatform = platform === 'all' || app.format === platform;

        card.style.display = matchesQuery && matchesPlatform ? 'flex' : 'none';
      });
    };

    searchInput.addEventListener('input', filterHandler);
    filterSelect.addEventListener('change', filterHandler);

    // Launch & Inspect buttons
    this.container.querySelectorAll('.launch-btn').forEach((btn) => {
      btn.addEventListener('click', async (e) => {
        const target = e.currentTarget as HTMLElement;
        const appId = target.dataset.appId;
        const app = this.apps.find((a) => a.id === appId);
        if (app) {
          await this.handleLaunch(app);
        }
      });
    });

    this.container.querySelectorAll('.inspect-btn').forEach((btn) => {
      btn.addEventListener('click', async (e) => {
        const target = e.currentTarget as HTMLElement;
        const appId = target.dataset.appId;
        const app = this.apps.find((a) => a.id === appId);
        if (app) {
          await this.handleInspect(app);
        }
      });
    });
  }

  private async handleLaunch(app: AppEntry) {
    try {
      // 1. Prepare session
      const prepRes = await this.client.prepareSession(app.filePath);

      // 2. Launch session
      await this.client.launchSession(prepRes.session_id);

      // 3. Callback to viewport
      if (this.onLaunchApp) {
        this.onLaunchApp(app, prepRes.session_id);
      }
    } catch (e: any) {
      if (this.onShowErrorModal) {
        this.onShowErrorModal(`Execution Rejected: ${app.name}`, e.rpcError?.data || e);
      } else {
        alert(`Launch error: ${e.message}`);
      }
    }
  }

  private async handleInspect(app: AppEntry) {
    try {
      const res = await this.client.negotiate(app.filePath);
      if (this.onShowInspectModal) {
        this.onShowInspectModal(app, res.negotiation, res.metadata, (targetApp) => {
          this.handleLaunch(targetApp);
        });
      }
    } catch (e: any) {
      if (this.onShowErrorModal) {
        this.onShowErrorModal(`Inspection Diagnostic: ${app.name}`, e.rpcError?.data || e);
      } else {
        alert(`Inspect error: ${e.message}`);
      }
    }
  }
}

