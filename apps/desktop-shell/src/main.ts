import { UMRClient } from './rpc-client';
import { LibraryComponent, AppEntry } from './components/library';
import { ViewportComponent } from './components/viewport';
import { DropzoneComponent } from './components/dropzone';
import { DiagnosticsComponent } from './components/diagnostics';
import { ModalManager } from './components/modals';

export class DesktopShellApp {
  private client: UMRClient;
  private library!: LibraryComponent;
  private viewport!: ViewportComponent;
  private dropzone!: DropzoneComponent;
  private diagnostics!: DiagnosticsComponent;
  private modals!: ModalManager;
  private connectionInterval: any = null;

  constructor() {
    // Automatically select origin or fallback to local IPC port
    const rpcUrl = window.location.port === '5173' || window.location.port === '9090' 
      ? '/rpc' 
      : 'http://127.0.0.1:9090/rpc';

    this.client = new UMRClient(rpcUrl);
    this.initLayout();
    this.initComponents();
    this.startConnectionCheck();
  }

  private initLayout() {
    const appEl = document.getElementById('app')!;
    appEl.innerHTML = `
      <header class="shell-header">
        <div class="brand-group">
          <span class="brand-logo">⚡</span>
          <div>
            <span class="brand-title">Universal Mobile Runtime</span>
            <span class="brand-subtitle">Desktop Shell</span>
          </div>
        </div>

        <div class="header-status-group">
          <div class="status-badge">
            <span class="status-dot disconnected" id="conn-dot"></span>
            <span id="conn-label">Connecting to IPC (127.0.0.1:9090)...</span>
          </div>
          <button class="btn btn-sm btn-outline" id="toggle-diag-btn">
            📋 Diagnostics Drawer
          </button>
        </div>
      </header>

      <main class="shell-main">
        <section class="panel-left">
          <div id="dropzone-container"></div>
          <div id="library-container"></div>
        </section>

        <section class="panel-center">
          <div id="viewport-container" style="width: 100%; height: 100%;"></div>
        </section>

        <aside class="panel-right" id="diagnostics-panel">
          <div id="diagnostics-container" style="width: 100%; height: 100%;"></div>
        </aside>
      </main>

      <div id="modal-container"></div>
    `;

    document.getElementById('toggle-diag-btn')?.addEventListener('click', () => {
      this.diagnostics.toggleDrawer();
    });
  }

  private initComponents() {
    const dropzoneContainer = document.getElementById('dropzone-container')!;
    const libraryContainer = document.getElementById('library-container')!;
    const viewportContainer = document.getElementById('viewport-container')!;
    const diagnosticsContainer = document.getElementById('diagnostics-container')!;
    const modalContainer = document.getElementById('modal-container')!;

    this.modals = new ModalManager(modalContainer);
    this.diagnostics = new DiagnosticsComponent(diagnosticsContainer, this.client);
    this.viewport = new ViewportComponent(viewportContainer, this.client);
    this.library = new LibraryComponent(libraryContainer, this.client);
    this.dropzone = new DropzoneComponent(dropzoneContainer, this.client);

    // Cross-component coordination
    this.library.setCallbacks(
      (app: AppEntry, sessionId: string) => {
        this.viewport.attachSession(sessionId, app.name);
      },
      (title: string, error: any) => {
        this.modals.showErrorModal(title, error);
      },
      (app: AppEntry, neg: any, meta: any, onLaunch?: (app: AppEntry, backendId?: string) => void) => {
        this.modals.showInspectModal(app, neg, meta, onLaunch);
      }
    );

    this.dropzone.setCallbacks(
      (app: AppEntry) => {
        this.library.addApp(app);
      },
      (title: string, error: any) => {
        this.modals.showErrorModal(title, error);
      }
    );

    this.viewport.setCallbacks(
      () => {
        console.log('Session stopped in viewport');
      },
      (sessionId: string) => {
        this.diagnostics.updateLogsForSession(sessionId);
      }
    );
  }

  private startConnectionCheck() {
    const dot = document.getElementById('conn-dot')!;
    const label = document.getElementById('conn-label')!;

    const check = async () => {
      try {
        const ping = await this.client.ping();
        dot.className = 'status-dot';
        label.textContent = `IPC Connected (v${ping.version}) • 127.0.0.1:9090`;
      } catch (e) {
        dot.className = 'status-dot disconnected';
        label.textContent = 'IPC Gateway Offline (Launch umr-ipc on 9090)';
      }
    };

    check();
    this.connectionInterval = setInterval(check, 3000);
  }
}

// Bootstrap application on DOM ready
window.addEventListener('DOMContentLoaded', () => {
  new DesktopShellApp();
});

