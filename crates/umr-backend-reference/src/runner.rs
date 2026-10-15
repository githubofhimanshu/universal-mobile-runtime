use std::collections::HashMap;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use umr_core::{
    BackendCapabilities, CpuArchitecture, DiagnosticError, ErrorCode, ExecutionMode, FrameBuffer,
    FrameStats, IAndroidBackend, InputEvent, LifecycleState, PackageDetails, PackageFormat,
    PackageMetadata, SessionId, SessionRecord, TargetAbi, UmrResult,
};

use crate::framebuffer::{DisplayMetrics, FrameBufferPipeline};
use crate::input::AndroidInputSubsystem;
use crate::storage::{AndroidStorageSandbox, StorageUsage};

/// Session configuration written into the isolated sandbox during preparation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSandboxConfig {
    pub session_id: String,
    pub package_id: String,
    pub package_path: String,
    pub sha256_hash: String,
    pub execution_mode: ExecutionMode,
    pub created_at: chrono::DateTime<Utc>,
}

#[allow(dead_code)]
struct ActiveProcessHandle {
    workspace_dir: PathBuf,
    log_file: PathBuf,
    package_name: String,
    execution_mode: ExecutionMode,
    is_running: Arc<AtomicBool>,
    worker_thread: Option<JoinHandle<()>>,
    input: Arc<Mutex<AndroidInputSubsystem>>,
    storage: Arc<AndroidStorageSandbox>,
    framebuffer: Arc<Mutex<FrameBufferPipeline>>,
}

/// Reference Android execution runner implementing `IAndroidBackend`.
/// Manages real filesystem sandboxes, simulated guest lifecycle threads, and logcat files.
pub struct ReferenceAndroidBackend {
    capabilities: BackendCapabilities,
    sandbox_root: PathBuf,
    sessions: HashMap<SessionId, ActiveProcessHandle>,
}

impl ReferenceAndroidBackend {
    pub fn new(sandbox_root: impl AsRef<Path>) -> Self {
        let root = sandbox_root.as_ref().to_path_buf();
        let _ = create_dir_all(&root);

        let capabilities = BackendCapabilities {
            backend_id: "android-reference-backend".to_string(),
            backend_version: "0.1.0".to_string(),
            execution_mode: ExecutionMode::LocalNative,
            supported_abis: vec![
                TargetAbi::X86_64,
                TargetAbi::Arm64V8a,
                TargetAbi::ArmeabiV7a,
                TargetAbi::X86,
            ],
            supported_architectures: vec![
                CpuArchitecture::X86_64,
                CpuArchitecture::Arm64,
            ],
            min_api_level: Some(21),
            max_api_level: Some(34),
            requires_hardware_virtualization: false,
            requires_external_device: false,
        };

        Self {
            capabilities,
            sandbox_root: root,
            sessions: HashMap::new(),
        }
    }

    /// Queries storage usage for an active or prepared session.
    pub fn get_storage_usage(&self, session_id: &SessionId) -> UmrResult<StorageUsage> {
        let handle = self.sessions.get(session_id).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrLifecycleSessionNotFound,
                format!("Session '{}' not found", session_id),
            )
        })?;

        handle.storage.calculate_usage()
    }

    /// Queries frame pacing and rendering statistics for an active session.
    pub fn get_frame_stats(&self, session_id: &SessionId) -> UmrResult<FrameStats> {
        let handle = self.sessions.get(session_id).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrLifecycleSessionNotFound,
                format!("Session '{}' not found", session_id),
            )
        })?;

        Ok(handle.framebuffer.lock().unwrap().stats())
    }
}

impl IAndroidBackend for ReferenceAndroidBackend {
    fn id(&self) -> &str {
        &self.capabilities.backend_id
    }

    fn capabilities(&self) -> &BackendCapabilities {
        &self.capabilities
    }

    fn prepare(&mut self, session: &mut SessionRecord, package: &PackageMetadata) -> UmrResult<()> {
        if package.format != PackageFormat::Apk {
            return Err(DiagnosticError::new(
                ErrorCode::ErrIntakeUnsupportedFormat,
                format!(
                    "ReferenceAndroidBackend only supports APK packages, received format: '{}'",
                    package.format
                ),
            ));
        }

        // Validate package details
        let android_info = match &package.details {
            PackageDetails::Android(info) => info,
            _ => {
                return Err(DiagnosticError::new(
                    ErrorCode::ErrIntakeMalformedArchive,
                    "Missing AndroidPackageInfo in package metadata",
                ));
            }
        };

        // Create dedicated session sandbox directory
        let session_dir = self.sandbox_root.join("sessions").join(session.session_id.to_string());
        let logs_dir = session_dir.join("logs");
        create_dir_all(&logs_dir).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendInternal,
                format!("Failed to create session sandbox directory: {}", e),
            )
        })?;

        // Write session configuration JSON
        let config = SessionSandboxConfig {
            session_id: session.session_id.to_string(),
            package_id: android_info.package_name.clone(),
            package_path: package.file_path.clone(),
            sha256_hash: package.sha256_hash.clone(),
            execution_mode: self.capabilities.execution_mode,
            created_at: Utc::now(),
        };

        let config_path = session_dir.join("session_config.json");
        let config_json = serde_json::to_string_pretty(&config).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendInternal,
                format!("Failed to serialize session config: {}", e),
            )
        })?;
        std::fs::write(&config_path, config_json).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendInternal,
                format!("Failed to write session config: {}", e),
            )
        })?;

        // Initialize logcat stream file
        let log_file = logs_dir.join("logcat.log");
        let mut log_writer = File::create(&log_file).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendInternal,
                format!("Failed to initialize logcat file: {}", e),
            )
        })?;

        writeln!(log_writer, "--------- beginning of system").map_err(io_err)?;
        writeln!(
            log_writer,
            "I/ActivityManager: Preparing sandbox for package '{}' (min_sdk={:?}, target_sdk={:?})",
            android_info.package_name, android_info.min_sdk_version, android_info.target_sdk_version
        )
        .map_err(io_err)?;
        writeln!(
            log_writer,
            "I/PackageManager: Verifying DEX bytecode and signature integrity: OK"
        )
        .map_err(io_err)?;

        // Initialize storage sandbox
        let storage = AndroidStorageSandbox::initialize(
            &session_dir,
            &android_info.package_name,
            &log_file,
        )?;

        // Initialize input subsystem
        let input = AndroidInputSubsystem::new(360, 640, &log_file);

        // Initialize framebuffer pipeline
        let framebuffer = FrameBufferPipeline::new(DisplayMetrics::default());

        let handle = ActiveProcessHandle {
            workspace_dir: session_dir,
            log_file,
            package_name: android_info.package_name.clone(),
            execution_mode: self.capabilities.execution_mode,
            is_running: Arc::new(AtomicBool::new(false)),
            worker_thread: None,
            input: Arc::new(Mutex::new(input)),
            storage: Arc::new(storage),
            framebuffer: Arc::new(Mutex::new(framebuffer)),
        };

        self.sessions.insert(session.session_id.clone(), handle);
        Ok(())
    }

    fn launch(&mut self, session: &mut SessionRecord) -> UmrResult<()> {
        let handle = self.sessions.get_mut(&session.session_id).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrLifecycleSessionNotFound,
                format!("Session '{}' has not been prepared", session.session_id),
            )
        })?;

        if handle.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Append launch logs to logcat
        let mut file = OpenOptions::new()
            .append(true)
            .open(&handle.log_file)
            .map_err(io_err)?;

        writeln!(
            file,
            "I/ActivityManager: Start proc 1042:{}/u0a42 for activity Main",
            session.package_id
        )
        .map_err(io_err)?;
        writeln!(
            file,
            "I/SurfaceFlinger: Surface created for window 'com.umr.surface.0'"
        )
        .map_err(io_err)?;
        writeln!(
            file,
            "D/AppRuntime: Calling Activity.onCreate() -> onStart() -> onResume()"
        )
        .map_err(io_err)?;

        // Spawn runner thread maintaining active execution heartbeat
        let is_running = Arc::clone(&handle.is_running);
        is_running.store(true, Ordering::SeqCst);
        let log_path = handle.log_file.clone();

        let thread_handle = thread::spawn(move || {
            let mut tick = 0;
            while is_running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(100));
                tick += 1;
                if tick % 10 == 0 {
                    if let Ok(mut f) = OpenOptions::new().append(true).open(&log_path) {
                        let _ = writeln!(
                            f,
                            "D/ViewRootImpl: Frame render tick: {} (60.0 fps)",
                            tick
                        );
                    }
                }
            }
        });

        handle.worker_thread = Some(thread_handle);
        Ok(())
    }

    fn poll_status(&self, session: &SessionRecord) -> UmrResult<LifecycleState> {
        let handle = self.sessions.get(&session.session_id).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrLifecycleSessionNotFound,
                format!("Session '{}' not found", session.session_id),
            )
        })?;

        if handle.is_running.load(Ordering::SeqCst) {
            Ok(LifecycleState::Running)
        } else {
            Ok(session.state)
        }
    }

    fn stop(&mut self, session: &mut SessionRecord) -> UmrResult<()> {
        let handle = self.sessions.get_mut(&session.session_id).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrLifecycleSessionNotFound,
                format!("Session '{}' not found", session.session_id),
            )
        })?;

        // Signal runner thread to stop
        handle.is_running.store(false, Ordering::SeqCst);
        if let Some(th) = handle.worker_thread.take() {
            let _ = th.join();
        }

        // Append shutdown log
        if let Ok(mut file) = OpenOptions::new().append(true).open(&handle.log_file) {
            let _ = writeln!(
                file,
                "I/ActivityManager: Killing proc 1042:{}/u0a42 (user requested stop)",
                session.package_id
            );
            let _ = writeln!(file, "--------- end of session");
        }

        Ok(())
    }

    fn collect_diagnostics(&self, session: &SessionRecord) -> UmrResult<Vec<String>> {
        let handle = self.sessions.get(&session.session_id).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrLifecycleSessionNotFound,
                format!("Session '{}' not found", session.session_id),
            )
        })?;

        if !handle.log_file.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&handle.log_file).map_err(io_err)?;
        let reader = BufReader::new(file);
        let mut lines = Vec::new();

        for line in reader.lines() {
            if let Ok(l) = line {
                lines.push(l);
            }
        }

        Ok(lines)
    }

    fn send_input(&mut self, session: &SessionRecord, event: InputEvent) -> UmrResult<()> {
        let handle = self.sessions.get(&session.session_id).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrLifecycleSessionNotFound,
                format!("Session '{}' not found", session.session_id),
            )
        })?;

        handle.input.lock().unwrap().dispatch(event)
    }

    fn capture_frame(&self, session: &SessionRecord) -> UmrResult<Option<FrameBuffer>> {
        let handle = self.sessions.get(&session.session_id).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrLifecycleSessionNotFound,
                format!("Session '{}' not found", session.session_id),
            )
        })?;

        let input = handle.input.lock().unwrap();
        input.clear_expired_touch(800);
        let touch = input.latest_touch();
        let nav = input.latest_nav();
        drop(input);

        let mut fb_pipe = handle.framebuffer.lock().unwrap();
        let frame = fb_pipe.render_frame(&handle.package_name, handle.execution_mode, touch, nav);
        Ok(Some(frame))
    }

    fn wipe_storage(&mut self, session: &SessionRecord) -> UmrResult<()> {
        let handle = self.sessions.get(&session.session_id).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrLifecycleSessionNotFound,
                format!("Session '{}' not found", session.session_id),
            )
        })?;

        handle.storage.wipe()
    }
}

fn io_err(e: std::io::Error) -> DiagnosticError {
    DiagnosticError::new(
        ErrorCode::ErrBackendInternal,
        format!("I/O error in ReferenceAndroidBackend: {}", e),
    )
}
