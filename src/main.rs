// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

#![deny(unsafe_code)]

use chrono::{DateTime, Months, Utc};
use codex_info::i18n::{I18n, PeriodKind, TextKey};
use codex_info::protocol_contract;
use codex_info::security;
use codex_info::thread_contract::{
    self, PageAcceptance, ThreadCycleAccumulator, ThreadCycleOutcome, ValidatedThreadCandidate,
};
use codex_info::thread_state;
use codex_info::usage_store::{self, UsageStore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use slint::winit_030::{winit, EventResult, WinitWindowAccessor};
use slint::{CloseRequestResponse, ComponentHandle, Timer, TimerMode};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

slint::include_modules!();

#[derive(Clone, Copy)]
enum AccountCommand {
    Read,
    Login,
    Stop,
}

#[derive(Clone, Copy)]
enum ThreadCommand {
    Read { auth_epoch: u64 },
    Stop,
}

#[derive(Clone, Copy)]
enum LocalCommand {
    Collect {
        auth_epoch: u64,
        reset_at: i64,
        window_seconds: i64,
    },
    Stop,
}

struct UsageEvent {
    remaining_percent: Option<f64>,
    reset_at: i64,
    window_seconds: i64,
    limit_name: String,
    quota_title: String,
    monthly: bool,
}

enum Event {
    Ready,
    Account {
        email: Option<String>,
        authenticated: bool,
        plan_type: Option<String>,
    },
    AuthUrl(String),
    Usage(Box<UsageEvent>),
    Error(String),
}

enum ThreadEvent {
    Ready,
    Update {
        auth_epoch: u64,
        update: ActiveThreadUpdate,
    },
    Error {
        auth_epoch: u64,
        message: String,
    },
}

struct LocalUsageResult {
    auth_epoch: u64,
    reset_at: i64,
    window_seconds: i64,
    model_usage: ModelUsageTotals,
    history_samples: Vec<UsageHistorySample>,
}

enum LocalEvent {
    Usage(LocalUsageResult),
    Error {
        auth_epoch: u64,
        reset_at: i64,
        window_seconds: i64,
    },
}

#[derive(Clone, Copy, Debug, Default)]
struct TokenSnapshot {
    total: u64,
    input: u64,
    cached_input: u64,
    output: u64,
}

const LOCAL_ESTIMATE_PRICE_VERSION: &str = "LOCAL_ESTIMATE_V1_2026-08-14";
const SOL_PRICE_PER_MILLION: (f64, f64, f64) = (5.0, 0.5, 30.0);
const TERRA_PRICE_PER_MILLION: (f64, f64, f64) = (2.0, 0.2, 12.0);
const LUNA_PRICE_PER_MILLION: (f64, f64, f64) = (0.2, 0.02, 1.2);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ModelUsageRow {
    name: String,
    tokens: u64,
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
}

impl ModelUsageRow {
    fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    fn add(&mut self, snapshot: TokenSnapshot) {
        self.tokens = self.tokens.saturating_add(snapshot.total);
        self.input_tokens = self.input_tokens.saturating_add(snapshot.input);
        self.cached_input_tokens = self
            .cached_input_tokens
            .saturating_add(snapshot.cached_input);
        self.output_tokens = self.output_tokens.saturating_add(snapshot.output);
    }

    fn dollar_costs(&self) -> (f64, f64, f64) {
        // The version is intentionally fixed with the rates; changing either
        // requires updating the contract fixture rather than silent drift.
        let _ = LOCAL_ESTIMATE_PRICE_VERSION;
        let (input_rate, cached_rate, output_rate) = match self.name.as_str() {
            "SOL" => SOL_PRICE_PER_MILLION,
            "TERRA" => TERRA_PRICE_PER_MILLION,
            "LUNA" => LUNA_PRICE_PER_MILLION,
            _ => (0.0, 0.0, 0.0),
        };
        let input = self.input_tokens.saturating_sub(self.cached_input_tokens) as f64;
        (
            input * input_rate / 1_000_000.0,
            self.cached_input_tokens as f64 * cached_rate / 1_000_000.0,
            self.output_tokens as f64 * output_rate / 1_000_000.0,
        )
    }
}

#[derive(Clone, Debug)]
struct ModelUsageTotals {
    sol: ModelUsageRow,
    terra: ModelUsageRow,
    luna: ModelUsageRow,
}

#[derive(Clone, Copy, Debug, Default)]
struct ModelDollarTotals {
    sol: f64,
    terra: f64,
    luna: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModelTokenTotals {
    sol: u64,
    terra: u64,
    luna: u64,
}

impl Default for ModelUsageTotals {
    fn default() -> Self {
        Self {
            sol: ModelUsageRow::new("SOL"),
            terra: ModelUsageRow::new("TERRA"),
            luna: ModelUsageRow::new("LUNA"),
        }
    }
}

impl ModelUsageTotals {
    fn add(&mut self, model: &str, snapshot: TokenSnapshot) {
        match Self::recognized_model(model) {
            Some("SOL") => self.sol.add(snapshot),
            Some("TERRA") => self.terra.add(snapshot),
            Some("LUNA") => self.luna.add(snapshot),
            _ => {}
        }
    }

    fn recognized_model(model: &str) -> Option<&'static str> {
        let model = model.to_ascii_lowercase();
        if model.contains("sol") {
            Some("SOL")
        } else if model.contains("terra") {
            Some("TERRA")
        } else if model.contains("luna") {
            Some("LUNA")
        } else {
            None
        }
    }

    fn rows(self) -> Vec<ModelUsageRow> {
        [self.sol, self.terra, self.luna]
            .into_iter()
            .filter(|row| row.tokens > 0)
            .collect()
    }

    fn dollar_totals(&self) -> ModelDollarTotals {
        fn total(row: &ModelUsageRow) -> f64 {
            let (input, cached_input, output) = row.dollar_costs();
            input + cached_input + output
        }

        ModelDollarTotals {
            sol: total(&self.sol),
            terra: total(&self.terra),
            luna: total(&self.luna),
        }
    }

    fn token_totals(&self) -> ModelTokenTotals {
        ModelTokenTotals {
            sol: self.sol.tokens,
            terra: self.terra.tokens,
            luna: self.luna.tokens,
        }
    }
}

impl ModelDollarTotals {
    fn from_rows(rows: &[ModelUsageRow]) -> Self {
        let mut totals = Self::default();
        for row in rows {
            let (input, cached_input, output) = row.dollar_costs();
            let total = input + cached_input + output;
            match row.name.as_str() {
                "SOL" => totals.sol = total,
                "TERRA" => totals.terra = total,
                "LUNA" => totals.luna = total,
                _ => {}
            }
        }
        totals
    }
}

const WEEK_SECONDS: i64 = 7 * 86_400;
const RESET_AT_TOLERANCE_SECONDS: i64 = 60;
const LEGACY_MOVING_RESET_HORIZON_TOLERANCE_SECONDS: i64 = 120;
const LEGACY_MOVING_RESET_PAIR_GAP_SECONDS: i64 = 3_600;
const LEGACY_MOVING_RESET_PAIR_HORIZON_TOLERANCE_SECONDS: i64 = 60;
#[cfg(test)]
const GRAPH_METRIC_OPTIONS: [&str; 2] = ["ドル", "トークン"];
const FIXED_WINDOW_WIDTH: u32 = 900;
const FIXED_WINDOW_HEIGHT: u32 = 480;
const GRAPH_WINDOW_WIDTH: u32 = 940;
const GRAPH_WINDOW_HEIGHT: u32 = 640;
const LEGAL_WINDOW_WIDTH: u32 = 720;
const LEGAL_WINDOW_HEIGHT: u32 = 520;
const UNAUTHENTICATED_WINDOW_TITLE: &str = "アカウント未接続 — プラン未設定";
// Keep the native title-bar purpose suffix ASCII: some X11 window managers
// render `_NET_WM_NAME` with a fallback font that turns Japanese glyphs into
// tofu. The in-window headings remain Japanese and carry the full meaning.
#[cfg(test)]
const THREADS_WINDOW_PURPOSE: &str = "Threads";
#[cfg(test)]
const GRAPH_WINDOW_PURPOSE: &str = "Graph";

#[derive(Clone, Copy)]
enum WindowPurpose {
    Threads,
    Graph,
    Legal,
}

impl WindowPurpose {
    fn native_label(self) -> &'static str {
        match self {
            Self::Threads => "Threads",
            Self::Graph => "Graph",
            Self::Legal => "Legal",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixedResizeDecision {
    Propagate,
    RejectAndRestore,
}

#[cfg(test)]
fn fixed_resize_decision(width: u32, height: u32) -> FixedResizeDecision {
    fixed_resize_decision_for_size(width, height, FIXED_WINDOW_WIDTH, FIXED_WINDOW_HEIGHT)
}

fn fixed_resize_decision_for_size(
    width: u32,
    height: u32,
    expected_width: u32,
    expected_height: u32,
) -> FixedResizeDecision {
    if width == 0 || height == 0 || (width == expected_width && height == expected_height) {
        FixedResizeDecision::Propagate
    } else {
        FixedResizeDecision::RejectAndRestore
    }
}

fn install_fixed_window_guard(window: &slint::Window) {
    install_window_size_guard(window, FIXED_WINDOW_WIDTH, FIXED_WINDOW_HEIGHT);
}

fn install_resizable_window(window: &slint::Window) {
    let _ = window.with_winit_window(|winit_window| winit_window.set_resizable(true));
}

#[derive(Clone, Copy)]
enum ManualX11WindowAction {
    Move,
    Resize(winit::window::ResizeDirection),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ManualX11Geometry {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

const MANUAL_X11_POLL_INTERVAL: Duration = Duration::from_millis(4);

static ACTIVE_MANUAL_X11_ACTIONS: OnceLock<Mutex<BTreeSet<X11Window>>> = OnceLock::new();

struct ManualX11ActionLease {
    keys: [X11Window; 2],
}

fn active_manual_x11_actions() -> &'static Mutex<BTreeSet<X11Window>> {
    ACTIVE_MANUAL_X11_ACTIONS.get_or_init(|| Mutex::new(BTreeSet::new()))
}

fn claim_manual_x11_action(target: X11Window, client: X11Window) -> Option<ManualX11ActionLease> {
    let mut active = active_manual_x11_actions()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if active.contains(&target) || (client != target && active.contains(&client)) {
        return None;
    }
    active.insert(target);
    active.insert(client);
    drop(active);
    Some(ManualX11ActionLease {
        keys: [target, client],
    })
}

impl Drop for ManualX11ActionLease {
    fn drop(&mut self) {
        let mut active = active_manual_x11_actions()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        active.remove(&self.keys[0]);
        active.remove(&self.keys[1]);
    }
}

fn finish_manual_x11_action(connection: &RustConnection, target: X11Window) {
    // A final round trip makes all requests flushed by this worker visible to
    // the X server before the per-target lease is released. This prevents a
    // new drag from racing an older configure request on the same window.
    let _ = connection.flush();
    if let Ok(cookie) = connection.get_geometry(target) {
        let _ = cookie.reply();
    }
}

fn configure_manual_x11_geometry(
    connection: &RustConnection,
    target: X11Window,
    action: ManualX11WindowAction,
    geometry: ManualX11Geometry,
) -> bool {
    let width = u32::try_from(geometry.width.max(1)).unwrap_or(u32::MAX);
    let height = u32::try_from(geometry.height.max(1)).unwrap_or(u32::MAX);
    let values = match action {
        ManualX11WindowAction::Move => ConfigureWindowAux::new().x(geometry.x).y(geometry.y),
        ManualX11WindowAction::Resize(_) => ConfigureWindowAux::new()
            .x(geometry.x)
            .y(geometry.y)
            .width(width)
            .height(height),
    };
    if connection.configure_window(target, &values).is_err() || connection.flush().is_err() {
        return false;
    }
    true
}

/// WSLg's Weston wrapper does not consistently honor `_NET_WM_MOVERESIZE` for
/// frameless clients. Keep the same left-button gesture usable there by
/// tracking the pointer on a private X11 connection and issuing configure
/// requests directly. On other backends the native winit operation remains
/// the fallback.
fn start_manual_x11_window_action(window: &slint::Window, action: ManualX11WindowAction) -> bool {
    let Some(window_id) = x11_window_id(window) else {
        return false;
    };
    let Ok((connection, screen_num)) = x11rb::connect(None) else {
        return false;
    };
    let Some(screen) = connection.setup().roots.get(screen_num) else {
        return false;
    };
    let root = screen.root;
    let target = x11_top_level_parent(&connection, window_id, root).unwrap_or(window_id);
    // A down event can reach more than one drag surface while the Slint item
    // tree is settling a grab. Only the first callback may own this target;
    // otherwise two polling workers can apply different pointer baselines and
    // visibly pull the window back and forth.
    // Keep the client XID in the lease as well as the managed wrapper.  The
    // wrapper lookup can transiently fall back to the client while a
    // compositor reparents the surface; the stable client key still prevents
    // two baselines from configuring one visible window.
    let Some(action_lease) = claim_manual_x11_action(target, window_id) else {
        return true;
    };
    let Ok(pointer_cookie) = connection.query_pointer(root) else {
        return false;
    };
    let Ok(pointer) = pointer_cookie.reply() else {
        return false;
    };
    let Ok(target_geometry_cookie) = connection.get_geometry(target) else {
        return false;
    };
    let Ok(target_geometry) = target_geometry_cookie.reply() else {
        return false;
    };
    let Ok(client_position_cookie) = connection.translate_coordinates(window_id, root, 0, 0) else {
        return false;
    };
    let Ok(client_position) = client_position_cookie.reply() else {
        return false;
    };
    let Ok(client_geometry_cookie) = connection.get_geometry(window_id) else {
        return false;
    };
    let Ok(client_geometry) = client_geometry_cookie.reply() else {
        return false;
    };
    let initial = ManualX11Geometry {
        // Pointer coordinates are relative to the X11 root.  The client
        // position must use that same coordinate space; the managed wrapper's
        // get_geometry() position is offset by the compositor frame.
        x: match action {
            ManualX11WindowAction::Move => i32::from(client_position.dst_x),
            ManualX11WindowAction::Resize(_) => i32::from(target_geometry.x),
        },
        y: match action {
            ManualX11WindowAction::Move => i32::from(client_position.dst_y),
            ManualX11WindowAction::Resize(_) => i32::from(target_geometry.y),
        },
        // Configure requests target the managed wrapper on WSLg, while its
        // width/height request is interpreted as the child client size.
        width: i32::from(client_geometry.width),
        height: i32::from(client_geometry.height),
    };
    let pointer_x = i32::from(pointer.root_x);
    let pointer_y = i32::from(pointer.root_y);

    thread::spawn(move || {
        let _action_lease = action_lease;
        let mut observed_button = false;
        let mut last_geometry = initial;
        let deadline = Instant::now() + Duration::from_millis(500);
        loop {
            let Ok(pointer_cookie) = connection.query_pointer(root) else {
                break;
            };
            let Ok(pointer) = pointer_cookie.reply() else {
                break;
            };
            let pressed = pointer.mask.contains(KeyButMask::BUTTON1);
            if !pressed {
                if observed_button {
                    // The release sample is the final pointer position.  Apply
                    // it once before ending the worker so a fast circular
                    // gesture cannot stop on an older queued coordinate.
                    let delta_x = i32::from(pointer.root_x) - pointer_x;
                    let delta_y = i32::from(pointer.root_y) - pointer_y;
                    let geometry = manual_window_geometry(initial, action, delta_x, delta_y);
                    if geometry != last_geometry
                        && !configure_manual_x11_geometry(&connection, target, action, geometry)
                    {
                        break;
                    }
                    break;
                }
                if Instant::now() >= deadline {
                    break;
                }
                thread::sleep(Duration::from_millis(4));
                continue;
            }
            observed_button = true;
            let delta_x = i32::from(pointer.root_x) - pointer_x;
            let delta_y = i32::from(pointer.root_y) - pointer_y;
            let geometry = manual_window_geometry(initial, action, delta_x, delta_y);
            if geometry == last_geometry {
                thread::sleep(MANUAL_X11_POLL_INTERVAL);
                continue;
            }
            if !configure_manual_x11_geometry(&connection, target, action, geometry) {
                break;
            }
            last_geometry = geometry;
            thread::sleep(MANUAL_X11_POLL_INTERVAL);
        }
        finish_manual_x11_action(&connection, target);
    });
    true
}

fn x11_top_level_parent(
    connection: &RustConnection,
    window: X11Window,
    root: X11Window,
) -> Option<X11Window> {
    let mut current = window;
    for _ in 0..8 {
        let reply = connection.query_tree(current).ok()?.reply().ok()?;
        if reply.parent == root || reply.parent == 0 {
            return Some(current);
        }
        current = reply.parent;
    }
    Some(current)
}

fn manual_window_geometry(
    initial: ManualX11Geometry,
    action: ManualX11WindowAction,
    delta_x: i32,
    delta_y: i32,
) -> ManualX11Geometry {
    match action {
        ManualX11WindowAction::Move => ManualX11Geometry {
            x: initial.x.saturating_add(delta_x),
            y: initial.y.saturating_add(delta_y),
            ..initial
        },
        ManualX11WindowAction::Resize(direction) => {
            manual_resize_geometry(initial, direction, delta_x, delta_y)
        }
    }
}

fn manual_resize_geometry(
    initial: ManualX11Geometry,
    direction: winit::window::ResizeDirection,
    delta_x: i32,
    delta_y: i32,
) -> ManualX11Geometry {
    const MIN_WIDTH: i32 = 700;
    const MIN_HEIGHT: i32 = 480;
    let east = matches!(
        direction,
        winit::window::ResizeDirection::East
            | winit::window::ResizeDirection::NorthEast
            | winit::window::ResizeDirection::SouthEast
    );
    let west = matches!(
        direction,
        winit::window::ResizeDirection::West
            | winit::window::ResizeDirection::NorthWest
            | winit::window::ResizeDirection::SouthWest
    );
    let north = matches!(
        direction,
        winit::window::ResizeDirection::North
            | winit::window::ResizeDirection::NorthEast
            | winit::window::ResizeDirection::NorthWest
    );
    let south = matches!(
        direction,
        winit::window::ResizeDirection::South
            | winit::window::ResizeDirection::SouthEast
            | winit::window::ResizeDirection::SouthWest
    );
    let width = if east {
        (initial.width.saturating_add(delta_x)).max(MIN_WIDTH)
    } else if west {
        (initial.width.saturating_sub(delta_x)).max(MIN_WIDTH)
    } else {
        initial.width
    };
    let height = if south {
        (initial.height.saturating_add(delta_y)).max(MIN_HEIGHT)
    } else if north {
        (initial.height.saturating_sub(delta_y)).max(MIN_HEIGHT)
    } else {
        initial.height
    };
    ManualX11Geometry {
        x: if west {
            initial
                .x
                .saturating_add(initial.width.saturating_sub(width))
        } else {
            initial.x
        },
        y: if north {
            initial
                .y
                .saturating_add(initial.height.saturating_sub(height))
        } else {
            initial.y
        },
        width,
        height,
    }
}

fn begin_window_drag(window: &slint::Window) {
    if start_manual_x11_window_action(window, ManualX11WindowAction::Move) {
        return;
    }
    let _ = window.with_winit_window(|winit_window| winit_window.drag_window());
}

fn minimize_window(window: &slint::Window) {
    let _ = window.with_winit_window(|winit_window| winit_window.set_minimized(true));
}

fn toggle_maximize_window(window: &slint::Window) {
    let _ = window.with_winit_window(|winit_window| {
        winit_window.set_maximized(!winit_window.is_maximized());
    });
}

fn parse_resize_direction(direction: &str) -> Option<winit::window::ResizeDirection> {
    Some(match direction {
        "east" => winit::window::ResizeDirection::East,
        "north" => winit::window::ResizeDirection::North,
        "north-east" => winit::window::ResizeDirection::NorthEast,
        "north-west" => winit::window::ResizeDirection::NorthWest,
        "south" => winit::window::ResizeDirection::South,
        "south-east" => winit::window::ResizeDirection::SouthEast,
        "south-west" => winit::window::ResizeDirection::SouthWest,
        "west" => winit::window::ResizeDirection::West,
        _ => return None,
    })
}

fn begin_window_resize(window: &slint::Window, direction: &str) {
    let Some(direction) = parse_resize_direction(direction) else {
        return;
    };
    if start_manual_x11_window_action(window, ManualX11WindowAction::Resize(direction)) {
        return;
    }
    let _ = window.with_winit_window(|winit_window| winit_window.drag_resize_window(direction));
}

fn install_window_size_guard(window: &slint::Window, expected_width: u32, expected_height: u32) {
    let _ = window.with_winit_window(|winit_window| winit_window.set_resizable(false));
    window.on_winit_window_event(move |slint_window, event| {
        let winit::event::WindowEvent::Resized(size) = event else {
            return EventResult::Propagate;
        };
        match fixed_resize_decision_for_size(
            size.width,
            size.height,
            expected_width,
            expected_height,
        ) {
            FixedResizeDecision::Propagate => EventResult::Propagate,
            FixedResizeDecision::RejectAndRestore => {
                let _ = slint_window.with_winit_window(|winit_window| {
                    winit_window.set_resizable(false);
                    let _ = winit_window.request_inner_size(winit::dpi::PhysicalSize::new(
                        expected_width,
                        expected_height,
                    ));
                });
                EventResult::PreventDefault
            }
        }
    });
}

/// Shows an existing secondary window and asks the native window manager to
/// activate and raise it.  `Window::show()` only maps a hidden Slint window; it
/// does not change the stacking order when the window already exists.
fn show_and_focus_window(
    window: &slint::Window,
    x11_monitor: Option<&X11WindowStateMonitor>,
) -> Result<(), slint::PlatformError> {
    let was_visible = window.is_visible();
    let window_id = was_visible.then(|| x11_window_id(window)).flatten();
    // Weston (the Xwayland window manager used by WSLg) ignores an explicit
    // raise request for a client that is already mapped.  Remapping the same
    // native window gives the WM its normal MapRequest path, which reliably
    // moves the existing window above its siblings without recreating it.
    if window_id.is_some() {
        window.hide()?;
    }
    window.show()?;
    let _ = window.with_winit_window(|winit_window| winit_window.focus_window());
    if let Some(x11_monitor) = x11_monitor {
        x11_monitor.raise_and_activate(window);
    }
    Ok(())
}

#[cfg(test)]
fn account_window_title(authenticated: bool, email: Option<&str>, plan_label: &str) -> String {
    if !authenticated {
        return UNAUTHENTICATED_WINDOW_TITLE.into();
    }
    let email = email
        .and_then(|value| security::bounded_email(value).ok())
        .filter(|value| !value.trim().is_empty());
    let Some(email) = email else {
        return UNAUTHENTICATED_WINDOW_TITLE.into();
    };
    let plan = security::bounded_plan(plan_label)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "プラン未設定".into());
    format!("{email} — {plan}")
}

#[cfg(test)]
fn detail_window_title(account_title: &str, purpose: &str) -> String {
    if account_title == UNAUTHENTICATED_WINDOW_TITLE {
        account_title.to_owned()
    } else {
        format!("{account_title} — {purpose}")
    }
}

#[cfg_attr(test, allow(dead_code))]
fn localized_plan_label(i18n: &I18n, plan_label: &str) -> String {
    match plan_label {
        "プラン未設定" => i18n.text(TextKey::PlanUnset).into(),
        "無料" => i18n.text(TextKey::PlanFree).into(),
        "エンタープライズ" => i18n.text(TextKey::PlanEnterprise).into(),
        "教育" => i18n.text(TextKey::PlanEducation).into(),
        other => other.to_owned(),
    }
}

#[cfg_attr(test, allow(dead_code))]
fn localized_account_window_title(
    i18n: &I18n,
    authenticated: bool,
    email: Option<&str>,
    plan_label: &str,
) -> String {
    if !authenticated {
        return i18n.text(TextKey::WindowUnauthenticated).into();
    }
    let email = email
        .and_then(|value| security::bounded_email(value).ok())
        .filter(|value| !value.trim().is_empty());
    let Some(email) = email else {
        return i18n.text(TextKey::WindowUnauthenticated).into();
    };
    let plan = security::bounded_plan(plan_label)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| localized_plan_label(i18n, &value))
        .unwrap_or_else(|| i18n.text(TextKey::PlanUnset).into());
    format!("{email} — {plan}")
}

fn native_detail_window_title(
    _i18n: &I18n,
    authenticated: bool,
    account_title: &str,
    purpose: WindowPurpose,
) -> String {
    if !authenticated {
        return "Codex Info".into();
    }
    format!(
        "{} - {}",
        native_account_window_title(account_title),
        purpose.native_label()
    )
}

/// Native title bars may be rendered by a window-manager fallback font that
/// does not contain the localized CJK glyphs. Keep the title-bar identity
/// ASCII-only while the in-window headings continue to use the locale catalog.
fn native_account_window_title(account_title: &str) -> String {
    if account_title == UNAUTHENTICATED_WINDOW_TITLE {
        return "Codex Info".into();
    }
    let Some((identity, plan)) = account_title.split_once(" — ") else {
        return "Codex Info".into();
    };
    let identity = ascii_title_part(identity, "Codex");
    let plan = ascii_title_part(plan, "Plan");
    format!("{identity} - {plan}")
}

fn ascii_title_part(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_graphic() || character == ' ')
    {
        value.to_owned()
    } else {
        fallback.to_owned()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct RateLimitSnapshot {
    remaining_percent: Option<f64>,
    reset_at: i64,
    window_seconds: i64,
    limit_name: String,
    quota_title: String,
    monthly: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ActiveThread {
    id: String,
    created_at: Option<i64>,
    updated_at: i64,
    title: String,
    model: String,
    model_label: String,
    total_tokens: Option<u64>,
    context_window_tokens: Option<u64>,
    last_user_message_at: Option<i64>,
    is_subagent: bool,
    parent_thread_id: Option<String>,
    depth: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ThreadPresentationRow {
    index: usize,
    forest_depth: usize,
    connected_to_parent: bool,
    has_children: bool,
    has_next_sibling: bool,
    ancestor_guides: [bool; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ActiveThreadUpdate {
    Snapshot(Vec<ActiveThread>),
    NoThread,
    Failed,
}

fn plan_type_label(plan_type: Option<&str>) -> String {
    protocol_contract::plan_label(plan_type)
}

fn monthly_window_seconds(reset_at: i64) -> i64 {
    let Some(end) = DateTime::<Utc>::from_timestamp(reset_at, 0) else {
        return 31 * 86_400;
    };
    end.checked_sub_months(Months::new(1))
        .map(|start| (end - start).num_seconds().max(1))
        .unwrap_or(31 * 86_400)
}

fn graph_period_end(reset_at: i64, current_reset_at: Option<i64>, now: i64) -> i64 {
    if current_reset_at.is_some_and(|current| same_reset_period(current, reset_at)) {
        now.min(reset_at)
    } else {
        reset_at
    }
}

fn parse_rate_limits(
    rate: &Value,
    plan_type: Option<&str>,
    _now: i64,
) -> Result<RateLimitSnapshot, ()> {
    protocol_contract::decode_quota_for_plan(rate, plan_type)
        .map_err(|_| ())?
        .ok_or(())
        .map(|quota| RateLimitSnapshot {
            remaining_percent: quota.remaining_percent.map(f64::from),
            reset_at: quota.reset_at,
            window_seconds: quota.window_seconds,
            limit_name: quota.limit_name,
            quota_title: if quota.monthly {
                "月間残り利用枠".into()
            } else if quota.unlimited {
                "利用枠".into()
            } else {
                "残り利用枠".into()
            },
            monthly: quota.monthly,
        })
}
fn same_rollout_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_rollout_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.is_file() && right.is_file()
}

fn complete_rollout_prefix_len(file: &mut File, snapshot_len: u64) -> Result<u64, ()> {
    if snapshot_len == 0 {
        return Ok(0);
    }
    let tail_len = snapshot_len.min(security::MAX_JSONL_LINE_BYTES as u64 + 1);
    let tail_start = snapshot_len.checked_sub(tail_len).ok_or(())?;
    file.seek(SeekFrom::Start(tail_start)).map_err(|_| ())?;
    let capacity = usize::try_from(tail_len).map_err(|_| ())?;
    let mut tail = Vec::with_capacity(capacity);
    file.take(tail_len).read_to_end(&mut tail).map_err(|_| ())?;
    if tail.len() != capacity {
        return Err(());
    }
    if tail.last() == Some(&b'\n') {
        return Ok(snapshot_len);
    }
    if let Some(position) = tail.iter().rposition(|byte| *byte == b'\n') {
        return tail_start
            .checked_add(u64::try_from(position).map_err(|_| ())?)
            .and_then(|position| position.checked_add(1))
            .ok_or(());
    }
    if snapshot_len > security::MAX_JSONL_LINE_BYTES as u64 {
        return Err(());
    }
    Ok(0)
}

fn read_thread_rollout(
    sessions_root: &Path,
    candidate: &ValidatedThreadCandidate,
) -> Result<thread_contract::ValidatedRollout, ()> {
    let candidate_path = candidate.path().ok_or(())?;
    read_thread_rollout_path(sessions_root, Path::new(candidate_path))
}

fn read_thread_rollout_path(
    sessions_root: &Path,
    candidate_path: &Path,
) -> Result<thread_contract::ValidatedRollout, ()> {
    let canonical =
        security::canonical_regular_file_under(sessions_root, candidate_path).map_err(|_| ())?;
    let before_path = fs::symlink_metadata(&canonical).map_err(|_| ())?;
    if before_path.file_type().is_symlink() || !before_path.is_file() {
        return Err(());
    }
    let mut file = File::open(&canonical).map_err(|_| ())?;
    let before_file = file.metadata().map_err(|_| ())?;
    if !same_rollout_identity(&before_path, &before_file)
        || before_file.len() > security::MAX_SESSION_FILE_BYTES
    {
        return Err(());
    }
    let snapshot_len = before_file.len();
    let complete_len = complete_rollout_prefix_len(&mut file, snapshot_len)?;
    file.seek(SeekFrom::Start(0)).map_err(|_| ())?;
    let rollout = {
        let mut reader = BufReader::new((&mut file).take(complete_len));
        thread_contract::parse_rollout_reader(&mut reader, complete_len).map_err(|_| ())?
    };

    let after_file = file.metadata().map_err(|_| ())?;
    let after_path = fs::symlink_metadata(&canonical).map_err(|_| ())?;
    if after_path.file_type().is_symlink()
        || !after_path.is_file()
        || !same_rollout_identity(&before_file, &after_file)
        || !same_rollout_identity(&after_file, &after_path)
        || after_file.len() < snapshot_len
    {
        return Err(());
    }
    Ok(rollout)
}

const MAX_PROC_PROCESS_ENTRIES: usize = 65_536;
const MAX_CODEX_PROCESS_FDS: usize = 16_384;
const MAX_OPEN_SESSION_FILES: usize = 1_024;

fn open_codex_session_paths(
    proc_root: &Path,
    sessions_root: &Path,
) -> Result<BTreeSet<PathBuf>, ()> {
    let mut process_entries = 0usize;
    let mut open_files = BTreeSet::new();
    for process in fs::read_dir(proc_root).map_err(|_| ())? {
        process_entries = process_entries.checked_add(1).ok_or(())?;
        if process_entries > MAX_PROC_PROCESS_ENTRIES {
            return Err(());
        }
        let process = match process {
            Ok(process) => process,
            Err(_) => continue,
        };
        let name = process.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let process_path = process.path();
        let comm = match File::open(process_path.join("comm")) {
            Ok(file) => {
                let mut bytes = Vec::new();
                if file.take(64).read_to_end(&mut bytes).is_err() {
                    continue;
                }
                bytes
            }
            Err(_) => continue,
        };
        if comm.strip_suffix(b"\n") != Some(b"codex") && comm.as_slice() != b"codex" {
            continue;
        }
        let executable = match fs::read_link(process_path.join("exe")) {
            Ok(executable) => executable,
            Err(_) => continue,
        };
        if executable.file_name().and_then(|name| name.to_str()) != Some("codex") {
            continue;
        }
        let descriptors = match fs::read_dir(process_path.join("fd")) {
            Ok(descriptors) => descriptors,
            Err(_) => continue,
        };
        let mut descriptor_count = 0usize;
        for descriptor in descriptors {
            descriptor_count = descriptor_count.checked_add(1).ok_or(())?;
            if descriptor_count > MAX_CODEX_PROCESS_FDS {
                return Err(());
            }
            let descriptor = match descriptor {
                Ok(descriptor) => descriptor,
                Err(_) => continue,
            };
            let target = match fs::read_link(descriptor.path()) {
                Ok(target) => target,
                Err(_) => continue,
            };
            let Ok(canonical) = security::canonical_regular_file_under(sessions_root, &target)
            else {
                continue;
            };
            if canonical
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("jsonl")
            {
                continue;
            }
            open_files.insert(canonical);
            if open_files.len() > MAX_OPEN_SESSION_FILES {
                return Err(());
            }
        }
    }
    Ok(open_files)
}

fn fetch_active_thread_update(
    input: &mut impl Write,
    output: &Receiver<RpcReadEvent>,
    next_id: &mut u64,
    codex_root: Option<&Path>,
) -> ActiveThreadUpdate {
    let Some(codex_root) = codex_root else {
        return ActiveThreadUpdate::Failed;
    };
    let sessions_root = codex_root.join("sessions");
    let active_paths = match open_codex_session_paths(Path::new("/proc"), &sessions_root) {
        Ok(paths) => paths,
        Err(_) => return ActiveThreadUpdate::Failed,
    };
    if active_paths.is_empty() {
        return ActiveThreadUpdate::NoThread;
    }
    fetch_active_thread_update_for_paths_and_state(
        input,
        output,
        next_id,
        &sessions_root,
        &active_paths,
        Some(codex_root),
    )
}

#[cfg(test)]
fn fetch_active_thread_update_for_paths(
    input: &mut impl Write,
    output: &Receiver<RpcReadEvent>,
    next_id: &mut u64,
    sessions_root: &Path,
    active_paths: &BTreeSet<PathBuf>,
) -> ActiveThreadUpdate {
    fetch_active_thread_update_for_paths_and_state(
        input,
        output,
        next_id,
        sessions_root,
        active_paths,
        None,
    )
}

fn fetch_active_thread_update_for_paths_and_state(
    input: &mut impl Write,
    output: &Receiver<RpcReadEvent>,
    next_id: &mut u64,
    sessions_root: &Path,
    active_paths: &BTreeSet<PathBuf>,
    codex_root: Option<&Path>,
) -> ActiveThreadUpdate {
    let mut accumulator = ThreadCycleAccumulator::new();
    let mut cursor: Option<String> = None;
    loop {
        let params = match thread_contract::thread_list_request(cursor.as_deref()) {
            Ok(params) => params,
            Err(_) => return ActiveThreadUpdate::Failed,
        };
        let request_id = *next_id;
        let Some(following_id) = next_id.checked_add(1) else {
            return ActiveThreadUpdate::Failed;
        };
        *next_id = following_id;
        let page = match request(input, output, request_id, "thread/list", params) {
            Ok(page) => page,
            Err(_) => return ActiveThreadUpdate::Failed,
        };
        match accumulator.accept_page(&page) {
            Ok(PageAcceptance::NeedNextPage { cursor: next }) => cursor = Some(next),
            Ok(PageAcceptance::Terminal) => break,
            Err(_) => return ActiveThreadUpdate::Failed,
        }
    }

    let owner_root_ids = match accumulator.clone().ordered_candidates() {
        Ok(candidates) => candidates
            .into_iter()
            .filter(|candidate| {
                candidate
                    .path()
                    .and_then(|path| {
                        security::canonical_regular_file_under(sessions_root, Path::new(path)).ok()
                    })
                    .is_some_and(|path| active_paths.contains(&path))
            })
            .map(|candidate| candidate.id().to_owned())
            .collect::<BTreeSet<_>>(),
        Err(_) => return ActiveThreadUpdate::Failed,
    };

    let root_outcome = thread_contract::select_active_threads_parsed_where(
        accumulator,
        |candidate| {
            candidate
                .path()
                .and_then(|path| {
                    security::canonical_regular_file_under(sessions_root, Path::new(path)).ok()
                })
                .is_some_and(|path| active_paths.contains(&path))
        },
        |candidate| read_thread_rollout(sessions_root, candidate),
    );
    let root_snapshots = match root_outcome {
        ThreadCycleOutcome::Snapshots(snapshots) => snapshots,
        ThreadCycleOutcome::NoThread => Vec::new(),
        ThreadCycleOutcome::CycleError => return ActiveThreadUpdate::Failed,
    };

    let mut threads = root_snapshots
        .into_iter()
        .map(|snapshot| ActiveThread {
            id: snapshot.thread_id,
            created_at: Some(snapshot.created_at),
            updated_at: snapshot.updated_at,
            title: snapshot.title,
            model: snapshot.model,
            model_label: snapshot.model_label,
            total_tokens: snapshot.total_tokens,
            context_window_tokens: snapshot.context_window_tokens,
            last_user_message_at: snapshot.last_user_message_at,
            is_subagent: snapshot.is_subagent,
            parent_thread_id: snapshot.parent_thread_id,
            depth: snapshot.depth,
        })
        .collect::<Vec<_>>();

    if let Some(codex_root) = codex_root {
        let descendants =
            match thread_state::load_native_descendants(codex_root, sessions_root, &owner_root_ids)
            {
                Ok(descendants) => descendants,
                Err(_) => return ActiveThreadUpdate::Failed,
            };
        for descendant in descendants {
            let rollout = match read_thread_rollout_path(sessions_root, &descendant.rollout_path) {
                Ok(rollout) => rollout,
                Err(_) => return ActiveThreadUpdate::Failed,
            };
            if !rollout.is_running() {
                continue;
            }
            threads.push(ActiveThread {
                id: descendant.id,
                created_at: descendant.created_at,
                updated_at: descendant.updated_at,
                title: descendant.title,
                model: rollout.model().to_owned(),
                model_label: rollout.model_label().to_owned(),
                total_tokens: rollout.total_tokens(),
                context_window_tokens: rollout.context_window_tokens(),
                last_user_message_at: rollout.last_user_message_at(),
                is_subagent: true,
                parent_thread_id: Some(descendant.parent_thread_id),
                depth: Some(descendant.depth),
            });
        }
    }

    let mut by_id = BTreeMap::new();
    for thread in threads {
        by_id.insert(thread.id.clone(), thread);
    }
    let mut threads = by_id.into_values().collect::<Vec<_>>();
    threads.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    if threads.is_empty() {
        ActiveThreadUpdate::NoThread
    } else {
        ActiveThreadUpdate::Snapshot(threads)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct UsageHistorySample {
    timestamp: i64,
    reset_at: i64,
    remaining_percent: f64,
    sol_dollars: f64,
    terra_dollars: f64,
    luna_dollars: f64,
    #[serde(default)]
    sol_tokens: u64,
    #[serde(default)]
    terra_tokens: u64,
    #[serde(default)]
    luna_tokens: u64,
}

impl UsageHistorySample {
    fn from_store(sample: usage_store::UsageHistorySample) -> Self {
        Self {
            timestamp: sample.timestamp,
            reset_at: sample.reset_at,
            remaining_percent: sample.remaining_percent.unwrap_or(-1.0),
            sol_dollars: sample.sol_dollars,
            terra_dollars: sample.terra_dollars,
            luna_dollars: sample.luna_dollars,
            sol_tokens: sample.sol_tokens,
            terra_tokens: sample.terra_tokens,
            luna_tokens: sample.luna_tokens,
        }
    }

    fn to_store(&self) -> usage_store::UsageHistorySample {
        usage_store::UsageHistorySample {
            timestamp: self.timestamp,
            reset_at: self.reset_at,
            remaining_percent: (self.remaining_percent >= 0.0).then_some(self.remaining_percent),
            sol_dollars: self.sol_dollars,
            terra_dollars: self.terra_dollars,
            luna_dollars: self.luna_dollars,
            sol_tokens: self.sol_tokens,
            terra_tokens: self.terra_tokens,
            luna_tokens: self.luna_tokens,
        }
    }

    #[cfg(test)]
    fn new(
        timestamp: i64,
        reset_at: i64,
        remaining_percent: f64,
        costs: ModelDollarTotals,
    ) -> Self {
        Self::new_with_usage(
            timestamp,
            reset_at,
            remaining_percent,
            costs,
            ModelTokenTotals::default(),
        )
    }

    fn new_with_usage(
        timestamp: i64,
        reset_at: i64,
        remaining_percent: f64,
        costs: ModelDollarTotals,
        tokens: ModelTokenTotals,
    ) -> Self {
        Self {
            // 1分ごとの取得値として、同じ分に複数回届いた場合も1点にまとめる。
            timestamp: timestamp.div_euclid(60) * 60,
            reset_at,
            remaining_percent: remaining_percent.clamp(0.0, 100.0),
            sol_dollars: costs.sol.max(0.0),
            terra_dollars: costs.terra.max(0.0),
            luna_dollars: costs.luna.max(0.0),
            sol_tokens: tokens.sol,
            terra_tokens: tokens.terra,
            luna_tokens: tokens.luna,
        }
    }

    #[cfg(test)]
    fn from_model_history(timestamp: i64, reset_at: i64, costs: ModelDollarTotals) -> Self {
        Self::from_model_history_with_usage(timestamp, reset_at, costs, ModelTokenTotals::default())
    }

    fn from_model_history_with_usage(
        timestamp: i64,
        reset_at: i64,
        costs: ModelDollarTotals,
        tokens: ModelTokenTotals,
    ) -> Self {
        Self {
            timestamp: timestamp.div_euclid(60) * 60,
            reset_at,
            // セッションログには残り利用枠の履歴がないため、グラフでは欠測として扱う。
            remaining_percent: -1.0,
            sol_dollars: costs.sol.max(0.0),
            terra_dollars: costs.terra.max(0.0),
            luna_dollars: costs.luna.max(0.0),
            sol_tokens: tokens.sol,
            terra_tokens: tokens.terra,
            luna_tokens: tokens.luna,
        }
    }

    fn is_valid(&self) -> bool {
        self.timestamp > 0
            && self.reset_at > 0
            && self.remaining_percent.is_finite()
            && self.sol_dollars.is_finite()
            && self.terra_dollars.is_finite()
            && self.luna_dollars.is_finite()
    }
}

fn same_reset_period(left: i64, right: i64) -> bool {
    left.abs_diff(right) <= RESET_AT_TOLERANCE_SECONDS as u64
}

fn merge_sample_values(existing: &mut UsageHistorySample, incoming: UsageHistorySample) {
    // Session backfill has no remaining-quota observation. Keep an existing
    // observed value while allowing a later API observation to replace it.
    let remaining_percent = if incoming.remaining_percent >= 0.0 {
        incoming.remaining_percent
    } else {
        existing.remaining_percent
    };
    let sol_dollars = existing.sol_dollars.max(incoming.sol_dollars);
    let terra_dollars = existing.terra_dollars.max(incoming.terra_dollars);
    let luna_dollars = existing.luna_dollars.max(incoming.luna_dollars);
    let sol_tokens = existing.sol_tokens.max(incoming.sol_tokens);
    let terra_tokens = existing.terra_tokens.max(incoming.terra_tokens);
    let luna_tokens = existing.luna_tokens.max(incoming.luna_tokens);
    *existing = incoming;
    existing.remaining_percent = remaining_percent;
    existing.sol_dollars = sol_dollars;
    existing.terra_dollars = terra_dollars;
    existing.luna_dollars = luna_dollars;
    existing.sol_tokens = sol_tokens;
    existing.terra_tokens = terra_tokens;
    existing.luna_tokens = luna_tokens;
}

fn merge_exact_sample(samples: &mut Vec<UsageHistorySample>, incoming: UsageHistorySample) {
    if let Some(existing) = samples.iter_mut().find(|existing| {
        existing.reset_at == incoming.reset_at && existing.timestamp == incoming.timestamp
    }) {
        merge_sample_values(existing, incoming);
    } else {
        samples.push(incoming);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HistoryPeriod {
    canonical_reset_at: i64,
    start: i64,
    end: i64,
    label: String,
}

fn legacy_moving_reset_artifact(
    candidate: &UsageHistorySample,
    samples: &[&UsageHistorySample],
    exact_reset_counts: &BTreeMap<i64, usize>,
) -> bool {
    if candidate.remaining_percent != 100.0
        || exact_reset_counts.get(&candidate.reset_at) != Some(&1)
    {
        return false;
    }
    let candidate_horizon = i128::from(candidate.reset_at) - i128::from(candidate.timestamp);
    if candidate_horizon.abs_diff(i128::from(WEEK_SECONDS))
        > LEGACY_MOVING_RESET_HORIZON_TOLERANCE_SECONDS as u128
    {
        return false;
    }
    samples.iter().any(|other| {
        if other.reset_at == candidate.reset_at
            || other.remaining_percent != 100.0
            || exact_reset_counts.get(&other.reset_at) != Some(&1)
            || candidate.timestamp.abs_diff(other.timestamp)
                > LEGACY_MOVING_RESET_PAIR_GAP_SECONDS as u64
        {
            return false;
        }
        let other_horizon = i128::from(other.reset_at) - i128::from(other.timestamp);
        other_horizon.abs_diff(i128::from(WEEK_SECONDS))
            <= LEGACY_MOVING_RESET_HORIZON_TOLERANCE_SECONDS as u128
            && candidate_horizon.abs_diff(other_horizon)
                <= LEGACY_MOVING_RESET_PAIR_HORIZON_TOLERANCE_SECONDS as u128
    })
}

fn display_history_samples(samples: &[UsageHistorySample]) -> Vec<&UsageHistorySample> {
    let valid_samples = samples
        .iter()
        .filter(|sample| sample.is_valid())
        .collect::<Vec<_>>();
    let mut exact_reset_counts = BTreeMap::new();
    for sample in &valid_samples {
        *exact_reset_counts.entry(sample.reset_at).or_insert(0) += 1;
    }
    valid_samples
        .iter()
        .copied()
        .filter(|sample| !legacy_moving_reset_artifact(sample, &valid_samples, &exact_reset_counts))
        .collect()
}

/// Groups reset observations by an anchored sixty-second window. The anchor
/// is never advanced by a member of the group, which prevents a chain of
/// small jitters from swallowing a distinct period.
fn history_periods_for_samples(
    samples: &[UsageHistorySample],
    now: i64,
    current_reset_at: Option<i64>,
) -> Vec<HistoryPeriod> {
    let mut sorted = display_history_samples(samples);
    sorted.sort_by_key(|sample| (sample.reset_at, sample.timestamp));

    let mut groups: Vec<(i64, i64, i64)> = Vec::new();
    let mut index = 0;
    while let Some(first) = sorted.get(index) {
        let anchor = first.reset_at;
        let mut canonical = anchor;
        let mut start = first.timestamp;
        while let Some(sample) = sorted.get(index) {
            if sample.reset_at.saturating_sub(anchor) > RESET_AT_TOLERANCE_SECONDS {
                break;
            }
            canonical = canonical.max(sample.reset_at);
            start = start.min(sample.timestamp);
            index += 1;
        }
        groups.push((anchor, canonical, start));
    }

    let mut periods = groups
        .iter()
        .enumerate()
        .map(|(index, &(anchor, canonical, start))| {
            let next_start = groups.get(index + 1).map(|group| group.2);
            let period_end = next_start.map_or(canonical, |next| canonical.min(next));
            let is_current = current_reset_at.is_some_and(|current| {
                current.abs_diff(canonical) <= RESET_AT_TOLERANCE_SECONDS as u64 && now < canonical
            });
            let end = if is_current {
                now.max(start).min(canonical)
            } else {
                period_end
            };
            let _ = anchor;
            // Labels are a presentation concern. Production UI labels are
            // rebuilt by `CodexInfoState::history_periods` with the one
            // startup-pinned I18n/timezone instance. The test-only label
            // keeps the legacy grouping fixtures readable without allowing a
            // fixed JST formatter into the runtime path.
            #[cfg(test)]
            let mut label = format_period_label(start, period_end);
            #[cfg(not(test))]
            let label = String::new();
            #[cfg(test)]
            if is_current {
                label.push_str("（現在）");
            }
            HistoryPeriod {
                canonical_reset_at: canonical,
                start,
                end,
                label,
            }
        })
        .collect::<Vec<_>>();
    // A burst of several reset groups can share the same observed start and
    // therefore the same base interval label. ComboBox selection must never
    // rely on an ambiguous label, so only colliding labels receive the
    // canonical reset timestamp as a deterministic, user-readable suffix.
    #[cfg(test)]
    {
        let base_labels = periods
            .iter()
            .map(|period| period.label.clone())
            .collect::<Vec<_>>();
        for index in 0..periods.len() {
            if base_labels
                .iter()
                .filter(|label| **label == base_labels[index])
                .count()
                > 1
            {
                let canonical_reset_at = periods[index].canonical_reset_at;
                let reset_label = format_period_timestamp(canonical_reset_at)
                    .unwrap_or_else(|| "時刻不明".into());
                periods[index]
                    .label
                    .push_str(&format!("（期限 {reset_label}）"));
            }
        }
    }
    periods.sort_by(|left, right| {
        right
            .start
            .cmp(&left.start)
            .then_with(|| right.canonical_reset_at.cmp(&left.canonical_reset_at))
    });
    periods
}

#[derive(Debug, Default)]
struct UsageHistory {
    path: Option<PathBuf>,
    db_path: Option<PathBuf>,
    samples: Vec<UsageHistorySample>,
    startup_maintenance_done: bool,
}

impl UsageHistory {
    fn load() -> Self {
        let mut history = Self::load_from_paths(usage_history_path(), usage_history_db_path());
        history.migrate_legacy_history();
        history.startup_maintenance(Utc::now());
        history
    }

    /// Performs an additive, idempotent import when a portable data directory
    /// is configured. The legacy database and JSON are never modified or
    /// deleted; the destination is merged by the composite primary key.
    fn migrate_legacy_history(&mut self) {
        let Some(destination_db) = self.db_path.as_ref() else {
            return;
        };
        let Some(destination_root) = destination_db.parent().and_then(Path::parent) else {
            return;
        };
        if let Some(parent) = destination_db.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let Some(legacy_root) = legacy_usage_root() else {
            return;
        };
        let Some(legacy_history) = legacy_root.join("history").canonicalize().ok() else {
            return;
        };
        let Some(destination_history) = destination_db
            .parent()
            .and_then(|path| path.canonicalize().ok())
        else {
            return;
        };
        if legacy_history == destination_history {
            return;
        }

        let marker = destination_root
            .join("history")
            .join("usage_history_v1.migrated");
        if marker.exists() {
            return;
        }

        let legacy_db = legacy_history.join("usage_history.sqlite3");
        let legacy_json = legacy_history.join("usage_history.json");
        let Ok(mut store) = UsageStore::open(destination_db) else {
            return;
        };
        let mut imported = 0usize;
        let mut failed = false;
        if legacy_db.is_file() {
            match store.import_v1_sqlite(&legacy_db) {
                Ok(count) => imported = imported.saturating_add(count),
                Err(error) => {
                    eprintln!("v1 SQLite migration skipped: {error}");
                    failed = true;
                }
            }
        }
        if legacy_json.is_file() {
            match store.import_v1_json(&legacy_json) {
                Ok(count) => imported = imported.saturating_add(count),
                Err(error) => {
                    eprintln!("v1 JSON migration skipped: {error}");
                    failed = true;
                }
            }
        }
        if failed {
            return;
        }
        if let Ok(samples) = store.load_all() {
            self.samples.clear();
            for sample in samples {
                merge_exact_sample(&mut self.samples, UsageHistorySample::from_store(sample));
            }
            self.normalize();
        }
        if let Some(parent) = marker.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let marker_contents = format!(
            "{{\"source\":\"{}\",\"imported_rows\":{},\"completed_at\":{}}}\n",
            legacy_history.display(),
            imported,
            Utc::now().timestamp()
        );
        let _ = fs::write(marker, marker_contents);
        self.save();
    }

    fn load_from_paths(path: Option<PathBuf>, db_path: Option<PathBuf>) -> Self {
        let database_samples = db_path
            .as_ref()
            .and_then(|path| UsageStore::open(path).ok())
            .and_then(|store| store.load_all().ok())
            .unwrap_or_default();
        let json_samples = path
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|contents| serde_json::from_str::<Vec<UsageHistorySample>>(&contents).ok())
            .unwrap_or_default();

        // Keep each persisted primary-key row in memory. Near reset periods
        // are merged only by display selection, so JSON migration cannot
        // make an existing DB row disappear.
        let mut samples = Vec::new();
        for sample in json_samples {
            if sample.is_valid() {
                merge_exact_sample(&mut samples, sample);
            }
        }
        for database_sample in database_samples
            .into_iter()
            .map(UsageHistorySample::from_store)
        {
            // DB側のモデル累積値を優先しつつ、DB移行前のJSONにしかない
            // 残量計測値を欠測(-1)で上書きしない。
            merge_exact_sample(&mut samples, database_sample);
        }
        let mut history = Self {
            path,
            db_path,
            samples,
            startup_maintenance_done: false,
        };
        history.normalize();
        if !history.samples.is_empty() {
            history.save();
        }
        history
    }

    fn preview(now: i64, reset_at: i64, costs: ModelDollarTotals) -> Self {
        let fractions = [0.08, 0.28, 0.48, 0.68, 0.88, 1.0];
        let preview_period =
            |period_reset: i64, period_start: i64, period_end: i64, cost_scale: f64| {
                let elapsed = period_end.saturating_sub(period_start).max(1) as f64;
                fractions.into_iter().map(move |fraction| {
                    // プレビュー点を現在時刻までの実測可能な範囲へ分散し、
                    // 未来の点を現在時刻へ丸めて同一X座標に重ねない。
                    let timestamp = period_start + (elapsed * fraction) as i64;
                    let used_percent = 10.0 + 76.0 * fraction;
                    let sol_scale = (0.18 + 0.82 * fraction) * cost_scale;
                    let terra_scale = (1.0 - 0.65 * fraction).max(0.1) * cost_scale;
                    let luna_scale = (0.35 + 0.65 * (1.0 - fraction).powi(2)).max(0.1) * cost_scale;
                    UsageHistorySample::new_with_usage(
                        timestamp,
                        period_reset,
                        100.0 - used_percent,
                        ModelDollarTotals {
                            sol: costs.sol * sol_scale,
                            terra: costs.terra * terra_scale,
                            luna: costs.luna * luna_scale,
                        },
                        ModelTokenTotals {
                            sol: (159_278_976.0 * sol_scale) as u64,
                            terra: (30_885_887.0 * terra_scale) as u64,
                            luna: (155_294_770.0 * luna_scale) as u64,
                        },
                    )
                })
            };
        let previous_reset_at = reset_at.saturating_sub(WEEK_SECONDS);
        let previous = preview_period(
            previous_reset_at,
            previous_reset_at.saturating_sub(WEEK_SECONDS),
            previous_reset_at,
            0.72,
        );
        let current = preview_period(
            reset_at,
            reset_at.saturating_sub(WEEK_SECONDS),
            now.min(reset_at),
            1.0,
        );
        let samples = previous.chain(current).collect();
        Self {
            path: None,
            db_path: None,
            samples,
            startup_maintenance_done: true,
        }
    }

    /// Performs the one destructive history operation during normal startup.
    ///
    /// The visible in-memory set is always bounded, even if persistent pruning
    /// is unavailable. A storage failure must never expose an old or future row.
    fn startup_maintenance(&mut self, now: DateTime<Utc>) {
        if self.startup_maintenance_done {
            return;
        }
        self.startup_maintenance_done = true;

        if let Some(path) = self.db_path.as_ref() {
            if let Ok(mut store) = UsageStore::open(path) {
                let _ = store.prune_older_than_three_months(now);
            }
        }

        let cutoff = three_months_before_utc(now);
        self.samples
            .retain(|sample| sample.timestamp >= cutoff && sample.timestamp <= now.timestamp());
        self.normalize();
    }

    fn record(&mut self, sample: UsageHistorySample) {
        if !sample.is_valid() {
            return;
        }
        let mut sample = sample;
        sample.reset_at = self.canonical_reset_at(sample.reset_at);
        merge_exact_sample(&mut self.samples, sample);
        self.normalize();
        self.save();
    }

    fn apply_backfill_samples(&mut self, reset_at: i64, samples: Vec<UsageHistorySample>) {
        if samples.is_empty() {
            return;
        }
        let storage_reset_at = self.canonical_reset_at(reset_at);
        for mut sample in samples {
            if !sample.is_valid() {
                continue;
            }
            sample.reset_at = storage_reset_at;
            merge_exact_sample(&mut self.samples, sample);
        }
        self.normalize();
        self.save();
    }

    fn canonical_reset_at(&self, reset_at: i64) -> i64 {
        history_periods_for_samples(&self.samples, 0, None)
            .into_iter()
            .find(|period| {
                self.samples.iter().any(|sample| {
                    sample.reset_at.abs_diff(period.canonical_reset_at)
                        <= RESET_AT_TOLERANCE_SECONDS as u64
                        && sample.reset_at.abs_diff(reset_at) <= RESET_AT_TOLERANCE_SECONDS as u64
                })
            })
            .map_or(reset_at, |period| period.canonical_reset_at)
    }

    fn graph_data_for_reset(&self, reset_at: i64) -> String {
        let samples = self.samples_for_reset(Some(reset_at));
        serde_json::to_string(&samples).unwrap_or_else(|_| "[]".into())
    }

    fn reset_periods_desc(&self) -> Vec<i64> {
        history_periods_for_samples(&self.samples, 0, None)
            .into_iter()
            .map(|period| period.canonical_reset_at)
            .collect()
    }

    fn periods(&self, now: i64, current_reset_at: Option<i64>) -> Vec<HistoryPeriod> {
        history_periods_for_samples(&self.samples, now, current_reset_at)
    }

    fn period_for_id(
        &self,
        canonical_reset_at: i64,
        now: i64,
        current_reset_at: Option<i64>,
    ) -> Option<HistoryPeriod> {
        self.periods(now, current_reset_at)
            .into_iter()
            .find(|period| period.canonical_reset_at == canonical_reset_at)
    }

    #[cfg(test)]
    fn period_id_for_label(
        &self,
        label: &str,
        now: i64,
        current_reset_at: Option<i64>,
    ) -> Option<i64> {
        self.periods(now, current_reset_at)
            .into_iter()
            .find(|period| period.label == label)
            .map(|period| period.canonical_reset_at)
    }

    #[cfg(test)]
    fn period_options(&self, now: i64, current_reset_at: Option<i64>) -> Vec<String> {
        let periods = self.periods(now, current_reset_at);
        if periods.is_empty() {
            vec!["履歴なし".into()]
        } else {
            periods.into_iter().map(|period| period.label).collect()
        }
    }

    fn samples_for_reset(&self, reset_at: Option<i64>) -> Vec<UsageHistorySample> {
        let Some(reset_at) = reset_at else {
            return Vec::new();
        };
        let selected_period = self.period_for_id(reset_at, 0, None).or_else(|| {
            self.periods(0, None).into_iter().find(|period| {
                period.canonical_reset_at.abs_diff(reset_at) <= RESET_AT_TOLERANCE_SECONDS as u64
            })
        });
        let Some(selected_period) = selected_period else {
            return Vec::new();
        };
        let mut selected = display_history_samples(&self.samples)
            .into_iter()
            .filter(|sample| {
                sample.reset_at
                    >= selected_period
                        .canonical_reset_at
                        .saturating_sub(RESET_AT_TOLERANCE_SECONDS)
                    && sample.reset_at <= selected_period.canonical_reset_at
            })
            .cloned()
            .collect::<Vec<_>>();
        selected.sort_by_key(|sample| (sample.timestamp, sample.reset_at));
        let reset_at = selected_period.canonical_reset_at;
        let mut merged: Vec<UsageHistorySample> = Vec::with_capacity(selected.len());
        for mut sample in selected {
            sample.reset_at = reset_at;
            if let Some(existing) = merged.last_mut() {
                if existing.timestamp == sample.timestamp {
                    merge_sample_values(existing, sample);
                    existing.reset_at = reset_at;
                    continue;
                }
            }
            merged.push(sample);
        }
        merged
    }

    fn normalize(&mut self) {
        self.samples.retain(UsageHistorySample::is_valid);
        self.samples
            .sort_by_key(|sample| (sample.reset_at, sample.timestamp));
        let mut normalized: Vec<UsageHistorySample> = Vec::with_capacity(self.samples.len());
        for sample in self.samples.drain(..) {
            if let Some(existing) = normalized.last_mut() {
                if existing.reset_at == sample.reset_at && existing.timestamp == sample.timestamp {
                    merge_sample_values(existing, sample);
                    continue;
                }
            }
            normalized.push(sample);
        }
        self.samples = normalized;
    }

    fn save(&self) {
        if let Some(path) = &self.db_path {
            if let Ok(mut store) = UsageStore::open(path) {
                let samples = self
                    .samples
                    .iter()
                    .map(UsageHistorySample::to_store)
                    .collect::<Vec<_>>();
                if store.upsert_samples(&samples).is_ok() {
                    return;
                }
            }
        }
        let Some(path) = &self.path else {
            return;
        };
        let Some(parent) = path.parent() else {
            return;
        };
        if fs::create_dir_all(parent).is_err() {
            return;
        }
        let Ok(contents) = serde_json::to_vec_pretty(&self.samples) else {
            return;
        };
        let temporary = path.with_extension("json.tmp");
        if fs::write(&temporary, contents).is_ok() {
            let _ = fs::rename(temporary, path);
        }
    }
}

fn usage_history_path() -> Option<PathBuf> {
    Some(
        usage_data_root()?
            .join("history")
            .join("usage_history.json"),
    )
}

fn usage_history_db_path() -> Option<PathBuf> {
    Some(
        usage_data_root()?
            .join("history")
            .join("usage_history.sqlite3"),
    )
}

fn default_codex_root() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
}

fn validated_configured_root(path: PathBuf) -> Option<PathBuf> {
    security::validate_absolute_root(&path).ok()
}

fn prepared_data_root(path: PathBuf) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    if !path.exists() {
        let ancestor = path.ancestors().find(|ancestor| ancestor.exists())?;
        security::validate_absolute_root(ancestor).ok()?;
        fs::create_dir_all(&path).ok()?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).ok()?;
    }
    validated_configured_root(path)
}

fn usage_data_root() -> Option<PathBuf> {
    let path = std::env::var_os("CODEX_INFO_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(default_codex_root);
    prepared_data_root(path)
}

fn legacy_usage_root() -> Option<PathBuf> {
    let path = std::env::var_os("CODEX_INFO_MIGRATE_FROM")
        .map(PathBuf::from)
        .unwrap_or_else(default_codex_root);
    validated_configured_root(path)
}

fn three_months_before_utc(now: DateTime<Utc>) -> i64 {
    now.checked_sub_months(Months::new(3))
        .expect("subtracting three calendar months from UTC now must be representable")
        .timestamp()
}

#[derive(Default)]
struct GraphPaths {
    remaining: String,
    remaining_markers: Vec<RemainingMarkerPosition>,
    unused_intervals: Vec<UnusedIntervalPosition>,
    sol: String,
    terra: String,
    luna: String,
    sol_flat: String,
    sol_rising: String,
    terra_flat: String,
    terra_rising: String,
    luna_flat: String,
    luna_rising: String,
    dollar_labels: [String; 5],
    current_remaining_label: String,
    current_sol_label: String,
    current_terra_label: String,
    current_luna_label: String,
    current_remaining_point_y: f32,
    current_sol_point_y: f32,
    current_terra_point_y: f32,
    current_luna_point_y: f32,
    current_remaining_y: f32,
    current_sol_y: f32,
    current_terra_y: f32,
    current_luna_y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RemainingMarkerPosition {
    x: f64,
    y: f64,
    boundary: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct UnusedIntervalPosition {
    start: f64,
    width: f64,
    preserve_boundary: bool,
}

fn graph_paths(samples: &[&UsageHistorySample], period_start: i64, period_end: i64) -> GraphPaths {
    let remaining_points = graph_points(samples, period_start, period_end, 100.0, |sample| {
        sample.remaining_percent
    });
    let raw_minute = graph_time_endpoints(
        minute_model_spend_for_metric(samples, false),
        period_start,
        period_end,
    );
    let minute = smooth_model_spend(&raw_minute);
    // Dollar series are independent cumulative values.  The old stacked
    // implementation used the sum here, which made a model's line depend on
    // whether another model was enabled and could make a flat SOL history
    // appear to move.  A shared axis still gives the three lines a meaningful
    // comparison, but its ceiling is the largest individual model value.
    let dollar_max = minute
        .iter()
        .map(|point| point.sol.max(point.terra).max(point.luna))
        .fold(0.0_f64, f64::max);
    let has_model_data = dollar_max > 0.0;
    let latest = minute.last().copied().unwrap_or_default();
    let has_remaining_observation = samples
        .iter()
        .any(|sample| sample.remaining_percent.is_finite() && sample.remaining_percent >= 0.0);
    // Use the same smoothed endpoint that is rendered by `remaining-path` so
    // the right-edge percentage cannot disagree with the visible line after
    // a non-monotonic reread is clamped.
    let remaining = has_remaining_observation
        .then(|| remaining_points.last().map(|(_, value)| *value))
        .flatten();
    let graph_y = |value: f64, maximum: f64| -> f32 {
        if maximum > 0.0 {
            ((99.0 - value / maximum * 98.0) / 100.0).clamp(0.01, 0.99) as f32
        } else {
            0.99
        }
    };
    // Detect idle bands from raw cumulative snapshots, not smoothed lines.
    let unused_intervals = unused_interval_positions(&raw_minute, period_start, period_end);
    GraphPaths {
        remaining: graph_path_from_points(&remaining_points, period_start, period_end, 100.0),
        remaining_markers: remaining_marker_positions_on_points(
            &remaining_points,
            period_start,
            period_end,
        ),
        unused_intervals,
        luna: metric_line_path(&minute, period_start, period_end, dollar_max, |point| {
            point.luna
        }),
        terra: metric_line_path(&minute, period_start, period_end, dollar_max, |point| {
            point.terra
        }),
        sol: metric_line_path(&minute, period_start, period_end, dollar_max, |point| {
            point.sol
        }),
        dollar_labels: dollar_axis_labels(dollar_max),
        current_remaining_label: remaining.map(format_percent).unwrap_or_else(|| "—".into()),
        current_sol_label: if has_model_data {
            format!("${:.2}", latest.sol)
        } else {
            String::new()
        },
        current_terra_label: if has_model_data {
            format!("${:.2}", latest.terra)
        } else {
            String::new()
        },
        current_luna_label: if has_model_data {
            format!("${:.2}", latest.luna)
        } else {
            String::new()
        },
        current_remaining_y: graph_y(remaining.unwrap_or(0.0), 100.0),
        current_sol_y: graph_y(latest.sol, dollar_max),
        current_terra_y: graph_y(latest.terra, dollar_max),
        current_luna_y: graph_y(latest.luna, dollar_max),
        current_remaining_point_y: graph_y(remaining.unwrap_or(0.0), 100.0),
        current_sol_point_y: graph_y(latest.sol, dollar_max),
        current_terra_point_y: graph_y(latest.terra, dollar_max),
        current_luna_point_y: graph_y(latest.luna, dollar_max),
        ..GraphPaths::default()
    }
}

/// Builds a view from the monotonic cumulative snapshots. Flat and increasing
/// segments are kept in separate paths so the UI can render distinct widths.
fn graph_paths_for_selection(
    samples: &[&UsageHistorySample],
    period_start: i64,
    period_end: i64,
    show_luna: bool,
    show_terra: bool,
    show_sol: bool,
    show_tokens: bool,
) -> GraphPaths {
    let mut paths = graph_paths(samples, period_start, period_end);
    let minute = graph_time_endpoints(
        minute_model_spend_for_metric(samples, show_tokens),
        period_start,
        period_end,
    );
    paths.unused_intervals = unused_interval_positions(&minute, period_start, period_end);
    let maximum = minute
        .iter()
        .map(|point| {
            if show_tokens {
                point.sol.max(point.terra).max(point.luna)
            } else {
                [
                    show_luna.then_some(point.luna),
                    show_terra.then_some(point.terra),
                    show_sol.then_some(point.sol),
                ]
                .into_iter()
                .flatten()
                .fold(0.0_f64, f64::max)
            }
        })
        .fold(0.0_f64, f64::max);
    let scale_maximum = maximum.max(1.0);
    let latest = minute.last().copied().unwrap_or_default();
    paths.dollar_labels = if show_tokens {
        token_axis_labels(scale_maximum)
    } else {
        dollar_axis_labels(scale_maximum)
    };
    paths.sol.clear();
    paths.terra.clear();
    paths.luna.clear();
    paths.sol_flat.clear();
    paths.sol_rising.clear();
    paths.terra_flat.clear();
    paths.terra_rising.clear();
    paths.luna_flat.clear();
    paths.luna_rising.clear();
    paths.current_sol_label.clear();
    paths.current_terra_label.clear();
    paths.current_luna_label.clear();
    paths.current_sol_point_y = 0.99;
    paths.current_terra_point_y = 0.99;
    paths.current_luna_point_y = 0.99;
    paths.current_sol_y = 0.99;
    paths.current_terra_y = 0.99;
    paths.current_luna_y = 0.99;
    let graph_y =
        |value: f64| ((99.0 - value / scale_maximum * 98.0) / 100.0).clamp(0.01, 0.99) as f32;
    if show_luna {
        (paths.luna_flat, paths.luna_rising) =
            split_metric_line_paths(&minute, period_start, period_end, scale_maximum, |point| {
                point.luna
            });
        paths.luna = metric_line_path(&minute, period_start, period_end, scale_maximum, |point| {
            point.luna
        });
        paths.current_luna_label = if maximum > 0.0 {
            format_metric_value(latest.luna, show_tokens)
        } else {
            String::new()
        };
        paths.current_luna_y = graph_y(latest.luna);
        paths.current_luna_point_y = paths.current_luna_y;
    }
    if show_terra {
        (paths.terra_flat, paths.terra_rising) =
            split_metric_line_paths(&minute, period_start, period_end, scale_maximum, |point| {
                point.terra
            });
        paths.terra = metric_line_path(&minute, period_start, period_end, scale_maximum, |point| {
            point.terra
        });
        paths.current_terra_label = if maximum > 0.0 {
            format_metric_value(latest.terra, show_tokens)
        } else {
            String::new()
        };
        paths.current_terra_y = graph_y(latest.terra);
        paths.current_terra_point_y = paths.current_terra_y;
    }
    if show_sol {
        (paths.sol_flat, paths.sol_rising) =
            split_metric_line_paths(&minute, period_start, period_end, scale_maximum, |point| {
                point.sol
            });
        paths.sol = metric_line_path(&minute, period_start, period_end, scale_maximum, |point| {
            point.sol
        });
        paths.current_sol_label = if maximum > 0.0 {
            format_metric_value(latest.sol, show_tokens)
        } else {
            String::new()
        };
        paths.current_sol_y = graph_y(latest.sol);
        paths.current_sol_point_y = paths.current_sol_y;
    }
    paths
}

#[cfg(test)]
fn graph_paths_for_model(
    samples: &[&UsageHistorySample],
    period_start: i64,
    period_end: i64,
    model: &str,
) -> GraphPaths {
    graph_paths_for_selection(
        samples,
        period_start,
        period_end,
        model == "ALL" || model == "LUNA",
        model == "ALL" || model == "TERRA",
        model == "ALL" || model == "SOL",
        false,
    )
}

#[cfg(test)]
fn remaining_marker_positions(
    samples: &[&UsageHistorySample],
    period_start: i64,
    period_end: i64,
) -> Vec<RemainingMarkerPosition> {
    let points = graph_points(samples, period_start, period_end, 100.0, |sample| {
        sample.remaining_percent
    });
    remaining_marker_positions_on_points(&points, period_start, period_end)
}

fn remaining_marker_positions_on_points(
    points: &[(i64, f64)],
    period_start: i64,
    period_end: i64,
) -> Vec<RemainingMarkerPosition> {
    let span = (period_end - period_start).max(1) as f64;
    let mut markers = Vec::new();
    let mut seen_boundaries = BTreeSet::new();
    let Some(&(mut previous_timestamp, mut previous_value)) = points.first() else {
        return markers;
    };

    // Pathと同じ平滑化済み点列を走査し、各整数%境界をその線分上で補間する。
    for &(timestamp, current) in points.iter().skip(1) {
        if timestamp > previous_timestamp && current < previous_value {
            let mut boundary = previous_value.floor() as i32;
            if (previous_value - boundary as f64).abs() <= f64::EPSILON {
                boundary -= 1;
            }
            let lowest_boundary = current.ceil() as i32;
            while boundary >= lowest_boundary {
                let boundary_value = boundary as f64;
                if boundary_value < previous_value
                    && boundary_value >= current
                    && seen_boundaries.insert(boundary)
                {
                    let fraction = ((boundary_value - previous_value) / (current - previous_value))
                        .clamp(0.0, 1.0);
                    let marker_timestamp = previous_timestamp as f64
                        + (timestamp - previous_timestamp) as f64 * fraction;
                    let x =
                        ((marker_timestamp - period_start as f64) / span * 100.0).clamp(0.0, 100.0);
                    markers.push(RemainingMarkerPosition {
                        x,
                        y: remaining_graph_y(boundary_value),
                        boundary,
                    });
                }
                boundary -= 1;
            }
        }
        previous_timestamp = timestamp;
        previous_value = current;
    }

    markers
}

fn graph_time_endpoints(
    points: Vec<HourlyModelSpend>,
    period_start: i64,
    period_end: i64,
) -> Vec<HourlyModelSpend> {
    if points.is_empty() {
        return points;
    }
    let mut extended = Vec::with_capacity(points.len() + 2);
    // セッション開始から最初の記録までは累積0を明示する。
    extended.push(HourlyModelSpend {
        timestamp: period_start,
        ..HourlyModelSpend::default()
    });
    for point in points
        .iter()
        .copied()
        .filter(|point| point.timestamp >= period_start && point.timestamp < period_end)
    {
        if let Some(last) = extended.last_mut() {
            if last.timestamp == point.timestamp {
                *last = point;
                continue;
            }
        }
        extended.push(point);
    }
    // 時間バケットの途中で終わらず、現在時刻を右端に固定する。
    if let Some(last) = points.last().copied() {
        let endpoint = HourlyModelSpend {
            timestamp: period_end,
            ..last
        };
        if let Some(existing) = extended.last_mut() {
            if existing.timestamp == period_end {
                *existing = endpoint;
            } else {
                extended.push(endpoint);
            }
        } else {
            extended.push(endpoint);
        }
    }
    extended
}

#[derive(Clone, Copy, Debug, Default)]
struct HourlyModelSpend {
    timestamp: i64,
    sol: f64,
    terra: f64,
    luna: f64,
}

#[cfg(test)]
fn minute_model_spend(samples: &[&UsageHistorySample]) -> Vec<HourlyModelSpend> {
    minute_model_spend_for_metric(samples, false)
}

fn minute_model_spend_for_metric(
    samples: &[&UsageHistorySample],
    show_tokens: bool,
) -> Vec<HourlyModelSpend> {
    let mut buckets: Vec<UsageHistorySample> = Vec::new();
    for sample in samples {
        let minute = sample.timestamp.div_euclid(60) * 60;
        let mut sample = (*sample).clone();
        sample.timestamp = minute;
        if let Some(previous) = buckets.last_mut() {
            if previous.timestamp == minute {
                *previous = sample;
                continue;
            }
        }
        buckets.push(sample);
    }
    if show_tokens {
        // The raw session counters are cumulative totals.  Older history
        // rows can contain zero because the token fields did not exist (or a
        // provider did not report them), so a zero after a known value must be
        // treated as an unknown sample and carried forward.  Taking the
        // maximum also protects the graph from stale/out-of-order rows.
        let mut cumulative = [0.0_f64; 3];
        return buckets
            .into_iter()
            .map(|sample| {
                let current = [
                    sample.sol_tokens as f64,
                    sample.terra_tokens as f64,
                    sample.luna_tokens as f64,
                ];
                for index in 0..3 {
                    if current[index] > cumulative[index] {
                        cumulative[index] = current[index];
                    }
                }
                HourlyModelSpend {
                    timestamp: sample.timestamp,
                    sol: cumulative[0],
                    terra: cumulative[1],
                    luna: cumulative[2],
                }
            })
            .collect();
    }

    let mut cumulative = [0.0_f64; 3];
    buckets
        .into_iter()
        .map(|sample| {
            let current = [
                sample.sol_dollars,
                sample.terra_dollars,
                sample.luna_dollars,
            ];
            for index in 0..3 {
                // Dollar history is also persisted as a cumulative snapshot.
                // A later API scan can temporarily report a smaller snapshot
                // (for example while a session file is still being indexed),
                // so never add the positive difference twice after such a
                // regression. Keep the greatest observed cumulative value,
                // just as the token path does above.
                if current[index] > cumulative[index] {
                    cumulative[index] = current[index];
                }
            }
            HourlyModelSpend {
                timestamp: sample.timestamp,
                sol: cumulative[0],
                terra: cumulative[1],
                luna: cumulative[2],
            }
        })
        .collect()
}

#[cfg(test)]
fn stacked_area_path(
    points: &[HourlyModelSpend],
    period_start: i64,
    period_end: i64,
    maximum: f64,
    bounds: impl Fn(&HourlyModelSpend) -> (f64, f64),
) -> String {
    if points.is_empty() {
        return String::new();
    }
    // すべて$0の期間も、下端に0基線を描いて「データなし」と区別する。
    if maximum <= 0.0 {
        return "M0.00 99.00 L100.00 99.00".into();
    }
    let span = (period_end - period_start).max(1) as f64;
    let coordinate = |timestamp: i64, value: f64| {
        let x = ((timestamp - period_start) as f64 / span * 100.0).clamp(0.0, 100.0);
        // strokeがclip領域の外へ半分切れないよう、0/最大値を内側へ1%だけ寄せる。
        let y = (99.0 - value / maximum * 98.0).clamp(1.0, 99.0);
        (x, y)
    };
    let upper = points
        .iter()
        .map(|point| coordinate(point.timestamp, bounds(point).1))
        .collect::<Vec<_>>();
    let lower = points
        .iter()
        .rev()
        .map(|point| coordinate(point.timestamp, bounds(point).0))
        .collect::<Vec<_>>();
    let mut commands = format!("M{:.2} {:.2}", upper[0].0, upper[0].1);
    for (x, y) in upper.iter().skip(1) {
        commands.push_str(&format!(" L{x:.2} {y:.2}"));
    }
    for (x, y) in lower {
        commands.push_str(&format!(" L{x:.2} {y:.2}"));
    }
    commands.push_str(" Z");
    commands
}

/// Draws one metric independently from the other model series. Token mode
/// uses this path so enabling LUNA cannot turn SOL into a LUNA+SOL boundary.
fn metric_line_path(
    points: &[HourlyModelSpend],
    period_start: i64,
    period_end: i64,
    maximum: f64,
    value: impl Fn(&HourlyModelSpend) -> f64,
) -> String {
    if points.is_empty() {
        return String::new();
    }
    if maximum <= 0.0 {
        return "M0.00 99.00 L100.00 99.00".into();
    }
    let span = (period_end - period_start).max(1) as f64;
    let coordinate = |timestamp: i64, raw: f64| {
        let x = ((timestamp - period_start) as f64 / span * 100.0).clamp(0.0, 100.0);
        let y = (99.0 - raw.max(0.0) / maximum * 98.0).clamp(1.0, 99.0);
        (x, y)
    };
    let mut iter = points.iter();
    let first = iter.next().expect("points is not empty");
    let (x, y) = coordinate(first.timestamp, value(first));
    let mut commands = format!("M{x:.2} {y:.2}");
    let mut previous = first;
    for point in iter {
        let (x, y) = coordinate(point.timestamp, value(point));
        // The reset anchor is synthetic when the first real measurement is
        // later than the period start.  Keep that unobserved interval flat at
        // zero, then make the first observed cumulative value explicit.  A
        // diagonal from the anchor would falsely imply spend before the first
        // record existed.
        if previous.timestamp == period_start
            && point.timestamp - previous.timestamp > 60
            && value(previous) <= 0.0
            && value(point) > 0.0
        {
            let (_, previous_y) = coordinate(point.timestamp, value(previous));
            commands.push_str(&format!(" L{x:.2} {previous_y:.2}"));
        }
        commands.push_str(&format!(" L{x:.2} {y:.2}"));
        previous = point;
    }
    commands
}

fn split_metric_line_paths(
    points: &[HourlyModelSpend],
    period_start: i64,
    period_end: i64,
    maximum: f64,
    value: impl Fn(&HourlyModelSpend) -> f64,
) -> (String, String) {
    let span = (period_end - period_start).max(1) as f64;
    let scale = maximum.max(1.0);
    let coordinate = |point: &HourlyModelSpend| {
        let x = ((point.timestamp - period_start) as f64 / span * 100.0).clamp(0.0, 100.0);
        let y = (99.0 - value(point).max(0.0) / scale * 98.0).clamp(1.0, 99.0);
        (x, y)
    };
    let mut flat = String::new();
    let mut rising = String::new();
    for pair in points.windows(2) {
        let previous = value(&pair[0]);
        let current = value(&pair[1]);
        if !previous.is_finite() || !current.is_finite() || current < previous {
            continue;
        }
        let (x1, y1) = coordinate(&pair[0]);
        let (x2, y2) = coordinate(&pair[1]);
        // The reset anchor is synthetic when the first observation arrives
        // later. Keep the unknown interval at zero and show the observed
        // increase at its actual timestamp instead of implying a diagonal
        // increase throughout the unobserved interval.
        if pair[0].timestamp == period_start
            && pair[1].timestamp.saturating_sub(pair[0].timestamp) > 60
            && previous == 0.0
            && current > 0.0
        {
            if !flat.is_empty() {
                flat.push(' ');
            }
            flat.push_str(&format!("M{x1:.2} {y1:.2} L{x2:.2} {y1:.2}"));
            if !rising.is_empty() {
                rising.push(' ');
            }
            rising.push_str(&format!("M{x2:.2} {y1:.2} L{x2:.2} {y2:.2}"));
            continue;
        }
        let target = if current == previous {
            &mut flat
        } else {
            &mut rising
        };
        if !target.is_empty() {
            target.push(' ');
        }
        target.push_str(&format!("M{x1:.2} {y1:.2} L{x2:.2} {y2:.2}"));
    }
    (flat, rising)
}

/// Return horizontal bands where none of the three cumulative model series
/// changes. These bands make idle time visible even when all flat paths sit on
/// top of one another at the chart baseline.
fn unused_interval_positions(
    points: &[HourlyModelSpend],
    period_start: i64,
    period_end: i64,
) -> Vec<UnusedIntervalPosition> {
    let span = (period_end - period_start).max(1) as f64;
    let to_x =
        |timestamp: i64| ((timestamp - period_start) as f64 / span * 100.0).clamp(0.0, 100.0);
    let mut intervals: Vec<UnusedIntervalPosition> = Vec::new();
    for pair in points.windows(2) {
        let [previous, current] = pair else {
            continue;
        };
        if current.timestamp <= previous.timestamp {
            continue;
        }
        let interval_start = previous.timestamp.max(period_start);
        let interval_end = current.timestamp.min(period_end);
        if interval_end <= interval_start {
            continue;
        }
        let unchanged = [
            (previous.sol, current.sol),
            (previous.terra, current.terra),
            (previous.luna, current.luna),
        ]
        .into_iter()
        .all(|(before, after)| before.is_finite() && after.is_finite() && before == after);
        let synthetic_zero_gap = previous.timestamp == period_start
            && current.timestamp.saturating_sub(previous.timestamp) > 60
            && previous.sol == 0.0
            && previous.terra == 0.0
            && previous.luna == 0.0
            && [current.sol, current.terra, current.luna]
                .into_iter()
                .any(|value| value.is_finite() && value > 0.0);
        if !unchanged && !synthetic_zero_gap {
            continue;
        }
        let start = to_x(interval_start);
        let end = to_x(interval_end);
        if end <= start {
            continue;
        }
        if let Some(last) = intervals.last_mut() {
            let last_end = last.start + last.width;
            if !last.preserve_boundary
                && !synthetic_zero_gap
                && (last_end - start).abs() <= f64::EPSILON
            {
                last.width = end - last.start;
                continue;
            }
        }
        intervals.push(UnusedIntervalPosition {
            start,
            width: end - start,
            preserve_boundary: synthetic_zero_gap,
        });
    }
    intervals
}

/// Keeps all visible right-edge labels inside the plot and at least 16px
/// apart at the minimum 204px path height. GraphWindow's 700x480 minimum
/// produces that path height; resizing only increases the physical spacing.
fn separate_current_label_positions(
    paths: &mut GraphPaths,
    show_remaining: bool,
    show_luna: bool,
    show_terra: bool,
    show_sol: bool,
) {
    const MIN_PATH_HEIGHT: f32 = 204.0;
    const HALF_LABEL: f32 = 8.0 / MIN_PATH_HEIGHT;
    const MIN_SEPARATION: f32 = 16.0 / MIN_PATH_HEIGHT;
    const LOWER: f32 = HALF_LABEL;
    const UPPER: f32 = 1.0 - HALF_LABEL;

    let mut labels = Vec::with_capacity(4);
    if show_remaining && !paths.current_remaining_label.is_empty() {
        labels.push((0_u8, paths.current_remaining_y));
    }
    if show_luna && !paths.current_luna_label.is_empty() {
        labels.push((1, paths.current_luna_y));
    }
    if show_terra && !paths.current_terra_label.is_empty() {
        labels.push((2, paths.current_terra_y));
    }
    if show_sol && !paths.current_sol_label.is_empty() {
        labels.push((3, paths.current_sol_y));
    }
    labels.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    if labels.is_empty() {
        return;
    }

    labels[0].1 = labels[0].1.clamp(LOWER, UPPER);
    for index in 1..labels.len() {
        labels[index].1 = labels[index]
            .1
            .clamp(LOWER, UPPER)
            .max(labels[index - 1].1 + MIN_SEPARATION);
    }
    if labels.last().is_some_and(|label| label.1 > UPPER) {
        let last = labels.len() - 1;
        labels[last].1 = UPPER;
        for index in (0..last).rev() {
            labels[index].1 = labels[index].1.min(labels[index + 1].1 - MIN_SEPARATION);
        }
    }

    for (kind, position) in labels {
        match kind {
            0 => paths.current_remaining_y = position,
            1 => paths.current_luna_y = position,
            2 => paths.current_terra_y = position,
            3 => paths.current_sol_y = position,
            _ => unreachable!("label kind is internal and bounded"),
        }
    }
}

/// Draw a short, color-matched leader from the series endpoint to its
/// right-edge label. Labels may be vertically separated to avoid overlap, so
/// the connector preserves the correspondence without stacking text on top of
/// another value.
fn current_label_connector_path(point_y: f32, label_y: f32, has_label: bool) -> String {
    if !has_label || !point_y.is_finite() || !label_y.is_finite() {
        return String::new();
    }
    let point_y = point_y.clamp(0.0, 1.0) * 100.0;
    let label_y = label_y.clamp(0.0, 1.0) * 100.0;
    format!("M0.00 {point_y:.2} L100.00 {label_y:.2}")
}

fn dollar_axis_labels(maximum: f64) -> [String; 5] {
    [1.0, 0.75, 0.5, 0.25, 0.0].map(|fraction| format!("${:.2}", maximum * fraction))
}

fn token_axis_labels(maximum: f64) -> [String; 5] {
    [1.0, 0.75, 0.5, 0.25, 0.0].map(|fraction| format_token_axis_value(maximum * fraction))
}

fn format_token_axis_value(value: f64) -> String {
    let value = value.max(0.0);
    if value >= 1_000_000_000.0 {
        format!("{:.1}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else {
        format_token_count(value.round() as u64)
    }
}

fn format_metric_value(value: f64, show_tokens: bool) -> String {
    if show_tokens {
        format_token_count(value.max(0.0).round() as u64)
    } else {
        format!("${value:.2}")
    }
}

fn graph_points(
    samples: &[&UsageHistorySample],
    period_start: i64,
    period_end: i64,
    maximum: f64,
    value: impl Fn(&UsageHistorySample) -> f64,
) -> Vec<(i64, f64)> {
    let mut observed = samples
        .iter()
        .filter_map(|sample| {
            let raw = value(sample);
            (raw.is_finite() && raw >= 0.0).then_some((sample.timestamp, raw))
        })
        .collect::<Vec<_>>();
    observed.sort_by_key(|(timestamp, _)| *timestamp);
    let has_observation = !observed.is_empty();

    // リセット開始時点は仕様上、残り利用枠100%である。最初の実測値が
    // 取得された時刻より後でも、グラフの左端を欠落させない。
    let mut points = vec![(period_start, maximum)];
    for (timestamp, raw) in observed {
        let timestamp = timestamp.clamp(period_start, period_end);
        if points
            .last()
            .is_some_and(|(last_timestamp, _)| *last_timestamp == timestamp)
        {
            points.last_mut().expect("points has an anchor").1 = raw;
            continue;
        }
        points.push((timestamp, raw));
    }

    if has_observation {
        if let Some((last_timestamp, last_raw)) = points.last().copied() {
            if last_timestamp < period_end {
                // 最新の実測値は現在時刻まで水平に保持する。未知の値を
                // 現在時刻へ斜めに補間しない。
                points.push((period_end, last_raw));
            }
        }
    }
    // Keep the observed change events as the anchors for the quota trend.
    // Repeated snapshots describe a hold, not another point on the visible
    // trend; retaining every one of them makes the line look like a staircase
    // when the provider reports the same percentage for several minutes.
    // Collapse those runs before smoothing so each segment connects one
    // change point to the next.
    smooth_remaining_points(&collapse_remaining_change_points(&points))
}

fn collapse_remaining_change_points(points: &[(i64, f64)]) -> Vec<(i64, f64)> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let mut collapsed = Vec::with_capacity(points.len());
    collapsed.push(points[0]);
    for (index, &(timestamp, raw)) in points.iter().enumerate().skip(1) {
        let previous = collapsed.last().expect("first point is present").1;
        // Remaining quota is monotonic between resets. Clamp a transient
        // upward reread before deciding whether this is a visible change.
        let value = raw.min(previous);
        let is_period_end = index + 1 == points.len();
        if value < previous || is_period_end {
            collapsed.push((timestamp, value));
        }
    }
    collapsed
}

fn graph_path_from_points(
    points: &[(i64, f64)],
    period_start: i64,
    period_end: i64,
    maximum: f64,
) -> String {
    let span = (period_end - period_start).max(1) as f64;
    points
        .iter()
        .enumerate()
        .map(|(index, (timestamp, raw))| {
            let x = ((timestamp - period_start) as f64 / span * 100.0).clamp(0.0, 100.0);
            let y = if maximum > 0.0 {
                (99.0 - raw / maximum * 98.0).clamp(1.0, 99.0)
            } else {
                99.0
            };
            let command = if index == 0 { "M" } else { "L" };
            format!("{command}{x:.2} {y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn remaining_graph_y(remaining: f64) -> f64 {
    (99.0 - remaining * 0.98).clamp(1.0, 99.0)
}

fn smooth_remaining_points(points: &[(i64, f64)]) -> Vec<(i64, f64)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut smoothed = Vec::with_capacity(points.len());
    smoothed.push(points[0]);
    for index in 1..points.len() - 1 {
        let average = (points[index - 1].1 + 2.0 * points[index].1 + points[index + 1].1) / 4.0;
        // 利用枠はリセットまで増えないため、計測ノイズによる逆戻りを除く。
        let value = average.min(smoothed.last().expect("anchor exists").1);
        smoothed.push((points[index].0, value));
    }
    let last = points.last().expect("points has at least three items");
    smoothed.push((
        last.0,
        last.1.min(smoothed.last().expect("anchor exists").1),
    ));
    smoothed
}

fn smooth_model_spend(points: &[HourlyModelSpend]) -> Vec<HourlyModelSpend> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut smoothed = Vec::with_capacity(points.len());
    smoothed.push(points[0]);
    for index in 1..points.len() - 1 {
        let previous = *smoothed.last().expect("zero anchor exists");
        let current = points[index];
        let next = points[index + 1];
        let smooth = |before: f64, value: f64, after: f64, floor: f64| {
            ((before + 2.0 * value + after) / 4.0).max(floor)
        };
        smoothed.push(HourlyModelSpend {
            timestamp: current.timestamp,
            sol: smooth(previous.sol, current.sol, next.sol, previous.sol),
            terra: smooth(previous.terra, current.terra, next.terra, previous.terra),
            luna: smooth(previous.luna, current.luna, next.luna, previous.luna),
        });
    }
    let last = *points.last().expect("points has at least three items");
    let previous = *smoothed.last().expect("anchor exists");
    smoothed.push(HourlyModelSpend {
        timestamp: last.timestamp,
        sol: last.sol.max(previous.sol),
        terra: last.terra.max(previous.terra),
        luna: last.luna.max(previous.luna),
    });
    smoothed
}

fn local_sessions_root() -> Option<PathBuf> {
    codex_home_root().map(|root| root.join("sessions"))
}

fn codex_home_root() -> Option<PathBuf> {
    let path = std::env::var_os("CODEX_HOME")
        .filter(|root| !root.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))?;
    validated_configured_root(path)
}

fn delegation_usage_recovery_path() -> Option<PathBuf> {
    codex_home_root().map(|root| root.join("history").join("delegation_usage_recovery.jsonl"))
}

#[derive(Default)]
struct SessionTraversalBudget {
    files: usize,
    total_bytes: u64,
}

impl SessionTraversalBudget {
    fn admit_file(
        &mut self,
        relative_depth: usize,
        bytes: u64,
    ) -> Result<(), security::SecurityError> {
        if relative_depth > security::MAX_SESSION_DEPTH || bytes > security::MAX_SESSION_FILE_BYTES
        {
            return Err(security::SecurityError::new(
                security::SecurityErrorKind::LimitExceeded,
            ));
        }
        let files = self.files.checked_add(1).ok_or_else(|| {
            security::SecurityError::new(security::SecurityErrorKind::LimitExceeded)
        })?;
        let total_bytes = self.total_bytes.checked_add(bytes).ok_or_else(|| {
            security::SecurityError::new(security::SecurityErrorKind::LimitExceeded)
        })?;
        if files > security::MAX_SESSION_FILES || total_bytes > security::MAX_SESSION_TOTAL_BYTES {
            return Err(security::SecurityError::new(
                security::SecurityErrorKind::LimitExceeded,
            ));
        }
        self.files = files;
        self.total_bytes = total_bytes;
        Ok(())
    }
}

fn session_jsonl_files(root: &Path) -> Result<Vec<PathBuf>, security::SecurityError> {
    fn visit(
        directory: &Path,
        depth: usize,
        budget: &mut SessionTraversalBudget,
        files: &mut Vec<PathBuf>,
    ) -> Result<(), security::SecurityError> {
        if depth > security::MAX_SESSION_DEPTH {
            return Err(security::SecurityError::new(
                security::SecurityErrorKind::LimitExceeded,
            ));
        }
        let entries = fs::read_dir(directory)
            .map_err(|_| security::SecurityError::new(security::SecurityErrorKind::UnsafePath))?;
        for entry in entries {
            let entry = entry.map_err(|_| {
                security::SecurityError::new(security::SecurityErrorKind::UnsafePath)
            })?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|_| {
                security::SecurityError::new(security::SecurityErrorKind::UnsafePath)
            })?;
            if metadata.file_type().is_symlink() {
                return Err(security::SecurityError::new(
                    security::SecurityErrorKind::UnsafePath,
                ));
            }
            if metadata.is_dir() {
                visit(&path, depth + 1, budget, files)?;
                continue;
            }
            if !metadata.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("jsonl")
            {
                continue;
            }
            budget.admit_file(depth + 1, metadata.len())?;
            files.push(path);
        }
        Ok(())
    }

    let root = security::validate_absolute_root(root)?;
    let mut files = Vec::new();
    visit(&root, 0, &mut SessionTraversalBudget::default(), &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_local_model_usage(
    reset_at: i64,
    window_seconds: i64,
) -> Result<ModelUsageTotals, security::SecurityError> {
    if reset_at <= 0 {
        return Ok(ModelUsageTotals::default());
    }
    let mut totals = ModelUsageTotals::default();
    let window_start = reset_at.saturating_sub(window_seconds.max(0));
    if let Some(root) = local_sessions_root() {
        for path in session_jsonl_files(&root)? {
            collect_session_file(&path, &mut totals, window_start)?;
        }
    }
    let window_end = reset_at;
    add_recovery_usage(
        delegation_usage_recovery_path().as_deref(),
        window_start,
        window_end,
        &mut totals,
    );
    Ok(totals)
}

#[derive(Debug, Deserialize)]
struct DelegationUsageRecoveryEntry {
    timestamp: i64,
    thread_id: String,
    model: String,
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
    reasoning_tokens: u64,
}

impl DelegationUsageRecoveryEntry {
    fn snapshot(&self) -> TokenSnapshot {
        // Recovery records contain reasoning tokens for auditing, but the
        // usage display's total is intentionally input plus output only.
        let _ = self.reasoning_tokens;
        TokenSnapshot {
            total: self.input_tokens.saturating_add(self.output_tokens),
            input: self.input_tokens,
            cached_input: self.cached_input_tokens,
            output: self.output_tokens,
        }
    }
}

fn read_recovery_entries(
    path: &Path,
    window_start: i64,
    window_end: i64,
) -> Vec<DelegationUsageRecoveryEntry> {
    if window_start > window_end {
        return Vec::new();
    }
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Vec::new();
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > security::MAX_SESSION_FILE_BYTES
    {
        return Vec::new();
    }
    let Ok(file) = File::open(path) else {
        return Vec::new();
    };
    let mut seen_threads = BTreeSet::new();
    let mut entries = Vec::new();
    let mut reader = BufReader::new(file);
    loop {
        let line = match security::read_bounded_jsonl_line(&mut reader) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(_) => return Vec::new(),
        };
        let Ok(entry) = serde_json::from_str::<DelegationUsageRecoveryEntry>(&line) else {
            continue;
        };
        if entry.timestamp < window_start
            || entry.timestamp > window_end
            || entry.timestamp <= 0
            || entry.thread_id.trim().is_empty()
            || ModelUsageTotals::recognized_model(&entry.model).is_none()
        {
            continue;
        }
        if seen_threads.insert(entry.thread_id.clone()) {
            entries.push(entry);
        }
    }
    entries
}

fn add_recovery_usage(
    path: Option<&Path>,
    window_start: i64,
    window_end: i64,
    totals: &mut ModelUsageTotals,
) {
    let Some(path) = path else {
        return;
    };
    for entry in read_recovery_entries(path, window_start, window_end) {
        totals.add(&entry.model, entry.snapshot());
    }
}

struct TimedModelUsage {
    timestamp: i64,
    model: String,
    delta: TokenSnapshot,
}

fn session_event_type(value: &Value) -> Option<&str> {
    let outer_type = value.get("type").and_then(Value::as_str);
    match outer_type {
        Some("token_count" | "turn_context" | "thread_context" | "thread_settings_applied") => {
            outer_type
        }
        _ => value
            .get("payload")
            .and_then(Value::as_object)
            .and_then(|payload| payload.get("type"))
            .and_then(Value::as_str),
    }
}

fn session_event_model(value: &Value) -> Option<String> {
    let payload = value.get("payload").and_then(Value::as_object);
    let root_model = value.get("model").and_then(Value::as_str);
    let model = match session_event_type(value) {
        Some("turn_context" | "thread_context") => payload
            .and_then(|payload| payload.get("model").and_then(Value::as_str))
            .or(root_model),
        Some("thread_settings_applied") => payload
            .and_then(|payload| {
                payload
                    .get("thread_settings")
                    .and_then(Value::as_object)
                    .and_then(|settings| settings.get("model"))
                    .and_then(Value::as_str)
                    .or_else(|| payload.get("model").and_then(Value::as_str))
            })
            .or(root_model),
        _ => None,
    }?;
    (!model.trim().is_empty()).then(|| model.to_owned())
}

fn session_token_snapshot(value: &Value) -> Option<TokenSnapshot> {
    if session_event_type(value) != Some("token_count") {
        return None;
    }
    let payload = value.get("payload").and_then(Value::as_object)?;
    let total_usage = payload
        .get("info")
        .and_then(|info| info.get("total_token_usage"))
        .and_then(Value::as_object)?;
    let total = total_usage.get("total_tokens").and_then(Value::as_u64)?;
    Some(TokenSnapshot {
        total,
        input: total_usage
            .get("input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cached_input: total_usage
            .get("cached_input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        output: total_usage
            .get("output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    })
}

fn session_event_timestamp(value: &Value) -> i64 {
    value
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.timestamp())
        .unwrap_or(0)
}

fn recovery_timed_usage(path: &Path, window_start: i64, window_end: i64) -> Vec<TimedModelUsage> {
    read_recovery_entries(path, window_start, window_end)
        .into_iter()
        .map(|entry| {
            let delta = entry.snapshot();
            TimedModelUsage {
                timestamp: entry.timestamp,
                model: entry.model,
                delta,
            }
        })
        .collect()
}

fn model_usage_timeline_from_events(
    mut events: Vec<TimedModelUsage>,
    reset_at: i64,
) -> Vec<UsageHistorySample> {
    events.sort_by_key(|event| event.timestamp);

    let mut totals = ModelUsageTotals::default();
    let mut samples: Vec<UsageHistorySample> = Vec::new();
    for event in events {
        let minute = event.timestamp.div_euclid(60) * 60;
        totals.add(&event.model, event.delta);
        let costs = totals.dollar_totals();
        let sample = UsageHistorySample::from_model_history_with_usage(
            minute,
            reset_at,
            costs,
            totals.token_totals(),
        );
        if let Some(previous) = samples.last_mut() {
            if previous.timestamp == sample.timestamp {
                *previous = sample;
                continue;
            }
        }
        samples.push(sample);
    }
    samples
}

fn collect_local_model_usage_timeline(
    reset_at: i64,
    window_seconds: i64,
) -> Result<Vec<UsageHistorySample>, security::SecurityError> {
    if reset_at <= 0 {
        return Ok(Vec::new());
    }
    let window_start = reset_at.saturating_sub(window_seconds.max(0));
    let now = Utc::now().timestamp().min(reset_at);
    let mut events = Vec::new();
    if let Some(root) = local_sessions_root() {
        for path in session_jsonl_files(&root)? {
            collect_session_timeline_file(&path, window_start, now, &mut events)?;
        }
    }
    if let Some(path) = delegation_usage_recovery_path() {
        events.extend(recovery_timed_usage(&path, window_start, now));
    }
    Ok(model_usage_timeline_from_events(events, reset_at))
}

fn collect_session_timeline_file(
    path: &Path,
    window_start: i64,
    window_end: i64,
    events: &mut Vec<TimedModelUsage>,
) -> Result<(), security::SecurityError> {
    let file = File::open(path)
        .map_err(|_| security::SecurityError::new(security::SecurityErrorKind::UnsafePath))?;
    let initial_len = events.len();
    let mut model: Option<String> = None;
    let mut previous = TokenSnapshot::default();
    let mut reader = BufReader::new(file);
    loop {
        let line = match security::read_bounded_jsonl_line(&mut reader) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) => {
                events.truncate(initial_len);
                return Err(error);
            }
        };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        // `thread_settings_applied` can precede the actual turn context. Keep
        // the previous model until `turn_context` confirms the model for the
        // token-count event; otherwise setup metadata is charged to the next
        // model (ccusage-compatible attribution).
        if session_event_type(&value) != Some("thread_settings_applied") || model.is_none() {
            if let Some(next_model) = session_event_model(&value) {
                model = Some(next_model);
            }
        }
        let Some(current) = session_token_snapshot(&value) else {
            continue;
        };
        let delta = TokenSnapshot {
            total: current.total.saturating_sub(previous.total),
            input: current.input.saturating_sub(previous.input),
            cached_input: current.cached_input.saturating_sub(previous.cached_input),
            output: current.output.saturating_sub(previous.output),
        };
        previous = current;
        let timestamp = session_event_timestamp(&value);
        if timestamp < window_start || timestamp > window_end {
            continue;
        }
        if let Some(model) = model.as_deref() {
            events.push(TimedModelUsage {
                timestamp,
                model: model.to_owned(),
                delta,
            });
        }
    }
    Ok(())
}

fn collect_session_file(
    path: &Path,
    totals: &mut ModelUsageTotals,
    window_start: i64,
) -> Result<(), security::SecurityError> {
    let file = File::open(path)
        .map_err(|_| security::SecurityError::new(security::SecurityErrorKind::UnsafePath))?;
    let original = totals.clone();
    let mut model: Option<String> = None;
    let mut previous = TokenSnapshot::default();
    let mut reader = BufReader::new(file);
    loop {
        let line = match security::read_bounded_jsonl_line(&mut reader) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) => {
                *totals = original;
                return Err(error);
            }
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if session_event_type(&value) != Some("thread_settings_applied") || model.is_none() {
            if let Some(next_model) = session_event_model(&value) {
                model = Some(next_model);
            }
        }
        let Some(current) = session_token_snapshot(&value) else {
            continue;
        };
        let delta = TokenSnapshot {
            total: current.total.saturating_sub(previous.total),
            input: current.input.saturating_sub(previous.input),
            cached_input: current.cached_input.saturating_sub(previous.cached_input),
            output: current.output.saturating_sub(previous.output),
        };
        previous = current;
        let timestamp = value
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.timestamp())
            .unwrap_or(0);
        if timestamp < window_start {
            continue;
        }
        if let Some(model) = model.as_deref() {
            totals.add(model, delta);
        }
    }
    Ok(())
}

fn resolved_executable(override_name: &str, command_name: &str) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(override_name) {
        return security::resolve_executable_path(Path::new(&path)).ok();
    }
    let path = std::env::var_os("PATH")?;
    security::resolve_executable_from_path(command_name, path).ok()
}

enum RpcReadEvent {
    Line(security::RpcLine),
    Closed,
    Failed,
}

fn rpc_reader(stdout: std::process::ChildStdout) -> Receiver<RpcReadEvent> {
    let (tx, rx) = mpsc::sync_channel(16);
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match security::RpcLine::read(&mut reader) {
                Ok(Some(line)) => {
                    if tx.send(RpcReadEvent::Line(line)).is_err() {
                        break;
                    }
                }
                Ok(None) => {
                    let _ = tx.send(RpcReadEvent::Closed);
                    break;
                }
                Err(_) => {
                    let _ = tx.send(RpcReadEvent::Failed);
                    break;
                }
            }
        }
    });
    rx
}

struct AppServerBridge<C, E> {
    tx: Sender<C>,
    rx: Receiver<E>,
}

impl<C, E> AppServerBridge<C, E> {
    fn inactive() -> Self {
        let (tx, _commands) = mpsc::channel();
        let (_events, rx) = mpsc::channel();
        Self { tx, rx }
    }

    fn send(&self, command: C) -> bool {
        self.tx.send(command).is_ok()
    }
}

impl AppServerBridge<AccountCommand, Event> {
    fn start() -> Self {
        let (tx, commands) = mpsc::channel::<AccountCommand>();
        let (events, rx) = mpsc::channel::<Event>();
        thread::spawn(move || account_server_worker(commands, events));
        Self { tx, rx }
    }
}

impl AppServerBridge<ThreadCommand, ThreadEvent> {
    fn start() -> Self {
        let (tx, commands) = mpsc::channel::<ThreadCommand>();
        let (events, rx) = mpsc::channel::<ThreadEvent>();
        thread::spawn(move || thread_server_worker(commands, events));
        Self { tx, rx }
    }
}

struct LocalUsageBridge {
    tx: Sender<LocalCommand>,
    rx: Receiver<LocalEvent>,
}

impl LocalUsageBridge {
    fn start() -> Self {
        let (tx, commands) = mpsc::channel::<LocalCommand>();
        let (events, rx) = mpsc::channel::<LocalEvent>();
        thread::spawn(move || local_usage_worker(commands, events));
        Self { tx, rx }
    }

    fn inactive() -> Self {
        let (tx, _commands) = mpsc::channel();
        let (_events, rx) = mpsc::channel();
        Self { tx, rx }
    }

    fn send(&self, command: LocalCommand) -> bool {
        self.tx.send(command).is_ok()
    }
}

fn account_server_worker(commands: Receiver<AccountCommand>, events: Sender<Event>) {
    let Some(codex) = resolved_executable("CODEX_INFO_CODEX_BIN", "codex") else {
        let _ = events.send(Event::Error(
            "Codex app-serverの安全な実行ファイルを確認できません。".into(),
        ));
        return;
    };
    let child_result = Command::new(codex)
        .args(["app-server", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let child = match child_result {
        Ok(child) => child,
        Err(_) => {
            let _ = events.send(Event::Error(
                "Codex app-serverを起動できませんでした。".into(),
            ));
            return;
        }
    };
    let mut child = security::ChildGuard::new(child);
    let Some(mut input) = child.child_mut().ok().and_then(|child| child.stdin.take()) else {
        let _ = events.send(Event::Error(
            "Codex app-serverの入出力を初期化できませんでした。".into(),
        ));
        return;
    };
    let Some(stdout) = child.child_mut().ok().and_then(|child| child.stdout.take()) else {
        let _ = events.send(Event::Error(
            "Codex app-serverの入出力を初期化できませんでした。".into(),
        ));
        return;
    };
    let output = rpc_reader(stdout);
    if let Err(e) = request(
        &mut input,
        &output,
        1,
        "initialize",
        json!({"clientInfo":{"name":"codex-info","version":"0.3.0"},"capabilities":{"experimentalApi":true}}),
    ) {
        let _ = events.send(Event::Error(e));
        return;
    }
    let _ = events.send(Event::Ready);
    let mut id = 2u64;
    while let Ok(command) = commands.recv() {
        match command {
            AccountCommand::Stop => {
                let _ = child.kill_and_reap();
                break;
            }
            AccountCommand::Login => {
                match request(
                    &mut input,
                    &output,
                    id,
                    "account/login/start",
                    json!({"type":"chatgpt"}),
                ) {
                    Ok(result) => {
                        if let Some(url) = result.get("authUrl").and_then(Value::as_str) {
                            match security::validate_auth_url(url) {
                                Ok(url) => {
                                    let _ = events.send(Event::AuthUrl(url.to_string()));
                                }
                                Err(_) => {
                                    let _ = events.send(Event::Error(
                                        "Codexから安全な認証URLを受け取れませんでした。".into(),
                                    ));
                                }
                            }
                        } else {
                            let _ = events.send(Event::Error(
                                "Codexから認証URLを受け取れませんでした。".into(),
                            ));
                        }
                    }
                    Err(e) => {
                        let _ = events.send(Event::Error(e));
                    }
                }
                id += 1;
            }
            AccountCommand::Read => {
                let account = request(&mut input, &output, id, "account/read", json!({}));
                id += 1;
                match account {
                    Ok(result) => {
                        let (email, authenticated, plan_type) =
                            match protocol_contract::decode_account(&result) {
                                Ok(protocol_contract::AccountOutcome::Supported {
                                    email,
                                    plan_type,
                                }) => (Some(email), true, Some(plan_type.as_str().to_owned())),
                                Ok(protocol_contract::AccountOutcome::AuthRequired)
                                | Ok(protocol_contract::AccountOutcome::UnsupportedNoData) => {
                                    (None, false, None)
                                }
                                Err(_) => {
                                    let _ = events.send(Event::Error(
                                        "アカウントの正本データを取得できませんでした。".into(),
                                    ));
                                    continue;
                                }
                            };
                        let _ = events.send(Event::Account {
                            email: email.clone(),
                            authenticated,
                            plan_type: plan_type.clone(),
                        });
                        if authenticated {
                            let rate_request_id = id;
                            id = id.saturating_add(1);
                            match request(
                                &mut input,
                                &output,
                                rate_request_id,
                                "account/rateLimits/read",
                                Value::Null,
                            ) {
                                Ok(rate) => {
                                    match parse_rate_limits(
                                        &rate,
                                        plan_type.as_deref(),
                                        Utc::now().timestamp(),
                                    ) {
                                        Ok(snapshot) => {
                                            let RateLimitSnapshot {
                                                remaining_percent,
                                                reset_at,
                                                window_seconds,
                                                limit_name,
                                                quota_title,
                                                monthly,
                                            } = snapshot;
                                            let recheck_id = id;
                                            let Some(next_id) = id.checked_add(1) else {
                                                let _ = events.send(Event::Error(
                                                    "Codex APIの要求IDが上限に達しました。".into(),
                                                ));
                                                continue;
                                            };
                                            id = next_id;
                                            let recheck = request(
                                                &mut input,
                                                &output,
                                                recheck_id,
                                                "account/read",
                                                json!({}),
                                            );
                                            let identity_is_current = match recheck {
                                                Ok(result) => {
                                                    match protocol_contract::decode_account(&result)
                                                    {
                                                        Ok(protocol_contract::AccountOutcome::Supported {
                                                            email: current_email,
                                                            plan_type: current_plan,
                                                        }) => {
                                                            let current_plan = current_plan
                                                                .as_str()
                                                                .to_owned();
                                                            if email.as_deref()
                                                                == Some(current_email.as_str())
                                                                && plan_type.as_deref()
                                                                    == Some(current_plan.as_str())
                                                            {
                                                                true
                                                            } else {
                                                                let _ = events.send(Event::Account {
                                                                    email: Some(current_email),
                                                                    authenticated: true,
                                                                    plan_type: Some(current_plan),
                                                                });
                                                                false
                                                            }
                                                        }
                                                        Ok(protocol_contract::AccountOutcome::AuthRequired)
                                                        | Ok(protocol_contract::AccountOutcome::UnsupportedNoData) => {
                                                            let _ = events.send(Event::Account {
                                                                email: None,
                                                                authenticated: false,
                                                                plan_type: None,
                                                            });
                                                            false
                                                        }
                                                        Err(_) => {
                                                            let _ = events.send(Event::Error(
                                                                "アカウントの再確認に失敗しました。"
                                                                    .into(),
                                                            ));
                                                            false
                                                        }
                                                    }
                                                }
                                                Err(error) => {
                                                    let _ = events.send(Event::Error(error));
                                                    false
                                                }
                                            };
                                            if !identity_is_current {
                                                continue;
                                            }
                                            let _ =
                                                events.send(Event::Usage(Box::new(UsageEvent {
                                                    remaining_percent,
                                                    reset_at,
                                                    window_seconds,
                                                    limit_name,
                                                    quota_title,
                                                    monthly,
                                                })));
                                        }
                                        Err(()) => {
                                            let _ = events.send(Event::Error(
                                                "利用枠の正本データを取得できませんでした。".into(),
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = events.send(Event::Error(e));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = events.send(Event::Error(e));
                    }
                }
            }
        }
    }
}

struct RunningAppServer {
    child: security::ChildGuard,
    input: std::process::ChildStdin,
    output: Receiver<RpcReadEvent>,
}

fn start_app_server() -> Result<RunningAppServer, String> {
    let Some(codex) = resolved_executable("CODEX_INFO_CODEX_BIN", "codex") else {
        return Err("Codex app-serverの安全な実行ファイルを確認できません。".into());
    };
    let child = Command::new(codex)
        .args(["app-server", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Codex app-serverを起動できませんでした。".to_owned())?;
    let mut child = security::ChildGuard::new(child);
    let Some(mut input) = child.child_mut().ok().and_then(|child| child.stdin.take()) else {
        return Err("Codex app-serverの入出力を初期化できませんでした。".into());
    };
    let Some(stdout) = child.child_mut().ok().and_then(|child| child.stdout.take()) else {
        return Err("Codex app-serverの入出力を初期化できませんでした。".into());
    };
    let output = rpc_reader(stdout);
    request(
        &mut input,
        &output,
        1,
        "initialize",
        json!({"clientInfo":{"name":"codex-info","version":"0.3.0"},"capabilities":{"experimentalApi":true}}),
    )?;
    Ok(RunningAppServer {
        child,
        input,
        output,
    })
}

fn thread_server_worker(commands: Receiver<ThreadCommand>, events: Sender<ThreadEvent>) {
    // The thread bridge is lazy: construction of CodexInfoState does not issue
    // thread/list before account authentication succeeds. Once started, this
    // worker owns its own child, stdin/stdout, reader and request-id sequence.
    let mut server: Option<RunningAppServer> = None;
    let mut next_id = 2u64;
    while let Ok(command) = commands.recv() {
        match command {
            ThreadCommand::Stop => {
                if let Some(mut server) = server.take() {
                    let _ = server.child.kill_and_reap();
                }
                break;
            }
            ThreadCommand::Read { auth_epoch } => {
                if server.is_none() {
                    match start_app_server() {
                        Ok(started) => {
                            server = Some(started);
                            let _ = events.send(ThreadEvent::Ready);
                        }
                        Err(message) => {
                            let _ = events.send(ThreadEvent::Error {
                                auth_epoch,
                                message,
                            });
                            continue;
                        }
                    }
                }
                let Some(server_ref) = server.as_mut() else {
                    continue;
                };
                let codex_root = codex_home_root();
                let update = fetch_active_thread_update(
                    &mut server_ref.input,
                    &server_ref.output,
                    &mut next_id,
                    codex_root.as_deref(),
                );
                if update == ActiveThreadUpdate::Failed {
                    let _ = events.send(ThreadEvent::Error {
                        auth_epoch,
                        message: "スレッド情報を安全に取得できませんでした。".into(),
                    });
                    // A framing, timeout, EOF or protocol-budget failure can
                    // leave this connection unusable. Reap only this isolated
                    // thread server so the next scheduled read starts cleanly.
                    if let Some(mut failed) = server.take() {
                        let _ = failed.child.kill_and_reap();
                    }
                    next_id = 2;
                } else {
                    let _ = events.send(ThreadEvent::Update { auth_epoch, update });
                }
            }
        }
    }
}

fn local_usage_worker(commands: Receiver<LocalCommand>, events: Sender<LocalEvent>) {
    while let Ok(command) = commands.recv() {
        match command {
            LocalCommand::Stop => break,
            LocalCommand::Collect {
                auth_epoch,
                reset_at,
                window_seconds,
            } => {
                let result = (|| {
                    let model_usage = collect_local_model_usage(reset_at, window_seconds)?;
                    let history_samples =
                        collect_local_model_usage_timeline(reset_at, window_seconds)?;
                    Ok::<_, security::SecurityError>((model_usage, history_samples))
                })();
                match result {
                    Ok((model_usage, history_samples)) => {
                        let _ = events.send(LocalEvent::Usage(LocalUsageResult {
                            auth_epoch,
                            reset_at,
                            window_seconds,
                            model_usage,
                            history_samples,
                        }));
                    }
                    Err(_) => {
                        let _ = events.send(LocalEvent::Error {
                            auth_epoch,
                            reset_at,
                            window_seconds,
                        });
                    }
                }
            }
        }
    }
}

fn request(
    input: &mut impl Write,
    output: &Receiver<RpcReadEvent>,
    id: u64,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    request_with_timeout(
        input,
        output,
        id,
        method,
        params,
        security::RPC_RESPONSE_TIMEOUT,
    )
}

fn request_with_timeout(
    input: &mut impl Write,
    output: &Receiver<RpcReadEvent>,
    id: u64,
    method: &str,
    params: Value,
    timeout: Duration,
) -> Result<Value, String> {
    let message = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
    writeln!(input, "{message}")
        .map_err(|_| "Codex app-serverへ要求を送信できませんでした。".to_owned())?;
    input
        .flush()
        .map_err(|_| "Codex app-serverへ要求を送信できませんでした。".to_owned())?;
    let deadline = Instant::now() + timeout;
    let limits = security::RpcLimits::standard();
    let mut ignored = 0usize;
    loop {
        let Some(wait) = deadline.checked_duration_since(Instant::now()) else {
            return Err("Codex app-serverの応答がタイムアウトしました。".into());
        };
        let line = match output.recv_timeout(wait) {
            Ok(RpcReadEvent::Line(line)) => line,
            Ok(RpcReadEvent::Closed) | Err(RecvTimeoutError::Disconnected) => {
                return Err("Codex app-serverが終了しました。".into());
            }
            Ok(RpcReadEvent::Failed) => {
                return Err("Codex app-serverから安全に応答を読めませんでした。".into());
            }
            Err(RecvTimeoutError::Timeout) => {
                return Err("Codex app-serverの応答がタイムアウトしました。".into());
            }
        };
        let value: Value = match serde_json::from_str(line.as_str()) {
            Ok(value) => value,
            Err(_) => {
                limits
                    .record_ignored_message(&mut ignored)
                    .map_err(|_| "Codex app-serverの応答数が上限を超えました。".to_owned())?;
                continue;
            }
        };
        if value.get("id").and_then(Value::as_u64) != Some(id) {
            limits
                .record_ignored_message(&mut ignored)
                .map_err(|_| "Codex app-serverの応答数が上限を超えました。".to_owned())?;
            continue;
        }
        if value.get("error").is_some() {
            return Err("Codex APIが要求を完了できませんでした。".into());
        }
        return Ok(value.get("result").cloned().unwrap_or(Value::Null));
    }
}

struct CodexInfoState {
    i18n: I18n,
    bridge: AppServerBridge<AccountCommand, Event>,
    thread_bridge: Option<AppServerBridge<ThreadCommand, ThreadEvent>>,
    local_bridge: LocalUsageBridge,
    auth_epoch: u64,
    email: Option<String>,
    authenticated: bool,
    plan_label: String,
    auth_url: Option<String>,
    remaining_percent: Option<f64>,
    has_quota_percent: bool,
    has_usage: bool,
    reset_at: Option<i64>,
    window_seconds: i64,
    limit_name: String,
    quota_title: String,
    monthly: bool,
    account_error: Option<String>,
    error: Option<String>,
    status: String,
    checking: bool,
    last_poll: Instant,
    last_success_at: Option<i64>,
    model_usage: Vec<ModelUsageRow>,
    active_threads: Vec<ActiveThread>,
    estimated_cost_label: String,
    history: UsageHistory,
    selected_reset_at: Option<i64>,
    selected_history_period: String,
    selected_metric: String,
    preview: bool,
    auth_polling: bool,
    thread_checking: bool,
    thread_error: bool,
    local_usage_error: bool,
    last_thread_poll: Instant,
}

impl CodexInfoState {
    #[allow(clippy::needless_return)]
    fn window_title(&self) -> String {
        #[cfg(test)]
        {
            return account_window_title(
                self.authenticated,
                self.email.as_deref(),
                &self.plan_label,
            );
        }
        #[cfg(not(test))]
        localized_account_window_title(
            &self.i18n,
            self.authenticated,
            self.email.as_deref(),
            &self.plan_label,
        )
    }

    fn new() -> Self {
        let i18n = I18n::detect();
        let bridge = AppServerBridge::<AccountCommand, Event>::start();
        bridge.send(AccountCommand::Read);
        Self {
            i18n,
            bridge,
            thread_bridge: None,
            local_bridge: LocalUsageBridge::start(),
            auth_epoch: 0,
            email: None,
            authenticated: false,
            plan_label: String::new(),
            auth_url: None,
            remaining_percent: None,
            has_quota_percent: false,
            has_usage: false,
            reset_at: None,
            window_seconds: WEEK_SECONDS,
            limit_name: "Codex".into(),
            quota_title: "残り利用枠".into(),
            monthly: false,
            account_error: None,
            error: None,
            status: "Codex app-serverへ接続しています…".into(),
            checking: true,
            last_poll: Instant::now(),
            last_success_at: None,
            model_usage: Vec::new(),
            active_threads: Vec::new(),
            estimated_cost_label: "概算 —".into(),
            history: UsageHistory::load(),
            selected_reset_at: None,
            selected_history_period: "履歴なし".into(),
            selected_metric: "ドル".into(),
            preview: false,
            auth_polling: false,
            thread_checking: false,
            thread_error: false,
            local_usage_error: false,
            last_thread_poll: Instant::now(),
        }
    }

    fn preview(kind: &str) -> Self {
        let i18n = I18n::detect();
        let bridge = AppServerBridge::<AccountCommand, Event>::inactive();
        let now = Utc::now().timestamp();
        let reset_at = now + 6 * 86_400 + 14 * 3_600;
        let model_usage = vec![
            preview_model_row("SOL", 159_278_976, 110_000_000, 30_000_000, 19_278_976),
            preview_model_row("TERRA", 30_885_887, 20_000_000, 7_000_000, 3_885_887),
            preview_model_row("LUNA", 155_294_770, 100_000_000, 40_000_000, 15_294_770),
        ];
        let preview_costs = ModelDollarTotals::from_rows(&model_usage);
        let mut state = Self {
            i18n,
            bridge,
            thread_bridge: None,
            local_bridge: LocalUsageBridge::inactive(),
            auth_epoch: 0,
            email: Some("preview@example.com".into()),
            authenticated: true,
            plan_label: "Pro".into(),
            auth_url: None,
            remaining_percent: Some(14.0),
            has_quota_percent: true,
            has_usage: true,
            reset_at: Some(reset_at),
            limit_name: "Codex".into(),
            quota_title: "残り利用枠".into(),
            monthly: false,
            account_error: None,
            error: None,
            status: String::new(),
            checking: false,
            last_poll: Instant::now(),
            last_success_at: Some(now - 60),
            window_seconds: WEEK_SECONDS,
            history: UsageHistory::preview(now, reset_at, preview_costs),
            model_usage,
            active_threads: vec![ActiveThread {
                id: "preview-thread".into(),
                created_at: Some(now - 600),
                updated_at: now,
                title: "長めの日本語タイトルで表示確認を行う実行中スレッド".into(),
                model: "gpt-5.6-sol".into(),
                model_label: "gpt-5.6-sol".into(),
                total_tokens: Some(12_345),
                context_window_tokens: Some(258_400),
                last_user_message_at: Some(now - 8),
                is_subagent: false,
                parent_thread_id: None,
                depth: None,
            }],
            estimated_cost_label: format_estimated_cost(preview_costs),
            preview: true,
            selected_reset_at: Some(reset_at),
            selected_history_period: String::new(),
            selected_metric: "ドル".into(),
            auth_polling: false,
            thread_checking: false,
            thread_error: false,
            local_usage_error: false,
            last_thread_poll: Instant::now(),
        };
        match kind {
            "auth" => {
                state.authenticated = false;
                state.email = None;
                state.remaining_percent = None;
                state.has_quota_percent = false;
                state.has_usage = false;
                state.reset_at = None;
                state.active_threads.clear();
                state.status = "未認証です。認証を開始してください。".into();
            }
            "idle" => {
                state.active_threads.clear();
                state.status = state.normal_status();
            }
            "multi-thread" => {
                // Keep this preview deliberately dense so the fixed detail window
                // exercises parent/child relationships and vertical scrolling.
                // Input is deliberately shuffled and every child is newer than
                // its parent so the presentation projection proves parent-first
                // subtree ordering instead of inheriting acquisition order.
                let model = "gpt-5.6-sol-subagent-review".to_owned();
                state.active_threads = vec![
                    ActiveThread {
                        id: "thread-child-tests".into(),
                        created_at: Some(now - 1_800),
                        updated_at: now - 1,
                        title: "複数候補と回帰テストを確認しているサブスレッド".into(),
                        model: "gpt-5.6-luna".into(),
                        model_label: "gpt-5.6-luna".into(),
                        total_tokens: Some(123_456),
                        context_window_tokens: Some(258_400),
                        last_user_message_at: Some(now - 12),
                        is_subagent: true,
                        parent_thread_id: Some("thread-z".into()),
                        depth: Some(1),
                    },
                    ActiveThread {
                        id: "thread-orphan".into(),
                        created_at: Some(now - 7_200),
                        updated_at: now - 30,
                        title: "親が完了した後も実行中のサブスレッド".into(),
                        model: "gpt-5.6-luna".into(),
                        model_label: "gpt-5.6-luna".into(),
                        total_tokens: Some(43_210),
                        context_window_tokens: Some(258_400),
                        last_user_message_at: Some(now - 90),
                        is_subagent: true,
                        parent_thread_id: Some("completed-parent".into()),
                        depth: Some(1),
                    },
                    ActiveThread {
                        id: "thread-grandchild-security".into(),
                        created_at: Some(now - 3_600),
                        updated_at: now + 1,
                        title: "脆弱性境界を確認する孫サブスレッド".into(),
                        model: "gpt-5.6-luna".into(),
                        model_label: "gpt-5.6-luna".into(),
                        total_tokens: Some(88_765),
                        context_window_tokens: Some(258_400),
                        last_user_message_at: Some(now - 25),
                        is_subagent: true,
                        parent_thread_id: Some("thread-child-review".into()),
                        depth: Some(2),
                    },
                    ActiveThread {
                        id: "thread-second-child".into(),
                        created_at: Some(now - 5_400),
                        updated_at: now - 5,
                        title: "別の親に属するサブスレッド".into(),
                        model: "gpt-5.6-terra".into(),
                        model_label: "gpt-5.6-terra".into(),
                        total_tokens: Some(54_321),
                        context_window_tokens: Some(200_000),
                        last_user_message_at: Some(now - 40),
                        is_subagent: true,
                        parent_thread_id: Some("thread-second-parent".into()),
                        depth: Some(1),
                    },
                    ActiveThread {
                        id: "thread-z".into(),
                        created_at: Some(now - 14_400),
                        updated_at: now - 10,
                        title: "利用状況画面を更新する親スレッド".into(),
                        model: model.clone(),
                        model_label: security::bounded_model_label(&model)
                            .expect("preview model is within the accepted bound"),
                        total_tokens: Some(9_876_543_210),
                        context_window_tokens: Some(258_400),
                        last_user_message_at: Some(now - 60),
                        is_subagent: false,
                        parent_thread_id: None,
                        depth: None,
                    },
                    ActiveThread {
                        id: "thread-review-source".into(),
                        created_at: Some(now - 9_000),
                        updated_at: now - 40,
                        title: "親IDを持たないレビュー用サブスレッド".into(),
                        model: "gpt-5.6-terra".into(),
                        model_label: "gpt-5.6-terra".into(),
                        total_tokens: Some(32_109),
                        context_window_tokens: Some(200_000),
                        last_user_message_at: Some(now - 120),
                        is_subagent: true,
                        parent_thread_id: None,
                        depth: None,
                    },
                    ActiveThread {
                        id: "thread-second-parent".into(),
                        created_at: Some(now - 10_800),
                        updated_at: now - 20,
                        title: "別の作業を進めている親スレッド".into(),
                        model: "gpt-5.6-sol".into(),
                        model_label: "gpt-5.6-sol".into(),
                        total_tokens: Some(765_432),
                        context_window_tokens: Some(258_400),
                        last_user_message_at: Some(now - 180),
                        is_subagent: false,
                        parent_thread_id: None,
                        depth: None,
                    },
                    ActiveThread {
                        id: "thread-child-review".into(),
                        created_at: Some(now - 2_400),
                        updated_at: now,
                        title: "表示崩れを独立評価しているサブスレッド".into(),
                        model: "gpt-5.6-terra".into(),
                        model_label: "gpt-5.6-terra".into(),
                        total_tokens: Some(456_789),
                        context_window_tokens: Some(200_000),
                        last_user_message_at: Some(now - 8),
                        is_subagent: true,
                        parent_thread_id: Some("thread-z".into()),
                        depth: Some(1),
                    },
                ];
                state.status = state.normal_status();
            }
            "graph-many" => {
                // Visual fixture: exercise the period list's bounded scroll
                // path with more entries than the four-row viewport.
                let mut samples = state.history.samples.clone();
                for index in 2..=7 {
                    let period_reset = reset_at.saturating_sub(index * WEEK_SECONDS);
                    samples.extend(
                        UsageHistory::preview(
                            now,
                            period_reset,
                            ModelDollarTotals::from_rows(&state.model_usage),
                        )
                        .samples,
                    );
                }
                state.history.samples = samples;
                state.selected_reset_at = Some(reset_at);
                state.status = state.normal_status();
            }
            "history-empty" => {
                state.history = UsageHistory::default();
                state.selected_reset_at = None;
                state.selected_history_period = "履歴なし".into();
                state.status = state.normal_status();
            }
            "monthly" => {
                let monthly_reset_at = now + 20 * 86_400 + 5 * 3_600;
                state.plan_label = "エンタープライズ".into();
                state.remaining_percent = Some(73.0);
                state.has_quota_percent = true;
                state.has_usage = true;
                state.reset_at = Some(monthly_reset_at);
                state.window_seconds = monthly_window_seconds(monthly_reset_at);
                state.quota_title = "月間残り利用枠".into();
                state.monthly = true;
                state.history = UsageHistory::preview(
                    now,
                    monthly_reset_at,
                    ModelDollarTotals::from_rows(&state.model_usage),
                );
                state.selected_reset_at = Some(monthly_reset_at);
                state.status = state.normal_status();
            }
            "unlimited" => {
                state.plan_label = "エンタープライズ".into();
                state.remaining_percent = None;
                state.has_quota_percent = false;
                state.has_usage = true;
                state.reset_at = None;
                state.model_usage.clear();
                state.window_seconds = WEEK_SECONDS;
                state.quota_title = "利用枠".into();
                state.monthly = false;
                state.history = UsageHistory::default();
                state.selected_reset_at = None;
                state.active_threads.clear();
                state.status = state.normal_status();
            }
            "warning" => {
                state.remaining_percent = Some(5.0);
                let warning_reset_at = now + 20 * 3_600;
                state.reset_at = Some(warning_reset_at);
                state.history = UsageHistory::preview(
                    now,
                    warning_reset_at,
                    ModelDollarTotals::from_rows(&state.model_usage),
                );
                state.selected_reset_at = Some(warning_reset_at);
                state.status = state.normal_status();
            }
            "reset-warning" => {
                state.remaining_percent = Some(50.0);
                let warning_reset_at = now + 20 * 3_600;
                state.reset_at = Some(warning_reset_at);
                state.history = UsageHistory::preview(
                    now,
                    warning_reset_at,
                    ModelDollarTotals::from_rows(&state.model_usage),
                );
                state.selected_reset_at = Some(warning_reset_at);
                state.status = state.normal_status();
            }
            "zero" => {
                state.remaining_percent = Some(0.0);
                state.status = state.normal_status();
            }
            "full" => {
                state.remaining_percent = Some(100.0);
                state.status = state.normal_status();
            }
            "error" => {
                state.error = Some("preview".into());
                state.status = "最新情報を取得できません。表示は12:34時点の値です。".into();
            }
            _ => {
                state.status = state.normal_status();
            }
        }
        state
    }

    fn request_read(&mut self, status: &str) {
        if self.preview {
            return;
        }
        if !self.bridge.send(AccountCommand::Read) {
            self.bridge = AppServerBridge::<AccountCommand, Event>::start();
            if !self.bridge.send(AccountCommand::Read) {
                self.apply_account_error(
                    "Codex app-serverへ更新要求を送信できませんでした。".into(),
                );
                return;
            }
        }
        self.checking = true;
        self.status = status.into();
    }

    fn advance_auth_epoch(&mut self) {
        self.auth_epoch = self.auth_epoch.saturating_add(1);
    }

    fn stop_thread_bridge(&mut self) {
        if let Some(bridge) = self.thread_bridge.take() {
            let _ = bridge.send(ThreadCommand::Stop);
        }
    }

    fn ensure_thread_bridge(&mut self) {
        if !self.preview && self.thread_bridge.is_none() {
            self.thread_bridge = Some(AppServerBridge::<ThreadCommand, ThreadEvent>::start());
        }
    }

    fn request_thread_update(&mut self) {
        if self.preview || !self.authenticated {
            return;
        }
        self.ensure_thread_bridge();
        let command = ThreadCommand::Read {
            auth_epoch: self.auth_epoch,
        };
        let sent = self
            .thread_bridge
            .as_ref()
            .is_some_and(|bridge| bridge.send(command));
        if !sent {
            self.thread_bridge = Some(AppServerBridge::<ThreadCommand, ThreadEvent>::start());
        }
        if self
            .thread_bridge
            .as_ref()
            .is_some_and(|bridge| sent || bridge.send(command))
        {
            self.thread_checking = true;
            self.last_thread_poll = Instant::now();
        } else {
            self.apply_thread_error(
                self.auth_epoch,
                "スレッド取得workerへ要求を送信できませんでした。".into(),
            );
        }
    }

    fn request_local_usage(&mut self, reset_at: i64, window_seconds: i64) {
        if !self.preview && self.authenticated {
            let command = LocalCommand::Collect {
                auth_epoch: self.auth_epoch,
                reset_at,
                window_seconds,
            };
            if !self.local_bridge.send(command) {
                self.local_bridge = LocalUsageBridge::start();
                if !self.local_bridge.send(command) {
                    self.apply_local_usage_error(self.auth_epoch, reset_at, window_seconds);
                }
            }
        }
    }

    fn clear_account_visible_state(&mut self) {
        self.advance_auth_epoch();
        self.stop_thread_bridge();
        self.email = None;
        self.authenticated = false;
        self.plan_label.clear();
        self.auth_url = None;
        self.remaining_percent = None;
        self.has_quota_percent = false;
        self.has_usage = false;
        self.reset_at = None;
        self.window_seconds = WEEK_SECONDS;
        self.limit_name = "Codex".into();
        self.quota_title = "残り利用枠".into();
        self.monthly = false;
        self.account_error = None;
        self.error = None;
        self.last_success_at = None;
        self.model_usage.clear();
        self.active_threads.clear();
        self.estimated_cost_label = "概算 —".into();
        self.thread_checking = false;
        self.thread_error = false;
        self.local_usage_error = false;
        self.history = UsageHistory::default();
        self.selected_reset_at = None;
        self.selected_history_period = "履歴なし".into();
    }

    fn apply_active_thread_update(&mut self, update: ActiveThreadUpdate) -> bool {
        match update {
            ActiveThreadUpdate::Snapshot(threads) => self.active_threads = threads,
            ActiveThreadUpdate::NoThread => self.active_threads.clear(),
            ActiveThreadUpdate::Failed => return true,
        }
        false
    }

    fn apply_usage_event(&mut self, event: UsageEvent) {
        let UsageEvent {
            remaining_percent,
            reset_at,
            window_seconds,
            limit_name,
            quota_title,
            monthly,
        } = event;
        let previous_reset_at = self.reset_at;
        let reset_changed =
            !previous_reset_at.is_some_and(|previous| same_reset_period(previous, reset_at));
        self.has_quota_percent = remaining_percent.is_some();
        self.has_usage = true;
        self.remaining_percent = remaining_percent.map(|value| value.clamp(0.0, 100.0));
        self.reset_at = (reset_at > 0).then_some(reset_at);
        self.window_seconds = window_seconds;
        self.limit_name = limit_name;
        self.quota_title = quota_title;
        self.monthly = monthly;
        self.account_error = None;
        if reset_changed {
            self.model_usage.clear();
            self.estimated_cost_label = "概算 —".into();
        }
        if self.selected_reset_at.is_none() {
            self.selected_reset_at = self.reset_at;
        }
        self.checking = false;
        self.last_success_at = Some(Utc::now().timestamp());
        // Quota is committed before the independent local worker is asked to
        // collect usage. The request carries the exact auth/period tuple.
        self.request_local_usage(reset_at, window_seconds);
        self.refresh_partial_failure_status();
    }

    fn apply_account_error(&mut self, error: String) {
        // The failed account connection is a publication boundary. Results
        // requested before this error may still be queued on the independent
        // thread/local channels, so invalidate their epoch without clearing
        // the last valid visible values. Let the thread scheduler issue a
        // fresh request instead of remaining stuck behind the stale one.
        self.advance_auth_epoch();
        self.thread_checking = false;
        self.checking = false;
        self.account_error = Some(error.clone());
        self.error = Some(error);
        self.status =
            "利用状況を取得できません。Codex app-serverへの接続を確認してください。".into();
    }

    fn apply_account_event(
        &mut self,
        email: Option<String>,
        authenticated: bool,
        plan_type: Option<String>,
    ) {
        let was_authenticated = self.authenticated;
        let next_plan_label = plan_type_label(plan_type.as_deref());
        let account_changed = self.authenticated
            && authenticated
            && (self.email != email || self.plan_label != next_plan_label);
        let entering_authenticated = !was_authenticated && authenticated;
        if !authenticated || account_changed {
            self.clear_account_visible_state();
        } else if entering_authenticated {
            // No auxiliary request is admitted before authentication, so an
            // epoch change is enough. Keep the durable history loaded at
            // startup, or reload it after an unauthenticated clear.
            self.advance_auth_epoch();
            if self.history.samples.is_empty() {
                self.history = UsageHistory::load();
            }
        }
        self.email = email;
        self.authenticated = authenticated;
        self.plan_label = if authenticated {
            next_plan_label
        } else {
            String::new()
        };
        self.checking = authenticated;
        if authenticated || was_authenticated {
            self.auth_polling = false;
        }
        if authenticated {
            self.auth_url = None;
        }
        self.status = if authenticated {
            "認証済みです。利用量を取得しています…"
        } else {
            "未認証です。認証を開始してください。"
        }
        .into();
        if authenticated
            && (entering_authenticated || account_changed || self.thread_bridge.is_none())
        {
            self.ensure_thread_bridge();
            self.request_thread_update();
        }
    }

    fn current_local_period_matches(&self, reset_at: i64, window_seconds: i64) -> bool {
        if !self.authenticated || self.window_seconds != window_seconds {
            return false;
        }
        if reset_at > 0 {
            self.reset_at == Some(reset_at)
        } else {
            self.reset_at.is_none()
        }
    }

    fn apply_local_usage_success(&mut self, result: LocalUsageResult) {
        if result.auth_epoch != self.auth_epoch
            || !self.current_local_period_matches(result.reset_at, result.window_seconds)
        {
            return;
        }
        let model_costs = result.model_usage.dollar_totals();
        let model_tokens = result.model_usage.token_totals();
        self.local_usage_error = false;
        self.model_usage = result.model_usage.rows();
        self.estimated_cost_label = format_estimated_cost(model_costs);
        if !self.preview {
            self.history
                .apply_backfill_samples(result.reset_at, result.history_samples);
        }
        if let Some(remaining_percent) = self.remaining_percent {
            self.history.record(UsageHistorySample::new_with_usage(
                Utc::now().timestamp(),
                result.reset_at,
                remaining_percent,
                model_costs,
                model_tokens,
            ));
        }
        self.refresh_partial_failure_status();
    }

    fn apply_local_usage_error(&mut self, auth_epoch: u64, reset_at: i64, window_seconds: i64) {
        if auth_epoch != self.auth_epoch
            || !self.current_local_period_matches(reset_at, window_seconds)
        {
            return;
        }
        self.local_usage_error = true;
        self.refresh_partial_failure_status();
    }

    fn apply_thread_result(&mut self, auth_epoch: u64, update: ActiveThreadUpdate) {
        if !self.authenticated || auth_epoch != self.auth_epoch {
            return;
        }
        let failed = self.apply_active_thread_update(update);
        self.thread_checking = false;
        self.thread_error = failed;
        self.last_thread_poll = Instant::now();
        self.refresh_partial_failure_status();
    }

    fn apply_thread_error(&mut self, auth_epoch: u64, message: String) {
        if !self.authenticated || auth_epoch != self.auth_epoch {
            return;
        }
        self.thread_checking = false;
        self.thread_error = true;
        let _ = message;
        self.refresh_partial_failure_status();
    }

    fn refresh_partial_failure_status(&mut self) {
        if let Some(account_error) = self.account_error.clone() {
            self.error = Some(account_error);
            self.status =
                "利用状況を取得できません。Codex app-serverへの接続を確認してください。".into();
            return;
        }
        match (self.local_usage_error, self.thread_error) {
            (true, true) => {
                self.error =
                    Some("ローカル履歴とスレッド情報を安全に取得できませんでした。".into());
                self.status =
                    "利用枠は更新しました。履歴とスレッドは前回値を保持しています。".into();
            }
            (true, false) => {
                self.error = Some("ローカル利用履歴を安全に集計できませんでした。".into());
                self.status = "利用枠は更新しました。履歴は前回値を保持しています。".into();
            }
            (false, true) => {
                self.error = Some("スレッド情報を安全に取得できませんでした。".into());
                self.status = "利用枠は更新しました。スレッド表示は前回値を保持しています。".into();
            }
            (false, false) => {
                self.error = None;
                self.status = self.normal_status();
            }
        }
    }

    /// Apply one FIFO batch from the current account bridge. An account error
    /// invalidates the connection, so later events already drained from that
    /// same receiver must not cross the replacement boundary.
    fn apply_account_event_batch(&mut self, events: Vec<Event>) -> bool {
        for event in events {
            match event {
                Event::Ready => {
                    if self.account_error.is_none() && self.checking {
                        self.status = "認証状態を確認しています…".into();
                    }
                }
                Event::Account {
                    email,
                    authenticated,
                    plan_type,
                } => self.apply_account_event(email, authenticated, plan_type),
                Event::AuthUrl(url) => {
                    self.auth_url = Some(url);
                    self.checking = false;
                    self.auth_polling = false;
                    self.account_error = None;
                    self.error = None;
                    self.status =
                        "認証URLを発行しました。「認証ページを開く」を押してください。".into();
                }
                Event::Usage(event) => self.apply_usage_event(*event),
                Event::Error(error) => {
                    self.apply_account_error(error);
                    return true;
                }
            }
        }
        false
    }

    fn poll(&mut self) {
        if self.preview {
            return;
        }
        let mut account_events = Vec::new();
        while let Ok(event) = self.bridge.rx.try_recv() {
            account_events.push(event);
        }
        if self.apply_account_event_batch(account_events) {
            // Do not retry in this callback. Replace only the failed
            // connection; the explicit retry or scheduled refresh sends the
            // next request through the fresh bridge.
            let _ = self.bridge.send(AccountCommand::Stop);
            self.bridge = AppServerBridge::<AccountCommand, Event>::start();
        }

        let mut thread_events = Vec::new();
        if let Some(bridge) = self.thread_bridge.as_ref() {
            while let Ok(event) = bridge.rx.try_recv() {
                thread_events.push(event);
            }
        }
        for event in thread_events {
            match event {
                ThreadEvent::Ready => {}
                ThreadEvent::Update { auth_epoch, update } => {
                    self.apply_thread_result(auth_epoch, update);
                }
                ThreadEvent::Error {
                    auth_epoch,
                    message,
                } => self.apply_thread_error(auth_epoch, message),
            }
        }

        let mut local_events = Vec::new();
        while let Ok(event) = self.local_bridge.rx.try_recv() {
            local_events.push(event);
        }
        for event in local_events {
            match event {
                LocalEvent::Usage(result) => self.apply_local_usage_success(result),
                LocalEvent::Error {
                    auth_epoch,
                    reset_at,
                    window_seconds,
                } => self.apply_local_usage_error(auth_epoch, reset_at, window_seconds),
            }
        }
    }

    #[allow(clippy::needless_return)]
    fn normal_status(&self) -> String {
        #[cfg(test)]
        {
            return normal_status_text(
                self.remaining_percent.unwrap_or(50.0),
                if self.has_quota_percent {
                    self.seconds_to_reset()
                } else {
                    i64::MAX
                },
                Some("12:34"),
            );
        }
        #[cfg(not(test))]
        {
            if !self.has_quota_percent {
                return self.i18n.format_last_updated(self.last_success_at);
            }
            let remaining = self.remaining_percent.unwrap_or(0.0);
            if remaining <= 2.0 {
                self.i18n.text(TextKey::QuotaNearlyGone).into()
            } else if remaining <= 10.0 {
                self.i18n.text(TextKey::QuotaLow).into()
            } else if self.seconds_to_reset().abs() <= 86_400 {
                self.i18n.text(TextKey::ResetWithinDay).into()
            } else {
                self.i18n.format_last_updated(self.last_success_at)
            }
        }
    }

    fn history_periods(&self) -> Vec<HistoryPeriod> {
        let now = Utc::now().timestamp();
        let mut periods = self.history.periods(now, self.reset_at);
        periods.retain(|period| {
            DateTime::<Utc>::from_timestamp(period.start, 0).is_some()
                && DateTime::<Utc>::from_timestamp(period.end, 0).is_some()
                && DateTime::<Utc>::from_timestamp(period.canonical_reset_at, 0).is_some()
        });
        for period in &mut periods {
            let is_current = self.reset_at.is_some_and(|current| {
                current.abs_diff(period.canonical_reset_at) <= RESET_AT_TOLERANCE_SECONDS as u64
                    && now < period.canonical_reset_at
            });
            // The visible current period runs through its next reset, while
            // `end` is intentionally clipped to `now` for graph rendering.
            let label_end = if is_current {
                period.canonical_reset_at
            } else {
                period.end
            };
            let Some(mut label) = self.i18n.format_period(period.start, label_end) else {
                period.label.clear();
                continue;
            };
            if is_current {
                label.push_str(self.i18n.text(TextKey::CurrentSuffix));
            }
            period.label = label;
        }
        let base_labels = periods
            .iter()
            .map(|period| period.label.clone())
            .collect::<Vec<_>>();
        for index in 0..periods.len() {
            if base_labels
                .iter()
                .filter(|label| **label == base_labels[index])
                .count()
                > 1
            {
                if let Some(suffix) = self
                    .i18n
                    .format_deadline_suffix(periods[index].canonical_reset_at)
                {
                    periods[index].label.push_str(&suffix);
                }
            }
        }
        periods.retain(|period| !period.label.is_empty());
        periods
    }

    fn history_period_options(&self) -> Vec<String> {
        let periods = self.history_periods();
        if periods.is_empty() {
            vec![self.i18n.text(TextKey::NoHistory).into()]
        } else {
            periods.into_iter().map(|period| period.label).collect()
        }
    }

    fn selected_history_period_label(&self) -> String {
        let periods = self.history_periods();
        if let Some(period) = periods
            .iter()
            .find(|period| period.label == self.selected_history_period)
        {
            return period.label.clone();
        }
        if let Some(selected) = self.selected_reset_at {
            if let Some(period) = periods
                .iter()
                .find(|period| period.canonical_reset_at == selected)
            {
                return period.label.clone();
            }
        }
        if let Some(current) = self.reset_at {
            if let Some(period) = periods.iter().find(|period| {
                period.canonical_reset_at.abs_diff(current) <= RESET_AT_TOLERANCE_SECONDS as u64
            }) {
                return period.label.clone();
            }
        }
        periods
            .first()
            .map(|period| period.label.clone())
            .unwrap_or_else(|| self.i18n.text(TextKey::NoHistory).into())
    }

    fn select_history(&mut self, label: &str) {
        if let Some(period) = self
            .history_periods()
            .into_iter()
            .find(|period| period.label == label)
        {
            self.selected_history_period = label.into();
            self.selected_reset_at = Some(period.canonical_reset_at);
        }
    }

    fn select_metric(&mut self, metric: &str) {
        if metric == "ドル" || metric == self.i18n.text(TextKey::DollarMetric) {
            self.selected_metric = "ドル".into();
        } else if metric == "トークン" || metric == self.i18n.text(TextKey::TokenMetric) {
            self.selected_metric = "トークン".into();
        }
    }

    fn graph_data(&self) -> String {
        let Some(reset_at) = self.selected_history_reset() else {
            return "[]".into();
        };
        self.history.graph_data_for_reset(reset_at)
    }

    fn selected_history_reset(&self) -> Option<i64> {
        let periods = self.history_periods();
        periods
            .iter()
            .find(|period| period.label == self.selected_history_period)
            .map(|period| period.canonical_reset_at)
            .or_else(|| {
                self.selected_reset_at.and_then(|selected| {
                    periods
                        .iter()
                        .find(|period| {
                            period.canonical_reset_at.abs_diff(selected)
                                <= RESET_AT_TOLERANCE_SECONDS as u64
                        })
                        .map(|period| period.canonical_reset_at)
                })
            })
            .or(self.reset_at)
            .or_else(|| periods.first().map(|period| period.canonical_reset_at))
    }

    fn select_latest_history(&mut self) {
        let periods = self.history_periods();
        let selected = self
            .reset_at
            .and_then(|reset| {
                periods
                    .iter()
                    .find(|period| {
                        period.canonical_reset_at.abs_diff(reset)
                            <= RESET_AT_TOLERANCE_SECONDS as u64
                    })
                    .or_else(|| periods.first())
            })
            .or_else(|| periods.first());
        if let Some(period) = selected {
            self.selected_history_period = period.label.clone();
            self.selected_reset_at = Some(period.canonical_reset_at);
        } else {
            self.selected_history_period = "履歴なし".into();
            self.selected_reset_at = None;
        }
    }

    fn select_older_history(&mut self) {
        let periods = self.history_periods();
        let Some(current) = self.selected_history_reset() else {
            return;
        };
        if let Some(index) = periods
            .iter()
            .position(|period| period.canonical_reset_at == current)
        {
            if let Some(period) = periods.get(index + 1) {
                self.select_history(&period.label.clone());
            }
        }
    }

    #[cfg(test)]
    fn select_newer_history(&mut self) {
        let periods = self.history_periods();
        let Some(current) = self.selected_history_reset() else {
            return;
        };
        if let Some(index) = periods
            .iter()
            .position(|period| period.canonical_reset_at == current)
        {
            if index > 0 {
                if let Some(period) = periods.get(index - 1) {
                    self.select_history(&period.label.clone());
                }
            }
        }
    }

    #[cfg(test)]
    fn history_navigation(&self) -> (bool, bool) {
        let periods = self.history_periods();
        let Some(current) = self.selected_history_reset() else {
            return (false, false);
        };
        let Some(index) = periods
            .iter()
            .position(|period| period.canonical_reset_at == current)
        else {
            return (false, false);
        };
        (index + 1 < periods.len(), index > 0)
    }

    fn period_seconds_for_reset(&self, reset_at: i64) -> i64 {
        let current_period_seconds = if self.monthly {
            monthly_window_seconds(reset_at)
        } else {
            self.window_seconds.max(WEEK_SECONDS)
        };
        if self
            .reset_at
            .is_some_and(|current| same_reset_period(current, reset_at))
        {
            return current_period_seconds;
        }

        // Historical periods do not inherit the current plan's calendar
        // month. Use the nearest newer reset as the period boundary when the
        // observed distance is plausible; this keeps an old weekly period
        // from being rendered as a 31-day window after a monthly switch.
        let periods = self.history.reset_periods_desc();
        if let Some(index) = periods
            .iter()
            .position(|period| same_reset_period(*period, reset_at))
        {
            if let Some(newer_reset) = index.checked_sub(1).and_then(|i| periods.get(i)) {
                let distance = newer_reset.saturating_sub(reset_at);
                if (3_600..=45 * 86_400).contains(&distance) {
                    return distance;
                }
            }
        }
        self.window_seconds.max(WEEK_SECONDS)
    }

    fn period_end_for_reset(&self, reset_at: i64) -> i64 {
        self.history
            .period_for_id(reset_at, Utc::now().timestamp(), self.reset_at)
            .map(|period| period.end)
            .unwrap_or_else(|| graph_period_end(reset_at, self.reset_at, Utc::now().timestamp()))
    }

    fn graph_paths_for_selection(
        &self,
        show_luna: bool,
        show_terra: bool,
        show_sol: bool,
        show_tokens: bool,
    ) -> GraphPaths {
        let selected_reset = self.selected_history_reset();
        let samples = self.history.samples_for_reset(selected_reset);
        let period = selected_reset.and_then(|reset| {
            self.history
                .period_for_id(reset, Utc::now().timestamp(), self.reset_at)
        });
        let reset_at = period
            .as_ref()
            .map(|period| period.canonical_reset_at)
            .or_else(|| samples.last().map(|sample| sample.reset_at))
            .unwrap_or(0);
        let period_start = period
            .as_ref()
            .map(|period| period.start)
            .unwrap_or_else(|| reset_at.saturating_sub(self.period_seconds_for_reset(reset_at)));
        let period_end = period
            .as_ref()
            .map(|period| period.end)
            .unwrap_or_else(|| self.period_end_for_reset(reset_at))
            .max(period_start + 1);
        let sample_references = samples.iter().collect::<Vec<_>>();
        let mut paths = graph_paths_for_selection(
            &sample_references,
            period_start,
            period_end,
            show_luna,
            show_terra,
            show_sol,
            show_tokens,
        );
        if !self.has_quota_percent {
            paths.remaining.clear();
            paths.remaining_markers.clear();
            paths.current_remaining_label.clear();
            paths.current_remaining_y = 0.99;
        }
        paths
    }
}

fn sync_graph_window(state: &CodexInfoState, graph: &GraphWindow) {
    graph.set_strings(ui_strings(&state.i18n));
    graph.set_window_title(
        native_detail_window_title(
            &state.i18n,
            state.authenticated,
            &state.window_title(),
            WindowPurpose::Graph,
        )
        .into(),
    );
    let token_metric = state.selected_metric == "トークン"
        || state.selected_metric == state.i18n.text(TextKey::TokenMetric);
    graph.set_show_tokens(token_metric);
    let mut paths = state.graph_paths_for_selection(
        graph.get_show_luna(),
        graph.get_show_terra(),
        graph.get_show_sol(),
        graph.get_show_tokens(),
    );
    separate_current_label_positions(
        &mut paths,
        graph.get_show_remaining(),
        graph.get_show_luna(),
        graph.get_show_terra(),
        graph.get_show_sol(),
    );
    let time_labels = state.graph_time_labels();
    graph.set_graph_data(state.graph_data().into());
    graph.set_unused_intervals(slint::ModelRc::new(slint::VecModel::from(
        paths
            .unused_intervals
            .iter()
            .map(|interval| GraphUnusedInterval {
                start: interval.start as f32,
                width: interval.width as f32,
            })
            .collect::<Vec<_>>(),
    )));
    let history_period_options = state.history_period_options();
    graph.set_has_history_options(
        !history_period_options.is_empty()
            && history_period_options[0] != state.i18n.text(TextKey::NoHistory),
    );
    let selected_history_period = state.selected_history_period_label();
    let selected_history_index = history_period_options
        .iter()
        .position(|period| period == &selected_history_period)
        .unwrap_or(0);
    graph.set_history_period_options(slint::ModelRc::new(slint::VecModel::from(
        history_period_options
            .into_iter()
            .map(slint::SharedString::from)
            .collect::<Vec<_>>(),
    )));
    graph.set_selected_history_index(i32::try_from(selected_history_index).unwrap_or(i32::MAX));
    graph.set_metric_options(slint::ModelRc::new(slint::VecModel::from(vec![
        slint::SharedString::from(state.i18n.text(TextKey::DollarMetric)),
        slint::SharedString::from(state.i18n.text(TextKey::TokenMetric)),
    ])));
    graph.set_selected_metric_index(if token_metric { 1 } else { 0 });
    graph.set_time_start_label(time_labels[0].clone().into());
    graph.set_time_25_label(time_labels[1].clone().into());
    graph.set_time_50_label(time_labels[2].clone().into());
    graph.set_time_75_label(time_labels[3].clone().into());
    graph.set_time_end_label(time_labels[4].clone().into());
    graph.set_remaining_path(paths.remaining.into());
    graph.set_remaining_markers(slint::ModelRc::new(slint::VecModel::from(
        paths
            .remaining_markers
            .iter()
            .map(|marker| RemainingMarker {
                x: marker.x as f32,
                y: marker.y as f32,
            })
            .collect::<Vec<_>>(),
    )));
    graph.set_sol_flat_path(paths.sol_flat.into());
    graph.set_sol_rising_path(paths.sol_rising.into());
    graph.set_terra_flat_path(paths.terra_flat.into());
    graph.set_terra_rising_path(paths.terra_rising.into());
    graph.set_luna_flat_path(paths.luna_flat.into());
    graph.set_luna_rising_path(paths.luna_rising.into());
    graph.set_dollar_top_label(paths.dollar_labels[0].clone().into());
    graph.set_dollar_75_label(paths.dollar_labels[1].clone().into());
    graph.set_dollar_50_label(paths.dollar_labels[2].clone().into());
    graph.set_dollar_25_label(paths.dollar_labels[3].clone().into());
    graph.set_dollar_bottom_label(paths.dollar_labels[4].clone().into());
    let has_current_remaining_label = !paths.current_remaining_label.is_empty();
    let has_current_sol_label = !paths.current_sol_label.is_empty();
    let has_current_terra_label = !paths.current_terra_label.is_empty();
    let has_current_luna_label = !paths.current_luna_label.is_empty();
    graph.set_current_remaining_label(paths.current_remaining_label.into());
    graph.set_current_sol_label(paths.current_sol_label.into());
    graph.set_current_terra_label(paths.current_terra_label.into());
    graph.set_current_luna_label(paths.current_luna_label.into());
    graph.set_current_remaining_connector_path(
        current_label_connector_path(
            paths.current_remaining_point_y,
            paths.current_remaining_y,
            has_current_remaining_label,
        )
        .into(),
    );
    graph.set_current_sol_connector_path(
        current_label_connector_path(
            paths.current_sol_point_y,
            paths.current_sol_y,
            has_current_sol_label,
        )
        .into(),
    );
    graph.set_current_terra_connector_path(
        current_label_connector_path(
            paths.current_terra_point_y,
            paths.current_terra_y,
            has_current_terra_label,
        )
        .into(),
    );
    graph.set_current_luna_connector_path(
        current_label_connector_path(
            paths.current_luna_point_y,
            paths.current_luna_y,
            has_current_luna_label,
        )
        .into(),
    );
    graph.set_current_remaining_y(paths.current_remaining_y);
    graph.set_current_sol_y(paths.current_sol_y);
    graph.set_current_terra_y(paths.current_terra_y);
    graph.set_current_luna_y(paths.current_luna_y);
}

fn classify_active_thread_model(model_label: &str) -> &'static str {
    let mut match_name = None;
    let mut known_count = 0;
    for token in model_label
        .to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        let candidate = match token {
            "sol" => Some("SOL"),
            "terra" => Some("TERRA"),
            "luna" => Some("LUNA"),
            _ => None,
        };
        if let Some(candidate) = candidate {
            known_count += 1;
            match_name = Some(candidate);
        }
    }
    if known_count == 1 {
        match_name.unwrap_or("OTHER")
    } else {
        "OTHER"
    }
}

#[cfg(test)]
fn active_thread_model_counts(threads: &[ActiveThread]) -> String {
    if threads.is_empty() {
        return String::new();
    }
    let [sol, terra, luna, other] = active_thread_model_count_values(threads);
    format!("SOL {sol}  TERRA {terra}  LUNA {luna}  その他 {other}")
}

fn active_thread_model_count_values(threads: &[ActiveThread]) -> [i32; 4] {
    let mut sol = 0usize;
    let mut terra = 0usize;
    let mut luna = 0usize;
    let mut other = 0usize;
    for thread in threads {
        match classify_active_thread_model(&thread.model_label) {
            "SOL" => sol += 1,
            "TERRA" => terra += 1,
            "LUNA" => luna += 1,
            _ => other += 1,
        }
    }
    [
        i32::try_from(sol).unwrap_or(i32::MAX),
        i32::try_from(terra).unwrap_or(i32::MAX),
        i32::try_from(luna).unwrap_or(i32::MAX),
        i32::try_from(other).unwrap_or(i32::MAX),
    ]
}

#[cfg(test)]
fn format_elapsed(now: i64, timestamp: Option<i64>) -> String {
    let Some(timestamp) = timestamp else {
        return "—".into();
    };
    if DateTime::<Utc>::from_timestamp(timestamp, 0).is_none() {
        return "—".into();
    }
    let age = now.saturating_sub(timestamp).max(0);
    if age < 60 {
        format!("{age}秒")
    } else if age < 3_600 {
        let minutes = age / 60;
        let seconds = age % 60;
        if seconds == 0 {
            format!("{minutes}分")
        } else {
            format!("{minutes}分{seconds}秒")
        }
    } else if age < 86_400 {
        let hours = age / 3_600;
        let minutes = (age % 3_600) / 60;
        if minutes == 0 {
            format!("{hours}時間")
        } else {
            format!("{hours}時間{minutes}分")
        }
    } else {
        let days = age / 86_400;
        let hours = (age % 86_400) / 3_600;
        if hours == 0 {
            format!("{days}日")
        } else {
            format!("{days}日{hours}時間")
        }
    }
}

fn sort_thread_indices(indices: &mut [usize], threads: &[ActiveThread]) {
    indices.sort_by(|left, right| {
        threads[*right]
            .updated_at
            .cmp(&threads[*left].updated_at)
            .then_with(|| threads[*right].id.cmp(&threads[*left].id))
    });
}

fn push_thread_subtree(
    index: usize,
    forest_depth: usize,
    has_next_sibling: bool,
    ancestor_guides: [bool; 3],
    children: &[Vec<usize>],
    visited: &mut [bool],
    rows: &mut Vec<ThreadPresentationRow>,
) {
    if visited[index] {
        return;
    }
    visited[index] = true;
    rows.push(ThreadPresentationRow {
        index,
        forest_depth,
        connected_to_parent: forest_depth > 0,
        has_children: !children[index].is_empty(),
        has_next_sibling: forest_depth > 0 && has_next_sibling,
        ancestor_guides,
    });

    let child_count = children[index].len();
    for (position, child) in children[index].iter().copied().enumerate() {
        let mut child_guides = ancestor_guides;
        if forest_depth > 0 {
            let visible_level = forest_depth.min(3);
            // A display depth of three is a capped lane. Once any ancestor
            // at that lane needs a continuation, a deeper descendant must
            // keep the guide even when its immediate parent is the last
            // sibling; assignment here would incorrectly erase that path.
            child_guides[visible_level - 1] |= has_next_sibling;
        }
        push_thread_subtree(
            child,
            forest_depth.saturating_add(1),
            position + 1 < child_count,
            child_guides,
            children,
            visited,
            rows,
        );
    }
}

fn thread_presentation_rows(threads: &[ActiveThread]) -> Vec<ThreadPresentationRow> {
    let mut by_id = BTreeMap::new();
    for (index, thread) in threads.iter().enumerate() {
        by_id.entry(thread.id.as_str()).or_insert(index);
    }

    let parent_indices = threads
        .iter()
        .map(|thread| {
            thread
                .is_subagent
                .then_some(thread.parent_thread_id.as_deref())
                .flatten()
                .and_then(|parent_id| by_id.get(parent_id).copied())
        })
        .collect::<Vec<_>>();
    let mut children = vec![Vec::new(); threads.len()];
    let mut roots = Vec::new();
    for (index, parent) in parent_indices.iter().copied().enumerate() {
        if let Some(parent) = parent {
            children[parent].push(index);
        } else {
            roots.push(index);
        }
    }
    sort_thread_indices(&mut roots, threads);
    for siblings in &mut children {
        sort_thread_indices(siblings, threads);
    }

    let mut visited = vec![false; threads.len()];
    let mut rows = Vec::with_capacity(threads.len());
    for root in roots {
        push_thread_subtree(
            root,
            0,
            false,
            [false; 3],
            &children,
            &mut visited,
            &mut rows,
        );
    }

    // Native acquisition rejects cycles atomically. This deterministic
    // fallback keeps hand-built/defensive inputs total without guessing an
    // edge: every unreachable node becomes one disconnected top-level row.
    let mut disconnected = visited
        .iter()
        .enumerate()
        .filter_map(|(index, was_visited)| (!*was_visited).then_some(index))
        .collect::<Vec<_>>();
    sort_thread_indices(&mut disconnected, threads);
    for index in disconnected {
        visited[index] = true;
        rows.push(ThreadPresentationRow {
            index,
            forest_depth: 0,
            connected_to_parent: false,
            has_children: false,
            has_next_sibling: false,
            ancestor_guides: [false; 3],
        });
    }
    rows
}

fn active_thread_rows_at_with_i18n(
    threads: &[ActiveThread],
    now: i64,
    i18n: &I18n,
) -> Vec<ActiveThreadRow> {
    thread_presentation_rows(threads)
        .into_iter()
        .map(|presentation| {
            let thread = &threads[presentation.index];
            let relation = if thread.is_subagent {
                let depth = if presentation.connected_to_parent {
                    i32::try_from(presentation.forest_depth).ok()
                } else {
                    thread.depth.filter(|depth| *depth > 0)
                };
                match depth {
                    Some(depth) if depth > 99 => format!("{} D99+", i18n.text(TextKey::SubRole)),
                    Some(depth) => format!("{} D{depth}", i18n.text(TextKey::SubRole)),
                    None => i18n.text(TextKey::SubRole).to_owned(),
                }
            } else {
                i18n.text(TextKey::MainRole).to_owned()
            };
            let parent_title = thread
                .parent_thread_id
                .as_deref()
                .map(|parent_id| {
                    threads
                        .iter()
                        .find(|candidate| candidate.id == parent_id)
                        .map(|parent| i18n.format_parent_title(&parent.title))
                        .unwrap_or_else(|| i18n.text(TextKey::ParentNotRunning).to_owned())
                })
                .unwrap_or_default();
            ActiveThreadRow {
                relation: relation.into(),
                is_main: !thread.is_subagent,
                title: security::shorten_unicode(&thread.title, security::MAX_THREAD_TITLE_SCALARS)
                    .into(),
                parent_title: security::shorten_unicode(
                    &parent_title,
                    security::MAX_THREAD_TITLE_SCALARS,
                )
                .into(),
                model: security::shorten_unicode(
                    &thread.model_label,
                    security::MAX_ACCOUNT_ACTIVITY_LABEL_SCALARS,
                )
                .into(),
                tokens: thread
                    .total_tokens
                    .map(|total| i18n.format_token_value(total))
                    .unwrap_or_else(|| "—".to_owned())
                    .into(),
                context_usage: match (thread.total_tokens, thread.context_window_tokens) {
                    (Some(used), Some(window)) if window > 0 => format!(
                        "{} / {}",
                        i18n.format_context_usage(used, window),
                        i18n.format_token_value(window)
                    ),
                    (None, Some(window)) if window > 0 => {
                        format!("— / {}", i18n.format_token_value(window))
                    }
                    _ => "—".to_owned(),
                }
                .into(),
                thread_age: i18n.format_elapsed(now, thread.created_at).into(),
                instruction_age: i18n.format_elapsed(now, thread.last_user_message_at).into(),
                tree_depth: i32::try_from(presentation.forest_depth).unwrap_or(i32::MAX),
                connected_to_parent: presentation.connected_to_parent,
                has_children: presentation.has_children,
                has_next_sibling: presentation.has_next_sibling,
                ancestor_guide_1: presentation.ancestor_guides[0],
                ancestor_guide_2: presentation.ancestor_guides[1],
                ancestor_guide_3: presentation.ancestor_guides[2],
            }
        })
        .collect()
}

#[cfg(test)]
fn active_thread_rows_at(threads: &[ActiveThread], now: i64) -> Vec<ActiveThreadRow> {
    active_thread_rows_at_with_i18n(
        threads,
        now,
        &I18n::from_parts(codex_info::i18n::Language::Japanese, chrono_tz::Tz::UTC),
    )
}

#[cfg(test)]
#[allow(dead_code)]
fn active_thread_rows(threads: &[ActiveThread]) -> Vec<ActiveThreadRow> {
    active_thread_rows_at(threads, Utc::now().timestamp())
}

fn sync_threads_window(state: &CodexInfoState, threads_window: &ThreadsWindow) {
    threads_window.set_strings(ui_strings(&state.i18n));
    threads_window.set_thread_count_label(
        state
            .i18n
            .format_thread_count(state.active_threads.len())
            .into(),
    );
    threads_window.set_window_title(
        native_detail_window_title(
            &state.i18n,
            state.authenticated,
            &state.window_title(),
            WindowPurpose::Threads,
        )
        .into(),
    );
    threads_window.set_thread_rows(slint::ModelRc::new(slint::VecModel::from(
        active_thread_rows_at_with_i18n(&state.active_threads, Utc::now().timestamp(), &state.i18n),
    )));
}

fn ui_strings(i18n: &I18n) -> UiStrings {
    UiStrings {
        font_family: i18n.text(TextKey::FontFamily).into(),
        usage_status: i18n.text(TextKey::UsageStatus).into(),
        graph: i18n.text(TextKey::Graph).into(),
        legal_notices: i18n.text(TextKey::LegalNotices).into(),
        running: i18n.text(TextKey::Running).into(),
        model_threads: i18n.text(TextKey::ModelThreads).into(),
        other: i18n.text(TextKey::Other).into(),
        details: i18n.text(TextKey::Details).into(),
        no_running_threads: i18n.text(TextKey::NoRunningThreads).into(),
        legal_code: i18n.text(TextKey::LegalCode).into(),
        legal_warranty: i18n.text(TextKey::LegalWarranty).into(),
        legal_license: i18n.text(TextKey::LegalLicense).into(),
        legal_font: i18n.text(TextKey::LegalFont).into(),
        legal_schema: i18n.text(TextKey::LegalSchema).into(),
        legal_dependencies: i18n.text(TextKey::LegalDependencies).into(),
        legal_details: i18n.text(TextKey::LegalDetails).into(),
        legal_distribution: i18n.text(TextKey::LegalDistribution).into(),
        close: i18n.text(TextKey::Close).into(),
        active_threads: i18n.text(TextKey::ActiveThreads).into(),
        context_usage: i18n.text(TextKey::Context).into(),
        instruction: i18n.text(TextKey::Instruction).into(),
        tokens: i18n.text(TextKey::Tokens).into(),
        model: i18n.text(TextKey::Model).into(),
        input: i18n.text(TextKey::Input).into(),
        cached: i18n.text(TextKey::Cached).into(),
        output: i18n.text(TextKey::Output).into(),
        retry: i18n.text(TextKey::Retry).into(),
        usage_trend: i18n.text(TextKey::UsageTrend).into(),
        remaining: i18n.text(TextKey::Remaining).into(),
        graph_token_description: i18n.text(TextKey::GraphTokenDescription).into(),
        graph_dollar_description: i18n.text(TextKey::GraphDollarDescription).into(),
        no_records: i18n.text(TextKey::NoRecords).into(),
        connect_account: i18n.text(TextKey::ConnectAccount).into(),
        auth_browser_instructions: i18n.text(TextKey::AuthBrowserInstructions).into(),
        auth_managed: i18n.text(TextKey::AuthManaged).into(),
        open_auth_page: i18n.text(TextKey::OpenAuthPage).into(),
        start_auth: i18n.text(TextKey::StartAuth).into(),
        checking: i18n.text(TextKey::Checking).into(),
        check_auth: i18n.text(TextKey::CheckAuth).into(),
        auth_cli: i18n.text(TextKey::AuthCli).into(),
        no_history: i18n.text(TextKey::NoHistory).into(),
    }
}

#[cfg(test)]
fn normal_status_text(remaining: f64, seconds: i64, last_success_at: Option<&str>) -> String {
    let quota_notice = if remaining <= 2.0 {
        Some("残り利用枠はほぼありません。")
    } else if remaining <= 10.0 {
        Some("残り利用枠が少なくなっています。")
    } else {
        None
    };
    if let Some(notice) = quota_notice {
        notice.into()
    } else if seconds.abs() <= 86_400 {
        "リセット前後24時間です。".into()
    } else {
        format!("最終更新 {}", last_success_at.unwrap_or("—"))
    }
}

fn automatic_refresh_interval(authenticated: bool, auth_polling: bool) -> Duration {
    if !authenticated && auth_polling {
        Duration::from_secs(2)
    } else {
        Duration::from_secs(60)
    }
}

fn open_validated_auth_url(value: &str) -> bool {
    let Ok(url) = security::validate_auth_url(value) else {
        return false;
    };
    let executables = if let Some(path) = std::env::var_os("CODEX_INFO_BROWSER_BIN") {
        security::resolve_executable_path(Path::new(&path))
            .ok()
            .into_iter()
            .collect::<Vec<_>>()
    } else {
        ["wslview", "xdg-open"]
            .into_iter()
            .filter_map(|name| resolved_executable("CODEX_INFO_BROWSER_BIN", name))
            .collect::<Vec<_>>()
    };
    for executable in executables {
        let child = Command::new(executable)
            .arg(url.as_str())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if let Ok(mut child) = child {
            thread::spawn(move || {
                let _ = child.wait();
            });
            return true;
        }
    }
    false
}

impl CodexInfoState {
    fn status_level(&self) -> &'static str {
        if self.error.is_some() {
            "error"
        } else if self.reset_at.is_some() && self.seconds_to_reset().abs() <= 86_400
            || (self.has_quota_percent && self.remaining_percent.unwrap_or(0.0) <= 10.0)
        {
            "warning"
        } else {
            "info"
        }
    }

    fn seconds_to_reset(&self) -> i64 {
        self.reset_at
            .and_then(|value| DateTime::from_timestamp(value, 0))
            .map(|time| (time - Utc::now()).num_seconds())
            .unwrap_or(0)
    }

    fn display_status(&self) -> String {
        if self.account_error.is_some() {
            return if self.last_success_at.is_some() {
                self.i18n.format_stale_status(self.last_success_at)
            } else {
                self.i18n.text(TextKey::CannotFetchUsage).into()
            };
        }
        match self.status.as_str() {
            "Codex app-serverへ接続しています…" => {
                self.i18n.text(TextKey::Connecting).into()
            }
            "認証状態を確認しています…" | "認証完了を確認しています…" => {
                self.i18n.text(TextKey::CheckingAuthStatus).into()
            }
            "認証済みです。利用量を取得しています…" => {
                self.i18n.text(TextKey::AuthenticatedLoading).into()
            }
            "未認証です。認証を開始してください。" => {
                self.i18n.text(TextKey::UnauthenticatedStart).into()
            }
            "認証URLを発行しました。「認証ページを開く」を押してください。" => {
                self.i18n.text(TextKey::AuthUrlIssued).into()
            }
            "認証URLを発行しています…" => {
                self.i18n.text(TextKey::IssuingAuthUrl).into()
            }
            "認証URLを開けませんでした。"
            | "認証URLを開けません。Codex CLIから認証を完了してください。" => {
                self.i18n.text(TextKey::AuthUrlOpenFailed).into()
            }
            "利用状況を更新しています…" => {
                self.i18n.text(TextKey::UpdatingUsage).into()
            }
            "利用状況を取得できません。Codex app-serverへの接続を確認してください。" => {
                self.i18n.text(TextKey::CannotFetchUsage).into()
            }
            "利用枠は更新しました。履歴とスレッドは前回値を保持しています。" => {
                self.i18n.text(TextKey::PartialHistoryThreads).into()
            }
            "利用枠は更新しました。履歴は前回値を保持しています。" => {
                self.i18n.text(TextKey::PartialHistory).into()
            }
            "利用枠は更新しました。スレッド表示は前回値を保持しています。" => {
                self.i18n.text(TextKey::PartialThreads).into()
            }
            "状態を表示できません。" => {
                self.i18n.text(TextKey::CannotDisplayStatus).into()
            }
            _ if self.status.is_empty() => self.i18n.text(TextKey::CannotDisplayStatus).into(),
            // `normal_status` is already formatted by the startup-pinned
            // catalog (for example, a localized last-updated clock). Keep
            // that value instead of replacing it with a generic Japanese
            // fallback. All asynchronous status keys above are canonical
            // internal values and are translated before reaching this arm.
            _ => self.status.clone(),
        }
    }

    fn open_auth(&mut self) {
        if let Some(url) = self.auth_url.clone() {
            let opened = open_validated_auth_url(&url);
            if !opened {
                self.apply_account_error(
                    "認証URLを開けません。Codex CLIから認証を完了してください。".into(),
                );
                self.status = "認証URLを開けませんでした。".into();
                self.auth_polling = false;
            } else {
                self.auth_polling = true;
                self.request_read("認証完了を確認しています…");
                self.last_poll = Instant::now();
            }
        } else {
            if !self.bridge.send(AccountCommand::Login) {
                self.bridge = AppServerBridge::<AccountCommand, Event>::start();
                if !self.bridge.send(AccountCommand::Login) {
                    self.apply_account_error(
                        "Codex app-serverへ認証要求を送信できませんでした。".into(),
                    );
                    return;
                }
            }
            self.checking = true;
            self.auth_polling = false;
            self.status = "認証URLを発行しています…".into();
        }
    }

    fn sync_ui(&self, ui: &MainWindow) {
        let remaining = self
            .remaining_percent
            .map(|remaining| remaining.clamp(0.0, 100.0))
            .unwrap_or(0.0);
        let seconds = self.seconds_to_reset();
        let period_seconds = self
            .reset_at
            .map(|reset_at| self.period_seconds_for_reset(reset_at))
            .unwrap_or(self.window_seconds.max(WEEK_SECONDS));
        ui.set_authenticated(self.authenticated);
        ui.set_strings(ui_strings(&self.i18n));
        ui.set_has_usage(self.has_usage);
        ui.set_has_auth_url(self.auth_url.is_some());
        ui.set_checking(self.checking);
        ui.set_has_error(self.error.is_some());
        ui.set_window_title(native_account_window_title(&self.window_title()).into());
        let quota_title = if self.monthly {
            self.i18n.text(TextKey::MonthlyQuotaRemaining)
        } else if self.quota_title == "利用枠" {
            self.i18n.text(TextKey::UsageLimit)
        } else {
            self.i18n.text(TextKey::QuotaRemaining)
        };
        ui.set_quota_title(
            security::shorten_unicode(quota_title, security::MAX_LIMIT_NAME_SCALARS).into(),
        );
        ui.set_has_quota_percent(self.has_quota_percent);
        ui.set_remaining_label(
            if self.has_quota_percent {
                format_percent(remaining)
            } else {
                self.i18n.text(TextKey::FixedLimitNone).into()
            }
            .into(),
        );
        ui.set_week_label(if self.has_quota_percent {
            self.i18n
                .format_period_remaining(
                    seconds,
                    if self.monthly {
                        PeriodKind::Monthly
                    } else {
                        PeriodKind::Weekly
                    },
                )
                .into()
        } else {
            "".into()
        });
        let (
            model_names,
            input_tokens,
            input_costs,
            cached_tokens,
            cached_costs,
            output_tokens,
            output_costs,
        ) = format_model_usage_columns(&self.model_usage);
        ui.set_has_model_usage(!model_names.is_empty());
        ui.set_model_usage_names(model_names.into());
        ui.set_model_usage_input_tokens(input_tokens.into());
        ui.set_model_usage_input_costs(input_costs.into());
        ui.set_model_usage_cached_tokens(cached_tokens.into());
        ui.set_model_usage_cached_costs(cached_costs.into());
        ui.set_model_usage_output_tokens(output_tokens.into());
        ui.set_model_usage_output_costs(output_costs.into());
        ui.set_model_usage_period(self.model_usage_period().into());
        let estimate = if self.model_usage.is_empty() {
            format!("{} —", self.i18n.text(TextKey::EstimatePrefix))
        } else {
            let total = self
                .model_usage
                .iter()
                .map(ModelUsageRow::dollar_costs)
                .map(|(sol, terra, luna)| sol + terra + luna)
                .sum::<f64>();
            self.i18n.format_estimate(total)
        };
        ui.set_estimated_cost_label(estimate.into());
        ui.set_status(self.display_status().into());
        ui.set_status_level(self.status_level().into());
        ui.set_remaining_percent(remaining as f32);
        ui.set_remaining_days(if self.has_quota_percent {
            (seconds.max(0) as f32 / period_seconds.max(1) as f32 * 7.0).clamp(0.0, 7.0)
        } else {
            0.0
        });
        if !self.active_threads.is_empty() {
            ui.set_has_active_thread(true);
            ui.set_active_thread_count(
                i32::try_from(self.active_threads.len()).unwrap_or(i32::MAX),
            );
            ui.set_active_thread_count_label(
                self.i18n
                    .format_thread_count(self.active_threads.len())
                    .into(),
            );
            let [sol, terra, luna, other] = active_thread_model_count_values(&self.active_threads);
            ui.set_active_thread_sol_count(sol);
            ui.set_active_thread_terra_count(terra);
            ui.set_active_thread_luna_count(luna);
            ui.set_active_thread_other_count(other);
        } else {
            ui.set_has_active_thread(false);
            ui.set_active_thread_count(0);
            ui.set_active_thread_sol_count(0);
            ui.set_active_thread_terra_count(0);
            ui.set_active_thread_luna_count(0);
            ui.set_active_thread_other_count(0);
            ui.set_active_thread_count_label(self.i18n.format_thread_count(0).into());
        }
    }

    fn model_usage_period(&self) -> String {
        self.history_periods()
            .into_iter()
            .find(|period| {
                self.reset_at.is_some_and(|reset| {
                    period.canonical_reset_at.abs_diff(reset) <= RESET_AT_TOLERANCE_SECONDS as u64
                })
            })
            .map(|period| period.label)
            .unwrap_or_else(|| "履歴なし".into())
    }

    fn graph_time_labels(&self) -> [String; 5] {
        let Some(reset_at) = self.selected_history_reset() else {
            return Default::default();
        };
        let Some(period) =
            self.history
                .period_for_id(reset_at, Utc::now().timestamp(), self.reset_at)
        else {
            return Default::default();
        };
        let period_start = period.start;
        let period_end = period.end.max(period_start + 1);
        let span = (period_end - period_start).max(1) as f64;
        [0.0, 0.25, 0.5, 0.75, 1.0].map(|fraction| {
            let timestamp = period_start + (span * fraction) as i64;
            self.i18n.format_graph_time(timestamp).unwrap_or_default()
        })
    }
}

#[cfg(test)]
#[allow(clippy::needless_return)]
fn format_period_timestamp(timestamp: i64) -> Option<String> {
    let time = DateTime::from_timestamp(timestamp, 0)?;
    Some(
        time.with_timezone(&chrono_tz::Asia::Tokyo)
            .format("%Y/%m/%d %H:%M:%S JST")
            .to_string(),
    )
}

#[cfg(test)]
fn format_period_label(start: i64, end: i64) -> String {
    let Some(start) = format_period_timestamp(start) else {
        return String::new();
    };
    let Some(end) = format_period_timestamp(end) else {
        return String::new();
    };
    format!("{start} ～ {end}")
}

impl Drop for CodexInfoState {
    fn drop(&mut self) {
        let _ = self.bridge.send(AccountCommand::Stop);
        self.stop_thread_bridge();
        let _ = self.local_bridge.send(LocalCommand::Stop);
    }
}

#[cfg(test)]
fn duration_parts(seconds: i64) -> (i64, i64, i64, i64) {
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    let (hours, rest) = (rest / 3_600, rest % 3_600);
    let (minutes, seconds) = (rest / 60, rest % 60);
    (days, hours, minutes, seconds)
}

#[cfg(test)]
fn week_remaining_text(seconds: i64) -> String {
    let (days, hours, minutes, _) = duration_parts(seconds.max(0));
    if days > 0 {
        format!("7日中、あと{days}日と{hours}時間{minutes}分")
    } else if hours > 0 {
        format!("7日中、あと{hours}時間{minutes}分")
    } else {
        format!("7日中、あと{minutes}分")
    }
}

#[cfg(test)]
fn period_remaining_text(seconds: i64, period_seconds: i64, monthly: bool) -> String {
    if monthly {
        let (days, hours, minutes, _) = duration_parts(seconds.max(0));
        let duration = if days > 0 {
            format!("{days}日と{hours}時間{minutes}分")
        } else if hours > 0 {
            format!("{hours}時間{minutes}分")
        } else if minutes > 0 {
            format!("{minutes}分")
        } else {
            "まもなくリセット".into()
        };
        format!("月間、あと{duration}")
    } else {
        // Keep the established seven-day copy and avoid an unnatural “0日”.
        let _ = period_seconds;
        week_remaining_text(seconds)
    }
}

fn format_percent(value: f64) -> String {
    if value.fract().abs() < 0.0001 {
        format!("{value:.0}%")
    } else {
        format!("{value:.1}%")
    }
}

fn preview_model_row(
    name: &str,
    tokens: u64,
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
) -> ModelUsageRow {
    ModelUsageRow {
        name: name.into(),
        tokens,
        input_tokens,
        cached_input_tokens,
        output_tokens,
    }
}

fn format_model_usage_columns(
    rows: &[ModelUsageRow],
) -> (String, String, String, String, String, String, String) {
    let names = rows.iter().map(|row| row.name.clone()).collect::<Vec<_>>();
    let input_tokens = rows
        .iter()
        .map(|row| format_token_count(row.input_tokens.saturating_sub(row.cached_input_tokens)))
        .collect::<Vec<_>>();
    let input_costs = rows
        .iter()
        .map(|row| format_dollar_cost(row.dollar_costs().0))
        .collect::<Vec<_>>();
    let cached_tokens = rows
        .iter()
        .map(|row| format_token_count(row.cached_input_tokens))
        .collect::<Vec<_>>();
    let cached_costs = rows
        .iter()
        .map(|row| format_dollar_cost(row.dollar_costs().1))
        .collect::<Vec<_>>();
    let output_tokens = rows
        .iter()
        .map(|row| format_token_count(row.output_tokens))
        .collect::<Vec<_>>();
    let output_costs = rows
        .iter()
        .map(|row| format_dollar_cost(row.dollar_costs().2))
        .collect::<Vec<_>>();
    (
        names.join("\n"),
        input_tokens.join("\n"),
        input_costs.join("\n"),
        cached_tokens.join("\n"),
        cached_costs.join("\n"),
        output_tokens.join("\n"),
        output_costs.join("\n"),
    )
}

fn format_dollar_cost(value: f64) -> String {
    format!("${}", value.max(0.0) as u64)
}

fn format_estimated_cost(costs: ModelDollarTotals) -> String {
    let total = [costs.sol, costs.terra, costs.luna]
        .into_iter()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .sum::<f64>();
    let total = if total.is_finite() && total >= 0.0 {
        total.min(u64::MAX as f64).round() as u64
    } else {
        0
    };
    format!("概算 ${}", format_token_count(total))
}

fn format_token_count(value: u64) -> String {
    format_unsigned_count(u128::from(value))
}

fn format_unsigned_count(value: u128) -> String {
    let mut reversed = String::new();
    for (index, character) in value.to_string().chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            reversed.push(',');
        }
        reversed.push(character);
    }
    reversed.chars().rev().collect()
}

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ClientMessageEvent, ConfigureWindowAux, ConnectionExt, EventMask, KeyButMask,
    PropMode, StackMode, Window as X11Window,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as WrapperConnectionExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct X11StateAtoms {
    wm_state: Atom,
    fullscreen: Atom,
    maximized_vert: Atom,
    maximized_horz: Atom,
    active_window: Option<Atom>,
}

struct X11WindowStateMonitor {
    connection: RustConnection,
    root: X11Window,
    atoms: X11StateAtoms,
    motif_wm_hints: Option<Atom>,
}

fn x11_window_id(window: &slint::Window) -> Option<X11Window> {
    let slint_handle = window.window_handle();
    let handle = <slint::WindowHandle as HasWindowHandle>::window_handle(&slint_handle)
        .ok()?
        .as_raw();
    match handle {
        RawWindowHandle::Xlib(handle) => {
            let window = handle.window as u32;
            (window != 0).then_some(window)
        }
        RawWindowHandle::Xcb(handle) => {
            let window = handle.window.get();
            (window != 0).then_some(window)
        }
        _ => None,
    }
}

const MOTIF_HINTS_FUNCTIONS: u32 = 1;
const MOTIF_FUNCTION_ALL: u32 = 1;
const MOTIF_FUNCTION_RESIZE: u32 = 1 << 1;
const MOTIF_FUNCTION_MOVE: u32 = 1 << 2;
const MOTIF_FUNCTION_MINIMIZE: u32 = 1 << 3;
const MOTIF_FUNCTION_MAXIMIZE: u32 = 1 << 4;
const MOTIF_FUNCTION_CLOSE: u32 = 1 << 5;

fn motif_wm_functions(existing_flags: u32) -> (u32, u32) {
    (
        existing_flags | MOTIF_HINTS_FUNCTIONS,
        MOTIF_FUNCTION_MOVE | MOTIF_FUNCTION_MINIMIZE | MOTIF_FUNCTION_CLOSE,
    )
}

fn motif_wm_resizable_functions(existing_flags: u32, existing_functions: u32) -> (u32, u32) {
    let functions = if existing_functions & MOTIF_FUNCTION_ALL == 0 {
        existing_functions
            | MOTIF_FUNCTION_RESIZE
            | MOTIF_FUNCTION_MOVE
            | MOTIF_FUNCTION_MINIMIZE
            | MOTIF_FUNCTION_MAXIMIZE
            | MOTIF_FUNCTION_CLOSE
    } else {
        existing_functions
    };
    (existing_flags | MOTIF_HINTS_FUNCTIONS, functions)
}

fn forbidden_x11_states(states: &[Atom], atoms: &X11StateAtoms) -> (bool, bool) {
    (
        states.contains(&atoms.fullscreen),
        states
            .iter()
            .any(|state| *state == atoms.maximized_vert || *state == atoms.maximized_horz),
    )
}

impl X11WindowStateMonitor {
    fn intern_atom(connection: &RustConnection, name: &[u8]) -> Option<Atom> {
        connection
            .intern_atom(false, name)
            .ok()?
            .reply()
            .ok()
            .map(|reply| reply.atom)
    }

    fn connect() -> Option<Self> {
        let (connection, screen_num) = x11rb::connect(None).ok()?;
        let root = connection.setup().roots.get(screen_num)?.root;
        let atoms = X11StateAtoms {
            wm_state: Self::intern_atom(&connection, b"_NET_WM_STATE")?,
            fullscreen: Self::intern_atom(&connection, b"_NET_WM_STATE_FULLSCREEN")?,
            maximized_vert: Self::intern_atom(&connection, b"_NET_WM_STATE_MAXIMIZED_VERT")?,
            maximized_horz: Self::intern_atom(&connection, b"_NET_WM_STATE_MAXIMIZED_HORZ")?,
            active_window: Self::intern_atom(&connection, b"_NET_ACTIVE_WINDOW"),
        };
        let motif_wm_hints = Self::intern_atom(&connection, b"_MOTIF_WM_HINTS");
        Some(Self {
            connection,
            root,
            atoms,
            motif_wm_hints,
        })
    }

    fn remove_state(&self, window: X11Window, first: Atom, second: Atom) {
        let event =
            ClientMessageEvent::new(32, window, self.atoms.wm_state, [0, first, second, 1, 0]);
        let event_mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
        if self
            .connection
            .send_event(false, self.root, event_mask, event)
            .is_ok()
        {
            let _ = self.connection.flush();
        }
    }

    fn enforce_motif_functions(&self, window: X11Window, motif_wm_hints: Atom) {
        let Ok(cookie) =
            self.connection
                .get_property(false, window, motif_wm_hints, AtomEnum::ANY, 0, 5)
        else {
            return;
        };
        let Ok(reply) = cookie.reply() else {
            return;
        };

        let mut hints = [0_u32; 5];
        match reply.format {
            0 => {}
            32 => {
                let Some(values) = reply.value32() else {
                    return;
                };
                for (hint, value) in hints.iter_mut().zip(values) {
                    *hint = value;
                }
            }
            _ => return,
        }

        let (flags, functions) = motif_wm_functions(hints[0]);
        if hints[0] == flags && hints[1] == functions {
            return;
        }
        hints[0] = flags;
        hints[1] = functions;
        if self
            .connection
            .change_property32(
                PropMode::REPLACE,
                window,
                motif_wm_hints,
                motif_wm_hints,
                &hints,
            )
            .is_ok()
        {
            let _ = self.connection.flush();
        }
    }

    fn allow_resize(&self, window: &slint::Window) {
        let Some(window_id) = x11_window_id(window) else {
            return;
        };
        let Some(motif_wm_hints) = self.motif_wm_hints else {
            return;
        };
        let Ok(cookie) =
            self.connection
                .get_property(false, window_id, motif_wm_hints, AtomEnum::ANY, 0, 5)
        else {
            return;
        };
        let Ok(reply) = cookie.reply() else {
            return;
        };
        let Some(values) = reply.value32() else {
            return;
        };
        let mut hints = [0_u32; 5];
        for (hint, value) in hints.iter_mut().zip(values) {
            *hint = value;
        }
        (hints[0], hints[1]) = motif_wm_resizable_functions(hints[0], hints[1]);
        if self
            .connection
            .change_property32(
                PropMode::REPLACE,
                window_id,
                motif_wm_hints,
                motif_wm_hints,
                &hints,
            )
            .is_ok()
        {
            let _ = self.connection.flush();
        }
    }

    fn enforce(&self, window: &slint::Window) {
        let Some(window_id) = x11_window_id(window) else {
            return;
        };
        if let Some(motif_wm_hints) = self.motif_wm_hints {
            self.enforce_motif_functions(window_id, motif_wm_hints);
        }
        let Ok(cookie) = self.connection.get_property(
            false,
            window_id,
            self.atoms.wm_state,
            AtomEnum::ATOM,
            0,
            u32::MAX,
        ) else {
            return;
        };
        let Ok(reply) = cookie.reply() else {
            return;
        };
        let Some(states) = reply.value32() else {
            return;
        };
        let states: Vec<Atom> = states.collect();
        let (fullscreen, maximized) = forbidden_x11_states(&states, &self.atoms);
        if fullscreen {
            self.remove_state(window_id, self.atoms.fullscreen, 0);
        }
        if maximized {
            self.remove_state(
                window_id,
                self.atoms.maximized_vert,
                self.atoms.maximized_horz,
            );
        }
    }

    fn raise_and_activate(&self, window: &slint::Window) {
        let Some(window_id) = x11_window_id(window) else {
            return;
        };
        // Xwayland/WSLg can place the client inside a compositor-owned
        // wrapper. Raising only the Slint client leaves the main window above
        // a frameless secondary surface, so raise the wrapper as well.
        let raise_target = self
            .connection
            .query_tree(window_id)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|reply| reply.parent)
            .filter(|parent| *parent != self.root)
            .unwrap_or(window_id);
        let stack_mode = ConfigureWindowAux::new().stack_mode(StackMode::ABOVE);
        let _ = self.connection.configure_window(raise_target, &stack_mode);
        if raise_target != window_id {
            let _ = self.connection.configure_window(window_id, &stack_mode);
        }

        if let Some(active_window) = self.atoms.active_window {
            let event_mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
            let event = ClientMessageEvent::new(
                32,
                window_id,
                active_window,
                [1, x11rb::CURRENT_TIME, 0, 0, 0],
            );
            let _ = self
                .connection
                .send_event(false, self.root, event_mask, event);
        }
        let _ = self.connection.flush();
    }
}

/// Parses the visual-review size override without applying window-specific
/// bounds. Invalid values leave the Slint defaults untouched; the graph
/// preview applies its minimum dimensions after parsing.
fn parse_preview_size(value: Option<&str>) -> Option<(u32, u32)> {
    let value = value?.trim();
    let (width, height) = value.split_once('x')?;
    if width.is_empty() || height.is_empty() || height.contains('x') {
        return None;
    }
    let width = width.parse::<u32>().ok()?;
    let height = height.parse::<u32>().ok()?;
    Some((width, height))
}

fn clamp_graph_preview_size((width, height): (u32, u32)) -> (u32, u32) {
    (width.max(700), height.max(480))
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    install_fixed_window_guard(ui.window());
    let preview_size = std::env::var("CODEX_INFO_PREVIEW_SIZE")
        .ok()
        .and_then(|value| parse_preview_size(Some(value.as_str())));
    let graph_preview_size = preview_size.map(clamp_graph_preview_size);
    let preview_kind = std::env::var("CODEX_INFO_PREVIEW").ok();
    let state = Rc::new(RefCell::new(
        preview_kind
            .clone()
            .map(|kind| CodexInfoState::preview(&kind))
            .unwrap_or_else(CodexInfoState::new),
    ));
    // One graph window owns the three model toggles. The initial state keeps
    // every series enabled, preserving the combined cumulative view.
    let graph_window = Rc::new(RefCell::new(None::<GraphWindow>));
    let threads_window = Rc::new(RefCell::new(None::<ThreadsWindow>));
    let legal_notice_window = Rc::new(RefCell::new(None::<LegalNoticeWindow>));
    let x11_monitor = Rc::new(X11WindowStateMonitor::connect());

    {
        let weak_ui = ui.as_weak();
        ui.on_begin_window_drag(move || {
            if let Some(ui) = weak_ui.upgrade() {
                begin_window_drag(ui.window());
            }
        });
    }
    {
        let weak_ui = ui.as_weak();
        ui.on_minimize_window(move || {
            if let Some(ui) = weak_ui.upgrade() {
                minimize_window(ui.window());
            }
        });
    }
    {
        let weak_ui = ui.as_weak();
        ui.on_close_window(move || {
            if let Some(ui) = weak_ui.upgrade() {
                let _ = ui.hide();
            }
            let _ = slint::quit_event_loop();
        });
    }

    {
        let state = Rc::clone(&state);
        ui.on_begin_auth(move || state.borrow_mut().open_auth());
    }
    {
        let state = Rc::clone(&state);
        ui.on_check_auth(move || state.borrow_mut().request_read("認証状態を確認しています…"));
    }
    {
        let state = Rc::clone(&state);
        ui.on_retry(move || state.borrow_mut().request_read("利用状況を更新しています…"));
    }
    {
        let state = Rc::clone(&state);
        let graph_window = Rc::clone(&graph_window);
        let x11_monitor = Rc::clone(&x11_monitor);
        let graph_old_preview = preview_kind.as_deref() == Some("graph-old");
        let graph_period_preview =
            matches!(preview_kind.as_deref(), Some("graph-period" | "graph-many"));
        ui.on_open_graph(move || {
            if !graph_old_preview {
                state.borrow_mut().select_latest_history();
            }
            let mut graph_window = graph_window.borrow_mut();
            if graph_window.is_none() {
                if let Ok(graph) = GraphWindow::new() {
                    graph.set_open_history_on_start(graph_period_preview);
                    let (graph_width, graph_height) =
                        graph_preview_size.unwrap_or((GRAPH_WINDOW_WIDTH, GRAPH_WINDOW_HEIGHT));
                    if graph_preview_size.is_some() {
                        graph.window().set_size(slint::LogicalSize::new(
                            graph_width as f32,
                            graph_height as f32,
                        ));
                    }
                    install_resizable_window(graph.window());
                    let weak_graph = graph.as_weak();
                    graph.on_begin_window_drag(move || {
                        if let Some(graph) = weak_graph.upgrade() {
                            begin_window_drag(graph.window());
                        }
                    });
                    let weak_graph = graph.as_weak();
                    graph.on_begin_window_resize(move |direction| {
                        if let Some(graph) = weak_graph.upgrade() {
                            begin_window_resize(graph.window(), direction.as_str());
                        }
                    });
                    let weak_graph = graph.as_weak();
                    graph.on_minimize_window(move || {
                        if let Some(graph) = weak_graph.upgrade() {
                            minimize_window(graph.window());
                        }
                    });
                    let weak_graph = graph.as_weak();
                    graph.on_toggle_maximize_window(move || {
                        if let Some(graph) = weak_graph.upgrade() {
                            toggle_maximize_window(graph.window());
                        }
                    });
                    let weak_graph = graph.as_weak();
                    graph.on_close_graph(move || {
                        if let Some(graph) = weak_graph.upgrade() {
                            graph.set_reset_close_buttons(true);
                            graph.set_reset_close_buttons(false);
                            let _ = graph.hide();
                        }
                    });
                    let weak_graph = graph.as_weak();
                    graph.on_close_window(move || {
                        if let Some(graph) = weak_graph.upgrade() {
                            graph.set_reset_close_buttons(true);
                            graph.set_reset_close_buttons(false);
                            let _ = graph.hide();
                        }
                    });
                    let weak_graph = graph.as_weak();
                    graph.window().on_close_requested(move || {
                        if let Some(graph) = weak_graph.upgrade() {
                            graph.set_reset_close_buttons(true);
                            graph.set_reset_close_buttons(false);
                            if graph.hide().is_ok() {
                                return CloseRequestResponse::KeepWindowShown;
                            }
                        }
                        CloseRequestResponse::HideWindow
                    });
                    let weak_graph = graph.as_weak();
                    let state_for_toggle = Rc::clone(&state);
                    graph.on_toggle_remaining(move || {
                        if let Some(graph) = weak_graph.upgrade() {
                            graph.set_show_remaining(!graph.get_show_remaining());
                            sync_graph_window(&state_for_toggle.borrow(), &graph);
                        }
                    });
                    let weak_graph = graph.as_weak();
                    let state_for_metric = Rc::clone(&state);
                    graph.on_select_metric(move |metric| {
                        if let Some(graph) = weak_graph.upgrade() {
                            state_for_metric.borrow_mut().select_metric(&metric);
                            sync_graph_window(&state_for_metric.borrow(), &graph);
                        }
                    });
                    let weak_graph = graph.as_weak();
                    let state_for_toggle = Rc::clone(&state);
                    graph.on_toggle_luna(move || {
                        if let Some(graph) = weak_graph.upgrade() {
                            graph.set_show_luna(!graph.get_show_luna());
                            sync_graph_window(&state_for_toggle.borrow(), &graph);
                        }
                    });
                    let weak_graph = graph.as_weak();
                    let state_for_toggle = Rc::clone(&state);
                    graph.on_toggle_terra(move || {
                        if let Some(graph) = weak_graph.upgrade() {
                            graph.set_show_terra(!graph.get_show_terra());
                            sync_graph_window(&state_for_toggle.borrow(), &graph);
                        }
                    });
                    let weak_graph = graph.as_weak();
                    let state_for_toggle = Rc::clone(&state);
                    graph.on_toggle_sol(move || {
                        if let Some(graph) = weak_graph.upgrade() {
                            graph.set_show_sol(!graph.get_show_sol());
                            sync_graph_window(&state_for_toggle.borrow(), &graph);
                        }
                    });
                    let weak_graph = graph.as_weak();
                    let state_for_history = Rc::clone(&state);
                    graph.on_select_history(move |label| {
                        if let Some(graph) = weak_graph.upgrade() {
                            state_for_history
                                .borrow_mut()
                                .select_history(label.as_str());
                            sync_graph_window(&state_for_history.borrow(), &graph);
                        }
                    });
                    *graph_window = Some(graph);
                }
            }
            if let Some(graph) = graph_window.as_ref() {
                graph.set_reset_close_buttons(true);
                graph.set_reset_close_buttons(false);
                sync_graph_window(&state.borrow(), graph);
                let _ = show_and_focus_window(graph.window(), x11_monitor.as_ref().as_ref());
                if let Some(monitor) = x11_monitor.as_ref() {
                    monitor.allow_resize(graph.window());
                }
            }
        });
    }
    {
        let state = Rc::clone(&state);
        let threads_window = Rc::clone(&threads_window);
        let x11_monitor = Rc::clone(&x11_monitor);
        ui.on_open_threads(move || {
            let mut threads_window = threads_window.borrow_mut();
            if threads_window.is_none() {
                if let Ok(window) = ThreadsWindow::new() {
                    install_fixed_window_guard(window.window());
                    let weak_window = window.as_weak();
                    window.on_begin_window_drag(move || {
                        if let Some(window) = weak_window.upgrade() {
                            begin_window_drag(window.window());
                        }
                    });
                    let weak_window = window.as_weak();
                    window.on_minimize_window(move || {
                        if let Some(window) = weak_window.upgrade() {
                            minimize_window(window.window());
                        }
                    });
                    let weak_window = window.as_weak();
                    window.on_close_threads(move || {
                        if let Some(window) = weak_window.upgrade() {
                            window.set_reset_close_buttons(true);
                            window.set_reset_close_buttons(false);
                            let _ = window.hide();
                        }
                    });
                    let weak_window = window.as_weak();
                    window.on_close_window(move || {
                        if let Some(window) = weak_window.upgrade() {
                            window.set_reset_close_buttons(true);
                            window.set_reset_close_buttons(false);
                            let _ = window.hide();
                        }
                    });
                    let weak_window = window.as_weak();
                    window.window().on_close_requested(move || {
                        if let Some(window) = weak_window.upgrade() {
                            window.set_reset_close_buttons(true);
                            window.set_reset_close_buttons(false);
                            if window.hide().is_ok() {
                                return CloseRequestResponse::KeepWindowShown;
                            }
                        }
                        CloseRequestResponse::HideWindow
                    });
                    *threads_window = Some(window);
                }
            }
            if let Some(window) = threads_window.as_ref() {
                window.set_reset_close_buttons(true);
                window.set_reset_close_buttons(false);
                sync_threads_window(&state.borrow(), window);
                let _ = show_and_focus_window(window.window(), x11_monitor.as_ref().as_ref());
            }
        });
    }
    {
        let state = Rc::clone(&state);
        let legal_notice_window = Rc::clone(&legal_notice_window);
        let x11_monitor = Rc::clone(&x11_monitor);
        ui.on_open_legal_notice(move || {
            let mut legal_notice_window = legal_notice_window.borrow_mut();
            if legal_notice_window.is_none() {
                if let Ok(window) = LegalNoticeWindow::new() {
                    install_window_size_guard(
                        window.window(),
                        LEGAL_WINDOW_WIDTH,
                        LEGAL_WINDOW_HEIGHT,
                    );
                    let weak_window = window.as_weak();
                    window.on_begin_window_drag(move || {
                        if let Some(window) = weak_window.upgrade() {
                            begin_window_drag(window.window());
                        }
                    });
                    let weak_window = window.as_weak();
                    window.on_minimize_window(move || {
                        if let Some(window) = weak_window.upgrade() {
                            minimize_window(window.window());
                        }
                    });
                    let weak_window = window.as_weak();
                    window.on_close_legal_notice(move || {
                        if let Some(window) = weak_window.upgrade() {
                            window.set_reset_close_buttons(true);
                            window.set_reset_close_buttons(false);
                            let _ = window.hide();
                        }
                    });
                    let weak_window = window.as_weak();
                    window.on_close_window(move || {
                        if let Some(window) = weak_window.upgrade() {
                            window.set_reset_close_buttons(true);
                            window.set_reset_close_buttons(false);
                            let _ = window.hide();
                        }
                    });
                    let weak_window = window.as_weak();
                    window.window().on_close_requested(move || {
                        if let Some(window) = weak_window.upgrade() {
                            window.set_reset_close_buttons(true);
                            window.set_reset_close_buttons(false);
                            if window.hide().is_ok() {
                                return CloseRequestResponse::KeepWindowShown;
                            }
                        }
                        CloseRequestResponse::HideWindow
                    });
                    *legal_notice_window = Some(window);
                }
            }
            if let Some(window) = legal_notice_window.as_ref() {
                let state_ref = state.borrow();
                window.set_strings(ui_strings(&state_ref.i18n));
                window.set_window_title(
                    native_detail_window_title(
                        &state_ref.i18n,
                        state_ref.authenticated,
                        &state_ref.window_title(),
                        WindowPurpose::Legal,
                    )
                    .into(),
                );
                window.set_reset_close_buttons(true);
                window.set_reset_close_buttons(false);
                let _ = show_and_focus_window(window.window(), x11_monitor.as_ref().as_ref());
            }
        });
    }

    if matches!(
        preview_kind.as_deref(),
        Some("graph" | "graph-old" | "graph-many" | "graph-period")
    ) {
        if preview_kind.as_deref() == Some("graph-old") {
            state.borrow_mut().select_latest_history();
            state.borrow_mut().select_older_history();
        }
        ui.invoke_open_graph();
    }
    if matches!(
        preview_kind.as_deref(),
        Some("multi-thread" | "single-thread")
    ) {
        ui.invoke_open_threads();
    }
    if preview_kind.as_deref() == Some("legal") {
        ui.invoke_open_legal_notice();
    }

    state.borrow().sync_ui(&ui);
    let weak_ui_for_bounds = ui.as_weak();
    let monitor = Rc::clone(&x11_monitor);
    let graph_window_for_resize = Rc::clone(&graph_window);
    let threads_window_for_bounds = Rc::clone(&threads_window);
    let legal_notice_window_for_bounds = Rc::clone(&legal_notice_window);
    let main_monitor_timer = Timer::default();
    main_monitor_timer.start(TimerMode::Repeated, Duration::from_millis(100), move || {
        if let Some(ui) = weak_ui_for_bounds.upgrade() {
            if let Some(monitor) = monitor.as_ref() {
                monitor.enforce(ui.window());
                if let Some(graph) = graph_window_for_resize.borrow().as_ref() {
                    if graph.window().is_visible() {
                        monitor.allow_resize(graph.window());
                    }
                }
                if let Some(window) = threads_window_for_bounds.borrow().as_ref() {
                    if window.window().is_visible() {
                        monitor.enforce(window.window());
                    }
                }
                if let Some(window) = legal_notice_window_for_bounds.borrow().as_ref() {
                    if window.window().is_visible() {
                        monitor.enforce(window.window());
                    }
                }
            }
        }
    });
    let weak_ui = ui.as_weak();
    let graph_window_for_timer = Rc::clone(&graph_window);
    let threads_window_for_timer = Rc::clone(&threads_window);
    let timer = Timer::default();
    if !state.borrow().preview {
        timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
            if let Some(ui) = weak_ui.upgrade() {
                let mut state = state.borrow_mut();
                state.poll();
                let refresh_interval =
                    automatic_refresh_interval(state.authenticated, state.auth_polling);
                if !state.checking && state.last_poll.elapsed() >= refresh_interval {
                    let status = if state.auth_polling && !state.authenticated {
                        "認証完了を確認しています…"
                    } else {
                        "利用状況を更新しています…"
                    };
                    state.request_read(status);
                    state.last_poll = Instant::now();
                }
                if state.authenticated
                    && !state.thread_checking
                    && state.last_thread_poll.elapsed() >= Duration::from_secs(5)
                {
                    state.request_thread_update();
                }
                state.sync_ui(&ui);
                if let Some(graph) = graph_window_for_timer.borrow().as_ref() {
                    if graph.window().is_visible() {
                        sync_graph_window(&state, graph);
                    }
                }
                if let Some(window) = threads_window_for_timer.borrow().as_ref() {
                    if window.window().is_visible() {
                        sync_threads_window(&state, window);
                    }
                }
            }
        });
    }
    ui.run()
}

#[cfg(test)]
mod tests {
    use super::winit;
    use super::{
        account_window_title, active_thread_model_counts, active_thread_rows_at,
        add_recovery_usage, automatic_refresh_interval, clamp_graph_preview_size,
        collapse_remaining_change_points, collect_session_file, complete_rollout_prefix_len,
        current_label_connector_path, detail_window_title, fetch_active_thread_update_for_paths,
        fetch_active_thread_update_for_paths_and_state, fixed_resize_decision, format_elapsed,
        format_estimated_cost, format_model_usage_columns, format_percent, format_period_label,
        graph_paths, graph_paths_for_selection, graph_period_end, graph_points,
        graph_time_endpoints, minute_model_spend, minute_model_spend_for_metric,
        model_usage_timeline_from_events, monthly_window_seconds, native_account_window_title,
        normal_status_text, open_codex_session_paths, parse_preview_size, parse_rate_limits,
        parse_resize_direction, period_remaining_text, plan_type_label, preview_model_row,
        read_recovery_entries, recovery_timed_usage, remaining_graph_y, remaining_marker_positions,
        remaining_marker_positions_on_points, request_with_timeout, same_rollout_identity,
        separate_current_label_positions, session_event_model, session_event_type,
        session_jsonl_files, session_token_snapshot, smooth_model_spend, smooth_remaining_points,
        split_metric_line_paths, stacked_area_path, thread_presentation_rows,
        three_months_before_utc, unused_interval_positions, week_remaining_text, ActiveThread,
        ActiveThreadUpdate, CodexInfoState, Event, FixedResizeDecision, GraphPaths, GraphWindow,
        HourlyModelSpend, LocalUsageResult, ManualX11Geometry, ManualX11WindowAction,
        ModelDollarTotals, ModelTokenTotals, ModelUsageRow, ModelUsageTotals, RpcReadEvent,
        SessionTraversalBudget, TokenSnapshot, UnusedIntervalPosition, UsageEvent, UsageHistory,
        UsageHistorySample, UsageStore, FIXED_WINDOW_HEIGHT, FIXED_WINDOW_WIDTH,
        GRAPH_METRIC_OPTIONS, GRAPH_WINDOW_PURPOSE, LOCAL_ESTIMATE_PRICE_VERSION,
        THREADS_WINDOW_PURPOSE, UNAUTHENTICATED_WINDOW_TITLE, WEEK_SECONDS,
    };
    use super::{
        claim_manual_x11_action, forbidden_x11_states, manual_resize_geometry,
        manual_window_geometry, motif_wm_functions, motif_wm_resizable_functions, X11StateAtoms,
    };
    use chrono::{TimeZone, Utc};
    use codex_info::thread_contract;
    use rusqlite::Connection;
    use serde_json::{json, Value};
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs::{self, File};
    use std::io::{BufReader, Read, Seek, SeekFrom, Write};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn enterprise_individual_limit_wins_and_uses_calendar_month() {
        let reset_at = 1_735_689_600;
        let fixture = json!({
            "rateLimits": {
                "planType": "enterprise",
                "primary": {"usedPercent": 12, "resetsAt": reset_at + 604800, "windowDurationMins": 10080},
                "individualLimit": {
                    "remainingPercent": 73,
                    "resetsAt": reset_at,
                    "limit": "1000000",
                    "used": "270000"
                }
            }
        });
        let parsed = parse_rate_limits(&fixture, Some("enterprise"), reset_at - 86_400)
            .expect("individualLimit fixture should parse");
        assert_eq!(parsed.remaining_percent, Some(73.0));
        assert!(parsed.monthly);
        assert_eq!(parsed.quota_title, "月間残り利用枠");
        assert_eq!(parsed.window_seconds, monthly_window_seconds(reset_at));
        assert_ne!(parsed.window_seconds, 7 * 86_400);
    }

    #[test]
    fn individual_limit_is_monthly_only_for_exact_enterprise_plans() {
        let monthly_reset_at = 1_735_689_600;
        let fixed_reset_at = monthly_reset_at + 604_800;
        for plan in [
            "enterprise",
            "ent26",
            "enterprise_cbp_automation",
            "enterprise_cbp_usage_based",
        ] {
            let fixture = json!({
                "rateLimits": {
                    "planType": plan,
                    "individualLimit": {
                        "remainingPercent": 73,
                        "resetsAt": monthly_reset_at,
                        "limit": "100",
                        "used": "27"
                    },
                    "primary": {
                        "usedPercent": 12,
                        "resetsAt": fixed_reset_at,
                        "windowDurationMins": 10080
                    }
                }
            });
            let parsed = parse_rate_limits(&fixture, Some(plan), 0)
                .expect("exact enterprise plan should parse individualLimit");
            assert!(parsed.monthly, "plan={plan}");
            assert_eq!(parsed.reset_at, monthly_reset_at, "plan={plan}");
            assert_eq!(parsed.remaining_percent, Some(73.0), "plan={plan}");
        }

        for plan in ["business", "self_serve_business_prolite", "pro"] {
            let fixture = json!({
                "rateLimits": {
                    "planType": plan,
                    "individualLimit": {
                        "remainingPercent": 73,
                        "resetsAt": monthly_reset_at,
                        "limit": "100",
                        "used": "27"
                    },
                    "primary": {
                        "usedPercent": 12,
                        "resetsAt": fixed_reset_at,
                        "windowDurationMins": 10080
                    }
                }
            });
            let parsed = parse_rate_limits(&fixture, Some(plan), 0)
                .expect("a fixed bucket should remain available for non-enterprise plans");
            assert!(!parsed.monthly, "plan={plan}");
            assert_eq!(parsed.remaining_percent, Some(88.0), "plan={plan}");
            assert_eq!(parsed.reset_at, fixed_reset_at, "plan={plan}");
            assert_eq!(parsed.window_seconds, 10080 * 60, "plan={plan}");
        }

        let alias = json!({"rateLimits": {"planType": "enterprise"}});
        assert!(
            parse_rate_limits(&alias, Some("chatgpt-enterprise"), 0).is_err(),
            "schema-external aliases must not be normalized"
        );
    }

    #[test]
    fn fixed_rate_limit_chooses_the_longest_valid_secondary_bucket() {
        let fixture = json!({
            "rateLimits": {
                "limitName": "Codex",
                "primary": {"usedPercent": 20, "resetsAt": 3000, "windowDurationMins": 10080},
                "secondary": {"usedPercent": 30, "resetsAt": 4000, "windowDurationMins": 43200}
            },
            "rateLimitsByLimitId": {
                "ignored": {"primary": {"usedPercent": 0, "resetsAt": 9000, "windowDurationMins": 527040}}
            }
        });
        let parsed =
            parse_rate_limits(&fixture, Some("pro"), 0).expect("fixed bucket should parse");
        assert_eq!(parsed.remaining_percent, Some(70.0));
        assert_eq!(parsed.reset_at, 4000);
        assert_eq!(parsed.window_seconds, 43200 * 60);
    }

    #[test]
    fn quota_candidate_tie_break_order_is_total_and_deterministic() {
        let fixed = json!({"rateLimits": {
            "limitName": "Codex",
            "primary":{"usedPercent":5, "resetsAt":8000, "windowDurationMins":10080},
            "secondary":{"usedPercent":3, "resetsAt":8000, "windowDurationMins":10080}
        }});
        let selected = parse_rate_limits(&fixed, Some("pro"), 0).unwrap();
        assert_eq!(selected.window_seconds, 10_080 * 60);
        assert_eq!(selected.reset_at, 8000);
        assert_eq!(selected.limit_name, "Codex");
        assert_eq!(selected.remaining_percent, Some(95.0));

        let later_secondary = json!({"rateLimits": {
            "primary":{"usedPercent":5, "resetsAt":8000, "windowDurationMins":10080},
            "secondary":{"usedPercent":3, "resetsAt":8001, "windowDurationMins":10080}
        }});
        assert_eq!(
            parse_rate_limits(&later_secondary, Some("pro"), 0)
                .unwrap()
                .remaining_percent,
            Some(97.0)
        );
    }

    #[test]
    fn local_estimate_price_version_cost_rounding_and_large_tokens_are_fixed() {
        assert_eq!(LOCAL_ESTIMATE_PRICE_VERSION, "LOCAL_ESTIMATE_V1_2026-08-14");
        let rows = [
            ModelUsageRow {
                name: "SOL".into(),
                tokens: 3_000_000,
                input_tokens: 2_000_000,
                cached_input_tokens: 1_000_000,
                output_tokens: 1_000_000,
            },
            ModelUsageRow {
                name: "TERRA".into(),
                tokens: 3_000_000,
                input_tokens: 2_000_000,
                cached_input_tokens: 1_000_000,
                output_tokens: 1_000_000,
            },
            ModelUsageRow {
                name: "LUNA".into(),
                tokens: 3_000_000,
                input_tokens: 2_000_000,
                cached_input_tokens: 1_000_000,
                output_tokens: 1_000_000,
            },
        ];
        let totals = ModelDollarTotals::from_rows(&rows);
        assert_eq!(totals.sol, 35.5);
        assert_eq!(totals.terra, 14.2);
        assert!((totals.luna - 1.42).abs() < f64::EPSILON);
        assert_eq!(format_estimated_cost(totals), "概算 $51");
        assert_eq!(
            format_estimated_cost(ModelDollarTotals {
                sol: 1_234.5,
                terra: 0.0,
                luna: 0.0,
            }),
            "概算 $1,235"
        );
        assert_eq!(
            format_estimated_cost(ModelDollarTotals::default()),
            "概算 $0"
        );

        let maximum = ModelUsageRow {
            name: "SOL".into(),
            tokens: u64::MAX,
            input_tokens: u64::MAX,
            cached_input_tokens: 0,
            output_tokens: u64::MAX,
        };
        let costs = maximum.dollar_costs();
        assert!(costs.0.is_finite() && costs.2.is_finite());
        assert!(
            format_estimated_cost(ModelDollarTotals::from_rows(&[maximum])).starts_with("概算 $")
        );

        let mut state = CodexInfoState::preview("normal");
        let before = state.estimated_cost_label.clone();
        state.select_latest_history();
        state.select_older_history();
        assert_eq!(state.estimated_cost_label, before);
    }

    #[test]
    fn unlimited_credits_never_create_a_fake_percentage() {
        let fixture = json!({"rateLimits": {
            "credits": {"hasCredits": false, "unlimited": true, "balance": null}
        }});
        let parsed =
            parse_rate_limits(&fixture, Some("enterprise"), 0).expect("unlimited should parse");
        assert_eq!(parsed.remaining_percent, None);
        assert_eq!(parsed.quota_title, "利用枠");
    }

    #[test]
    fn enterprise_plan_variants_have_a_single_japanese_display_name() {
        for plan in [
            "enterprise",
            "ent26",
            "enterprise_cbp_automation",
            "enterprise_cbp_usage_based",
        ] {
            assert_eq!(plan_type_label(Some(plan)), "エンタープライズ");
        }
        for plan in [
            "chatgpt_enterprise",
            "enterprise_trial",
            "enterprise_customer",
            "enterprise-edu",
        ] {
            assert_eq!(plan_type_label(Some(plan)), "プラン未設定");
        }
    }

    #[test]
    fn codex_plan_values_have_stable_display_labels() {
        for (plan, expected) in [
            ("free", "無料"),
            ("go", "Go"),
            ("plus", "Plus"),
            ("pro", "Pro"),
            ("prolite", "Pro Lite"),
            ("team", "Team"),
            ("self_serve_business_prolite", "Business"),
            ("self_serve_business_usage_based", "Business"),
            ("business", "Business"),
            ("ent26", "エンタープライズ"),
            ("enterprise_cbp_automation", "エンタープライズ"),
            ("enterprise_cbp_usage_based", "エンタープライズ"),
            ("enterprise", "エンタープライズ"),
            ("edu", "教育"),
            ("unknown", "プラン未設定"),
        ] {
            assert_eq!(plan_type_label(Some(plan)), expected, "plan={plan}");
        }
        assert_eq!(plan_type_label(Some("chatgpt-plus")), "プラン未設定");
        assert_eq!(plan_type_label(Some("chatgpt-business")), "プラン未設定");
    }

    #[test]
    fn plan_normalization_and_monthly_boundary_matrix() {
        for plan in [
            None,
            Some(""),
            Some("unknown-plan"),
            Some("エンタープライズ"),
            Some(" \tenterprise\r\n"),
            Some("ENTERPRISE"),
            Some("Pro-Lite"),
        ] {
            assert_eq!(plan_type_label(plan), "プラン未設定", "plan={plan:?}");
        }

        let leap_month_end = Utc.with_ymd_and_hms(2024, 3, 31, 12, 0, 0).unwrap();
        let previous = Utc.with_ymd_and_hms(2024, 2, 29, 12, 0, 0).unwrap();
        assert_eq!(
            monthly_window_seconds(leap_month_end.timestamp()),
            (leap_month_end - previous).num_seconds()
        );
    }

    #[test]
    fn rate_limit_parser_projects_31_used_to_69_remaining_without_fallback() {
        let fixture = json!({
            "rateLimits": {
                "limitName": "Codex weekly",
                "primary": {
                    "usedPercent": 31,
                    "resetsAt": 2000,
                    "windowDurationMins": 10080
                }
            },
            "rateLimitsByLimitId": {
                "codex": {
                    "primary": {
                        "usedPercent": 0,
                        "resetsAt": 9000,
                        "windowDurationMins": 10080
                    }
                }
            }
        });
        let parsed = parse_rate_limits(&fixture, Some("pro"), 0)
            .expect("canonical fixed bucket should parse");
        assert_eq!(parsed.remaining_percent, Some(69.0));
        assert_eq!(parsed.limit_name, "Codex weekly");
        assert_eq!(parsed.quota_title, "残り利用枠");

        let invalid_numeric_string = json!({"rateLimits": {
            "primary": {"usedPercent": "31", "resetsAt": 2000, "windowDurationMins": 10080}
        }});
        assert!(
            parse_rate_limits(&invalid_numeric_string, Some("pro"), 0).is_err(),
            "schema-invalid input must be unavailable, never synthesized as 100% remaining"
        );
    }

    fn usage_event(remaining_percent: Option<f64>, reset_at: i64) -> UsageEvent {
        UsageEvent {
            remaining_percent,
            reset_at,
            window_seconds: 604_800,
            limit_name: "Codex".into(),
            quota_title: "残り利用枠".into(),
            monthly: false,
        }
    }

    #[test]
    fn quota_projection_and_thread_state_transitions_are_atomic() {
        let mut state = CodexInfoState::preview("normal");
        let previous = state.active_threads.clone();
        let reset_at = state.reset_at.expect("preview quota has reset");
        let history_before = state.history.samples.clone();

        state.apply_usage_event(usage_event(Some(69.0), reset_at));
        assert_eq!(state.remaining_percent, Some(69.0));
        assert_eq!(state.active_threads, previous);
        assert_eq!(state.history.samples, history_before);

        state.apply_thread_result(state.auth_epoch, ActiveThreadUpdate::Failed);
        assert!(state.thread_error);
        assert_eq!(
            state.status,
            "利用枠は更新しました。スレッド表示は前回値を保持しています。"
        );

        let replacement = ActiveThread {
            id: "replacement".into(),
            created_at: Some(60),
            updated_at: 123,
            title: "replacement title".into(),
            model: "gpt-5.6-terra".into(),
            model_label: "gpt-5.6-terra".into(),
            total_tokens: Some(98_765),
            context_window_tokens: None,
            last_user_message_at: Some(120),
            is_subagent: true,
            parent_thread_id: Some("parent".into()),
            depth: Some(1),
        };
        state.apply_thread_result(
            state.auth_epoch,
            ActiveThreadUpdate::Snapshot(vec![replacement.clone()]),
        );
        assert_eq!(state.active_threads, [replacement]);
        assert!(state.error.is_none());

        state.apply_usage_event(usage_event(Some(67.0), reset_at));
        state.apply_thread_result(state.auth_epoch, ActiveThreadUpdate::NoThread);
        assert!(state.active_threads.is_empty());
    }

    #[test]
    fn local_usage_failure_keeps_valid_quota_and_never_invents_zero_history() {
        let mut state = CodexInfoState::preview("normal");
        let reset_at = state.reset_at.expect("preview quota has reset");
        let history_len = state.history.samples.len();
        let previous_cost = state.estimated_cost_label.clone();
        let previous_columns = format_model_usage_columns(&state.model_usage);
        let threads = state.active_threads.clone();

        state.apply_usage_event(usage_event(Some(24.0), reset_at));
        state.apply_thread_result(
            state.auth_epoch,
            ActiveThreadUpdate::Snapshot(threads.clone()),
        );
        state.apply_local_usage_error(state.auth_epoch, reset_at, WEEK_SECONDS);

        assert!(state.has_usage);
        assert_eq!(state.remaining_percent, Some(24.0));
        assert_eq!(state.reset_at, Some(reset_at));
        assert_eq!(state.active_threads, threads);
        assert_eq!(state.estimated_cost_label, previous_cost);
        assert_eq!(
            format_model_usage_columns(&state.model_usage),
            previous_columns
        );
        assert_eq!(state.history.samples.len(), history_len);
        assert!(state.local_usage_error);
        assert_eq!(
            state.status,
            "利用枠は更新しました。履歴は前回値を保持しています。"
        );

        state.model_usage.clear();
        state.estimated_cost_label = "概算 —".into();
        state.history = UsageHistory::default();
        state.reset_at = None;
        let next_reset = reset_at + WEEK_SECONDS;
        state.apply_usage_event(usage_event(Some(24.0), next_reset));
        state.apply_local_usage_error(state.auth_epoch, next_reset, WEEK_SECONDS);
        assert!(state.has_usage);
        assert_eq!(state.remaining_percent, Some(24.0));
        assert!(state.model_usage.is_empty());
        assert_eq!(state.estimated_cost_label, "概算 —");
        assert!(state.history.samples.is_empty());
    }

    #[test]
    fn quota_event_is_pure_and_account_read_branch_has_no_thread_or_local_calls() {
        let source = include_str!("main.rs");
        let usage_definition = source
            .split_once("struct UsageEvent {")
            .and_then(|(_, rest)| rest.split_once("}\n\nenum Event"))
            .map(|(body, _)| body)
            .expect("UsageEvent definition must remain explicit");
        assert!(!usage_definition.contains("ActiveThread"));
        assert!(!usage_definition.contains("ModelUsage"));
        assert!(!usage_definition.contains("model_cost"));

        let account_worker = source
            .split_once("fn account_server_worker")
            .and_then(|(_, rest)| rest.split_once("fn start_app_server"))
            .map(|(body, _)| body)
            .expect("account worker boundary must remain explicit");
        assert!(!account_worker.contains("fetch_active_thread_update"));
        assert!(!account_worker.contains("collect_local_model_usage"));
        assert!(!account_worker.contains("LocalCommand"));

        let thread_worker = source
            .split_once("fn thread_server_worker")
            .and_then(|(_, rest)| rest.split_once("fn local_usage_worker"))
            .map(|(body, _)| body)
            .expect("thread worker boundary must remain explicit");
        assert!(thread_worker.contains("server.take()"));
        assert!(thread_worker.contains("kill_and_reap()"));
        assert!(thread_worker.contains("next_id = 2"));
    }

    #[test]
    fn thread_failure_preserves_quota_plan_reset_and_history() {
        let mut state = CodexInfoState::preview("normal");
        let reset_at = state.reset_at.expect("preview reset");
        state.apply_usage_event(usage_event(Some(23.0), reset_at));
        let plan = state.plan_label.clone();
        let history = state.history.samples.clone();
        state.thread_checking = true;
        state.apply_thread_error(state.auth_epoch, "thread failure".into());

        assert_eq!(state.remaining_percent, Some(23.0));
        assert_eq!(state.reset_at, Some(reset_at));
        assert_eq!(state.plan_label, plan);
        assert_eq!(state.history.samples, history);
        assert!(state.thread_error);
        assert!(!state.thread_checking);
    }

    #[test]
    fn local_failure_preserves_same_period_quota_and_history() {
        let mut state = CodexInfoState::preview("normal");
        let reset_at = state.reset_at.expect("preview reset");
        state.apply_usage_event(usage_event(Some(23.0), reset_at));
        let plan = state.plan_label.clone();
        let history = state.history.samples.clone();
        let model_usage = state.model_usage.clone();
        let cost = state.estimated_cost_label.clone();
        state.apply_local_usage_error(state.auth_epoch, reset_at, WEEK_SECONDS);

        assert_eq!(state.remaining_percent, Some(23.0));
        assert_eq!(state.reset_at, Some(reset_at));
        assert_eq!(state.plan_label, plan);
        assert_eq!(state.history.samples, history);
        assert_eq!(state.model_usage, model_usage);
        assert_eq!(state.estimated_cost_label, cost);
        assert!(state.local_usage_error);
    }

    #[test]
    fn stale_thread_and_local_results_are_complete_no_ops() {
        let mut state = CodexInfoState::preview("normal");
        let reset_at = state.reset_at.expect("preview reset");
        let old_threads = state.active_threads.clone();
        let old_history = state.history.samples.clone();
        let old_usage = state.model_usage.clone();
        let old_cost = state.estimated_cost_label.clone();
        let old_status = state.status.clone();
        state.auth_epoch = 9;
        state.thread_checking = true;
        state.thread_error = false;
        state.local_usage_error = false;

        state.apply_thread_result(
            8,
            ActiveThreadUpdate::Snapshot(vec![ActiveThread {
                id: "stale".into(),
                ..ActiveThread::default()
            }]),
        );
        state.apply_thread_error(8, "stale thread error".into());
        state.apply_local_usage_success(LocalUsageResult {
            auth_epoch: 8,
            reset_at,
            window_seconds: WEEK_SECONDS,
            model_usage: ModelUsageTotals::default(),
            history_samples: vec![UsageHistorySample::new(
                10,
                reset_at,
                1.0,
                ModelDollarTotals::default(),
            )],
        });
        state.apply_local_usage_error(8, reset_at, WEEK_SECONDS);

        assert_eq!(state.active_threads, old_threads);
        assert_eq!(state.history.samples, old_history);
        assert_eq!(state.model_usage, old_usage);
        assert_eq!(state.estimated_cost_label, old_cost);
        assert_eq!(state.status, old_status);
        assert!(state.thread_checking);
        assert!(!state.thread_error);
        assert!(!state.local_usage_error);
    }

    #[test]
    fn stale_local_result_from_old_period_is_a_no_op() {
        let mut state = CodexInfoState::preview("normal");
        let reset_at = state.reset_at.expect("preview reset");
        let old_history = state.history.samples.clone();
        let old_usage = state.model_usage.clone();
        let old_cost = state.estimated_cost_label.clone();
        state.apply_local_usage_success(LocalUsageResult {
            auth_epoch: state.auth_epoch,
            reset_at: reset_at + WEEK_SECONDS,
            window_seconds: WEEK_SECONDS,
            model_usage: ModelUsageTotals::default(),
            history_samples: vec![UsageHistorySample::new(
                10,
                reset_at + WEEK_SECONDS,
                0.0,
                ModelDollarTotals::default(),
            )],
        });
        state.apply_local_usage_error(state.auth_epoch, reset_at + WEEK_SECONDS, WEEK_SECONDS);
        assert_eq!(state.history.samples, old_history);
        assert_eq!(state.model_usage, old_usage);
        assert_eq!(state.estimated_cost_label, old_cost);
        assert!(!state.local_usage_error);
    }

    #[test]
    fn local_success_is_the_only_path_that_commits_usage_and_history() {
        let mut state = CodexInfoState::preview("normal");
        let reset_at = state.reset_at.expect("preview reset");
        let mut totals = ModelUsageTotals::default();
        totals.add(
            "gpt-5.6-sol",
            TokenSnapshot {
                total: 12,
                input: 8,
                cached_input: 2,
                output: 4,
            },
        );
        state.apply_usage_event(usage_event(Some(23.0), reset_at));
        state.apply_local_usage_success(LocalUsageResult {
            auth_epoch: state.auth_epoch,
            reset_at,
            window_seconds: WEEK_SECONDS,
            model_usage: totals,
            history_samples: vec![UsageHistorySample::new_with_usage(
                20,
                reset_at,
                23.0,
                ModelDollarTotals::default(),
                ModelTokenTotals::default(),
            )],
        });

        assert_eq!(state.model_usage.len(), 1);
        assert_eq!(state.model_usage[0].name, "SOL");
        assert!(state
            .history
            .samples
            .iter()
            .any(|sample| sample.remaining_percent == 23.0));
        assert!(!state.local_usage_error);
    }

    #[test]
    fn clearing_or_changing_authentication_advances_epoch() {
        let mut state = CodexInfoState::preview("normal");
        let initial = state.auth_epoch;
        state.clear_account_visible_state();
        assert_eq!(state.auth_epoch, initial + 1);
        state.clear_account_visible_state();
        assert_eq!(state.auth_epoch, initial + 2);
    }

    #[test]
    fn account_error_does_not_clear_thread_failure_state() {
        let mut state = CodexInfoState::preview("normal");
        state.thread_checking = true;
        state.thread_error = true;
        state.apply_account_error("account failure".into());
        assert!(!state.thread_checking);
        assert!(state.thread_error);

        let account_status = state.status.clone();
        state.apply_thread_result(state.auth_epoch, ActiveThreadUpdate::NoThread);
        assert!(state.account_error.is_some());
        assert!(state.error.is_some());
        assert_eq!(state.status, account_status);
    }

    #[test]
    fn account_error_fences_later_events_from_the_failed_bridge_batch() {
        let mut state = CodexInfoState::preview("normal");
        let previous_remaining = state.remaining_percent;
        let previous_reset = state.reset_at;
        let previous_history = state.history.samples.clone();
        let previous_threads = state.active_threads.clone();

        let restart = state.apply_account_event_batch(vec![
            Event::Error("failed bridge".into()),
            Event::Usage(Box::new(usage_event(Some(100.0), 9_999_999_999))),
        ]);

        assert!(restart);
        assert_eq!(state.remaining_percent, previous_remaining);
        assert_eq!(state.reset_at, previous_reset);
        assert_eq!(state.history.samples, previous_history);
        assert_eq!(state.active_threads, previous_threads);
        assert!(state.account_error.is_some());

        let mut ordered = CodexInfoState::preview("normal");
        let reset_at = ordered.reset_at.expect("preview reset");
        let restart = ordered.apply_account_event_batch(vec![
            Event::Usage(Box::new(usage_event(Some(33.0), reset_at))),
            Event::Error("later failure".into()),
        ]);
        assert!(restart);
        assert_eq!(ordered.remaining_percent, Some(33.0));
        assert!(ordered.account_error.is_some());
    }

    #[test]
    fn account_error_fences_queued_thread_and_local_results_without_clearing_last_valid_values() {
        let mut state = CodexInfoState::preview("normal");
        let stale_epoch = state.auth_epoch;
        let reset_at = state.reset_at.expect("preview reset");
        let remaining = state.remaining_percent;
        let plan = state.plan_label.clone();
        let history = state.history.samples.clone();
        let model_usage = state.model_usage.clone();
        let cost = state.estimated_cost_label.clone();
        let threads = state.active_threads.clone();
        state.thread_checking = true;

        state.apply_account_error("failed account bridge".into());
        let error_status = state.status.clone();

        assert_eq!(state.auth_epoch, stale_epoch + 1);
        assert!(!state.thread_checking);
        assert_eq!(state.remaining_percent, remaining);
        assert_eq!(state.plan_label, plan);
        assert_eq!(state.history.samples, history);
        assert_eq!(state.model_usage, model_usage);
        assert_eq!(state.estimated_cost_label, cost);
        assert_eq!(state.active_threads, threads);

        state.apply_thread_result(
            stale_epoch,
            ActiveThreadUpdate::Snapshot(vec![ActiveThread {
                id: "stale-thread".into(),
                ..ActiveThread::default()
            }]),
        );
        state.apply_thread_error(stale_epoch, "stale thread error".into());
        state.apply_local_usage_success(LocalUsageResult {
            auth_epoch: stale_epoch,
            reset_at,
            window_seconds: WEEK_SECONDS,
            model_usage: ModelUsageTotals::default(),
            history_samples: vec![UsageHistorySample::new(
                10,
                reset_at,
                0.0,
                ModelDollarTotals::default(),
            )],
        });
        state.apply_local_usage_error(stale_epoch, reset_at, WEEK_SECONDS);

        assert_eq!(state.remaining_percent, remaining);
        assert_eq!(state.plan_label, plan);
        assert_eq!(state.history.samples, history);
        assert_eq!(state.model_usage, model_usage);
        assert_eq!(state.estimated_cost_label, cost);
        assert_eq!(state.active_threads, threads);
        assert_eq!(state.status, error_status);
        assert!(state.account_error.is_some());
        assert!(!state.thread_error);
        assert!(!state.local_usage_error);
    }

    #[test]
    fn initial_authenticated_event_preserves_loaded_history_and_advances_epoch() {
        let mut state = CodexInfoState::preview("normal");
        let expected_history = state.history.samples.clone();
        assert!(!expected_history.is_empty());
        state.authenticated = false;
        state.email = None;
        state.plan_label.clear();
        let old_epoch = state.auth_epoch;

        state.apply_account_event(Some("preview@example.com".into()), true, Some("pro".into()));

        assert_eq!(state.auth_epoch, old_epoch + 1);
        assert_eq!(state.history.samples, expected_history);
        assert!(state.authenticated);
    }

    #[test]
    fn account_loss_or_switch_clears_every_visible_account_value() {
        let mut state = CodexInfoState::preview("normal");
        state.clear_account_visible_state();

        assert!(!state.authenticated);
        assert!(state.email.is_none());
        assert!(state.plan_label.is_empty());
        assert!(!state.has_usage);
        assert!(!state.has_quota_percent);
        assert!(state.remaining_percent.is_none());
        assert!(state.reset_at.is_none());
        assert!(state.model_usage.is_empty());
        assert!(state.active_threads.is_empty());
        assert_eq!(state.estimated_cost_label, "概算 —");
        assert!(state.history.samples.is_empty());
        assert_eq!(state.selected_history_period, "履歴なし");
    }

    #[test]
    fn window_titles_use_only_validated_identity_for_every_state() {
        for preview in [
            "normal",
            "warning",
            "reset-warning",
            "error",
            "zero",
            "full",
            "idle",
            "multi-thread",
            "single-thread",
            "history-empty",
        ] {
            assert_eq!(
                CodexInfoState::preview(preview).window_title(),
                "preview@example.com — Pro",
                "preview state: {preview}"
            );
        }
        for preview in ["monthly", "unlimited"] {
            assert_eq!(
                CodexInfoState::preview(preview).window_title(),
                "preview@example.com — エンタープライズ",
                "preview state: {preview}"
            );
        }
        assert_eq!(
            CodexInfoState::preview("auth").window_title(),
            UNAUTHENTICATED_WINDOW_TITLE
        );
        assert_eq!(
            account_window_title(false, Some("stale@example.com"), "Pro"),
            UNAUTHENTICATED_WINDOW_TITLE
        );
        assert_eq!(
            account_window_title(true, Some("user@example.com"), "プラン未設定"),
            "user@example.com — プラン未設定"
        );
        assert_eq!(
            detail_window_title("user@example.com — Pro", THREADS_WINDOW_PURPOSE),
            "user@example.com — Pro — Threads"
        );
        assert_eq!(
            detail_window_title("user@example.com — Pro", GRAPH_WINDOW_PURPOSE),
            "user@example.com — Pro — Graph"
        );
        assert_eq!(
            detail_window_title(UNAUTHENTICATED_WINDOW_TITLE, THREADS_WINDOW_PURPOSE),
            UNAUTHENTICATED_WINDOW_TITLE
        );
    }

    #[test]
    fn native_title_bars_are_ascii_safe_and_keep_move_context() {
        assert_eq!(
            native_account_window_title("salty919@gmail.com — Pro Lite"),
            "salty919@gmail.com - Pro Lite"
        );
        assert_eq!(
            native_account_window_title("salty919@gmail.com — エンタープライズ"),
            "salty919@gmail.com - Plan"
        );
        assert_eq!(
            super::native_detail_window_title(
                &super::I18n::detect(),
                true,
                "salty919@gmail.com — Pro Lite",
                super::WindowPurpose::Threads,
            ),
            "salty919@gmail.com - Pro Lite - Threads"
        );
        assert!(super::native_detail_window_title(
            &super::I18n::detect(),
            true,
            "salty919@gmail.com — エンタープライズ",
            super::WindowPurpose::Graph,
        )
        .is_ascii());
    }

    #[test]
    fn window_title_email_is_one_line_bounded_and_control_free() {
        assert_eq!(
            account_window_title(true, Some("a\n\tb"), "Pro"),
            "a b — Pro"
        );
        assert_eq!(
            account_window_title(true, Some("a   b"), "Pro"),
            "a b — Pro"
        );
        for forbidden in [
            '\u{0000}', '\u{001f}', '\u{007f}', '\u{009f}', '\u{061c}', '\u{200e}', '\u{200f}',
            '\u{2028}', '\u{202e}', '\u{2066}', '\u{2069}',
        ] {
            let email = format!("a{forbidden}{forbidden}b");
            assert_eq!(
                account_window_title(true, Some(&email), "Pro"),
                "a b — Pro",
                "forbidden scalar U+{:04X}",
                forbidden as u32
            );
        }
        let email_254 = "x".repeat(254);
        assert_eq!(
            account_window_title(true, Some(&email_254), "Pro"),
            format!("{email_254} — Pro")
        );
        let email_255 = "x".repeat(255);
        assert_eq!(
            account_window_title(true, Some(&email_255), "Pro"),
            format!("{}… — Pro", "x".repeat(253))
        );
        assert_eq!(
            account_window_title(true, Some("   "), "Pro"),
            UNAUTHENTICATED_WINDOW_TITLE
        );
        assert_eq!(
            account_window_title(true, Some("user@example.com"), &"p".repeat(65)),
            "user@example.com — プラン未設定"
        );
    }

    #[test]
    fn window_title_retains_valid_identity_on_refresh_error_and_clears_on_switch() {
        let mut state = CodexInfoState::preview("normal");
        state.email = Some("a@example.com".into());
        assert_eq!(state.window_title(), "a@example.com — Pro");
        state.apply_account_error("refresh failed".into());
        assert_eq!(state.window_title(), "a@example.com — Pro");

        state.clear_account_visible_state();
        assert_eq!(state.window_title(), UNAUTHENTICATED_WINDOW_TITLE);
        state.apply_account_event(Some("b@example.com".into()), true, Some("plus".into()));
        assert_eq!(state.window_title(), "b@example.com — Plus");
    }

    #[test]
    fn active_thread_rows_preserve_all_threads_and_expose_parent_relationships() {
        let threads = vec![
            ActiveThread {
                id: "parent".into(),
                created_at: Some(10),
                updated_at: 20,
                title: "親タイトル".into(),
                model: "model-parent".into(),
                model_label: "model-parent".into(),
                total_tokens: Some(u64::MAX),
                context_window_tokens: Some(258_400),
                last_user_message_at: Some(19),
                is_subagent: false,
                parent_thread_id: None,
                depth: None,
            },
            ActiveThread {
                id: "child".into(),
                created_at: Some(10),
                updated_at: 19,
                title: "子タイトル".into(),
                model: "model-child".into(),
                model_label: "model-child".into(),
                total_tokens: Some(1_234),
                context_window_tokens: None,
                last_user_message_at: Some(18),
                is_subagent: true,
                parent_thread_id: Some("parent".into()),
                depth: Some(1),
            },
            ActiveThread {
                id: "orphan".into(),
                created_at: None,
                updated_at: 18,
                title: "親が完了済みの子".into(),
                model: "model-orphan".into(),
                model_label: "model-orphan".into(),
                total_tokens: None,
                context_window_tokens: None,
                last_user_message_at: None,
                is_subagent: true,
                parent_thread_id: Some("completed-parent".into()),
                depth: Some(120),
            },
        ];

        let rows = active_thread_rows_at(&threads, 20);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].relation.as_str(), "メイン");
        assert_eq!(
            rows[0].tokens.as_str(),
            "18,446,744,073,709,551,615トークン"
        );
        assert_eq!(rows[0].context_usage.as_str(), "100% / 258,400トークン");
        assert_eq!(rows[1].relation.as_str(), "サブ D1");
        assert_eq!(rows[1].tree_depth, 1);
        assert!(rows[1].connected_to_parent);
        assert!(!rows[1].has_next_sibling);
        assert_eq!(rows[1].parent_title.as_str(), "親: 親タイトル");
        assert_eq!(rows[1].thread_age.as_str(), "10秒");
        assert_eq!(rows[1].instruction_age.as_str(), "2秒");
        assert_eq!(rows[2].relation.as_str(), "サブ D99+");
        assert_eq!(rows[2].tree_depth, 0);
        assert!(!rows[2].connected_to_parent);
        assert_eq!(rows[2].parent_title.as_str(), "親スレッドは現在非実行");
        assert_eq!(rows[2].tokens.as_str(), "—");
    }

    #[test]
    fn thread_presentation_is_parent_first_subtree_contiguous_and_total() {
        let thread = |id: &str,
                      updated_at: i64,
                      is_subagent: bool,
                      parent: Option<&str>,
                      depth: Option<i32>| ActiveThread {
            id: id.into(),
            created_at: Some(updated_at.saturating_sub(10)),
            updated_at,
            title: id.into(),
            model: "model".into(),
            model_label: "model".into(),
            total_tokens: None,
            context_window_tokens: None,
            last_user_message_at: Some(updated_at.saturating_sub(1)),
            is_subagent,
            parent_thread_id: parent.map(str::to_owned),
            depth,
        };
        let threads = vec![
            thread("grand", 60, true, Some("child-new"), None),
            thread("orphan", 30, true, Some("missing"), Some(7)),
            thread("cycle-b", 90, true, Some("cycle-a"), Some(1)),
            thread("z-child", 70, true, Some("root-z"), Some(99)),
            thread("root-a", 10, false, None, Some(8)),
            thread("sibling", 40, true, Some("root-a"), Some(-4)),
            thread("parentless", 25, true, None, None),
            thread("cycle-a", 100, true, Some("cycle-b"), Some(1)),
            thread("child-new", 50, true, Some("root-a"), Some(42)),
            thread("root-z", 20, false, None, None),
        ];

        let presentation = thread_presentation_rows(&threads);
        let ids = presentation
            .iter()
            .map(|row| threads[row.index].id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            [
                "orphan",
                "parentless",
                "root-z",
                "z-child",
                "root-a",
                "child-new",
                "grand",
                "sibling",
                "cycle-a",
                "cycle-b",
            ]
        );
        assert_eq!(
            ids.iter().copied().collect::<BTreeSet<_>>(),
            threads
                .iter()
                .map(|thread| thread.id.as_str())
                .collect::<BTreeSet<_>>()
        );

        let child = presentation
            .iter()
            .find(|row| threads[row.index].id == "child-new")
            .unwrap();
        assert_eq!(child.forest_depth, 1);
        assert!(child.connected_to_parent);
        assert!(child.has_children);
        assert!(child.has_next_sibling);
        assert_eq!(child.ancestor_guides, [false; 3]);
        let grand = presentation
            .iter()
            .find(|row| threads[row.index].id == "grand")
            .unwrap();
        assert_eq!(grand.forest_depth, 2);
        assert_eq!(grand.ancestor_guides, [true, false, false]);
        let orphan = presentation
            .iter()
            .find(|row| threads[row.index].id == "orphan")
            .unwrap();
        assert_eq!(orphan.forest_depth, 0);
        assert!(!orphan.connected_to_parent);
        let cycle = presentation
            .iter()
            .find(|row| threads[row.index].id == "cycle-a")
            .unwrap();
        assert!(!cycle.connected_to_parent);
        assert!(!cycle.has_children);

        let rows = active_thread_rows_at(&threads, 100);
        let rows_by_title = rows
            .iter()
            .map(|row| (row.title.as_str(), row))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(rows_by_title["child-new"].relation.as_str(), "サブ D1");
        assert_eq!(rows_by_title["grand"].relation.as_str(), "サブ D2");
        assert_eq!(rows_by_title["sibling"].relation.as_str(), "サブ D1");
        assert_eq!(rows_by_title["orphan"].relation.as_str(), "サブ D7");
        assert_eq!(rows_by_title["parentless"].relation.as_str(), "サブ");
        assert_eq!(rows_by_title["root-a"].relation.as_str(), "メイン");
    }

    #[test]
    fn thread_presentation_keeps_capped_ancestor_guide_through_deeper_rows() {
        let thread = |id: &str, updated_at: i64, parent: Option<&str>| ActiveThread {
            id: id.into(),
            created_at: Some(updated_at.saturating_sub(10)),
            updated_at,
            title: id.into(),
            model: "model".into(),
            model_label: "model".into(),
            total_tokens: None,
            context_window_tokens: None,
            last_user_message_at: None,
            is_subagent: parent.is_some(),
            parent_thread_id: parent.map(str::to_owned),
            depth: None,
        };
        let threads = vec![
            thread("root", 100, None),
            thread("level-1", 90, Some("root")),
            thread("level-2", 80, Some("level-1")),
            thread("level-3-first", 70, Some("level-2")),
            thread("level-3-last", 60, Some("level-2")),
            thread("level-4", 50, Some("level-3-first")),
            thread("level-5", 40, Some("level-4")),
        ];

        let presentation = thread_presentation_rows(&threads);
        let level_3_first = presentation
            .iter()
            .find(|row| threads[row.index].id == "level-3-first")
            .expect("level 3 first sibling");
        assert!(level_3_first.has_next_sibling);
        let level_5 = presentation
            .iter()
            .find(|row| threads[row.index].id == "level-5")
            .expect("deep descendant");
        assert_eq!(level_5.forest_depth, 5);
        assert!(level_5.ancestor_guides[2]);
    }

    #[test]
    fn active_thread_model_counts_use_exact_known_tokens_and_keep_named_zeroes() {
        let thread = |id: &str, model_label: &str| ActiveThread {
            id: id.into(),
            created_at: Some(1),
            updated_at: 1,
            title: id.into(),
            model: "model".into(),
            model_label: model_label.into(),
            total_tokens: None,
            context_window_tokens: None,
            last_user_message_at: None,
            is_subagent: false,
            parent_thread_id: None,
            depth: None,
        };

        assert_eq!(active_thread_model_counts(&[]), "");
        assert_eq!(
            active_thread_model_counts(&[
                thread("sol", "gpt-5.6-SOL"),
                thread("terra", "gpt-5.6-terra"),
                thread("luna", "gpt-5.6-luna"),
                thread("unknown", "gpt-5.6-sol-terra"),
            ]),
            "SOL 1  TERRA 1  LUNA 1  その他 1"
        );
    }

    #[test]
    fn thread_age_uses_fixed_boundaries_and_clamps_future() {
        let now = 86_400;
        assert_eq!(format_elapsed(now, Some(now)), "0秒");
        assert_eq!(format_elapsed(now, Some(now - 59)), "59秒");
        assert_eq!(format_elapsed(now, Some(now - 60)), "1分");
        assert_eq!(format_elapsed(now, Some(now - 83)), "1分23秒");
        assert_eq!(format_elapsed(now, Some(now - 3_599)), "59分59秒");
        assert_eq!(format_elapsed(now, Some(now - 3_600)), "1時間");
        assert_eq!(format_elapsed(now, Some(now - 3_661)), "1時間1分");
        assert_eq!(format_elapsed(now, Some(now - 86_399)), "23時間59分");
        assert_eq!(format_elapsed(now, Some(now - 86_400)), "1日");
        assert_eq!(format_elapsed(now, Some(now + 60)), "0秒");
        assert_eq!(format_elapsed(now, Some(i64::MAX)), "—");
        assert_eq!(format_elapsed(now, None), "—");
    }

    #[cfg(unix)]
    #[test]
    fn open_codex_session_paths_accepts_only_bounded_codex_fds_under_sessions_root() {
        use std::os::unix::fs::symlink;

        static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "codex-info-proc-fixture-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        let sessions = root.join("sessions");
        let proc_root = root.join("proc");
        let process = proc_root.join("100");
        let ignored_process = proc_root.join("200");
        fs::create_dir_all(process.join("fd")).unwrap();
        fs::create_dir_all(ignored_process.join("fd")).unwrap();
        fs::create_dir_all(&sessions).unwrap();
        fs::write(process.join("comm"), "codex\n").unwrap();
        fs::write(ignored_process.join("comm"), "not-codex\n").unwrap();
        let executable = root.join("codex");
        fs::write(&executable, "fixture").unwrap();
        symlink(&executable, process.join("exe")).unwrap();
        symlink(&executable, ignored_process.join("exe")).unwrap();
        let active = sessions.join("active.jsonl");
        let ignored = sessions.join("ignored.jsonl");
        let outside = root.join("outside.jsonl");
        fs::write(&active, "{}\n").unwrap();
        fs::write(&ignored, "{}\n").unwrap();
        fs::write(&outside, "{}\n").unwrap();
        symlink(&active, process.join("fd/3")).unwrap();
        symlink(&outside, process.join("fd/4")).unwrap();
        symlink(&ignored, ignored_process.join("fd/3")).unwrap();

        assert_eq!(
            open_codex_session_paths(&proc_root, &sessions).unwrap(),
            BTreeSet::from([fs::canonicalize(active).unwrap()])
        );
        let _ = fs::remove_dir_all(root);
    }

    fn thread_list_item(id: &str, updated_at: i64, path: &Path) -> Value {
        json!({
            "cliVersion": "0.147.0",
            "createdAt": 1,
            "cwd": "/tmp/codex-info",
            "ephemeral": false,
            "id": id,
            "modelProvider": "openai",
            "preview": format!("preview-{id}"),
            "sessionId": format!("session-{id}"),
            "source": "cli",
            "status": {"type": "idle"},
            "turns": [],
            "updatedAt": updated_at,
            "name": format!("title-{id}"),
            "path": path.to_string_lossy()
        })
    }

    #[test]
    fn active_thread_adapter_paginates_and_falls_back_to_the_next_valid_rollout() {
        static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "codex-info-thread-adapter-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let newest_path = root.join("newest.jsonl");
        let fallback_path = root.join("fallback.jsonl");
        fs::write(
            &newest_path,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\"}}\n",
        )
        .unwrap();
        fs::write(
            &fallback_path,
            [
                json!({"type":"event_msg","payload":{"type":"task_started"}}),
                json!({"type":"event_msg","payload":{"type":"turn_context","model":"gpt-5.6-sol"}}),
                json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":12345}}}}),
            ]
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join("\n")
                + "\n",
        )
        .unwrap();

        let (sender, receiver) = mpsc::channel();
        for response in [
            json!({
                "id": 50,
                "result": {
                    "data": [thread_list_item("newest", 20, &newest_path)],
                    "nextCursor": "page-2"
                }
            }),
            json!({
                "id": 51,
                "result": {
                    "data": [thread_list_item("fallback", 10, &fallback_path)]
                }
            }),
        ] {
            sender
                .send(RpcReadEvent::Line(
                    super::security::RpcLine::new(response.to_string()).unwrap(),
                ))
                .unwrap();
        }

        let mut input = Vec::new();
        let mut next_id = 50;
        let active_paths = BTreeSet::from([
            fs::canonicalize(&newest_path).unwrap(),
            fs::canonicalize(&fallback_path).unwrap(),
        ]);
        let update = fetch_active_thread_update_for_paths(
            &mut input,
            &receiver,
            &mut next_id,
            &root,
            &active_paths,
        );
        assert_eq!(
            update,
            ActiveThreadUpdate::Snapshot(vec![ActiveThread {
                id: "fallback".into(),
                created_at: Some(1),
                updated_at: 10,
                title: "title-fallback".into(),
                model: "gpt-5.6-sol".into(),
                model_label: "gpt-5.6-sol".into(),
                total_tokens: Some(12_345),
                context_window_tokens: None,
                last_user_message_at: None,
                is_subagent: false,
                parent_thread_id: None,
                depth: None,
            }])
        );
        assert_eq!(next_id, 52);

        let requests = String::from_utf8(input)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0]["id"], 50);
        assert!(requests[0]["params"].get("cursor").is_none());
        assert_eq!(requests[1]["id"], 51);
        assert_eq!(requests[1]["params"]["cursor"], "page-2");
        assert_eq!(
            requests[0]["params"]["sourceKinds"],
            json!([
                "cli",
                "vscode",
                "exec",
                "appServer",
                "subAgent",
                "subAgentReview",
                "subAgentCompact",
                "subAgentThreadSpawn",
                "subAgentOther",
                "unknown"
            ])
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn multiple_running_threads_are_all_published_with_stable_order() {
        static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "codex-info-multiple-running-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();

        let completed_path = root.join("completed.jsonl");
        let running_a_path = root.join("running-a.jsonl");
        let running_z_path = root.join("running-z.jsonl");
        fs::write(
            &completed_path,
            [
                json!({"type":"task_started"}),
                json!({"type":"task_complete"}),
            ]
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join("\n")
                + "\n",
        )
        .unwrap();
        for (path, model, total_tokens) in [
            (&running_a_path, "model-a", 111_u64),
            (&running_z_path, "model-z", 999_u64),
        ] {
            fs::write(
                path,
                [
                    json!({"type":"thread_context","model":model}),
                    json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":total_tokens}}}}),
                    json!({"type":"task_started"}),
                ]
                .into_iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join("\n")
                    + "\n",
            )
            .unwrap();
        }

        let mut child_item = thread_list_item("thread-a", 20, &running_a_path);
        child_item["source"] = json!({"subAgent":{"thread_spawn":{
            "parent_thread_id":"thread-z","depth":1
        }}});
        let (sender, receiver) = mpsc::channel();
        sender
            .send(RpcReadEvent::Line(
                super::security::RpcLine::new(
                    json!({
                        "id": 70,
                        "result": {
                            "data": [
                                child_item,
                                thread_list_item("completed-newest", 30, &completed_path),
                                thread_list_item("thread-z", 20, &running_z_path)
                            ]
                        }
                    })
                    .to_string(),
                )
                .unwrap(),
            ))
            .unwrap();

        let mut input = Vec::new();
        let mut next_id = 70;
        let active_paths = BTreeSet::from([
            fs::canonicalize(&completed_path).unwrap(),
            fs::canonicalize(&running_a_path).unwrap(),
            fs::canonicalize(&running_z_path).unwrap(),
        ]);
        let update = fetch_active_thread_update_for_paths(
            &mut input,
            &receiver,
            &mut next_id,
            &root,
            &active_paths,
        );
        assert_eq!(
            update,
            ActiveThreadUpdate::Snapshot(vec![
                ActiveThread {
                    id: "thread-z".into(),
                    created_at: Some(1),
                    updated_at: 20,
                    title: "title-thread-z".into(),
                    model: "model-z".into(),
                    model_label: "model-z".into(),
                    total_tokens: Some(999),
                    context_window_tokens: None,
                    last_user_message_at: None,
                    is_subagent: false,
                    parent_thread_id: None,
                    depth: None,
                },
                ActiveThread {
                    id: "thread-a".into(),
                    created_at: Some(1),
                    updated_at: 20,
                    title: "title-thread-a".into(),
                    model: "model-a".into(),
                    model_label: "model-a".into(),
                    total_tokens: Some(111),
                    context_window_tokens: None,
                    last_user_message_at: None,
                    is_subagent: true,
                    parent_thread_id: Some("thread-z".into()),
                    depth: Some(1),
                },
            ])
        );
        assert_eq!(next_id, 71);
        assert_eq!(String::from_utf8(input).unwrap().lines().count(), 1);

        let _ = fs::remove_dir_all(root);
    }

    fn create_native_state_schema(root: &Path) {
        fs::create_dir_all(root.join("sessions")).unwrap();
        let connection = Connection::open(root.join("state_5.sqlite")).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE threads (
                    id TEXT PRIMARY KEY,
                    rollout_path TEXT NOT NULL,
                    updated_at INTEGER NOT NULL,
                    archived INTEGER NOT NULL,
                    name TEXT,
                    preview TEXT NOT NULL,
                    thread_source TEXT
                );
                CREATE TABLE thread_spawn_edges (
                    parent_thread_id TEXT NOT NULL,
                    child_thread_id TEXT NOT NULL PRIMARY KEY,
                    status TEXT NOT NULL
                );",
            )
            .unwrap();
    }

    fn add_native_state_thread(root: &Path, id: &str, rollout_path: &Path) {
        let connection = Connection::open(root.join("state_5.sqlite")).unwrap();
        connection
            .execute(
                "INSERT INTO threads
                 (id, rollout_path, updated_at, archived, name, preview, thread_source)
                 VALUES (?1, ?2, 1, 0, ?3, ?4, 'subagent')",
                rusqlite::params![
                    id,
                    rollout_path.to_string_lossy().as_ref(),
                    format!("title-{id}"),
                    format!("preview-{id}"),
                ],
            )
            .unwrap();
    }

    fn add_native_state_edge(root: &Path, parent: &str, child: &str) {
        let connection = Connection::open(root.join("state_5.sqlite")).unwrap();
        connection
            .execute(
                "INSERT INTO thread_spawn_edges
                 (parent_thread_id, child_thread_id, status) VALUES (?1, ?2, 'active')",
                rusqlite::params![parent, child],
            )
            .unwrap();
    }

    fn write_native_rollout(path: &Path, completed: bool) {
        let mut records = vec![
            json!({"type":"thread_context","model":"native-model"}),
            json!({"type":"task_started"}),
        ];
        if completed {
            records.push(json!({"type":"task_complete"}));
        }
        fs::write(
            path,
            records
                .into_iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join("\n")
                + "\n",
        )
        .unwrap();
    }

    #[test]
    fn native_completed_rollout_is_excluded_from_published_snapshot() {
        static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "codex-info-native-completed-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        create_native_state_schema(&root);
        let sessions = root.join("sessions");
        let root_rollout = sessions.join("root.jsonl");
        let completed_rollout = sessions.join("completed-child.jsonl");
        write_native_rollout(&root_rollout, false);
        write_native_rollout(&completed_rollout, true);
        add_native_state_thread(&root, "completed-child", &completed_rollout);
        add_native_state_edge(&root, "root", "completed-child");

        let (sender, receiver) = mpsc::channel();
        sender
            .send(RpcReadEvent::Line(
                super::security::RpcLine::new(
                    json!({
                        "id": 80,
                        "result": {"data": [thread_list_item("root", 10, &root_rollout)]}
                    })
                    .to_string(),
                )
                .unwrap(),
            ))
            .unwrap();
        let active_paths = BTreeSet::from([fs::canonicalize(&root_rollout).unwrap()]);
        let mut input = Vec::new();
        let mut next_id = 80;
        let update = fetch_active_thread_update_for_paths_and_state(
            &mut input,
            &receiver,
            &mut next_id,
            &sessions,
            &active_paths,
            Some(&root),
        );
        assert_eq!(
            update,
            ActiveThreadUpdate::Snapshot(vec![ActiveThread {
                id: "root".into(),
                created_at: Some(1),
                updated_at: 10,
                title: "title-root".into(),
                model: "native-model".into(),
                model_label: "native-model".into(),
                total_tokens: None,
                context_window_tokens: None,
                last_user_message_at: None,
                is_subagent: false,
                parent_thread_id: None,
                depth: None,
            }])
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn native_descendant_failure_rejects_root_snapshot_atomically() {
        static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "codex-info-native-atomic-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        create_native_state_schema(&root);
        let sessions = root.join("sessions");
        let root_rollout = sessions.join("root.jsonl");
        let invalid_rollout = sessions.join("invalid-child.jsonl");
        write_native_rollout(&root_rollout, false);
        fs::write(&invalid_rollout, b"{not-json}\n").unwrap();
        add_native_state_thread(&root, "invalid-child", &invalid_rollout);
        add_native_state_edge(&root, "root", "invalid-child");

        let (sender, receiver) = mpsc::channel();
        sender
            .send(RpcReadEvent::Line(
                super::security::RpcLine::new(
                    json!({
                        "id": 81,
                        "result": {"data": [thread_list_item("root", 10, &root_rollout)]}
                    })
                    .to_string(),
                )
                .unwrap(),
            ))
            .unwrap();
        let active_paths = BTreeSet::from([fs::canonicalize(&root_rollout).unwrap()]);
        let mut input = Vec::new();
        let mut next_id = 81;
        let update = fetch_active_thread_update_for_paths_and_state(
            &mut input,
            &receiver,
            &mut next_id,
            &sessions,
            &active_paths,
            Some(&root),
        );
        assert_eq!(update, ActiveThreadUpdate::Failed);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn session_traversal_budgets_and_symlink_rejection_have_exact_boundaries() {
        assert_eq!(super::security::MAX_SESSION_FILE_BYTES, 256 * 1024 * 1024);
        assert_eq!(
            super::security::MAX_SESSION_TOTAL_BYTES,
            2 * 1024 * 1024 * 1024
        );
        assert!(SessionTraversalBudget::default()
            .admit_file(1, 64 * 1024 * 1024 + 1)
            .is_ok());

        let mut files = SessionTraversalBudget::default();
        for _ in 0..super::security::MAX_SESSION_FILES {
            files.admit_file(1, 0).expect("file budget boundary");
        }
        assert!(files.admit_file(1, 0).is_err());

        let mut total = SessionTraversalBudget::default();
        for _ in 0..8 {
            total
                .admit_file(1, super::security::MAX_SESSION_FILE_BYTES)
                .expect("total byte boundary");
        }
        assert_eq!(total.total_bytes, super::security::MAX_SESSION_TOTAL_BYTES);
        assert!(total.admit_file(1, 1).is_err());
        assert!(SessionTraversalBudget::default()
            .admit_file(1, super::security::MAX_SESSION_FILE_BYTES + 1)
            .is_err());
        assert!(SessionTraversalBudget::default()
            .admit_file(super::security::MAX_SESSION_DEPTH + 1, 0)
            .is_err());

        let root = std::env::temp_dir().join(format!(
            "codex-info-traversal-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir(&root).unwrap();
        let safe = root.join("safe.jsonl");
        fs::write(&safe, "{}\n").unwrap();
        assert_eq!(session_jsonl_files(&root).unwrap(), vec![safe]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink("safe.jsonl", root.join("linked.jsonl")).unwrap();
            assert!(session_jsonl_files(&root).is_err());
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn active_rollout_snapshot_accepts_append_and_defers_partial_tail() {
        let root = std::env::temp_dir().join(format!(
            "codex-info-rollout-append-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("active.jsonl");
        let complete = concat!(
            "{\"type\":\"thread_context\",\"model\":\"gpt-5.6-sol\"}\n",
            "{\"type\":\"task_started\"}\n"
        );
        let partial = "{\"type\":\"event_msg\"";
        fs::write(&path, format!("{complete}{partial}")).unwrap();

        let before = fs::metadata(&path).unwrap();
        let mut file = File::open(&path).unwrap();
        let snapshot_len = file.metadata().unwrap().len();
        let complete_len = complete_rollout_prefix_len(&mut file, snapshot_len).unwrap();
        assert_eq!(complete_len, complete.len() as u64);
        file.seek(SeekFrom::Start(0)).unwrap();
        let rollout = {
            let mut reader = BufReader::new((&mut file).take(complete_len));
            thread_contract::parse_rollout_reader(&mut reader, complete_len).unwrap()
        };
        assert!(rollout.is_running());
        assert_eq!(rollout.model(), "gpt-5.6-sol");
        assert_eq!(rollout.total_tokens(), None);

        let remainder = concat!(
            ",\"payload\":{\"type\":\"token_count\",\"info\":{",
            "\"total_token_usage\":{\"total_tokens\":77}}}}\n"
        );
        let mut append = fs::OpenOptions::new().append(true).open(&path).unwrap();
        append.write_all(remainder.as_bytes()).unwrap();
        append.flush().unwrap();
        drop(append);
        let after = fs::metadata(&path).unwrap();
        assert!(same_rollout_identity(&before, &after));
        assert!(after.len() > before.len());

        let mut file = File::open(&path).unwrap();
        let complete_len = complete_rollout_prefix_len(&mut file, after.len()).unwrap();
        assert_eq!(complete_len, after.len());
        file.seek(SeekFrom::Start(0)).unwrap();
        let rollout = {
            let mut reader = BufReader::new((&mut file).take(complete_len));
            thread_contract::parse_rollout_reader(&mut reader, complete_len).unwrap()
        };
        assert!(rollout.is_running());
        assert_eq!(rollout.total_tokens(), Some(77));

        let other = root.join("other.jsonl");
        fs::write(&other, "{}\n").unwrap();
        assert!(!same_rollout_identity(
            &after,
            &fs::metadata(&other).unwrap()
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rpc_request_enforces_mismatch_timeout_and_error_redaction() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..super::security::MAX_RPC_IGNORED_MESSAGES {
            tx.send(RpcReadEvent::Line(
                super::security::RpcLine::new(r#"{"jsonrpc":"2.0","id":2,"result":{}}"#).unwrap(),
            ))
            .unwrap();
        }
        tx.send(RpcReadEvent::Line(
            super::security::RpcLine::new(r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#)
                .unwrap(),
        ))
        .unwrap();
        let mut input = Vec::new();
        assert_eq!(
            request_with_timeout(
                &mut input,
                &rx,
                1,
                "test/read",
                Value::Null,
                Duration::from_millis(50),
            )
            .unwrap(),
            json!({"ok":true})
        );

        let (tx, rx) = mpsc::channel();
        for _ in 0..=super::security::MAX_RPC_IGNORED_MESSAGES {
            tx.send(RpcReadEvent::Line(
                super::security::RpcLine::new(r#"{"jsonrpc":"2.0","id":2,"result":{}}"#).unwrap(),
            ))
            .unwrap();
        }
        assert!(request_with_timeout(
            &mut Vec::new(),
            &rx,
            1,
            "test/read",
            Value::Null,
            Duration::from_millis(50),
        )
        .unwrap_err()
        .contains("上限"));

        let (_tx, rx) = mpsc::channel();
        assert!(request_with_timeout(
            &mut Vec::new(),
            &rx,
            1,
            "test/read",
            Value::Null,
            Duration::from_millis(1),
        )
        .unwrap_err()
        .contains("タイムアウト"));

        let (tx, rx) = mpsc::channel();
        tx.send(RpcReadEvent::Line(
            super::security::RpcLine::new(
                r#"{"jsonrpc":"2.0","id":1,"error":{"secret":"token-value"}}"#,
            )
            .unwrap(),
        ))
        .unwrap();
        let error = request_with_timeout(
            &mut Vec::new(),
            &rx,
            1,
            "test/read",
            Value::Null,
            Duration::from_millis(50),
        )
        .unwrap_err();
        assert!(!error.contains("token-value"));
    }

    #[test]
    fn preview_size_parser_only_parses_syntax() {
        assert_eq!(parse_preview_size(Some("1200x800")), Some((1200, 800)));
        assert_eq!(parse_preview_size(Some("699x479")), Some((699, 479)));
        assert_eq!(parse_preview_size(Some("700x480x1")), None);
        assert_eq!(parse_preview_size(Some("not-a-size")), None);
        assert_eq!(parse_preview_size(None), None);
    }

    #[test]
    fn login_confirmation_poll_is_fast_only_while_authentication_is_pending() {
        assert_eq!(
            automatic_refresh_interval(false, true),
            Duration::from_secs(2)
        );
        assert_eq!(
            automatic_refresh_interval(false, false),
            Duration::from_secs(60)
        );
        assert_eq!(
            automatic_refresh_interval(true, true),
            Duration::from_secs(60)
        );
    }

    #[test]
    fn preview_size_keeps_main_fixed_and_applies_graph_minimums() {
        assert_eq!(
            parse_preview_size(Some("600x400")).map(|_| (900, 480)),
            Some((900, 480))
        );
        assert_eq!(
            parse_preview_size(Some("1200x800")).map(|_| (900, 480)),
            Some((900, 480))
        );
        assert_eq!(
            clamp_graph_preview_size(parse_preview_size(Some("600x400")).unwrap()),
            (700, 480)
        );
        assert_eq!(
            clamp_graph_preview_size(parse_preview_size(Some("1200x800")).unwrap()),
            (1200, 800)
        );
        assert_eq!(parse_preview_size(Some("700x540x")), None);
    }

    #[test]
    fn fixed_resize_decision_preserves_minimize_and_rejects_every_wrong_surface() {
        assert_eq!(
            fixed_resize_decision(FIXED_WINDOW_WIDTH, FIXED_WINDOW_HEIGHT),
            FixedResizeDecision::Propagate
        );
        for (width, height) in [(0, 0), (0, 480), (900, 0)] {
            assert_eq!(
                fixed_resize_decision(width, height),
                FixedResizeDecision::Propagate,
                "zero-sized minimize event {width}x{height}"
            );
        }
        for (width, height) in [(899, 480), (901, 480), (900, 479), (900, 481), (1080, 600)] {
            assert_eq!(
                fixed_resize_decision(width, height),
                FixedResizeDecision::RejectAndRestore,
                "non-zero resize {width}x{height}"
            );
        }
    }

    #[test]
    fn graph_resize_handles_cover_all_edges_and_corners() {
        let directions = [
            ("north", winit::window::ResizeDirection::North),
            ("south", winit::window::ResizeDirection::South),
            ("east", winit::window::ResizeDirection::East),
            ("west", winit::window::ResizeDirection::West),
            ("north-east", winit::window::ResizeDirection::NorthEast),
            ("north-west", winit::window::ResizeDirection::NorthWest),
            ("south-east", winit::window::ResizeDirection::SouthEast),
            ("south-west", winit::window::ResizeDirection::SouthWest),
        ];
        for (name, expected) in directions {
            assert_eq!(parse_resize_direction(name), Some(expected));
        }
        assert_eq!(parse_resize_direction("invalid"), None);

        let source = include_str!("../ui/components.slint");
        let graph = source
            .split("export component GraphWindow inherits Window {")
            .nth(1)
            .expect("GraphWindow");
        assert!(graph.contains("callback begin-window-resize(string);"));
        for direction in [
            "direction: \"north\";",
            "direction: \"south\";",
            "direction: \"east\";",
            "direction: \"west\";",
            "direction: \"north-east\";",
            "direction: \"north-west\";",
            "direction: \"south-east\";",
            "direction: \"south-west\";",
        ] {
            assert!(
                graph.contains(direction),
                "missing graph resize handle: {direction}"
            );
        }
        for marker in [
            "width: root.width - 28px;",
            "height: 14px;",
            "height: 28px;",
            "resize-cursor: MouseCursor.nwse-resize;",
            "resize-cursor: MouseCursor.nesw-resize;",
            "corner: true;",
        ] {
            assert!(
                graph.contains(marker),
                "missing resize affordance: {marker}"
            );
        }
        let main = include_str!("../src/main.rs");
        assert!(main.contains("graph.on_begin_window_resize"));
        assert!(main.contains("drag_resize_window(direction)"));
    }

    #[test]
    fn manual_resize_geometry_keeps_corner_direction_and_minimum() {
        let initial = ManualX11Geometry {
            x: 100,
            y: 80,
            width: 940,
            height: 640,
        };
        assert_eq!(
            manual_resize_geometry(initial, winit::window::ResizeDirection::SouthEast, 120, 80,),
            ManualX11Geometry {
                x: 100,
                y: 80,
                width: 1060,
                height: 720,
            }
        );
        assert_eq!(
            manual_resize_geometry(initial, winit::window::ResizeDirection::NorthWest, 120, 80,),
            ManualX11Geometry {
                x: 220,
                y: 160,
                width: 820,
                height: 560,
            }
        );
        assert_eq!(
            manual_resize_geometry(
                initial,
                winit::window::ResizeDirection::NorthWest,
                1_000,
                1_000,
            ),
            ManualX11Geometry {
                x: 340,
                y: 240,
                width: 700,
                height: 480,
            }
        );
    }

    #[test]
    fn manual_move_geometry_preserves_client_origin_and_applies_pointer_delta() {
        let initial = ManualX11Geometry {
            x: 2_506,
            y: 1_296,
            width: 900,
            height: 480,
        };
        assert_eq!(
            manual_window_geometry(initial, ManualX11WindowAction::Move, 0, 0),
            initial
        );
        assert_eq!(
            manual_window_geometry(initial, ManualX11WindowAction::Move, 60, 40),
            ManualX11Geometry {
                x: 2_566,
                y: 1_336,
                ..initial
            }
        );
        assert_eq!(
            manual_window_geometry(initial, ManualX11WindowAction::Move, -40, -30),
            ManualX11Geometry {
                x: 2_466,
                y: 1_266,
                ..initial
            }
        );
    }

    #[test]
    fn manual_x11_action_claim_is_exclusive_per_target_and_released_after_finish() {
        let target = u32::MAX - 17;
        let other_target = target - 1;
        let client = target - 2;
        let other_client = target - 3;
        let lease =
            claim_manual_x11_action(target, client).expect("first target claim should succeed");
        assert!(claim_manual_x11_action(target, other_client).is_none());
        assert!(claim_manual_x11_action(other_target, client).is_none());
        let other_lease = claim_manual_x11_action(other_target, other_target);
        assert!(other_lease.is_some());
        drop(lease);
        let final_lease = claim_manual_x11_action(target, target);
        assert!(final_lease.is_some());
        drop(final_lease);
        drop(other_lease);
    }

    #[test]
    fn manual_x11_move_uses_root_client_coordinates_and_skips_static_click_configure() {
        let source = include_str!("../src/main.rs");
        assert!(source.contains("connection.translate_coordinates(window_id, root, 0, 0)"));
        assert!(source.contains("let mut last_geometry = initial;"));
        assert!(source.contains("if geometry == last_geometry"));
        assert!(source.contains("finish_manual_x11_action(&connection, target);"));
        assert!(source.contains(
            "ManualX11WindowAction::Move => ConfigureWindowAux::new().x(geometry.x).y(geometry.y)"
        ));
    }

    #[test]
    fn native_window_contracts_keep_non_graph_windows_move_only() {
        let main = include_str!("../ui/app.slint");
        assert!(main.contains("title: root.window-title;"));
        assert!(main.contains("no-frame: true;"));
        assert!(main.contains("resize-border-width: 0px;"));
        assert!(main.contains("z: -5;"));
        assert!(main.contains("width: root.width;\n        height: root.height;"));
        assert!(!main.contains("title: \"Codex Info\";"));
        let components = include_str!("../ui/components.slint");
        assert!(components.contains("export component WindowControls"));
        assert!(components.contains("export component WindowDragArea"));
        let header = components
            .split("export component Header inherits Rectangle {")
            .nth(1)
            .and_then(|source| source.split("export component RemainingQuota").next())
            .expect("Header component");
        assert!(header.contains("private property <length> action-start:"));
        assert!(header.contains("width: root.action-start;"));
        let threads = components
            .split("export component ThreadsWindow inherits Window {")
            .nth(1)
            .expect("ThreadsWindow");
        let graph = components
            .split("export component GraphWindow inherits Window {")
            .nth(1)
            .expect("GraphWindow");
        let legal_notice = components
            .split("export component LegalNoticeWindow inherits Window {")
            .nth(1)
            .expect("LegalNoticeWindow");
        assert!(threads.contains("title: root.window-title;"));
        assert!(threads.contains("no-frame: true;"));
        assert!(threads.contains("resize-border-width: 0px;"));
        assert!(threads.contains("WindowControls"));
        assert!(threads.contains("WindowDragArea"));
        assert!(threads.contains("z: -5;"));
        assert!(threads.contains("width: root.width;\n        height: root.height;"));
        assert!(graph.contains("title: root.window-title;"));
        assert!(graph.contains("no-frame: true;"));
        assert!(graph.contains("resize-border-width: 6px;"));
        assert!(graph.contains("show-maximize: true;"));
        assert!(graph.contains("WindowControls"));
        assert!(graph.contains("WindowDragArea"));
        assert!(graph.contains("z: -5;"));
        assert!(graph.contains("width: root.width;\n        height: root.height;"));
        assert!(legal_notice.contains("title: root.window-title;"));
        assert!(legal_notice.contains("no-frame: true;"));
        assert!(legal_notice.contains("resize-border-width: 0px;"));
        assert!(legal_notice.contains("WindowControls"));
        assert!(legal_notice.contains("WindowDragArea"));
        assert!(legal_notice.contains("z: -5;"));
        assert!(legal_notice.contains("width: root.width;\n        height: root.height;"));
        for marker in [
            "preferred-width: 720px;",
            "preferred-height: 520px;",
            "min-width: 720px;",
            "max-width: 720px;",
            "min-height: 520px;",
            "max-height: 520px;",
        ] {
            assert!(
                legal_notice.contains(marker),
                "missing LegalNoticeWindow marker: {marker}"
            );
        }
        assert!(main.contains("callback open-legal-notice();"));
        for fixed_source in [main, threads] {
            assert!(fixed_source.contains("min-width: 900px;"));
            assert!(fixed_source.contains("max-width: 900px;"));
        }
        assert!(!graph.contains("max-width: 940px;"));
        let rust_source = include_str!("main.rs")
            .split_once("#[cfg(test)]\nmod tests")
            .map(|(source, _)| source)
            .expect("production source");
        assert!(rust_source.contains("on_winit_window_event"));
        assert!(rust_source.contains("EventResult::PreventDefault"));
        assert!(rust_source.contains("request_inner_size"));
        assert_eq!(
            rust_source
                .matches("install_fixed_window_guard(ui.window())")
                .count(),
            1
        );
        assert_eq!(
            rust_source
                .matches("install_fixed_window_guard(window.window())")
                .count(),
            1
        );
        assert!(rust_source.contains("install_resizable_window(graph.window());"));
        assert!(!rust_source
            .contains("install_window_size_guard(graph.window(), graph_width, graph_height);"));
        assert!(rust_source.contains(
            "install_window_size_guard(\n                        window.window(),\n                        LEGAL_WINDOW_WIDTH,\n                        LEGAL_WINDOW_HEIGHT,\n                    );"
        ));
        assert!(rust_source.contains("winit_window.set_resizable(false)"));
        assert!(rust_source.contains("winit_window.set_resizable(true)"));
        assert_eq!(rust_source.matches("LegalNoticeWindow::new()").count(), 1);
        assert!(rust_source.contains("ui.on_open_legal_notice"));
    }

    #[test]
    fn thread_rails_have_fixed_geometry_and_sufficient_contrast() {
        let source = include_str!("../ui/components.slint");
        for marker in [
            "width: 2px;",
            "height: root.thread-row-height;",
            "property <length> tree-base-x: 24px;",
            "property <length> tree-depth-step: 16px;",
            "property <length> tree-junction-y: 36px;",
            "x: parent.tree-base-x + parent.tree-depth-step;",
            "x: parent.tree-base-x + 2 * parent.tree-depth-step;",
            "y: parent.tree-junction-y - 1px;",
            "width: parent.title-x - self.x - 20px;",
            "background: DesignTokens.warning;",
            "height: root.thread-row-height - parent.tree-junction-y;",
            "border-radius: 2px;",
            "ancestor-guide-1",
            "ancestor-guide-2",
            "ancestor-guide-3",
        ] {
            assert!(source.contains(marker), "missing rail geometry: {marker}");
        }
        assert!(!source.contains("tree-guide"));
        assert!(!source.contains("row.indent"));

        fn luminance(rgb: [u8; 3]) -> f64 {
            let linear = rgb.map(|component| {
                let component = f64::from(component) / 255.0;
                if component <= 0.04045 {
                    component / 12.92
                } else {
                    ((component + 0.055) / 1.055).powf(2.4)
                }
            });
            0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]
        }
        let rail = luminance([0xe6, 0xa2, 0x3c]);
        for row in [[0x0d, 0x13, 0x1e], [0x14, 0x1d, 0x2d]] {
            let background = luminance(row);
            assert!((rail + 0.05) / (background + 0.05) >= 7.719);
        }
    }

    #[test]
    fn thread_rails_keep_every_text_lane_outside_the_tree_gutter() {
        let source = include_str!("../ui/components.slint");
        for marker in [
            "property <length> tree-base-x: 24px;",
            "property <length> tree-depth-step: 16px;",
            "property <length> tree-junction-y: 36px;",
            "x: root.single-thread ? 20px : 72px;",
            ": 172px + self.display-depth * 24px;",
            "width: parent.title-x - self.x - 20px;",
            "x: parent.title-x - 24px;",
            "if !root.single-thread && row.ancestor-guide-1 : Rectangle {",
            "if !root.single-thread && row.connected-to-parent : Rectangle {",
            "if !root.single-thread && row.has-children : Rectangle {",
        ] {
            assert!(
                source.contains(marker),
                "missing non-overlap contract: {marker}"
            );
        }
    }

    #[test]
    fn forbidden_x11_states_identifies_fullscreen_maximize_and_unrelated_atoms() {
        let atoms = X11StateAtoms {
            wm_state: 1,
            fullscreen: 2,
            maximized_vert: 3,
            maximized_horz: 4,
            active_window: None,
        };
        assert_eq!(forbidden_x11_states(&[], &atoms), (false, false));
        assert_eq!(
            forbidden_x11_states(&[atoms.fullscreen], &atoms),
            (true, false)
        );
        assert_eq!(
            forbidden_x11_states(&[atoms.maximized_horz], &atoms),
            (false, true)
        );
        assert_eq!(
            forbidden_x11_states(&[atoms.wm_state], &atoms),
            (false, false)
        );
    }

    #[test]
    fn motif_functions_allow_move_minimize_close_without_resize_or_maximize() {
        assert_eq!(motif_wm_functions(0), (1, 0x2c));
        assert_eq!(motif_wm_functions(0x40), (0x41, 0x2c));
    }

    #[test]
    fn motif_functions_allow_graph_resize_and_maximize() {
        assert_eq!(motif_wm_resizable_functions(0x2, 0), (0x3, 0x3e));
        assert_eq!(motif_wm_resizable_functions(0x3, 1), (0x3, 1));
    }

    #[test]
    fn preview_size_bounds_match_slint_window_constraints() {
        let main = include_str!("../ui/app.slint");
        assert!(main.contains("min-width: 900px;"));
        assert!(main.contains("max-width: 900px;"));
        assert!(main.contains("preferred-width: 900px;"));
        assert!(main.contains("min-height: 480px;"));
        assert!(main.contains("max-height: 480px;"));
        assert!(main.contains("preferred-height: 480px;"));
        for marker in [
            "changed maximized =>",
            "changed full-screen =>",
            "self.maximized = false;",
            "self.full-screen = false;",
        ] {
            assert!(
                main.contains(marker),
                "missing MainWindow runtime guard: {marker}"
            );
        }
        for marker in ["changed width =>", "changed height =>"] {
            assert!(
                !main.contains(marker),
                "unexpected MainWindow size guard: {marker}"
            );
        }

        let threads = include_str!("../ui/components.slint")
            .split("export component ThreadsWindow inherits Window {")
            .nth(1)
            .expect("ThreadsWindow");
        for marker in [
            "preferred-width: 900px;",
            "preferred-height: 480px;",
            "min-width: 900px;",
            "max-width: 900px;",
            "min-height: 480px;",
            "max-height: 480px;",
            "changed maximized =>",
            "changed full-screen =>",
            "for row[index] in root.thread-rows",
        ] {
            assert!(
                threads.contains(marker),
                "missing ThreadsWindow contract: {marker}"
            );
        }

        let graph = include_str!("../ui/components.slint")
            .split("export component GraphWindow inherits Window {")
            .nth(1)
            .expect("GraphWindow");
        assert!(graph.contains("min-width: 700px;"));
        assert!(graph.contains("preferred-width: 940px;"));
        assert!(graph.contains("min-height: 480px;"));
        assert!(graph.contains("preferred-height: 640px;"));
        for marker in [
            "max-width: 940px;",
            "max-height: 640px;",
            "changed width =>",
            "changed height =>",
            "changed maximized =>",
            "changed full-screen =>",
        ] {
            assert!(
                !graph.contains(marker),
                "unexpected GraphWindow bound or runtime guard: {marker}"
            );
        }
    }

    #[test]
    fn graph_layout_formula_matches_minimum_initial_and_expanded_contract() {
        let source = include_str!("../ui/components.slint");
        for expression in [
            "20px + (root.width - 700px) / 24",
            "root.width - 2 * root.content-x",
            "root.content-width - root.plot-left - root.current-label-gap - root.current-label-width - root.current-label-right-padding",
            "height: parent.height - root.history-toggle-y - 32px;",
            "height: parent.height - 52px;",
        ] {
            assert!(source.contains(expression), "missing layout formula: {expression}");
        }
        let geometry = |width: f64, height: f64| {
            let margin = (20.0 + (width - 700.0) / 24.0).clamp(20.0, 30.0);
            let content = width - 2.0 * margin;
            // 92px plot gutter + 10px leader gap + 80px dollar labels + 4px
            // right padding. Token mode reserves a wider label column.
            let plot_width = content - 186.0;
            let plot_height = height - 276.0;
            (margin, plot_width, plot_height)
        };
        assert_eq!(geometry(700.0, 480.0), (20.0, 474.0, 204.0));
        assert_eq!(geometry(940.0, 640.0), (30.0, 694.0, 364.0));
        assert_eq!(geometry(1_200.0, 800.0), (30.0, 954.0, 524.0));
        assert_eq!(geometry(1_201.0, 801.0).1 - geometry(1_200.0, 800.0).1, 1.0);
        assert_eq!(geometry(1_201.0, 801.0).2 - geometry(1_200.0, 800.0).2, 1.0);
    }

    #[test]
    fn fixed_windows_have_x11_state_monitor_without_runtime_size_repair() {
        let source = include_str!("main.rs");
        let source = source
            .split_once("#[cfg(test)]\nmod tests")
            .map(|(source, _)| source)
            .unwrap();
        assert!(!source.contains("struct WindowBounds"));
        assert!(!source.contains("enforce_window_bounds"));
        assert!(!source.contains("window.set_size(slint::LogicalSize::new("));
        assert!(source.contains("monitor.enforce(ui.window());"));
        assert!(source.contains("monitor.enforce(window.window());"));
        assert!(!source.contains("monitor.enforce(graph.window());"));
        assert_eq!(source.matches("Duration::from_millis(100)").count(), 1);
        assert_eq!(source.matches("GraphWindow::new()").count(), 1);
        assert_eq!(
            source
                .matches("show_and_focus_window(graph.window(),")
                .count(),
            1
        );
        assert_eq!(source.matches("graph.hide()").count(), 3);
    }

    #[test]
    fn existing_secondary_windows_are_raised_without_recreation() {
        let source = include_str!("main.rs")
            .split_once("#[cfg(test)]\nmod tests")
            .map(|(source, _)| source)
            .expect("production source");
        assert!(source.contains("fn show_and_focus_window("));
        assert!(source.contains("let was_visible = window.is_visible()"));
        assert!(
            source.contains("window.with_winit_window(|winit_window| winit_window.focus_window())")
        );
        assert!(source.contains("x11_monitor.raise_and_activate(window)"));
        assert!(source.contains("ConfigureWindowAux::new().stack_mode(StackMode::ABOVE)"));
        assert_eq!(
            source
                .matches("show_and_focus_window(graph.window(),")
                .count(),
            1
        );
        assert_eq!(
            source
                .matches("show_and_focus_window(window.window(),")
                .count(),
            2
        );
        assert_eq!(source.matches("ThreadsWindow::new()").count(), 1);
        assert_eq!(source.matches("LegalNoticeWindow::new()").count(), 1);
        assert!(!source.contains("graph.show()"));
        assert!(!source.contains("let _ = window.show();"));
    }

    #[test]
    fn secondary_close_hides_before_native_close_and_skips_hidden_work() {
        let source = include_str!("main.rs")
            .split_once("#[cfg(test)]\nmod tests")
            .map(|(source, _)| source)
            .expect("production source");
        assert_eq!(source.matches("on_close_requested(move ||").count(), 3);
        assert_eq!(
            source
                .matches("CloseRequestResponse::KeepWindowShown")
                .count(),
            3
        );
        assert!(source.contains("if graph.hide().is_ok()"));
        assert!(source.contains("if window.hide().is_ok()"));
        assert!(source.contains("if graph.window().is_visible()"));
        assert!(source.contains("if window.window().is_visible()"));

        let components = include_str!("../ui/components.slint");
        let action_button = components
            .split_once("component ActionButton inherits Rectangle {")
            .and_then(|(_, source)| source.split_once("export component WeekGauge"))
            .map(|(source, _)| source)
            .expect("ActionButton component");
        assert!(action_button.contains("touch-area := TouchArea"));
        assert!(action_button.contains("touch-area.pressed"));
        assert!(action_button.contains("activate-on-press: false"));
        assert!(action_button.contains("reset-press-state: false"));
        assert!(action_button.contains("changed pressed =>"));
        assert_eq!(components.matches("activate-on-press: true;").count(), 0);
        assert_eq!(
            components
                .matches("reset-press-state: root.reset-close-buttons;")
                .count(),
            0
        );
        assert_eq!(source.matches("set_reset_close_buttons(true)").count(), 12);
    }

    #[test]
    fn run_script_has_one_exec_without_retry_loop() {
        let run = include_str!("../run.sh");
        assert_eq!(run.matches("cargo run --manifest-path").count(), 1);
        assert!(run.contains("exec cargo run --manifest-path"));
        assert!(run.contains(r#"export WINIT_X11_SCALE_FACTOR="1""#));
        assert!(!run.contains("WINIT_X11_SCALE_FACTOR+x"));
        assert!(run.contains("--release --locked"));
        assert!(!run.contains("for attempt"));
        assert!(!run.contains("sleep 1"));
    }

    #[test]
    fn historical_period_uses_the_nearest_newer_reset_boundary() {
        let mut state = CodexInfoState::preview("monthly");
        let current_reset = 1_800_000_000;
        let previous_reset = current_reset - 7 * 86_400;
        state.reset_at = Some(current_reset);
        state.monthly = true;
        state.history.samples = vec![
            UsageHistorySample::new(
                1_700_000_000,
                current_reset,
                80.0,
                ModelDollarTotals::default(),
            ),
            UsageHistorySample::new(
                1_700_000_001,
                previous_reset,
                80.0,
                ModelDollarTotals::default(),
            ),
        ];
        assert_eq!(state.period_seconds_for_reset(previous_reset), 7 * 86_400);
        assert!(state.period_seconds_for_reset(current_reset) > 7 * 86_400);
    }

    #[test]
    fn codex_state_selects_latest_older_newer_periods_and_navigation_flags() {
        let mut state = CodexInfoState::preview("normal");
        state.reset_at = Some(300);
        state.history.samples = [300, 200, 100]
            .into_iter()
            .map(|reset_at| {
                UsageHistorySample::new(reset_at, reset_at, 80.0, ModelDollarTotals::default())
            })
            .collect();
        state.selected_reset_at = Some(100);

        state.select_latest_history();
        assert_eq!(state.selected_history_reset(), Some(300));
        assert_eq!(state.history_navigation(), (true, false));

        state.select_older_history();
        assert_eq!(state.selected_history_reset(), Some(200));
        assert_eq!(state.history_navigation(), (true, true));

        state.select_older_history();
        assert_eq!(state.selected_history_reset(), Some(100));
        assert_eq!(state.history_navigation(), (false, true));

        state.select_newer_history();
        assert_eq!(state.selected_history_reset(), Some(200));
        state.select_newer_history();
        assert_eq!(state.selected_history_reset(), Some(300));
        assert_eq!(state.history_navigation(), (true, false));
    }

    #[test]
    fn unlimited_status_has_no_countdown_copy() {
        assert_eq!(
            normal_status_text(50.0, i64::MAX, Some("12:34")),
            "最終更新 12:34"
        );
        let state = CodexInfoState::preview("unlimited");
        assert!(!state.has_quota_percent);
        assert!(state.reset_at.is_none());
        assert!(state.model_usage.is_empty());
        assert_eq!(state.quota_title, "利用枠");
        assert_eq!(state.normal_status(), "最終更新 12:34");
    }

    #[test]
    fn monthly_copy_avoids_zero_day_and_zero_hour_phrases() {
        let text = period_remaining_text(30 * 60, 31 * 86_400, true);
        assert!(text.contains("月間、あと30分"));
        assert!(!text.contains("0日"));
        assert!(!text.contains("0時間"));
    }

    #[test]
    fn history_periods_navigate_newest_older_newer_and_end_past_period_at_reset() {
        let mut history = UsageHistory::default();
        for (timestamp, reset_at) in [(100, 300), (200, 200), (300, 100)] {
            history.samples.push(UsageHistorySample::new(
                timestamp,
                reset_at,
                80.0,
                ModelDollarTotals::default(),
            ));
        }
        assert_eq!(history.reset_periods_desc(), vec![100, 200, 300]);
        assert_eq!(graph_period_end(200, Some(300), 350), 200);
        assert_eq!(graph_period_end(300, Some(300), 250), 250);
    }

    #[test]
    fn percentage_precision_is_limited_to_one_decimal() {
        assert_eq!(format_percent(64.04), "64.0%");
        assert_eq!(format_percent(64.0), "64%");
    }

    #[test]
    fn status_does_not_repeat_the_countdown() {
        assert_eq!(
            normal_status_text(5.0, 19 * 3_600, Some("12:34")),
            "残り利用枠が少なくなっています。"
        );
        assert_eq!(
            normal_status_text(50.0, 19 * 3_600, Some("12:34")),
            "リセット前後24時間です。"
        );
    }

    #[test]
    fn reset_warning_preview_exposes_the_reset_notice_without_low_quota_precedence() {
        let state = CodexInfoState::preview("reset-warning");
        assert_eq!(state.status, "リセット前後24時間です。");
        assert_eq!(state.status_level(), "warning");
    }

    #[test]
    fn refresh_copy_has_one_display_owner() {
        let slint = include_str!("../ui/app.slint");
        let rust = include_str!("main.rs");
        let rust_production = rust
            .split_once("#[cfg(test)]\nmod tests {")
            .map_or(rust, |(production, _)| production);
        let old_interval_copy = ["1分ごと", "に更新"].concat();
        assert!(!slint.contains(&old_interval_copy));
        assert!(!rust_production.contains(&old_interval_copy));
        assert_eq!(slint.matches("自動更新").count(), 0);
        assert_eq!(slint.matches("確認中…").count(), 0);
        assert!(rust_production.contains("最終更新 {}"));
    }

    #[test]
    fn account_activity_places_model_counts_on_a_separate_row() {
        let slint = include_str!("../ui/components.slint");
        let account = slint
            .split("export component AccountActivity inherits Rectangle {")
            .nth(1)
            .and_then(|body| {
                body.split("export component ThreadsWindow inherits Window {")
                    .next()
            })
            .expect("AccountActivity");
        assert!(account.contains("text: root.active-thread-count-label;"));
        assert!(account.contains("text: root.strings.model-threads;"));
        assert!(account.contains("label: \"SOL\";"));
        assert!(account.contains("label: \"TERRA\";"));
        assert!(account.contains("label: \"LUNA\";"));
        assert!(account.contains("label: root.strings.other;"));
        assert!(account.contains("x: parent.width - 112px;"));
        assert!(account.contains("width: 100px;\n        height: 24px;"));
    }

    #[test]
    fn dollar_graph_is_presented_as_independent_lines() {
        let slint = include_str!("../ui/components.slint");
        assert!(slint.contains("root.strings.graph-dollar-description"));
        assert!(!slint.contains("累積消費ドル（積み上げ）"));
        assert!(slint.contains("model: root.metric-options;"));
        assert!(slint.contains("current-index: root.selected-metric-index;"));
        assert!(!slint.contains("current-value:"));
        for marker in [
            "current-remaining-connector-path",
            "current-sol-connector-path",
            "current-terra-connector-path",
            "current-luna-connector-path",
            "current-label-gap: 10px;",
            "current-label-width: root.show-tokens ? 112px : 80px;",
            "current-label-right-padding: 4px;",
        ] {
            assert!(
                slint.contains(marker),
                "missing graph label mapping: {marker}"
            );
        }
        // Connector coordinates are normalized to the 0..100 viewbox. They
        // must fill the narrow label gap; otherwise Slint treats the values as
        // raw pixels and paints stray lines near the plot center/top.
        assert_eq!(
            slint
                .matches("fit: fill;\n                commands: root.current-")
                .count(),
            4
        );
        // An open SVG path must never be implicitly closed and painted to the
        // baseline; that was the visual source of the old stacked-area graph.
        assert_eq!(slint.matches("fill: transparent;").count(), 11);
    }

    #[test]
    fn model_usage_is_explicitly_token_based() {
        assert_eq!(
            format_model_usage_columns(&[
                preview_model_row("SOL", 1_234_567, 1_234_567, 234_567, 234_567),
                preview_model_row("TERRA", 99, 99, 0, 0),
                preview_model_row("LUNA", 42, 42, 0, 0),
            ]),
            (
                "SOL\nTERRA\nLUNA".into(),
                "1,000,000\n99\n42".into(),
                "$5\n$0\n$0".into(),
                "234,567\n0\n0".into(),
                "$0\n$0\n$0".into(),
                "234,567\n0\n0".into(),
                "$7\n$0\n$0".into()
            )
        );
    }

    #[test]
    fn model_rows_exclude_unknown_models() {
        let mut totals = ModelUsageTotals::default();
        totals.add(
            "gpt-5.6-sol",
            TokenSnapshot {
                total: 10,
                input: 8,
                cached_input: 2,
                output: 2,
            },
        );
        totals.add(
            "some-other-model",
            TokenSnapshot {
                total: 999,
                input: 999,
                cached_input: 0,
                output: 0,
            },
        );
        assert_eq!(
            totals
                .rows()
                .iter()
                .map(|row| row.name.as_str())
                .collect::<Vec<_>>(),
            ["SOL"]
        );
    }

    #[test]
    fn nested_session_events_preserve_the_sol_model_for_token_counts() {
        let context = json!({
            "type": "event_msg",
            "payload": {"type": "turn_context", "model": "gpt-5.6-sol"}
        });
        assert_eq!(session_event_type(&context), Some("turn_context"));
        assert_eq!(
            session_event_model(&context).as_deref(),
            Some("gpt-5.6-sol")
        );
        let top_level_context = json!({"type": "turn_context", "model": "gpt-5.6-sol"});
        assert_eq!(
            session_event_model(&top_level_context).as_deref(),
            Some("gpt-5.6-sol")
        );

        let token_count = json!({
            "timestamp": "2026-08-11T10:00:00Z",
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {"total_token_usage": {
                    "total_tokens": 120,
                    "input_tokens": 100,
                    "cached_input_tokens": 80,
                    "output_tokens": 20
                }}
            }
        });
        assert_eq!(session_event_type(&token_count), Some("token_count"));
        assert_eq!(session_token_snapshot(&token_count).unwrap().total, 120);
    }

    #[test]
    fn session_collector_counts_sol_when_model_context_is_nested() {
        let path = std::env::temp_dir().join(format!(
            "codex-info-sol-session-{}.jsonl",
            std::process::id()
        ));
        let lines = [
            json!({
                "timestamp": "2026-08-11T10:00:00Z",
                "type": "event_msg",
                "payload": {"type": "turn_context", "model": "gpt-5.6-sol"}
            }),
            json!({
                "timestamp": "2026-08-11T10:00:01Z",
                "type": "event_msg",
                "payload": {"type": "token_count", "info": {"total_token_usage": {
                    "total_tokens": 100, "input_tokens": 80,
                    "cached_input_tokens": 20, "output_tokens": 20
                }}}
            }),
            json!({
                "timestamp": "2026-08-11T10:00:02Z",
                "type": "event_msg",
                "payload": {"type": "token_count", "info": {"total_token_usage": {
                    "total_tokens": 150, "input_tokens": 120,
                    "cached_input_tokens": 30, "output_tokens": 30
                }}}
            }),
        ];
        fs::write(
            &path,
            lines
                .iter()
                .map(|line| serde_json::to_string(line).unwrap())
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        let mut totals = ModelUsageTotals::default();
        collect_session_file(&path, &mut totals, 0).unwrap();
        assert_eq!(totals.sol.tokens, 150);
        assert_eq!(totals.sol.output_tokens, 30);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn session_settings_do_not_reassign_tokens_before_the_next_turn_context() {
        let path = std::env::temp_dir().join(format!(
            "codex-info-model-switch-{}.jsonl",
            std::process::id()
        ));
        let lines = [
            json!({
                "timestamp": "2026-08-11T10:00:00Z",
                "type": "turn_context",
                "model": "gpt-5.6-luna"
            }),
            json!({
                "timestamp": "2026-08-11T10:00:01Z",
                "type": "event_msg",
                "payload": {"type": "token_count", "info": {"total_token_usage": {
                    "total_tokens": 100, "input_tokens": 80,
                    "cached_input_tokens": 20, "output_tokens": 20
                }}}
            }),
            json!({
                "timestamp": "2026-08-11T10:00:02Z",
                "type": "event_msg",
                "payload": {"type": "thread_settings_applied",
                    "thread_settings": {"model": "gpt-5.6-sol"}}
            }),
            json!({
                "timestamp": "2026-08-11T10:00:03Z",
                "type": "event_msg",
                "payload": {"type": "token_count", "info": {"total_token_usage": {
                    "total_tokens": 150, "input_tokens": 120,
                    "cached_input_tokens": 30, "output_tokens": 30
                }}}
            }),
            json!({
                "timestamp": "2026-08-11T10:00:04Z",
                "type": "turn_context",
                "model": "gpt-5.6-sol"
            }),
            json!({
                "timestamp": "2026-08-11T10:00:05Z",
                "type": "event_msg",
                "payload": {"type": "token_count", "info": {"total_token_usage": {
                    "total_tokens": 200, "input_tokens": 160,
                    "cached_input_tokens": 40, "output_tokens": 40
                }}}
            }),
        ];
        fs::write(
            &path,
            lines
                .iter()
                .map(|line| serde_json::to_string(line).unwrap())
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        let mut totals = ModelUsageTotals::default();
        collect_session_file(&path, &mut totals, 0).unwrap();
        assert_eq!(totals.luna.tokens, 150);
        assert_eq!(totals.sol.tokens, 50);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn week_text_includes_minutes_for_a_full_countdown() {
        assert_eq!(
            week_remaining_text(6 * 86_400 + 9 * 3_600 + 12 * 60),
            "7日中、あと6日と9時間12分"
        );
    }

    #[test]
    fn history_replaces_a_minute_without_discarding_the_previous_reset_period() {
        let mut history = UsageHistory::default();
        let previous_reset_at = 1_700_100_000;
        let next_reset_at = 1_700_200_000;
        history.record(UsageHistorySample::new(
            1_700_000_001,
            previous_reset_at,
            80.0,
            ModelDollarTotals {
                sol: 1.0,
                terra: 2.0,
                luna: 3.0,
            },
        ));
        history.record(UsageHistorySample::new(
            1_700_000_039,
            previous_reset_at,
            75.0,
            ModelDollarTotals {
                sol: 4.0,
                terra: 5.0,
                luna: 6.0,
            },
        ));
        assert_eq!(history.samples.len(), 1);
        assert_eq!(history.samples[0].remaining_percent, 75.0);
        assert_eq!(history.samples[0].sol_dollars, 4.0);

        history.record(UsageHistorySample::new(
            1_700_000_120,
            next_reset_at,
            70.0,
            ModelDollarTotals::default(),
        ));
        assert_eq!(history.samples.len(), 2);
        assert!(history
            .samples
            .iter()
            .any(|sample| sample.reset_at == previous_reset_at));
        assert!(history
            .samples
            .iter()
            .any(|sample| sample.reset_at == next_reset_at));
        assert_eq!(history.samples_for_reset(Some(previous_reset_at)).len(), 1);
        assert_eq!(history.samples_for_reset(Some(next_reset_at)).len(), 1);
    }

    #[test]
    fn reset_at_jitter_is_one_period_and_duplicate_timestamps_merge_for_display() {
        let mut history = UsageHistory::default();
        let first = UsageHistorySample::new(
            1_700_000_001,
            1_700_100_000,
            80.0,
            ModelDollarTotals {
                sol: 1.0,
                terra: 2.0,
                luna: 3.0,
            },
        );
        let jittered = UsageHistorySample::new(
            1_700_000_039,
            1_700_100_003,
            75.0,
            ModelDollarTotals {
                sol: 4.0,
                terra: 5.0,
                luna: 6.0,
            },
        );

        history.record(first);
        history.record(jittered);

        assert_eq!(history.samples.len(), 1);
        assert_eq!(history.samples[0].reset_at, 1_700_100_000);
        assert_eq!(history.samples[0].remaining_percent, 75.0);
        assert_eq!(history.samples_for_reset(Some(1_700_100_003)).len(), 1);

        history.samples = vec![
            UsageHistorySample::new(
                1_700_000_000,
                1_700_100_000,
                80.0,
                ModelDollarTotals {
                    sol: 1.0,
                    terra: 2.0,
                    luna: 3.0,
                },
            ),
            UsageHistorySample::new(
                1_700_000_000,
                1_700_100_003,
                75.0,
                ModelDollarTotals {
                    sol: 4.0,
                    terra: 5.0,
                    luna: 6.0,
                },
            ),
        ];
        let displayed = history.samples_for_reset(Some(1_700_100_000));
        assert_eq!(displayed.len(), 1);
        // A sixty-second jitter group uses its greatest observed reset time as
        // the stable period identifier.
        assert_eq!(displayed[0].reset_at, 1_700_100_003);
        assert_eq!(displayed[0].remaining_percent, 75.0);
        assert_eq!(displayed[0].sol_dollars, 4.0);
    }

    #[test]
    fn session_backfill_keeps_an_observed_remaining_value() {
        let mut history = UsageHistory {
            samples: vec![UsageHistorySample::new(
                60,
                1_000,
                42.0,
                ModelDollarTotals {
                    sol: 1.0,
                    terra: 1.0,
                    luna: 1.0,
                },
            )],
            ..UsageHistory::default()
        };
        let backfill = UsageHistorySample::from_model_history(
            60,
            1_003,
            ModelDollarTotals {
                sol: 9.0,
                terra: 8.0,
                luna: 7.0,
            },
        );

        history.apply_backfill_samples(1_003, vec![backfill]);

        assert_eq!(history.samples.len(), 1);
        assert_eq!(history.samples[0].remaining_percent, 42.0);
        assert_eq!(history.samples[0].sol_dollars, 9.0);
        assert_eq!(history.samples[0].reset_at, 1_000);
    }

    #[test]
    fn startup_maintenance_prunes_before_the_calendar_cutoff_only_once() {
        let (json_path, db_path) = test_history_paths("startup-maintenance");
        let now = Utc.with_ymd_and_hms(2024, 5, 31, 12, 34, 56).unwrap();
        let cutoff = three_months_before_utc(now);
        let samples = [
            UsageHistorySample {
                timestamp: cutoff - 1,
                reset_at: cutoff + 10_000,
                remaining_percent: 80.0,
                sol_dollars: 1.0,
                terra_dollars: 0.0,
                luna_dollars: 0.0,
                sol_tokens: 0,
                terra_tokens: 0,
                luna_tokens: 0,
            },
            UsageHistorySample {
                timestamp: cutoff,
                reset_at: cutoff + 20_000,
                remaining_percent: 70.0,
                sol_dollars: 2.0,
                terra_dollars: 0.0,
                luna_dollars: 0.0,
                sol_tokens: 0,
                terra_tokens: 0,
                luna_tokens: 0,
            },
        ];
        let mut store = UsageStore::open(&db_path).unwrap();
        store
            .upsert_samples(
                &samples
                    .iter()
                    .map(UsageHistorySample::to_store)
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        drop(store);

        let mut history =
            UsageHistory::load_from_paths(Some(json_path.clone()), Some(db_path.clone()));
        history.startup_maintenance(now);

        assert_eq!(
            history
                .samples
                .iter()
                .map(|sample| sample.timestamp)
                .collect::<Vec<_>>(),
            vec![cutoff]
        );
        let persisted = UsageStore::open(&db_path).unwrap().load_all().unwrap();
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].timestamp, cutoff);

        history.samples.push(samples[0].clone());
        history.startup_maintenance(now);
        assert!(history
            .samples
            .iter()
            .any(|sample| sample.timestamp == cutoff - 1));
        let _ = fs::remove_dir_all(json_path.parent().unwrap());
    }

    #[test]
    fn startup_maintenance_still_bounds_visible_memory_when_store_pruning_fails() {
        let (json_path, _db_path) = test_history_paths("startup-maintenance-error");
        let db_path = json_path.parent().unwrap().join("not-a-database");
        fs::create_dir_all(&db_path).unwrap();
        let now = Utc.with_ymd_and_hms(2024, 5, 31, 12, 34, 56).unwrap();
        let sample = |timestamp| UsageHistorySample {
            timestamp,
            reset_at: now.timestamp() + 1,
            remaining_percent: 80.0,
            sol_dollars: 1.0,
            terra_dollars: 0.0,
            luna_dollars: 0.0,
            sol_tokens: 0,
            terra_tokens: 0,
            luna_tokens: 0,
        };
        let mut history = UsageHistory {
            path: None,
            db_path: Some(db_path.clone()),
            samples: vec![
                sample(1),
                sample(now.timestamp()),
                sample(now.timestamp() + 1),
            ],
            startup_maintenance_done: false,
        };

        history.startup_maintenance(now);

        assert_eq!(history.samples.len(), 1);
        assert_eq!(history.samples[0].timestamp, now.timestamp());
        let _ = fs::remove_dir_all(json_path.parent().unwrap());
    }

    fn write_recovery_ledger(name: &str, rows: &[serde_json::Value]) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "codex-info-recovery-{name}-{}-{}.jsonl",
            std::process::id(),
            rows.len()
        ));
        let contents = rows
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn recovery_ledger_filters_invalid_unknown_duplicate_and_out_of_window_rows() {
        let path = write_recovery_ledger(
            "filters",
            &[
                json!({
                    "timestamp": 100,
                    "thread_id": "luna-thread",
                    "model": "gpt-5.6-luna",
                    "input_tokens": 100,
                    "cached_input_tokens": 25,
                    "output_tokens": 30,
                    "reasoning_tokens": 40
                }),
                json!({
                    "timestamp": 101,
                    "thread_id": "luna-thread",
                    "model": "gpt-5.6-luna",
                    "input_tokens": 900,
                    "cached_input_tokens": 0,
                    "output_tokens": 900,
                    "reasoning_tokens": 0
                }),
                json!({
                    "timestamp": 200,
                    "thread_id": "sol-thread",
                    "model": "gpt-5.6-sol",
                    "input_tokens": 8,
                    "cached_input_tokens": 2,
                    "output_tokens": 4,
                    "reasoning_tokens": 1
                }),
                json!({
                    "timestamp": 150,
                    "thread_id": "unknown-thread",
                    "model": "gpt-5.6-unknown",
                    "input_tokens": 1000,
                    "cached_input_tokens": 0,
                    "output_tokens": 1000,
                    "reasoning_tokens": 0
                }),
                json!({
                    "timestamp": 99,
                    "thread_id": "before-thread",
                    "model": "gpt-5.6-luna",
                    "input_tokens": 1000,
                    "cached_input_tokens": 0,
                    "output_tokens": 1000,
                    "reasoning_tokens": 0
                }),
                json!({
                    "timestamp": 201,
                    "thread_id": "after-thread",
                    "model": "gpt-5.6-luna",
                    "input_tokens": 1000,
                    "cached_input_tokens": 0,
                    "output_tokens": 1000,
                    "reasoning_tokens": 0
                }),
                json!({"timestamp": 150, "thread_id": "invalid"}),
                json!({
                    "timestamp": 150,
                    "thread_id": "blank-model",
                    "model": "",
                    "input_tokens": 1,
                    "cached_input_tokens": 0,
                    "output_tokens": 1,
                    "reasoning_tokens": 0
                }),
            ],
        );

        let entries = read_recovery_entries(&path, 100, 200);
        assert_eq!(entries.len(), 2);
        let mut totals = ModelUsageTotals::default();
        add_recovery_usage(Some(&path), 100, 200, &mut totals);
        assert_eq!(totals.luna.tokens, 130);
        assert_eq!(totals.luna.input_tokens, 100);
        assert_eq!(totals.luna.cached_input_tokens, 25);
        assert_eq!(totals.luna.output_tokens, 30);
        assert_eq!(totals.sol.tokens, 12);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn recovery_ledger_events_become_timestamped_cumulative_timeline_points() {
        let path = write_recovery_ledger(
            "timeline",
            &[
                json!({
                    "timestamp": 120,
                    "thread_id": "first",
                    "model": "gpt-5.6-luna",
                    "input_tokens": 100,
                    "cached_input_tokens": 20,
                    "output_tokens": 30,
                    "reasoning_tokens": 10
                }),
                json!({
                    "timestamp": 180,
                    "thread_id": "second",
                    "model": "gpt-5.6-luna",
                    "input_tokens": 50,
                    "cached_input_tokens": 0,
                    "output_tokens": 10,
                    "reasoning_tokens": 5
                }),
            ],
        );

        let events = recovery_timed_usage(&path, 120, 180);
        let samples = model_usage_timeline_from_events(events, 240);
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].timestamp, 120);
        assert_eq!(samples[1].timestamp, 180);
        assert!(samples[1].luna_dollars > samples[0].luna_dollars);
        let _ = fs::remove_file(path);
    }

    fn test_history_paths(name: &str) -> (PathBuf, PathBuf) {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "codex-info-history-{name}-{}-{id}",
            std::process::id()
        ));
        (
            root.join("usage_history.json"),
            root.join("usage_history.sqlite3"),
        )
    }

    fn history_keys(samples: &[super::usage_store::UsageHistorySample]) -> BTreeSet<(i64, i64)> {
        samples
            .iter()
            .map(|sample| (sample.reset_at, sample.timestamp))
            .collect()
    }

    #[test]
    fn sqlite_history_cutoff_and_period_list_integration() {
        let (json_path, db_path) = test_history_paths("cutoff-period-list");
        let now = Utc.with_ymd_and_hms(2024, 5, 31, 12, 34, 56).unwrap();
        let cutoff = three_months_before_utc(now);
        let record = |timestamp, reset_at, remaining_percent| UsageHistorySample {
            timestamp,
            reset_at,
            remaining_percent,
            sol_dollars: 0.0,
            terra_dollars: 0.0,
            luna_dollars: 0.0,
            sol_tokens: 0,
            terra_tokens: 0,
            luna_tokens: 0,
        };
        let records = [
            record(cutoff - 1, cutoff + 10_000, 90.0),
            record(cutoff, cutoff + 20_000, 80.0),
            record(now.timestamp(), now.timestamp() + 30_000, 70.0),
            record(now.timestamp() + 1, now.timestamp() + 40_000, 60.0),
        ];
        let mut store = UsageStore::open(&db_path).unwrap();
        store
            .upsert_samples(
                &records
                    .iter()
                    .map(UsageHistorySample::to_store)
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        drop(store);

        let mut history =
            UsageHistory::load_from_paths(Some(json_path.clone()), Some(db_path.clone()));
        history.startup_maintenance(now);
        assert_eq!(
            history
                .samples
                .iter()
                .map(|sample| sample.timestamp)
                .collect::<Vec<_>>(),
            vec![cutoff, now.timestamp()]
        );
        let periods = history.periods(now.timestamp(), Some(now.timestamp() + 30_000));
        assert_eq!(periods.len(), 2);
        assert_eq!(history.period_options(now.timestamp(), None).len(), 2);
        for period in periods {
            assert_eq!(
                history.period_id_for_label(
                    &period.label,
                    now.timestamp(),
                    Some(now.timestamp() + 30_000),
                ),
                Some(period.canonical_reset_at)
            );
        }
        let persisted = UsageStore::open(&db_path).unwrap().load_all().unwrap();
        assert_eq!(
            persisted
                .iter()
                .map(|sample| sample.timestamp)
                .collect::<Vec<_>>(),
            vec![cutoff, now.timestamp(), now.timestamp() + 1]
        );
        let _ = fs::remove_dir_all(json_path.parent().unwrap());
    }

    #[test]
    fn history_period_grouping_boundary_and_label_mapping_are_unambiguous() {
        let (json_path, db_path) = test_history_paths("period-grouping-contract");
        let samples = [1_000, 1_060, 1_061, 1_121, 1_122].map(|reset_at| {
            UsageHistorySample::new(100, reset_at, 80.0, ModelDollarTotals::default())
        });
        let mut store = UsageStore::open(&db_path).unwrap();
        store
            .upsert_samples(
                &samples
                    .iter()
                    .map(UsageHistorySample::to_store)
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        drop(store);
        let history = UsageHistory::load_from_paths(Some(json_path.clone()), Some(db_path));

        let periods = history.periods(2_000, None);
        assert_eq!(
            periods
                .iter()
                .map(|period| period.canonical_reset_at)
                .collect::<Vec<_>>(),
            vec![1_122, 1_121, 1_060]
        );
        let labels = periods
            .iter()
            .map(|period| period.label.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(labels.len(), periods.len());
        assert!(
            periods
                .iter()
                .filter(|period| period.label.contains("（期限 "))
                .count()
                >= 2
        );
        assert!(periods
            .iter()
            .filter(|period| period.label.contains("（期限 "))
            .all(|period| period.label.contains("JST")));
        for period in &periods {
            assert_eq!(
                history.period_id_for_label(&period.label, 2_000, None),
                Some(period.canonical_reset_at)
            );
            assert_eq!(
                history
                    .samples_for_reset(Some(period.canonical_reset_at))
                    .len(),
                1
            );
        }
        assert_eq!(
            UsageHistory::default().period_options(2_000, None),
            vec!["履歴なし"]
        );
        let _ = fs::remove_dir_all(json_path.parent().unwrap());
    }

    #[test]
    fn period_list_hides_legacy_moving_resets_but_keeps_real_singletons_and_db_rows() {
        let (json_path, db_path) = test_history_paths("rolling-reset-artifacts");
        let base = 1_699_999_980;
        let stable_reset = base + 400_000;
        let ghost_one = base + 120 + WEEK_SECONDS + 23;
        let ghost_two = base + 180 + WEEK_SECONDS + 48;
        let real_singleton_timestamp = base + 7_200;
        let real_singleton_reset = real_singleton_timestamp + WEEK_SECONDS + 30;
        let samples = [
            UsageHistorySample::new(base, stable_reset, 72.0, ModelDollarTotals::default()),
            UsageHistorySample::new(base + 60, stable_reset, 71.0, ModelDollarTotals::default()),
            UsageHistorySample::new(base + 120, ghost_one, 100.0, ModelDollarTotals::default()),
            UsageHistorySample::new(base + 180, ghost_two, 100.0, ModelDollarTotals::default()),
            UsageHistorySample::new(
                real_singleton_timestamp,
                real_singleton_reset,
                100.0,
                ModelDollarTotals::default(),
            ),
        ];
        let mut store = UsageStore::open(&db_path).unwrap();
        store
            .upsert_samples(
                &samples
                    .iter()
                    .map(UsageHistorySample::to_store)
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        drop(store);

        let history = UsageHistory::load_from_paths(Some(json_path.clone()), Some(db_path.clone()));
        let period_ids = history
            .periods(base + 300, None)
            .into_iter()
            .map(|period| period.canonical_reset_at)
            .collect::<Vec<_>>();
        assert_eq!(period_ids, vec![real_singleton_reset, stable_reset]);
        assert!(history.samples_for_reset(Some(ghost_one)).is_empty());
        assert_eq!(
            history.samples_for_reset(Some(real_singleton_reset)).len(),
            1
        );
        assert_eq!(
            UsageStore::open(&db_path)
                .unwrap()
                .load_all()
                .unwrap()
                .len(),
            5
        );
        let _ = fs::remove_dir_all(json_path.parent().unwrap());
    }

    #[test]
    fn json_migration_is_idempotent_and_keeps_the_db_set_as_a_superset() {
        let (json_path, db_path) = test_history_paths("migration");
        let json_only = UsageHistorySample::new(
            1_700_000_060,
            1_700_100_000,
            80.0,
            ModelDollarTotals::default(),
        );
        let db_only = UsageHistorySample::new(
            1_700_000_120,
            1_700_200_000,
            70.0,
            ModelDollarTotals {
                sol: 4.0,
                terra: 5.0,
                luna: 6.0,
            },
        );

        let store = UsageStore::open(&db_path).unwrap();
        store.upsert_sample(&db_only.to_store()).unwrap();
        let db_before = history_keys(&store.load_all().unwrap());
        drop(store);
        fs::write(
            &json_path,
            serde_json::to_vec(&vec![json_only.clone()]).unwrap(),
        )
        .unwrap();

        let first_load =
            UsageHistory::load_from_paths(Some(json_path.clone()), Some(db_path.clone()));
        let after_first = history_keys(&UsageStore::open(&db_path).unwrap().load_all().unwrap());
        assert!(after_first.is_superset(&db_before));
        assert!(after_first.contains(&(json_only.reset_at, json_only.timestamp)));
        drop(first_load);

        let second_load =
            UsageHistory::load_from_paths(Some(json_path.clone()), Some(db_path.clone()));
        let after_second = history_keys(&UsageStore::open(&db_path).unwrap().load_all().unwrap());
        assert_eq!(after_second, after_first);
        drop(second_load);
        let _ = fs::remove_dir_all(json_path.parent().unwrap());
    }

    #[test]
    fn graph_json_only_contains_the_selected_reset_period() {
        let history = UsageHistory {
            samples: vec![
                UsageHistorySample::new(
                    1_700_000_000,
                    1_700_100_000,
                    50.0,
                    ModelDollarTotals::default(),
                ),
                UsageHistorySample::new(
                    1_700_000_060,
                    1_700_200_000,
                    40.0,
                    ModelDollarTotals::default(),
                ),
            ],
            ..UsageHistory::default()
        };
        let data = history.graph_data_for_reset(1_700_200_000);
        assert!(data.contains("1700200000"));
        assert!(!data.contains("1700100000"));
        assert!(data.contains("remaining_percent"));
    }

    #[test]
    fn preview_history_points_are_spread_before_now() {
        let now = 1_700_000_000;
        let history = UsageHistory::preview(now, now + 6 * 86_400, ModelDollarTotals::default());
        assert_eq!(history.samples.len(), 12);
        assert_eq!(history.reset_periods_desc().len(), 2);
        assert_eq!(
            history
                .samples
                .iter()
                .filter(|sample| sample.reset_at == now + 6 * 86_400)
                .count(),
            6
        );
        assert_eq!(
            history
                .samples
                .iter()
                .filter(|sample| sample.reset_at == now - 86_400)
                .count(),
            6
        );
        assert!(history
            .samples
            .windows(2)
            .all(|pair| pair[0].timestamp < pair[1].timestamp));
        assert!(history.samples.iter().all(|sample| sample.timestamp <= now));
    }

    #[test]
    fn model_cost_history_keeps_cumulative_totals() {
        let reset_at = 1_700_100_000;
        let first = UsageHistorySample::new(
            1_700_000_000,
            reset_at,
            50.0,
            ModelDollarTotals {
                sol: 1.0,
                terra: 2.0,
                luna: 0.0,
            },
        );
        let second = UsageHistorySample::new(
            1_700_001_800,
            reset_at,
            45.0,
            ModelDollarTotals {
                sol: 3.0,
                terra: 4.0,
                luna: 1.0,
            },
        );
        let third = UsageHistorySample::new(
            1_700_004_000,
            reset_at,
            40.0,
            ModelDollarTotals {
                sol: 5.0,
                terra: 5.0,
                luna: 2.0,
            },
        );
        let samples = [&first, &second, &third];
        let minute = minute_model_spend(&samples);
        assert_eq!(minute.len(), 3);
        assert_eq!(
            (minute[0].sol, minute[0].terra, minute[0].luna),
            (1.0, 2.0, 0.0)
        );
        assert_eq!(
            (minute[1].sol, minute[1].terra, minute[1].luna),
            (3.0, 4.0, 1.0)
        );
        assert_eq!(
            (minute[2].sol, minute[2].terra, minute[2].luna),
            (5.0, 5.0, 2.0)
        );
    }

    #[test]
    fn dollar_graph_does_not_recount_a_regressed_snapshot() {
        let reset_at = 1_700_100_000;
        let samples = [
            UsageHistorySample::new(
                100,
                reset_at,
                90.0,
                ModelDollarTotals {
                    sol: 50.0,
                    terra: 2.0,
                    luna: 1.0,
                },
            ),
            // A transiently incomplete scan must not reset the cumulative
            // total and make the next 51-dollar observation count as +51.
            UsageHistorySample::new(
                160,
                reset_at,
                89.0,
                ModelDollarTotals {
                    sol: 49.0,
                    terra: 2.0,
                    luna: 1.0,
                },
            ),
            UsageHistorySample::new(
                220,
                reset_at,
                88.0,
                ModelDollarTotals {
                    sol: 51.0,
                    terra: 2.0,
                    luna: 1.0,
                },
            ),
        ];
        let references = samples.iter().collect::<Vec<_>>();
        let points = minute_model_spend(&references);
        assert_eq!(points[0].sol, 50.0);
        assert_eq!(points[1].sol, 50.0);
        assert_eq!(points[2].sol, 51.0);

        let graph = graph_paths_for_selection(&references, 0, 240, false, false, true, false);
        assert_eq!(graph.current_sol_label, "$51.00");
    }

    #[test]
    fn graph_model_points_are_anchored_to_start_and_now() {
        let points = graph_time_endpoints(
            vec![HourlyModelSpend {
                timestamp: 120,
                sol: 3.0,
                terra: 2.0,
                luna: 1.0,
            }],
            100,
            300,
        );
        assert_eq!(points.first().map(|point| point.timestamp), Some(100));
        assert_eq!(
            (points[0].sol, points[0].terra, points[0].luna),
            (0.0, 0.0, 0.0)
        );
        assert_eq!(points.last().map(|point| point.timestamp), Some(300));
        assert_eq!(
            (points[2].sol, points[2].terra, points[2].luna),
            (3.0, 2.0, 1.0)
        );
    }

    #[test]
    fn graph_model_endpoints_do_not_duplicate_measurement_x_coordinates() {
        let points = graph_time_endpoints(
            vec![HourlyModelSpend {
                timestamp: 100,
                sol: 3.0,
                terra: 2.0,
                luna: 1.0,
            }],
            100,
            100,
        );
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].timestamp, 100);
        assert_eq!(
            (points[0].sol, points[0].terra, points[0].luna),
            (3.0, 2.0, 1.0)
        );
    }

    #[test]
    fn graph_paths_span_from_reset_start_to_current_time() {
        let reset_at = 7_200;
        let first = UsageHistorySample::new(
            600,
            reset_at,
            80.0,
            ModelDollarTotals {
                sol: 1.0,
                terra: 0.0,
                luna: 0.0,
            },
        );
        let latest = UsageHistorySample::new(
            3_600,
            reset_at,
            70.0,
            ModelDollarTotals {
                sol: 4.0,
                terra: 2.0,
                luna: 1.0,
            },
        );
        let paths = graph_paths(&[&first, &latest], 0, 3_900);
        assert!(paths.remaining.starts_with("M0.00 1.00"));
        assert!(paths.remaining.contains("L15.38"));
        assert!(!paths.remaining.contains("L15.38 1.00 L15.38"));
        assert!(paths.remaining.contains("L100.00"));
        assert!(paths.sol.starts_with("M0.00 99.00"));
        assert!(paths.sol.contains("L100.00"));
        assert!(paths.terra.contains("L100.00"));
        assert!(paths.luna.contains("L100.00"));
        assert_eq!(paths.current_remaining_label, "70%");
        assert_eq!(paths.current_sol_label, "$4.00");
        assert_eq!(paths.current_terra_label, "$2.00");
        assert_eq!(paths.current_luna_label, "$1.00");
        assert!((paths.current_sol_y - 0.01).abs() < 0.0001);
        assert!((paths.current_terra_y - 0.50).abs() < 0.0001);
        assert!((paths.current_luna_y - 0.745).abs() < 0.0001);
    }

    #[test]
    fn remaining_graph_collapses_unchanged_runs_between_change_points() {
        let samples = [
            UsageHistorySample::new(0, 1_000, 100.0, ModelDollarTotals::default()),
            UsageHistorySample::new(60, 1_000, 90.0, ModelDollarTotals::default()),
            UsageHistorySample::new(120, 1_000, 90.0, ModelDollarTotals::default()),
            UsageHistorySample::new(180, 1_000, 70.0, ModelDollarTotals::default()),
        ];
        let references = samples.iter().collect::<Vec<_>>();
        let points = graph_points(&references, 0, 240, 100.0, |sample| {
            sample.remaining_percent
        });

        // The unchanged 90% snapshot is not a visible anchor. The endpoint
        // remains explicit so the rendered line reaches the current-time edge.
        assert_eq!(
            points
                .iter()
                .map(|(timestamp, _)| *timestamp)
                .collect::<Vec<_>>(),
            vec![0, 60, 180, 240]
        );
        let path = graph_paths(&references, 0, 240).remaining;
        assert!(!path.contains("L50.00"));
        assert!(path.contains("L25.00") && path.contains("L75.00"));
    }

    #[test]
    fn remaining_change_point_collapse_clamps_upward_rereads() {
        let points = collapse_remaining_change_points(&[
            (0, 100.0),
            (60, 80.0),
            (120, 85.0),
            (180, 70.0),
            (240, 70.0),
        ]);
        assert_eq!(
            points,
            vec![(0, 100.0), (60, 80.0), (180, 70.0), (240, 70.0)]
        );
    }

    #[test]
    fn remaining_label_matches_smoothed_path_endpoint() {
        let samples = [
            UsageHistorySample::new(0, 1_000, 100.0, ModelDollarTotals::default()),
            UsageHistorySample::new(60, 1_000, 0.0, ModelDollarTotals::default()),
            // A transient upward reread must not move the line endpoint back
            // above the last monotonic value.
            UsageHistorySample::new(120, 1_000, 10.0, ModelDollarTotals::default()),
        ];
        let references = samples.iter().collect::<Vec<_>>();
        let paths = graph_paths(&references, 0, 120);
        assert_eq!(paths.current_remaining_label, "0%");
        assert!(paths.remaining.ends_with("L100.00 99.00"));
    }

    #[test]
    fn remaining_markers_interpolate_each_integer_boundary() {
        let first = UsageHistorySample::new(0, 1_000, 100.0, ModelDollarTotals::default());
        let second = UsageHistorySample::new(60, 1_000, 99.0, ModelDollarTotals::default());
        let third = UsageHistorySample::new(120, 1_000, 97.0, ModelDollarTotals::default());

        let markers = remaining_marker_positions(&[&first, &second, &third], 0, 120);

        assert_eq!(
            markers
                .iter()
                .map(|marker| marker.boundary)
                .collect::<Vec<_>>(),
            [99, 98, 97]
        );
        assert!((markers[0].x - 40.0).abs() < f64::EPSILON);
        assert!((markers[1].x - (500.0 / 7.0)).abs() < 0.000_000_1);
        assert!((markers[2].x - 100.0).abs() < f64::EPSILON);
        for (marker, boundary) in markers.iter().zip([99.0, 98.0, 97.0]) {
            let expected_y = 99.0 - boundary * 0.98;
            assert!((marker.y - expected_y).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn remaining_markers_are_on_the_same_smoothed_line_segments() {
        let first = UsageHistorySample::new(0, 1_000, 100.0, ModelDollarTotals::default());
        let second = UsageHistorySample::new(60, 1_000, 99.0, ModelDollarTotals::default());
        let third = UsageHistorySample::new(120, 1_000, 97.0, ModelDollarTotals::default());
        let samples = [&first, &second, &third];
        let points = graph_points(&samples, 0, 120, 100.0, |sample| sample.remaining_percent);
        let markers = remaining_marker_positions_on_points(&points, 0, 120);

        assert_eq!(points.first(), Some(&(0, 100.0)));
        assert!(points.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for marker in markers {
            let marker_timestamp = marker.x / 100.0 * 120.0;
            let Some([before, after]) = points.windows(2).find_map(|window| {
                let [before, after] = window else { return None };
                (marker_timestamp >= before.0 as f64
                    && marker_timestamp <= after.0 as f64
                    && after.0 > before.0)
                    .then_some([before, after])
            }) else {
                panic!("marker must lie on a remaining path segment: {marker:?}");
            };
            let fraction = (marker_timestamp - before.0 as f64) / (after.0 - before.0) as f64;
            let line_value = before.1 + (after.1 - before.1) * fraction;
            assert!((marker.y - remaining_graph_y(line_value)).abs() < 0.000_000_1);
        }
    }

    #[test]
    fn remaining_markers_use_the_reset_anchor_for_the_first_observation() {
        let sample = UsageHistorySample::new(60, 1_000, 97.0, ModelDollarTotals::default());

        let markers = remaining_marker_positions(&[&sample], 0, 120);

        assert_eq!(
            markers
                .iter()
                .map(|marker| marker.boundary)
                .collect::<Vec<_>>(),
            [99, 98, 97]
        );
    }

    #[test]
    fn remaining_markers_do_not_duplicate_a_boundary() {
        let samples = [
            UsageHistorySample::new(0, 1_000, 100.0, ModelDollarTotals::default()),
            UsageHistorySample::new(60, 1_000, 99.0, ModelDollarTotals::default()),
            UsageHistorySample::new(120, 1_000, 97.0, ModelDollarTotals::default()),
            UsageHistorySample::new(180, 1_000, 98.0, ModelDollarTotals::default()),
            UsageHistorySample::new(240, 1_000, 96.0, ModelDollarTotals::default()),
        ];

        let references = samples.iter().collect::<Vec<_>>();
        let markers = remaining_marker_positions(&references, 0, 240);

        assert_eq!(
            markers
                .iter()
                .map(|marker| marker.boundary)
                .collect::<Vec<_>>(),
            [99, 98, 97, 96]
        );
    }

    #[test]
    fn remaining_markers_filter_out_missing_values() {
        let first = UsageHistorySample::new(0, 1_000, 100.0, ModelDollarTotals::default());
        let second = UsageHistorySample::new(60, 1_000, 99.0, ModelDollarTotals::default());
        let missing =
            UsageHistorySample::from_model_history(120, 1_000, ModelDollarTotals::default());
        let last = UsageHistorySample::new(180, 1_000, 97.0, ModelDollarTotals::default());

        let markers = remaining_marker_positions(&[&first, &second, &missing, &last], 0, 180);

        assert_eq!(
            markers
                .iter()
                .map(|marker| marker.boundary)
                .collect::<Vec<_>>(),
            [99, 98, 97]
        );
    }

    #[test]
    fn remaining_markers_keep_reset_anchor_before_multiple_missing_values() {
        let first_missing =
            UsageHistorySample::from_model_history(0, 1_000, ModelDollarTotals::default());
        let second_missing =
            UsageHistorySample::from_model_history(60, 1_000, ModelDollarTotals::default());
        let first_observed =
            UsageHistorySample::new(120, 1_000, 97.0, ModelDollarTotals::default());

        let markers =
            remaining_marker_positions(&[&first_missing, &second_missing, &first_observed], 0, 120);

        assert_eq!(
            markers
                .iter()
                .map(|marker| marker.boundary)
                .collect::<Vec<_>>(),
            [99, 98, 97]
        );
        assert_eq!(markers.last().map(|marker| marker.x), Some(100.0));
    }

    #[test]
    fn remaining_markers_are_empty_without_data() {
        assert!(remaining_marker_positions(&[], 0, 120).is_empty());
    }

    #[test]
    fn graph_paths_hide_model_labels_without_spend_data() {
        let empty = graph_paths(&[], 100, 300);
        assert_eq!(empty.current_remaining_label, "—");
        assert!(empty.current_sol_label.is_empty());
        assert!(empty.current_terra_label.is_empty());
        assert!(empty.current_luna_label.is_empty());

        let zero = UsageHistorySample::new(120, 0, 70.0, ModelDollarTotals::default());
        let all_zero = graph_paths(&[&zero], 0, 300);
        assert_eq!(all_zero.current_remaining_label, "70%");
        assert!(all_zero.current_sol_label.is_empty());
        assert!(all_zero.current_terra_label.is_empty());
        assert!(all_zero.current_luna_label.is_empty());
    }

    #[test]
    fn current_label_connector_path_links_series_endpoint_to_displaced_label() {
        assert_eq!(
            current_label_connector_path(0.80, 0.68, true),
            "M0.00 80.00 L100.00 68.00"
        );
        assert_eq!(current_label_connector_path(0.80, 0.68, false), "");
        assert_eq!(current_label_connector_path(f32::NAN, 0.68, true), "");
    }

    #[test]
    fn focused_model_graph_rebases_the_selected_area_to_zero() {
        let samples = [
            UsageHistorySample::new(
                100,
                1_000,
                90.0,
                ModelDollarTotals {
                    sol: 4.0,
                    terra: 2.0,
                    luna: 1.0,
                },
            ),
            UsageHistorySample::new(
                160,
                1_000,
                85.0,
                ModelDollarTotals {
                    sol: 8.0,
                    terra: 3.0,
                    luna: 2.0,
                },
            ),
        ];
        let references = samples.iter().collect::<Vec<_>>();
        let paths = super::graph_paths_for_model(&references, 0, 200, "TERRA");
        assert!(paths.sol.is_empty());
        assert!(paths.luna.is_empty());
        assert!(paths.terra.starts_with("M0.00"));
        assert_eq!(paths.current_terra_label, "$3.00");
        assert!(paths.current_sol_label.is_empty());
    }

    #[test]
    fn token_graph_uses_token_axis_and_current_labels_without_changing_dollars() {
        let samples = [
            UsageHistorySample::new_with_usage(
                100,
                1_000,
                90.0,
                ModelDollarTotals {
                    sol: 1.0,
                    terra: 2.0,
                    luna: 3.0,
                },
                ModelTokenTotals {
                    sol: 1_000,
                    terra: 2_000,
                    luna: 3_000,
                },
            ),
            UsageHistorySample::new_with_usage(
                160,
                1_000,
                85.0,
                ModelDollarTotals {
                    sol: 2.0,
                    terra: 4.0,
                    luna: 5.0,
                },
                ModelTokenTotals {
                    sol: 2_000,
                    terra: 4_000,
                    luna: 8_000,
                },
            ),
        ];
        let references = samples.iter().collect::<Vec<_>>();
        let dollars = graph_paths_for_selection(&references, 0, 200, true, true, true, false);
        let tokens = graph_paths_for_selection(&references, 0, 200, true, true, true, true);
        assert_eq!(dollars.dollar_labels[0], "$5.00");
        assert_eq!(tokens.dollar_labels[0], "8.0K");
        assert_eq!(tokens.current_luna_label, "8,000");
        assert_eq!(dollars.current_luna_label, "$5.00");
        assert!(!tokens.sol.contains('Z'));
        assert!(!tokens.luna.contains('Z'));
        assert!(!dollars.sol.contains('Z'));
        let sol_only = graph_paths_for_selection(&references, 0, 200, false, false, true, true);
        assert_eq!(sol_only.dollar_labels[0], "8.0K");
        assert_eq!(sol_only.sol, tokens.sol);
    }

    #[test]
    fn token_graph_carries_cumulative_values_across_legacy_zero_rows() {
        let samples = [
            UsageHistorySample::new_with_usage(
                100,
                1_000,
                95.0,
                ModelDollarTotals {
                    sol: 1.0,
                    terra: 2.0,
                    luna: 3.0,
                },
                ModelTokenTotals {
                    sol: 1_000,
                    terra: 2_000,
                    luna: 3_000,
                },
            ),
            // This is the shape of a legacy row: dollars are present, but
            // token columns were not available yet. It must not reset the
            // cumulative counters or turn the next observation into a delta.
            UsageHistorySample::new(
                160,
                1_000,
                94.0,
                ModelDollarTotals {
                    sol: 1.5,
                    terra: 2.5,
                    luna: 3.5,
                },
            ),
            UsageHistorySample::new_with_usage(
                220,
                1_000,
                93.0,
                ModelDollarTotals {
                    sol: 2.0,
                    terra: 3.0,
                    luna: 4.0,
                },
                ModelTokenTotals {
                    sol: 2_000,
                    terra: 4_000,
                    luna: 8_000,
                },
            ),
            // A later legacy row must not erase the latest known endpoint.
            UsageHistorySample::new(
                280,
                1_000,
                92.0,
                ModelDollarTotals {
                    sol: 2.5,
                    terra: 3.5,
                    luna: 4.5,
                },
            ),
        ];
        let references = samples.iter().collect::<Vec<_>>();
        let points = minute_model_spend_for_metric(&references, true);
        assert_eq!(points.len(), 4);
        assert_eq!(
            (points[0].sol, points[0].terra, points[0].luna),
            (1_000.0, 2_000.0, 3_000.0)
        );
        assert_eq!(
            (points[1].sol, points[1].terra, points[1].luna),
            (1_000.0, 2_000.0, 3_000.0)
        );
        assert_eq!(
            (points[2].sol, points[2].terra, points[2].luna),
            (2_000.0, 4_000.0, 8_000.0)
        );
        assert_eq!(
            (points[3].sol, points[3].terra, points[3].luna),
            (2_000.0, 4_000.0, 8_000.0)
        );

        let graph = graph_paths_for_selection(&references, 0, 300, true, true, true, true);
        assert_eq!(graph.current_luna_label, "8,000");
        assert_eq!(graph.current_terra_label, "4,000");
        assert_eq!(graph.current_sol_label, "2,000");
    }

    #[test]
    fn metric_selector_and_fixed_token_scale_contract() {
        let mut state = CodexInfoState::preview("normal");
        assert_eq!(state.selected_metric, "ドル");
        state.select_metric("トークン");
        assert_eq!(state.selected_metric, "トークン");
        state.select_metric("不正値");
        assert_eq!(state.selected_metric, "トークン");

        assert_eq!(GRAPH_METRIC_OPTIONS, ["ドル", "トークン"]);

        let sample = UsageHistorySample::new_with_usage(
            60,
            300,
            73.0,
            ModelDollarTotals {
                sol: 1.0,
                terra: 10.0,
                luna: 5.0,
            },
            ModelTokenTotals {
                sol: 100,
                terra: 1_000,
                luna: 500,
            },
        );
        let references = [&sample];
        let dollars_all = graph_paths_for_selection(&references, 0, 120, true, true, true, false);
        let dollars_sol = graph_paths_for_selection(&references, 0, 120, false, false, true, false);
        assert_eq!(dollars_all.dollar_labels[0], "$10.00");
        assert_eq!(dollars_sol.dollar_labels[0], "$1.00");

        let tokens_all = graph_paths_for_selection(&references, 0, 120, true, true, true, true);
        let tokens_sol = graph_paths_for_selection(&references, 0, 120, false, false, true, true);
        assert_eq!(tokens_all.dollar_labels, tokens_sol.dollar_labels);
        assert_eq!(tokens_all.dollar_labels[0], "1.0K");
        assert_eq!(tokens_all.sol_flat, tokens_sol.sol_flat);
        assert_eq!(tokens_all.sol_rising, tokens_sol.sol_rising);
        assert_eq!(tokens_all.remaining, dollars_all.remaining);
        assert_eq!(tokens_all.current_remaining_label, "73%");

        let zero = UsageHistorySample::new_with_usage(
            60,
            300,
            100.0,
            ModelDollarTotals::default(),
            ModelTokenTotals::default(),
        );
        let zero_paths = graph_paths_for_selection(&[&zero], 0, 120, true, true, true, true);
        assert_eq!(zero_paths.dollar_labels[0], "1");

        let source = include_str!("../ui/components.slint");
        assert!(source.contains("x: parent.width - 244px;"));
        assert!(source.contains("model: root.metric-options;"));
        assert!(source.contains("selected(value) => { root.select-metric(value); }"));
    }

    #[test]
    fn graph_selection_recalculates_axis_for_independent_enabled_models() {
        let samples = [
            UsageHistorySample::new(
                100,
                1_000,
                90.0,
                ModelDollarTotals {
                    sol: 4.0,
                    terra: 2.0,
                    luna: 1.0,
                },
            ),
            UsageHistorySample::new(
                160,
                1_000,
                85.0,
                ModelDollarTotals {
                    sol: 8.0,
                    terra: 3.0,
                    luna: 2.0,
                },
            ),
        ];
        let references = samples.iter().collect::<Vec<_>>();
        let all = graph_paths_for_selection(&references, 0, 200, true, true, true, false);
        let terra_only = graph_paths_for_selection(&references, 0, 200, false, true, false, false);
        assert_eq!(all.dollar_labels[0], "$8.00");
        assert_eq!(terra_only.dollar_labels[0], "$3.00");
        assert!(terra_only.terra.starts_with("M0.00"));
        assert!(!all.sol.contains('Z'));
        assert!(!all.terra.contains('Z'));
        assert!(!all.luna.contains('Z'));
        assert!(terra_only.luna.is_empty());
        assert!(terra_only.sol.is_empty());
    }

    #[test]
    fn dollar_paths_are_independent_and_keep_sol_shape_when_other_lines_toggle() {
        let samples = [
            UsageHistorySample::new(
                0,
                1_000,
                100.0,
                ModelDollarTotals {
                    sol: 1.0,
                    terra: 0.0,
                    luna: 0.0,
                },
            ),
            UsageHistorySample::new(
                60,
                1_000,
                99.0,
                ModelDollarTotals {
                    sol: 2.0,
                    terra: 1.0,
                    luna: 0.5,
                },
            ),
            UsageHistorySample::new(
                120,
                1_000,
                98.0,
                ModelDollarTotals {
                    sol: 2.0,
                    terra: 1.0,
                    luna: 0.5,
                },
            ),
            UsageHistorySample::new(
                180,
                1_000,
                97.0,
                ModelDollarTotals {
                    sol: 4.0,
                    terra: 1.0,
                    luna: 0.5,
                },
            ),
        ];
        let references = samples.iter().collect::<Vec<_>>();
        let all = graph_paths_for_selection(&references, 0, 240, true, true, true, false);
        let sol_only = graph_paths_for_selection(&references, 0, 240, false, false, true, false);

        assert_eq!(all.dollar_labels[0], "$4.00");
        assert_eq!(all.current_sol_label, "$4.00");
        assert_eq!(all.sol, sol_only.sol);
        assert!(!all.sol.contains('Z'));
        assert!(!all.terra.contains('Z'));
        assert!(!all.luna.contains('Z'));

        let spend = smooth_model_spend(&graph_time_endpoints(
            minute_model_spend(&references),
            0,
            240,
        ));
        assert!(spend.windows(2).all(|pair| {
            pair[0].sol <= pair[1].sol
                && pair[0].terra <= pair[1].terra
                && pair[0].luna <= pair[1].luna
        }));
    }

    #[test]
    fn independent_lines_hold_zero_until_the_first_real_measurement() {
        let sample = UsageHistorySample::new(
            180,
            1_000,
            90.0,
            ModelDollarTotals {
                sol: 4.0,
                terra: 0.0,
                luna: 0.0,
            },
        );
        let paths = graph_paths(&[&sample], 0, 240);
        // x=75 is the first recorded point; x=0..75 must remain at the
        // baseline rather than becoming a fabricated diagonal spend trend.
        assert!(paths.sol.contains("L75.00 99.00 L75.00"));
    }

    #[test]
    fn segment_splitter_never_connects_an_invalid_decrease() {
        let points = [
            HourlyModelSpend {
                timestamp: 0,
                sol: 1.0,
                ..HourlyModelSpend::default()
            },
            HourlyModelSpend {
                timestamp: 60,
                sol: 1.0,
                ..HourlyModelSpend::default()
            },
            HourlyModelSpend {
                timestamp: 120,
                sol: 3.0,
                ..HourlyModelSpend::default()
            },
            HourlyModelSpend {
                timestamp: 180,
                sol: 2.0,
                ..HourlyModelSpend::default()
            },
            HourlyModelSpend {
                timestamp: 240,
                sol: 2.0,
                ..HourlyModelSpend::default()
            },
        ];
        let (flat, rising) = split_metric_line_paths(&points, 0, 240, 3.0, |point| point.sol);

        assert!(flat.contains("M0.00 66.33 L25.00 66.33"));
        assert!(flat.contains("M75.00 33.67 L100.00 33.67"));
        assert!(rising.contains("M25.00 66.33 L50.00 1.00"));
        // The 3 -> 2 decrease at x=50..75 is a disconnected boundary.
        assert!(!flat.contains("M50.00"));
        assert!(!rising.contains("M50.00"));
    }

    #[test]
    fn graph_selection_uses_one_monotonic_series_for_lines_and_current_values() {
        let reset_at = 1_000;
        let sample = |timestamp, dollars, tokens| {
            UsageHistorySample::new_with_usage(
                timestamp,
                reset_at,
                80.0,
                ModelDollarTotals {
                    sol: dollars,
                    ..ModelDollarTotals::default()
                },
                ModelTokenTotals {
                    sol: tokens,
                    ..ModelTokenTotals::default()
                },
            )
        };
        let samples = [
            sample(60, 10.0, 100),
            sample(120, 0.0, 0),
            sample(180, 12.0, 120),
        ];
        let references = samples.iter().collect::<Vec<_>>();

        let dollars = graph_paths_for_selection(&references, 0, 240, false, false, true, false);
        assert_eq!(dollars.current_sol_label, "$12.00");
        assert!(!dollars.sol_rising.contains("M50.00 99.00"));
        assert!(dollars.sol_flat.contains("M25.00 17.33 L50.00 17.33"));

        let tokens = graph_paths_for_selection(&references, 0, 240, false, false, true, true);
        assert_eq!(tokens.current_sol_label, "120");
        assert!(!tokens.sol_rising.contains("M50.00 99.00"));
        assert!(tokens.sol_flat.contains("M25.00 17.33 L50.00 17.33"));
    }

    #[test]
    fn first_observation_does_not_fabricate_a_diagonal_rise() {
        let points = graph_time_endpoints(
            vec![HourlyModelSpend {
                timestamp: 180,
                sol: 4.0,
                ..HourlyModelSpend::default()
            }],
            0,
            240,
        );
        let (flat, rising) = split_metric_line_paths(&points, 0, 240, 4.0, |point| point.sol);

        assert!(flat.contains("M0.00 99.00 L75.00 99.00"));
        assert!(flat.contains("M75.00 1.00 L100.00 1.00"));
        assert!(rising.contains("M75.00 99.00 L75.00 1.00"));
        assert!(!rising.contains("M0.00 99.00 L75.00 1.00"));
    }

    #[test]
    fn unused_intervals_mark_idle_segments_and_preserve_first_use_boundary() {
        let points = [
            HourlyModelSpend {
                timestamp: 0,
                ..HourlyModelSpend::default()
            },
            HourlyModelSpend {
                timestamp: 60,
                ..HourlyModelSpend::default()
            },
            HourlyModelSpend {
                timestamp: 120,
                sol: 1.0,
                ..HourlyModelSpend::default()
            },
            HourlyModelSpend {
                timestamp: 180,
                sol: 1.0,
                ..HourlyModelSpend::default()
            },
            HourlyModelSpend {
                timestamp: 240,
                sol: 2.0,
                ..HourlyModelSpend::default()
            },
        ];
        assert_eq!(
            unused_interval_positions(&points, 0, 240),
            vec![
                UnusedIntervalPosition {
                    start: 0.0,
                    width: 25.0,
                    preserve_boundary: false,
                },
                UnusedIntervalPosition {
                    start: 50.0,
                    width: 25.0,
                    preserve_boundary: false,
                },
            ]
        );

        let first_use = [
            HourlyModelSpend {
                timestamp: 0,
                ..HourlyModelSpend::default()
            },
            HourlyModelSpend {
                timestamp: 180,
                sol: 4.0,
                ..HourlyModelSpend::default()
            },
            HourlyModelSpend {
                timestamp: 240,
                sol: 4.0,
                ..HourlyModelSpend::default()
            },
        ];
        assert_eq!(
            unused_interval_positions(&first_use, 0, 240),
            vec![
                UnusedIntervalPosition {
                    start: 0.0,
                    width: 75.0,
                    preserve_boundary: true,
                },
                UnusedIntervalPosition {
                    start: 75.0,
                    width: 25.0,
                    preserve_boundary: false,
                },
            ]
        );
    }

    #[test]
    fn unused_intervals_merge_adjacent_flat_segments() {
        let points = [
            HourlyModelSpend {
                timestamp: 0,
                sol: 2.0,
                terra: 1.0,
                luna: 3.0,
            },
            HourlyModelSpend {
                timestamp: 60,
                sol: 2.0,
                terra: 1.0,
                luna: 3.0,
            },
            HourlyModelSpend {
                timestamp: 120,
                sol: 2.0,
                terra: 1.0,
                luna: 3.0,
            },
            HourlyModelSpend {
                timestamp: 180,
                sol: 2.0,
                terra: 1.0,
                luna: 3.0,
            },
        ];
        assert_eq!(
            unused_interval_positions(&points, 0, 180),
            vec![UnusedIntervalPosition {
                start: 0.0,
                width: 100.0,
                preserve_boundary: false,
            }]
        );
    }

    #[test]
    fn unused_intervals_use_the_selected_dollar_or_token_metric() {
        let samples = [
            UsageHistorySample::new_with_usage(
                0,
                1_000,
                100.0,
                ModelDollarTotals::default(),
                ModelTokenTotals::default(),
            ),
            UsageHistorySample::new_with_usage(
                60,
                1_000,
                99.0,
                ModelDollarTotals {
                    sol: 1.0,
                    ..ModelDollarTotals::default()
                },
                ModelTokenTotals {
                    sol: 100,
                    ..ModelTokenTotals::default()
                },
            ),
            UsageHistorySample::new_with_usage(
                120,
                1_000,
                98.0,
                ModelDollarTotals {
                    sol: 1.0,
                    ..ModelDollarTotals::default()
                },
                ModelTokenTotals {
                    sol: 200,
                    ..ModelTokenTotals::default()
                },
            ),
            UsageHistorySample::new_with_usage(
                180,
                1_000,
                97.0,
                ModelDollarTotals {
                    sol: 1.0,
                    ..ModelDollarTotals::default()
                },
                ModelTokenTotals {
                    sol: 200,
                    ..ModelTokenTotals::default()
                },
            ),
        ];
        let references = samples.iter().collect::<Vec<_>>();
        let dollars = graph_paths_for_selection(&references, 0, 180, true, true, true, false);
        let tokens = graph_paths_for_selection(&references, 0, 180, true, true, true, true);

        assert_eq!(dollars.unused_intervals.len(), 1);
        assert!((dollars.unused_intervals[0].start - 33.3333333333).abs() < 0.000_001);
        assert!((dollars.unused_intervals[0].width - 66.6666666667).abs() < 0.000_001);
        assert_eq!(tokens.unused_intervals.len(), 1);
        assert!((tokens.unused_intervals[0].start - 66.6666666667).abs() < 0.000_001);
        assert!((tokens.unused_intervals[0].width - 33.3333333333).abs() < 0.000_001);
    }

    #[test]
    fn dollar_idle_bands_use_raw_cumulative_values_before_line_smoothing() {
        let samples = [
            UsageHistorySample::new(0, 1_000, 100.0, ModelDollarTotals::default()),
            UsageHistorySample::new(60, 1_000, 99.0, ModelDollarTotals::default()),
            UsageHistorySample::new(
                120,
                1_000,
                98.0,
                ModelDollarTotals {
                    sol: 1.0,
                    ..ModelDollarTotals::default()
                },
            ),
            UsageHistorySample::new(
                180,
                1_000,
                97.0,
                ModelDollarTotals {
                    sol: 1.0,
                    ..ModelDollarTotals::default()
                },
            ),
        ];
        let references = samples.iter().collect::<Vec<_>>();
        let paths = graph_paths(&references, 0, 180);

        assert_eq!(paths.unused_intervals.len(), 2);
        assert!((paths.unused_intervals[0].start - 0.0).abs() < 0.000_001);
        assert!((paths.unused_intervals[0].width - 33.3333333333).abs() < 0.000_001);
        assert!((paths.unused_intervals[1].start - 66.6666666667).abs() < 0.000_001);
        assert!((paths.unused_intervals[1].width - 33.3333333333).abs() < 0.000_001);
    }

    #[test]
    fn current_graph_labels_are_clamped_and_separated_at_minimum_size() {
        let mut paths = GraphPaths {
            current_remaining_label: "50%".into(),
            current_luna_label: "$1.00".into(),
            current_terra_label: "$1.00".into(),
            current_sol_label: "$1.00".into(),
            current_remaining_y: 0.5,
            current_luna_y: 0.5,
            current_terra_y: 0.5,
            current_sol_y: 0.5,
            ..GraphPaths::default()
        };
        separate_current_label_positions(&mut paths, true, true, true, true);
        let mut positions = [
            paths.current_remaining_y,
            paths.current_luna_y,
            paths.current_terra_y,
            paths.current_sol_y,
        ];
        positions.sort_by(f32::total_cmp);
        let minimum = 8.0 / 204.0;
        let maximum = 1.0 - minimum;
        assert!(positions[0] >= minimum);
        assert!(positions[3] <= maximum);
        for pair in positions.windows(2) {
            assert!((pair[1] - pair[0]) * 204.0 >= 15.999);
        }
    }

    #[test]
    fn remaining_graph_keeps_the_reset_start_anchor_without_observations() {
        let paths = graph_paths(&[], 100, 300);
        assert_eq!(paths.remaining, "M0.00 1.00");
    }

    #[test]
    fn smoothing_keeps_remaining_and_spend_cumulative() {
        let remaining =
            smooth_remaining_points(&[(0, 100.0), (60, 60.0), (120, 70.0), (180, 20.0)]);
        assert!(remaining.windows(2).all(|pair| pair[0].1 >= pair[1].1));

        let spend = smooth_model_spend(&[
            HourlyModelSpend {
                timestamp: 0,
                sol: 0.0,
                terra: 0.0,
                luna: 0.0,
            },
            HourlyModelSpend {
                timestamp: 60,
                sol: 1.0,
                terra: 2.0,
                luna: 3.0,
            },
            HourlyModelSpend {
                timestamp: 120,
                sol: 4.0,
                terra: 5.0,
                luna: 6.0,
            },
        ]);
        assert!(spend.windows(2).all(|pair| {
            pair[0].sol <= pair[1].sol
                && pair[0].terra <= pair[1].terra
                && pair[0].luna <= pair[1].luna
        }));
    }

    #[test]
    fn zero_cost_period_draws_a_visible_baseline() {
        let points = [
            HourlyModelSpend {
                timestamp: 0,
                ..HourlyModelSpend::default()
            },
            HourlyModelSpend {
                timestamp: 60,
                ..HourlyModelSpend::default()
            },
        ];
        assert_eq!(
            stacked_area_path(&points, 0, 60, 0.0, |_| (0.0, 0.0)),
            "M0.00 99.00 L100.00 99.00"
        );
    }

    #[test]
    fn period_label_has_month_and_day_for_both_endpoints() {
        let label = format_period_label(1_700_000_000, 1_700_086_400);
        assert_eq!(label, "2023/11/15 07:13:20 JST ～ 2023/11/16 07:13:20 JST");
        assert_eq!(label.matches('/').count(), 4);
        assert_eq!(label.matches(" JST").count(), 2);
        assert_eq!(label.matches(" ～ ").count(), 1);
        let endpoints: Vec<_> = label.split(" ～ ").collect();
        assert_eq!(endpoints.len(), 2);
        assert!(endpoints
            .iter()
            .all(|endpoint| endpoint.contains('/') && endpoint.contains(':')));
    }

    #[test]
    fn graph_period_row_only_displays_period_label() {
        let source = include_str!("../ui/components.slint");
        let graph = source
            .split("export component GraphWindow inherits Window {")
            .nth(1)
            .expect("GraphWindow");
        assert!(graph.contains("model: root.history-period-options;"));
        assert!(graph.contains("current-index: root.selected-history-index;"));
        assert!(!graph.contains("古い期間"));
        assert!(!graph.contains("新しい期間"));
    }

    #[test]
    fn graph_history_placeholder_is_not_selectable() {
        let source = include_str!("../ui/components.slint");
        let graph_select = source
            .split_once("export component GraphSelect inherits Rectangle {")
            .and_then(|(_, source)| source.split_once("export component Header"))
            .map(|(source, _)| source)
            .expect("GraphSelect component");
        assert!(graph_select.contains(
            "enabled: root.model.length > 0 && !(root.model.length == 1 && root.model[0] == \"履歴なし\");"
        ));
    }

    #[test]
    fn graph_history_popup_overlays_plot_without_reflowing_it() {
        let source = include_str!("../ui/components.slint");
        let graph = source
            .split("export component GraphWindow inherits Window {")
            .nth(1)
            .expect("GraphWindow");
        assert!(graph.contains("popup-above: false;"));
        assert!(graph.contains("y: 72px;"));
        assert!(graph.contains("history-toggle-y: 144px;"));
        assert!(!graph.contains("history-toggle-y: history-select.popup-open ?"));
        assert!(graph.contains("z: 2;"));
        assert!(graph.contains("y: root.history-toggle-y + 32px;"));
        assert!(source.contains("y: root.popup-above ? 0px : root.height;"));
        assert!(source.contains(
            "out property <length> popup-height: min(130px, max(root.item-height + 2px, root.model.length * root.item-height + 2px));"
        ));
        assert!(source.contains("popup-list := ListView"));
    }

    #[test]
    fn graph_metric_popup_uses_the_reserved_left_band() {
        let source = include_str!("../ui/components.slint");
        let graph = source
            .split("export component GraphWindow inherits Window {")
            .nth(1)
            .expect("GraphWindow");
        let metric = graph
            .split("model: root.metric-options;")
            .nth(1)
            .expect("metric selector");
        assert!(metric.contains("popup-above: true;"));
        assert!(metric.contains("popup-x: -128px;"));
    }

    #[test]
    fn graph_controls_use_one_visual_boundary_and_show_short_histories() {
        let source = include_str!("../ui/components.slint");
        let graph = source
            .split("export component GraphWindow inherits Window {")
            .nth(1)
            .expect("GraphWindow");
        assert!(source.contains(
            "out property <length> popup-height: min(130px, max(root.item-height + 2px, root.model.length * root.item-height + 2px));"
        ));
        assert!(graph.contains("background: DesignTokens.graph-control-surface;"));
        assert!(graph.contains("opacity: 0.72;"));
        assert!(graph.contains("in-out property <[GraphUnusedInterval]> unused-intervals;"));
        assert!(graph.contains("for interval in root.unused-intervals: Rectangle"));
        assert!(graph.contains("background: DesignTokens.text-muted;"));
        let toggle = source
            .split("component GraphToggle inherits Rectangle {")
            .nth(1)
            .and_then(|body| body.split("export struct RemainingMarker").next())
            .expect("GraphToggle");
        assert!(toggle.contains("background: transparent;"));
        assert!(!toggle.contains("border-width: 1px;"));
        assert!(toggle.contains("text: root.label;"));
        assert!(!toggle.contains("strings.on"));
        assert!(!toggle.contains("strings.off"));
    }

    #[test]
    fn graph_idle_model_paths_use_quiet_strokes() {
        let source = include_str!("../ui/components.slint");
        let graph = source
            .split("export component GraphWindow inherits Window {")
            .nth(1)
            .expect("GraphWindow");
        for path_name in ["luna-flat-path", "terra-flat-path", "sol-flat-path"] {
            let path = graph
                .split("Path {")
                .find(|body| body.contains(&format!("commands: root.{path_name};")))
                .expect(path_name);
            assert!(path.contains("stroke-width: 1px;"), "{path_name}");
            assert!(path.contains("opacity: 0.5;"), "{path_name}");
        }
    }

    #[test]
    fn graph_many_preview_exercises_scrollable_period_history() {
        let state = CodexInfoState::preview("graph-many");
        assert!(state.history_periods().len() >= 6);
        assert!(state.history_period_options().len() >= 6);
    }

    #[test]
    fn graph_period_preview_opens_the_history_selector_for_visual_review() {
        let source = include_str!("../ui/components.slint");
        assert!(source.contains("in property <bool> open-on-start: false;"));
        assert!(source.contains("open-on-start: root.open-history-on-start;"));
        assert!(source.contains("interval: 100ms;"));
        let main = include_str!("main.rs");
        assert!(
            main.contains("Some(\"graph\" | \"graph-old\" | \"graph-many\" | \"graph-period\")")
        );
        assert!(main.contains("graph.set_open_history_on_start(graph_period_preview);"));
    }

    #[test]
    fn non_graph_surfaces_do_not_add_outer_frames() {
        let source = include_str!("../ui/components.slint");
        for name in [
            "export component RemainingQuota inherits Rectangle {",
            "export component WeekGauge inherits Rectangle {",
            "export component AccountActivity inherits Rectangle {",
            "export component ModelUsage inherits Rectangle {",
            "export component StatusBanner inherits Rectangle {",
        ] {
            let body = source.split(name).nth(1).expect(name);
            let header = body.lines().take(12).collect::<Vec<_>>().join("\n");
            assert!(
                !header.contains("border-width: 1px;"),
                "unexpected frame: {name}"
            );
        }
    }

    #[test]
    fn graph_window_receives_a_full_width_graph_path() {
        let Ok(graph) = GraphWindow::new() else {
            return;
        };
        let commands = "M0.00 99.00 L100.00 99.00";
        graph.set_sol_flat_path(commands.into());
        graph.set_sol_rising_path(commands.into());
        assert_eq!(graph.get_sol_flat_path().as_str(), commands);
        assert_eq!(graph.get_sol_rising_path().as_str(), commands);
        assert!(graph.get_show_remaining());
        assert!(graph.get_show_luna());
        assert!(graph.get_show_terra());
        assert!(graph.get_show_sol());
        assert!(!graph.get_show_tokens());
        graph.set_show_remaining(false);
        assert!(!graph.get_show_remaining());
        graph.set_show_tokens(true);
        assert!(graph.get_show_tokens());
    }

    #[test]
    fn threads_window_list_explicitly_clips_rows_below_the_fixed_header() {
        let threads = include_str!("../ui/components.slint")
            .split("export component ThreadsWindow inherits Window {")
            .nth(1)
            .expect("ThreadsWindow");
        let thread_list_clip = threads
            .split("thread-list-clip := Rectangle {")
            .nth(1)
            .expect("thread-list clip rectangle");
        assert!(thread_list_clip.contains("y: 76px;"));
        assert!(thread_list_clip.contains("width: 840px;"));
        assert!(thread_list_clip.contains("height: 384px;"));
        assert!(thread_list_clip.contains("clip: true;"));
        assert!(thread_list_clip.contains("thread-list := ListView {"));
    }

    #[test]
    fn threads_window_uses_readable_primary_metadata_layout() {
        let threads = include_str!("../ui/components.slint")
            .split("export component ThreadsWindow inherits Window {")
            .nth(1)
            .expect("ThreadsWindow");
        assert!(threads.contains("property <bool> single-thread: root.thread-rows.length == 1;"));
        assert!(threads
            .contains("property <length> thread-row-height: root.single-thread ? 384px : 128px;"));
        assert!(threads.contains("height: root.thread-row-height;"));
        assert!(threads.contains("font-size: root.single-thread ? 28px : 20px;"));
        assert!(threads.contains("font-size: root.single-thread ? 22px : 18px;"));
        assert!(threads.contains("font-size: 28px;"));
        assert!(threads.contains("font-size: 24px;"));
        assert!(threads.contains("font-size: 18px;"));
        assert!(threads.contains("width: root.single-thread ? 90px : 78px;"));
        assert!(threads.contains("width: 268px;"));
        assert!(threads.contains("text: row.model;"));
        assert!(threads.contains("text: root.strings.running + \" \" + row.thread-age;"));
        assert!(threads.contains("text: root.strings.instruction + \" \" + row.instruction-age;"));
        assert!(threads.contains("text: root.strings.running;"));
        assert!(threads.contains("text: root.strings.instruction;"));
        assert!(threads.contains("text: root.strings.tokens;"));
        assert!(threads.contains("text: row.tokens;"));
        assert!(threads.contains("text: row.context-usage;"));
        assert!(threads.contains("text: root.strings.context-usage;"));
        assert!(threads.contains("text: row.context-usage;"));
        assert!(threads.contains("text: root.strings.context-usage;"));
        assert!(threads.contains("width: parent.width - 486px;"));
        assert!(threads.contains("property <bool> has-parent-title: row.parent-title != \"\";"));
        assert!(threads.contains("visible: !root.single-thread || parent.has-parent-title;"));
        assert!(threads.contains("y: parent.has-parent-title ? 132px : 84px;"));
        assert!(!threads.contains("row.elapsed"));
        assert!(threads.contains("x: root.single-thread ? 400px : parent.width - 560px;"));
        assert!(threads.contains("x: parent.width - 300px;"));
    }

    #[test]
    fn single_thread_preview_uses_the_full_detail_viewport() {
        let source = include_str!("main.rs")
            .split_once("#[cfg(test)]\nmod tests")
            .map(|(source, _)| source)
            .expect("production source");
        assert!(source.contains("Some(\"multi-thread\" | \"single-thread\")"));
        let state = CodexInfoState::preview("single-thread");
        assert_eq!(state.active_threads.len(), 1);
    }

    #[test]
    fn threads_window_header_stays_above_the_scrolling_rows() {
        let threads = include_str!("../ui/components.slint")
            .split("export component ThreadsWindow inherits Window {")
            .nth(1)
            .expect("ThreadsWindow");
        let header = threads
            .split("header-panel := Rectangle {")
            .nth(1)
            .and_then(|source| source.split("thread-list-clip := Rectangle {").next())
            .expect("fixed header panel");
        assert!(header.contains("x: 30px;"));
        assert!(header.contains("y: 20px;"));
        assert!(header.contains("width: 840px;"));
        assert!(header.contains("height: 48px;"));
        assert!(header.contains("background: DesignTokens.canvas;"));
        assert!(header.contains("font-size: 22px;"));
        assert!(header.contains("font-size: 16px;"));
        assert!(header.contains("z: 2;"));
    }
}
