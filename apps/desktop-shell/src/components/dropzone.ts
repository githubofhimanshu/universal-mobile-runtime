import { UMRClient } from '../rpc-client';
import { AppEntry } from './library';

export class DropzoneComponent {
  private container: HTMLElement;
  private client: UMRClient;
  private onAppImported?: (app: AppEntry) => void;
  private onShowErrorModal?: (title: string, error: any) => void;

  constructor(container: HTMLElement, client: UMRClient) {
    this.container = container;
    this.client = client;
    this.render();
  }

  setCallbacks(
    onImport: (app: AppEntry) => void,
    onShowError: (title: string, error: any) => void
  ) {
    this.onAppImported = onImport;
    this.onShowErrorModal = onShowError;
  }

  private render() {
    this.container.innerHTML = `
      <div class="dropzone-card" id="dropzone-box">
        <div class="dropzone-content">
          <div class="dropzone-icon">📥</div>
          <div class="dropzone-text">
            <h3>Drop Android (.apk) or iOS (.ipa) Package Here</h3>
            <p>Packages are defensively parsed by the sandboxed intake worker</p>
          </div>
          <div class="dropzone-input-group">
            <input 
              type="text" 
              class="input-path" 
              id="dz-path-input" 
              placeholder="Or enter host package path (e.g. tests/fixtures/synthetic-app.apk)" 
            />
            <button class="btn btn-primary" id="dz-inspect-btn">
              ⚡ Inspect & Import
            </button>
          </div>
        </div>
      </div>
    `;

    this.attachEventListeners();
  }

  private attachEventListeners() {
    const box = this.container.querySelector('#dropzone-box') as HTMLElement;
    const pathInput = this.container.querySelector('#dz-path-input') as HTMLInputElement;
    const inspectBtn = this.container.querySelector('#dz-inspect-btn') as HTMLElement;

    // Drag-over styling
    box.addEventListener('dragover', (e) => {
      e.preventDefault();
      box.classList.add('dropzone-active');
    });

    box.addEventListener('dragleave', () => {
      box.classList.remove('dropzone-active');
    });

    box.addEventListener('drop', async (e) => {
      e.preventDefault();
      box.classList.remove('dropzone-active');

      if (e.dataTransfer && e.dataTransfer.files.length > 0) {
        const file = e.dataTransfer.files[0];
        // In browser environments without full FS access, we fallback to file.name or entered path
        pathInput.value = file.name;
        await this.inspectPath(file.name);
      }
    });

    inspectBtn.addEventListener('click', async () => {
      const p = pathInput.value.trim();
      if (p) {
        await this.inspectPath(p);
      }
    });

    pathInput.addEventListener('keydown', async (e) => {
      if (e.key === 'Enter') {
        const p = pathInput.value.trim();
        if (p) {
          await this.inspectPath(p);
        }
      }
    });
  }

  private async inspectPath(filePath: string) {
    const box = this.container.querySelector('#dropzone-box') as HTMLElement;
    box.classList.add('dropzone-loading');

    try {
      const res = await this.client.negotiate(filePath);
      const meta = res.metadata;
      const neg = res.negotiation;

      let name = 'Imported App';
      let pkgName = filePath;
      let summary = '';
      let format = meta.format;

      if (meta.details.android) {
        pkgName = meta.details.android.package_name;
        name = meta.details.android.main_activity || pkgName;
        summary = `Android (SDK ${meta.details.android.min_sdk_version || '?'}..${meta.details.android.target_sdk_version || '?'}) • ${meta.details.android.native_abis.join(', ') || 'Pure DEX'}`;
      } else if (meta.details.ios) {
        pkgName = meta.details.ios.bundle_identifier;
        name = pkgName;
        summary = `iOS (min ${meta.details.ios.minimum_os_version || '?'}) • ${meta.details.ios.architectures.join(', ')}`;
      }

      let execMode: any = 'local_native';
      let modeLabel = 'Local (Native)';
      let isCompatible = true;

      if (neg.result === 'compatible') {
        execMode = neg.execution_mode;
        modeLabel = this.formatMode(neg.execution_mode);
      } else if (neg.result === 'remote_required') {
        execMode = neg.suggested_mode;
        modeLabel = this.formatMode(neg.suggested_mode);
      } else if (neg.result === 'incompatible') {
        execMode = 'unsupported';
        modeLabel = 'Unsupported';
        isCompatible = false;
      }

      const newApp: AppEntry = {
        id: `imported-${Date.now()}`,
        name,
        packageName: pkgName,
        version: '1.0.0',
        format,
        filePath,
        executionMode: execMode,
        executionModeLabel: modeLabel,
        detailsSummary: summary,
        iconText: format === 'apk' ? '🤖' : '🍎',
        isCompatible,
      };

      if (this.onAppImported) {
        this.onAppImported(newApp);
      }

      if (!isCompatible && this.onShowErrorModal) {
        this.onShowErrorModal(
          `Intake Warning: ${name}`,
          (neg as any).error || { message: 'Incompatible package' }
        );
      }
    } catch (e: any) {
      if (this.onShowErrorModal) {
        this.onShowErrorModal('Intake Inspection Failed', e.rpcError?.data || e);
      } else {
        alert(`Intake failed: ${e.message}`);
      }
    } finally {
      box.classList.remove('dropzone-loading');
    }
  }

  private formatMode(mode: string): string {
    switch (mode) {
      case 'local_native':
        return 'Local (Native)';
      case 'local_virtualized':
        return 'Local (Virtualized)';
      case 'remote_stream':
        return 'Remote / Device Stream';
      case 'local_hle':
        return 'Local (HLE)';
      case 'local_translated':
        return 'Local (Translated)';
      default:
        return 'Unsupported';
    }
  }
}

