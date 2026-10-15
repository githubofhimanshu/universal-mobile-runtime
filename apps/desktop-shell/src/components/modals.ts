import { AppEntry } from './library';
import { NegotiationResult, PackageMetadata } from '../types';

export class ModalManager {
  private container: HTMLElement;

  constructor(container: HTMLElement) {
    this.container = container;
  }

  showErrorModal(title: string, error: any) {
    const errData = error?.rpcError?.data || error?.data || error;
    const code = errData?.error_code || error?.error_code || error?.code || error?.rpcError?.code || 'ERR_UNKNOWN';
    const message = errData?.message || error?.message || (typeof error === 'string' ? error : JSON.stringify(error));
    const suggestion = errData?.suggestion || errData?.recommendation || error?.suggestion || error?.recommendation || 'Review package integrity and host configuration.';
    const context = errData?.context || error?.context || '';

    this.container.innerHTML = `
      <div class="modal-backdrop">
        <div class="modal-card modal-danger">
          <div class="modal-header">
            <div class="modal-title">
              <span class="modal-icon">⚠️</span>
              <h3>${title}</h3>
            </div>
            <button class="modal-close-btn" id="modal-close">&times;</button>
          </div>

          <div class="modal-body">
            <div class="error-code-badge">${code}</div>
            <p class="error-message">${message}</p>

            ${context ? `<div class="error-context"><strong>Context:</strong> <code>${context}</code></div>` : ''}

            <div class="remediation-box">
              <div class="remediation-title">💡 Actionable Recommendation:</div>
              <p class="remediation-text">${suggestion}</p>
            </div>
          </div>

          <div class="modal-footer">
            <button class="btn btn-primary" id="modal-confirm-btn">Understood</button>
          </div>
        </div>
      </div>
    `;

    this.attachCloseHandlers();
  }

  showInspectModal(app: AppEntry, neg: NegotiationResult, meta: PackageMetadata) {
    const androidInfo = meta.details.android;
    const iosInfo = meta.details.ios;

    let negContent = '';
    if (neg.result === 'compatible') {
      negContent = `
        <div class="neg-box neg-compatible">
          <span class="neg-status">✓ COMPATIBLE</span>
          <span>Matched Backend: <strong>${neg.backend_id}</strong> (${neg.execution_mode})</span>
        </div>
      `;
    } else if (neg.result === 'remote_required') {
      negContent = `
        <div class="neg-box neg-remote">
          <span class="neg-status">⚡ REMOTE DEVICE REQUIRED</span>
          <span>${neg.reason}</span>
        </div>
      `;
    } else {
      negContent = `
        <div class="neg-box neg-incompatible">
          <span class="neg-status">✕ INCOMPATIBLE</span>
          <span>${(neg as any).error?.message || 'Unsupported configuration'}</span>
        </div>
      `;
    }

    this.container.innerHTML = `
      <div class="modal-backdrop">
        <div class="modal-card">
          <div class="modal-header">
            <div class="modal-title">
              <span class="modal-icon">🔍</span>
              <h3>Package Inspection Details: ${app.name}</h3>
            </div>
            <button class="modal-close-btn" id="modal-close">&times;</button>
          </div>

          <div class="modal-body">
            <div class="meta-section">
              <h4>Capability Negotiation</h4>
              ${negContent}
            </div>

            <div class="meta-section">
              <h4>Package Attributes</h4>
              <table class="meta-table">
                <tr><td>Package ID</td><td><code>${app.packageName}</code></td></tr>
                <tr><td>Archive Path</td><td><code>${meta.file_path}</code></td></tr>
                <tr><td>Archive Format</td><td><strong>${meta.format.toUpperCase()}</strong></td></tr>
                <tr><td>File Size</td><td>${(meta.file_size_bytes / 1024).toFixed(1)} KB</td></tr>
                <tr><td>SHA-256 Hash</td><td><code class="hash-tag">${meta.sha256_hash.substring(0, 24)}...</code></td></tr>
                ${
                  androidInfo
                    ? `
                  <tr><td>Min / Target SDK</td><td>API ${androidInfo.min_sdk_version || '?'} .. API ${androidInfo.target_sdk_version || '?'}</td></tr>
                  <tr><td>Native ABIs</td><td><code>${androidInfo.native_abis.join(', ') || 'Pure DEX (No native code)'}</code></td></tr>
                  <tr><td>Main Activity</td><td><code>${androidInfo.main_activity || 'Default'}</code></td></tr>
                `
                    : ''
                }
                ${
                  iosInfo
                    ? `
                  <tr><td>Bundle Identifier</td><td><code>${iosInfo.bundle_identifier}</code></td></tr>
                  <tr><td>Minimum OS Version</td><td>iOS ${iosInfo.minimum_os_version || '?'}</td></tr>
                  <tr><td>Architectures</td><td><code>${iosInfo.architectures.join(', ')}</code></td></tr>
                  <tr><td>FairPlay DRM Status</td><td>${iosInfo.encryption.status}</td></tr>
                  <tr><td>Simulator Build</td><td>${iosInfo.is_simulator_build ? 'Yes' : 'No'}</td></tr>
                `
                    : ''
                }
              </table>
            </div>
          </div>

          <div class="modal-footer">
            <button class="btn btn-primary" id="modal-confirm-btn">Close</button>
          </div>
        </div>
      </div>
    `;

    this.attachCloseHandlers();
  }

  private attachCloseHandlers() {
    const closeBtn = this.container.querySelector('#modal-close');
    const confirmBtn = this.container.querySelector('#modal-confirm-btn');
    const backdrop = this.container.querySelector('.modal-backdrop');

    const close = () => {
      this.container.innerHTML = '';
    };

    closeBtn?.addEventListener('click', close);
    confirmBtn?.addEventListener('click', close);
    backdrop?.addEventListener('click', (e) => {
      if (e.target === backdrop) close();
    });
  }
}

