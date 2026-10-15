import { UMRClient } from '../rpc-client';
import { InputEventPayload } from '../types';

export class ViewportComponent {
  private client: UMRClient;
  private container: HTMLElement;
  private canvas!: HTMLCanvasElement;
  private ctx!: CanvasRenderingContext2D;
  private activeSessionId: string | null = null;
  private isPointerDown = false;
  private isRunning = false;
  private animFrameId: number | null = null;
  private lastFrameTimestamp = 0;
  private frameCount = 0;
  private fpsCounterEl!: HTMLElement;
  private sessionStateEl!: HTMLElement;
  private packageTitleEl!: HTMLElement;
  private onStopCallback?: () => void;
  private onLogUpdateCallback?: (sessionId: string) => void;

  constructor(container: HTMLElement, client: UMRClient) {
    this.container = container;
    this.client = client;
    this.renderLayout();
  }

  setCallbacks(onStop: () => void, onLogUpdate: (sessionId: string) => void) {
    this.onStopCallback = onStop;
    this.onLogUpdateCallback = onLogUpdate;
  }

  private renderLayout() {
    this.container.innerHTML = `
      <div class="viewport-wrapper">
        <div class="viewport-toolbar">
          <div class="viewport-session-info">
            <span class="status-indicator running" id="vp-status-dot"></span>
            <span class="viewport-pkg-name" id="vp-pkg-title">No Active Session</span>
            <span class="badge badge-subtle" id="vp-state-badge">READY</span>
          </div>
          <div class="viewport-metrics">
            <span class="metric-item">FPS: <strong id="vp-fps">0.0</strong></span>
            <button class="btn btn-sm btn-outline" id="vp-wipe-btn" title="Wipe sandbox app cache and data">
              🗑 Wipe Storage
            </button>
            <button class="btn btn-sm btn-danger" id="vp-stop-btn" title="Terminate running session">
              ■ Stop Session
            </button>
          </div>
        </div>

        <div class="phone-frame">
          <div class="phone-notch">
            <div class="phone-speaker"></div>
            <div class="phone-camera"></div>
          </div>

          <div class="phone-screen-container">
            <canvas id="vp-canvas" width="360" height="640"></canvas>
            <div class="viewport-placeholder" id="vp-placeholder">
              <div class="placeholder-icon">📱</div>
              <div class="placeholder-text">Select or Import an app from Library to launch</div>
            </div>
          </div>

          <div class="phone-navbar">
            <button class="nav-btn" id="nav-back-btn" title="Back (KEYCODE_BACK)">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                <path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/>
              </svg>
            </button>
            <button class="nav-btn" id="nav-home-btn" title="Home (KEYCODE_HOME)">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                <circle cx="12" cy="12" r="6"/>
              </svg>
            </button>
            <button class="nav-btn" id="nav-recents-btn" title="Recents (KEYCODE_APP_SWITCH)">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                <rect x="5" y="5" width="14" height="14" rx="2"/>
              </svg>
            </button>
          </div>
        </div>
      </div>
    `;

    this.canvas = this.container.querySelector('#vp-canvas') as HTMLCanvasElement;
    this.ctx = this.canvas.getContext('2d')!;
    this.fpsCounterEl = this.container.querySelector('#vp-fps') as HTMLElement;
    this.sessionStateEl = this.container.querySelector('#vp-state-badge') as HTMLElement;
    this.packageTitleEl = this.container.querySelector('#vp-pkg-title') as HTMLElement;

    this.setupEventListeners();
  }

  private setupEventListeners() {
    // Canvas pointer/touch events
    this.canvas.addEventListener('mousedown', (e) => this.handlePointer('down', e));
    this.canvas.addEventListener('mousemove', (e) => this.handlePointer('move', e));
    window.addEventListener('mouseup', (e) => {
      if (this.isPointerDown) {
        this.handlePointer('up', e);
      }
    });

    // Touch events for mobile/tablet screens
    this.canvas.addEventListener('touchstart', (e) => {
      e.preventDefault();
      if (e.touches.length > 0) {
        this.handleTouch('down', e.touches[0]);
      }
    });
    this.canvas.addEventListener('touchmove', (e) => {
      e.preventDefault();
      if (e.touches.length > 0) {
        this.handleTouch('move', e.touches[0]);
      }
    });
    this.canvas.addEventListener('touchend', (e) => {
      e.preventDefault();
      this.handlePointerAction('up', 0.5, 0.5);
      this.isPointerDown = false;
    });

    // Navigation buttons
    this.container.querySelector('#nav-back-btn')?.addEventListener('click', () => {
      this.dispatchNavButton('back');
    });
    this.container.querySelector('#nav-home-btn')?.addEventListener('click', () => {
      this.dispatchNavButton('home');
    });
    this.container.querySelector('#nav-recents-btn')?.addEventListener('click', () => {
      this.dispatchNavButton('recents');
    });

    // Session controls
    this.container.querySelector('#vp-stop-btn')?.addEventListener('click', async () => {
      if (this.activeSessionId) {
        await this.stopSession();
      }
    });

    this.container.querySelector('#vp-wipe-btn')?.addEventListener('click', async () => {
      if (this.activeSessionId) {
        try {
          await this.client.wipeStorage(this.activeSessionId);
          this.flashFeedback('Storage purged cleanly');
        } catch (e: any) {
          alert(`Wipe error: ${e.message}`);
        }
      }
    });
  }

  private handlePointer(action: 'down' | 'move' | 'up', e: MouseEvent) {
    if (!this.activeSessionId) return;

    if (action === 'down') {
      this.isPointerDown = true;
    } else if (action === 'move' && !this.isPointerDown) {
      return;
    } else if (action === 'up') {
      this.isPointerDown = false;
    }

    const rect = this.canvas.getBoundingClientRect();
    const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const y = Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height));

    this.handlePointerAction(action, x, y);
  }

  private handleTouch(action: 'down' | 'move' | 'up', touch: Touch) {
    if (!this.activeSessionId) return;
    const rect = this.canvas.getBoundingClientRect();
    const x = Math.max(0, Math.min(1, (touch.clientX - rect.left) / rect.width));
    const y = Math.max(0, Math.min(1, (touch.clientY - rect.top) / rect.height));
    this.handlePointerAction(action, x, y);
  }

  private handlePointerAction(action: 'down' | 'move' | 'up', x: number, y: number) {
    if (!this.activeSessionId) return;

    const payload: InputEventPayload = {
      type: 'pointer',
      x,
      y,
      action,
    };

    this.client.sendInput(this.activeSessionId, payload).catch((err) => {
      console.warn('sendInput failed:', err);
    });
  }

  private dispatchNavButton(button: 'back' | 'home' | 'recents') {
    if (!this.activeSessionId) return;

    const payload: InputEventPayload = {
      type: 'navigation',
      button,
    };

    this.client.sendInput(this.activeSessionId, payload).catch((err) => {
      console.warn('Navigation sendInput failed:', err);
    });
  }

  async attachSession(sessionId: string, packageName: string) {
    this.activeSessionId = sessionId;
    this.packageTitleEl.textContent = packageName;
    this.sessionStateEl.textContent = 'RUNNING';
    this.sessionStateEl.className = 'badge badge-success';

    const placeholder = this.container.querySelector('#vp-placeholder') as HTMLElement;
    if (placeholder) placeholder.style.display = 'none';

    this.isRunning = true;
    this.frameCount = 0;
    this.lastFrameTimestamp = performance.now();
    this.startRenderLoop();
  }

  private startRenderLoop() {
    let lastPoll = 0;

    const loop = async (timestamp: number) => {
      if (!this.isRunning || !this.activeSessionId) return;

      // Throttle capture to ~60 FPS (approx 16ms delta)
      if (timestamp - lastPoll >= 16) {
        lastPoll = timestamp;
        try {
          const frame = await this.client.captureFrame(this.activeSessionId);
          this.renderFrame(frame.base64_rgba, frame.width, frame.height);

          this.frameCount += 1;
          const deltaSec = (timestamp - this.lastFrameTimestamp) / 1000;
          if (deltaSec >= 0.5) {
            const currentFps = (this.frameCount / deltaSec).toFixed(1);
            this.fpsCounterEl.textContent = currentFps;
            this.frameCount = 0;
            this.lastFrameTimestamp = timestamp;
          }

          if (this.onLogUpdateCallback) {
            this.onLogUpdateCallback(this.activeSessionId);
          }
        } catch (e) {
          // If session ended, break loop
          console.debug('Frame capture loop paused or ended', e);
        }
      }

      this.animFrameId = requestAnimationFrame(loop);
    };

    this.animFrameId = requestAnimationFrame(loop);
  }

  private renderFrame(base64Rgba: string, width: number, height: number) {
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
    }

    try {
      const binaryString = atob(base64Rgba);
      const len = binaryString.length;
      const bytes = new Uint8ClampedArray(len);
      for (let i = 0; i < len; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }

      const imgData = new ImageData(bytes, width, height);
      this.ctx.putImageData(imgData, 0, 0);
    } catch (e) {
      console.error('Failed to decode RGBA frame', e);
    }
  }

  async stopSession() {
    if (!this.activeSessionId) return;

    this.isRunning = false;
    if (this.animFrameId) {
      cancelAnimationFrame(this.animFrameId);
      this.animFrameId = null;
    }

    try {
      await this.client.stopSession(this.activeSessionId);
    } catch (e) {
      console.warn('Stop session error:', e);
    }

    this.sessionStateEl.textContent = 'TERMINATED';
    this.sessionStateEl.className = 'badge badge-danger';

    if (this.onStopCallback) {
      this.onStopCallback();
    }
  }

  private flashFeedback(msg: string) {
    const original = this.packageTitleEl.textContent;
    this.packageTitleEl.textContent = `✓ ${msg}`;
    setTimeout(() => {
      this.packageTitleEl.textContent = original;
    }, 1500);
  }
}

