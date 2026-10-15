export type ExecutionMode =
  | 'local_native'
  | 'local_virtualized'
  | 'local_translated'
  | 'local_hle'
  | 'remote_stream'
  | 'unsupported';

export interface BackendCapabilities {
  backend_id: string;
  backend_version: string;
  execution_mode: ExecutionMode;
  supported_abis: string[];
  supported_architectures: string[];
  min_api_level?: number;
  max_api_level?: number;
  requires_hardware_virtualization: boolean;
  requires_external_device: boolean;
}

export interface DiagnosticError {
  code: string;
  message: string;
  suggestion?: string;
  context?: string;
}

export interface AndroidPackageInfo {
  package_name: string;
  version_code?: number;
  version_name?: string;
  min_sdk_version?: number;
  target_sdk_version?: number;
  main_activity?: string;
  native_abis: string[];
  is_pure_dex: boolean;
}

export interface IosPackageInfo {
  bundle_identifier: string;
  bundle_version?: string;
  bundle_short_version?: string;
  minimum_os_version?: string;
  architectures: string[];
  encryption: {
    status: string;
    cryptid?: number;
  };
  is_simulator_build: boolean;
}

export type PackageDetails =
  | { type: 'android'; [key: string]: any }
  | { type: 'ios'; [key: string]: any };

export interface PackageMetadata {
  format: 'apk' | 'ipa';
  file_path: string;
  file_size_bytes: number;
  sha256_hash: string;
  details: {
    android?: AndroidPackageInfo;
    ios?: IosPackageInfo;
  };
  inspected_at: string;
}

export type NegotiationResult =
  | { result: 'compatible'; backend_id: string; execution_mode: ExecutionMode }
  | { result: 'remote_required'; reason: string; suggested_mode: ExecutionMode }
  | { result: 'incompatible'; error: DiagnosticError };

export interface FrameDataResponse {
  width: u32;
  height: u32;
  format: string;
  timestamp_ms: number;
  frame_index: number;
  base64_rgba: string;
}

type u32 = number;

export interface PointerEventPayload {
  type: 'pointer';
  x: number;
  y: number;
  action: 'down' | 'move' | 'up' | 'cancel';
}

export interface KeyEventPayload {
  type: 'key';
  key_code: number;
  action: 'down' | 'up';
}

export interface NavigationEventPayload {
  type: 'navigation';
  button: 'back' | 'home' | 'recents';
}

export type InputEventPayload = PointerEventPayload | KeyEventPayload | NavigationEventPayload;

export interface DiagnosticEntry {
  timestamp: string;
  level: string;
  component: string;
  message: string;
}

export interface DiagnosticReport {
  session_id: string;
  package_id: string;
  created_at: string;
  entries: DiagnosticEntry[];
}

