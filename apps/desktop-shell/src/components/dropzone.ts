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
            <input type="file" id="dz-file-picker" accept=".apk,.ipa" style="display: none;" />
            <button class="btn btn-secondary" id="dz-browse-btn" type="button" style="white-space: nowrap;">
              📁 Browse...
            </button>
            <button class="btn btn-primary" id="dz-inspect-btn" style="white-space: nowrap;">
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
    const browseBtn = this.container.querySelector('#dz-browse-btn') as HTMLElement;
    const filePicker = this.container.querySelector('#dz-file-picker') as HTMLInputElement;

    browseBtn.addEventListener('click', () => {
      filePicker.click();
    });

    filePicker.addEventListener('change', async () => {
      if (filePicker.files && filePicker.files.length > 0) {
        const file = filePicker.files[0];
        pathInput.value = file.name;
        await this.uploadAndInspectFile(file);
      }
    });

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
        pathInput.value = file.name;
        await this.uploadAndInspectFile(file);
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

  private async uploadAndInspectFile(file: File) {
    const box = this.container.querySelector('#dropzone-box') as HTMLElement;
    box.classList.add('dropzone-loading');
    const pathInput = this.container.querySelector('#dz-path-input') as HTMLInputElement;

    try {
      // 1. Upload file binary to backend
      const res = await fetch('/upload', {
        method: 'POST',
        headers: {
          'X-Filename': encodeURIComponent(file.name),
        },
        body: await file.arrayBuffer(),
      });

      if (!res.ok) {
        const err = await res.json().catch(() => ({ error: 'Upload failed' }));
        throw new Error(err.error || 'Failed to upload package to runtime sandbox');
      }

      const uploadData = await res.json();
      pathInput.value = uploadData.package_path;

      // 2. Negotiate and inspect uploaded package
      await this.inspectPath(uploadData.package_path);
    } catch (err: any) {
      box.classList.remove('dropzone-loading');
      if (this.onShowErrorModal) {
        this.onShowErrorModal('Package Upload & Intake Failed', {
          error_code: 'ERR_UPLOAD_FAILED',
          message: err.message || String(err),
          recommendation: 'Check that the runtime daemon is running and has write permissions to target/umr-sandbox.',
        });
      }
    }
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

      const d: any = meta.details || {};
      const androidInfo = d.android || (d.platform === 'android' ? d : null) || (meta.format === 'apk' ? d : null);
      const iosInfo = d.ios || (d.platform === 'ios' ? d : null) || (meta.format === 'ipa' ? d : null);

      if (androidInfo && androidInfo.package_name) {
        pkgName = androidInfo.package_name;
        const shortName = androidInfo.main_activity || androidInfo.package_name.split('.').pop() || 'App';
        name = shortName.charAt(0).toUpperCase() + shortName.slice(1);
        const abis = androidInfo.native_abis && androidInfo.native_abis.length > 0
          ? androidInfo.native_abis.join(', ')
          : 'Pure DEX';
        summary = `Android (SDK ${androidInfo.min_sdk_version || '?'}..${androidInfo.target_sdk_version || '?'}) • ${abis}`;
      } else if (iosInfo && iosInfo.bundle_identifier) {
        pkgName = iosInfo.bundle_identifier;
        const shortName = iosInfo.bundle_identifier.split('.').pop() || 'App';
        name = shortName.charAt(0).toUpperCase() + shortName.slice(1);
        const archs = iosInfo.architectures && iosInfo.architectures.join(', ') || 'arm64';
        summary = `iOS (min ${iosInfo.minimum_os_version || '?'}) • ${archs}`;
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

