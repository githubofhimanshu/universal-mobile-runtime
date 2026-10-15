import {
  BackendCapabilities,
  DiagnosticReport,
  FrameDataResponse,
  InputEventPayload,
  NegotiationResult,
  PackageMetadata,
} from './types';

export interface RpcResponse<T = any> {
  jsonrpc: '2.0';
  id?: number | string | null;
  result?: T;
  error?: {
    code: number;
    message: string;
    data?: any;
  };
}

export class UMRClient {
  private endpoint: string;
  private reqId = 0;

  constructor(endpoint = '/rpc') {
    this.endpoint = endpoint;
  }

  setEndpoint(endpoint: string) {
    this.endpoint = endpoint;
  }

  getEndpoint(): string {
    return this.endpoint;
  }

  async call<T = any>(method: string, params?: any): Promise<T> {
    this.reqId += 1;
    const body = {
      jsonrpc: '2.0',
      id: this.reqId,
      method,
      params,
    };

    const response = await fetch(this.endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      throw new Error(`HTTP Error ${response.status}: ${response.statusText}`);
    }

    const data: RpcResponse<T> = await response.json();
    if (data.error) {
      const err = new Error(data.error.message);
      (err as any).rpcError = data.error;
      throw err;
    }

    return data.result as T;
  }

  async ping(): Promise<{ status: string; version: string }> {
    return this.call('umr.ping');
  }

  async listBackends(): Promise<BackendCapabilities[]> {
    return this.call('umr.backends.list');
  }

  async inspectPackage(package_path: string): Promise<PackageMetadata> {
    return this.call('umr.intake.inspect', { package_path });
  }

  async negotiate(
    package_path: string
  ): Promise<{ metadata: PackageMetadata; negotiation: NegotiationResult }> {
    return this.call('umr.negotiate', { package_path });
  }

  async prepareSession(
    package_path: string,
    backend_id?: string
  ): Promise<{ session_id: string; backend_id: string; state: string }> {
    return this.call('umr.session.prepare', { package_path, backend_id });
  }

  async launchSession(session_id: string): Promise<{ status: string; state: string }> {
    return this.call('umr.session.launch', { session_id });
  }

  async stopSession(session_id: string): Promise<{ status: string; state: string }> {
    return this.call('umr.session.stop', { session_id });
  }

  async pollStatus(session_id: string): Promise<{ state: string }> {
    return this.call('umr.session.status', { session_id });
  }

  async getReport(session_id: string): Promise<DiagnosticReport> {
    return this.call('umr.session.report', { session_id });
  }

  async sendInput(session_id: string, event: InputEventPayload): Promise<{ status: string }> {
    return this.call('umr.session.send_input', { session_id, event });
  }

  async captureFrame(session_id: string): Promise<FrameDataResponse> {
    return this.call('umr.session.capture_frame', { session_id });
  }

  async wipeStorage(session_id: string): Promise<{ status: string }> {
    return this.call('umr.session.wipe_storage', { session_id });
  }
}

