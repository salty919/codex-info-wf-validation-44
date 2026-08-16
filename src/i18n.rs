// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

//! Startup-localized presentation helpers.
//!
//! The application keeps protocol and timestamp values language neutral.  This
//! module is the only owner of user-facing fixed copy, locale selection, and
//! timezone conversion.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use std::env;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Japanese,
    English,
    SimplifiedChinese,
    Korean,
    Spanish,
    French,
    German,
    Portuguese,
    Italian,
    Russian,
}

impl Language {
    pub const ALL: [Self; 10] = [
        Self::Japanese,
        Self::English,
        Self::SimplifiedChinese,
        Self::Korean,
        Self::Spanish,
        Self::French,
        Self::German,
        Self::Portuguese,
        Self::Italian,
        Self::Russian,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::Japanese => "ja",
            Self::English => "en",
            Self::SimplifiedChinese => "zh-Hans",
            Self::Korean => "ko",
            Self::Spanish => "es",
            Self::French => "fr",
            Self::German => "de",
            Self::Portuguese => "pt",
            Self::Italian => "it",
            Self::Russian => "ru",
        }
    }

    fn from_primary(primary: &str) -> Option<Self> {
        Some(match primary {
            "ja" => Self::Japanese,
            "en" => Self::English,
            // The product deliberately uses one simplified Chinese catalog;
            // regional and script tags are presentation hints, not catalogs.
            "zh" => Self::SimplifiedChinese,
            "ko" => Self::Korean,
            "es" => Self::Spanish,
            "fr" => Self::French,
            "de" => Self::German,
            "pt" => Self::Portuguese,
            "it" => Self::Italian,
            "ru" => Self::Russian,
            _ => return None,
        })
    }

    /// Detect a locale from the POSIX precedence chain. Empty values are
    /// skipped; the first non-empty unsupported/C/POSIX value is English and
    /// does not fall through to a lower-priority variable.
    pub fn detect_from_values(
        lc_all: Option<&str>,
        lc_messages: Option<&str>,
        lang: Option<&str>,
    ) -> Self {
        for value in [lc_all, lc_messages, lang] {
            let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
                continue;
            };
            return Self::parse_locale(value).unwrap_or(Self::English);
        }
        Self::English
    }

    pub fn detect() -> Self {
        // A non-Unicode environment value is an invalid first candidate. We
        // intentionally do not let a lower-priority variable change it.
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            match env::var_os(key) {
                None => continue,
                Some(value) => {
                    let Some(value) = value.to_str() else {
                        return Self::English;
                    };
                    if value.trim().is_empty() {
                        continue;
                    }
                    return Self::parse_locale(value).unwrap_or(Self::English);
                }
            }
        }
        Self::English
    }

    fn parse_locale(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        if normalized == "c"
            || normalized == "posix"
            || normalized.starts_with("c.")
            || normalized.starts_with("c@")
        {
            return None;
        }
        let normalized = normalized
            .split_once('.')
            .map_or(normalized.as_str(), |(head, _)| head)
            .split_once('@')
            .map_or(normalized.as_str(), |(head, _)| head)
            .replace('-', "_");
        let primary = normalized.split('_').next().unwrap_or_default();
        Self::from_primary(primary)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodKind {
    Weekly,
    Monthly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextKey {
    FontFamily,
    WindowUnauthenticated,
    PlanUnset,
    PlanFree,
    PlanEnterprise,
    PlanEducation,
    UsageStatus,
    Graph,
    LegalNotices,
    Running,
    ModelThreads,
    Other,
    Details,
    NoRunningThreads,
    LegalCode,
    LegalWarranty,
    LegalLicense,
    LegalFont,
    LegalSchema,
    LegalDependencies,
    LegalDetails,
    LegalDistribution,
    Close,
    ActiveThreads,
    Context,
    Instruction,
    Tokens,
    Model,
    Input,
    Cached,
    Output,
    Retry,
    UsageTrend,
    Remaining,
    GraphTokenDescription,
    GraphDollarDescription,
    NoRecords,
    ConnectAccount,
    AuthBrowserInstructions,
    AuthManaged,
    OpenAuthPage,
    StartAuth,
    Checking,
    CheckAuth,
    AuthCli,
    NoHistory,
    On,
    Off,
    Connecting,
    UpdatingUsage,
    CheckingAuthStatus,
    AuthenticatedLoading,
    UnauthenticatedStart,
    AuthUrlIssued,
    IssuingAuthUrl,
    AuthUrlOpenFailed,
    CannotFetchUsage,
    CannotDisplayStatus,
    QuotaNearlyGone,
    QuotaLow,
    ResetWithinDay,
    LastUpdated,
    PartialHistoryThreads,
    PartialHistory,
    PartialThreads,
    MainRole,
    SubRole,
    ParentNotRunning,
    ParentPrefix,
    CurrentSuffix,
    DeadlinePrefix,
    EstimatePrefix,
    SoonReset,
    FixedLimitNone,
    QuotaRemaining,
    MonthlyQuotaRemaining,
    UsageLimit,
    DollarMetric,
    TokenMetric,
}

impl TextKey {
    pub const ALL: &'static [Self] = &[
        Self::FontFamily,
        Self::WindowUnauthenticated,
        Self::PlanUnset,
        Self::PlanFree,
        Self::PlanEnterprise,
        Self::PlanEducation,
        Self::UsageStatus,
        Self::Graph,
        Self::LegalNotices,
        Self::Running,
        Self::ModelThreads,
        Self::Other,
        Self::Details,
        Self::NoRunningThreads,
        Self::LegalCode,
        Self::LegalWarranty,
        Self::LegalLicense,
        Self::LegalFont,
        Self::LegalSchema,
        Self::LegalDependencies,
        Self::LegalDetails,
        Self::LegalDistribution,
        Self::Close,
        Self::ActiveThreads,
        Self::Context,
        Self::Instruction,
        Self::Tokens,
        Self::Model,
        Self::Input,
        Self::Cached,
        Self::Output,
        Self::Retry,
        Self::UsageTrend,
        Self::Remaining,
        Self::GraphTokenDescription,
        Self::GraphDollarDescription,
        Self::NoRecords,
        Self::ConnectAccount,
        Self::AuthBrowserInstructions,
        Self::AuthManaged,
        Self::OpenAuthPage,
        Self::StartAuth,
        Self::Checking,
        Self::CheckAuth,
        Self::AuthCli,
        Self::NoHistory,
        Self::On,
        Self::Off,
        Self::Connecting,
        Self::UpdatingUsage,
        Self::CheckingAuthStatus,
        Self::AuthenticatedLoading,
        Self::UnauthenticatedStart,
        Self::AuthUrlIssued,
        Self::IssuingAuthUrl,
        Self::AuthUrlOpenFailed,
        Self::CannotFetchUsage,
        Self::CannotDisplayStatus,
        Self::QuotaNearlyGone,
        Self::QuotaLow,
        Self::ResetWithinDay,
        Self::LastUpdated,
        Self::PartialHistoryThreads,
        Self::PartialHistory,
        Self::PartialThreads,
        Self::MainRole,
        Self::SubRole,
        Self::ParentNotRunning,
        Self::ParentPrefix,
        Self::CurrentSuffix,
        Self::DeadlinePrefix,
        Self::EstimatePrefix,
        Self::SoonReset,
        Self::FixedLimitNone,
        Self::QuotaRemaining,
        Self::MonthlyQuotaRemaining,
        Self::UsageLimit,
        Self::DollarMetric,
        Self::TokenMetric,
    ];
}

#[derive(Clone, Debug)]
pub struct I18n {
    language: Language,
    timezone: Tz,
}

impl I18n {
    pub fn detect() -> Self {
        Self {
            language: Language::detect(),
            timezone: detect_timezone(),
        }
    }

    pub fn from_parts(language: Language, timezone: Tz) -> Self {
        Self { language, timezone }
    }

    pub const fn language(&self) -> Language {
        self.language
    }

    pub const fn timezone(&self) -> Tz {
        self.timezone
    }

    pub fn text(&self, key: TextKey) -> &'static str {
        use TextKey::*;
        match self.language {
            Language::Japanese => match key {
                FontFamily => "Noto Sans JP",
                WindowUnauthenticated => "アカウント未接続 — プラン未設定",
                PlanUnset => "プラン未設定",
                PlanFree => "無料",
                PlanEnterprise => "エンタープライズ",
                PlanEducation => "教育",
                UsageStatus => "利用状況",
                Graph => "グラフ",
                LegalNotices => "法的通知",
                Running => "稼働",
                ModelThreads => "モデル別スレッド",
                Other => "その他",
                Details => "詳細",
                NoRunningThreads => "実行中のスレッドなし",
                LegalCode => "Codex Info の独自コードと文書: GPL-3.0-only",
                LegalWarranty => {
                    "本ソフトウェアは無保証です。GPL-3.0-only の条件で再配布できます。"
                }
                LegalLicense => "ライセンス本文: LICENSE",
                LegalFont => "Noto Sans JP / Noto Sans CJK KR: OFL-1.1 / Adobe 2014-2021",
                LegalSchema => "Codex生成スキーマ: Apache-2.0 / Copyright 2025 OpenAI",
                LegalDependencies => "Slint と Rust 依存クレートは各上流ライセンスを保持します。",
                LegalDetails => "詳細: THIRD_PARTY_NOTICES.md と LICENSES/",
                LegalDistribution => "バイナリ配布時は各依存の LICENSE/NOTICE を同梱してください。",
                Close => "閉じる",
                ActiveThreads => "実行中のスレッド",
                Context => "コンテキスト使用率",
                Instruction => "指示",
                Tokens => "トークン",
                Model => "モデル",
                Input => "入力",
                Cached => "キャッシュ",
                Output => "出力",
                Retry => "再試行",
                UsageTrend => "利用状況の推移",
                Remaining => "残量",
                GraphTokenDescription => "時間ごとのトークン使用量（モデル別） / 残量%",
                GraphDollarDescription => "時間ごとの累積消費ドル（モデル別） / 残量%",
                NoRecords => "記録なし",
                ConnectAccount => "Codexアカウントを接続",
                AuthBrowserInstructions => {
                    "ブラウザで認証を完了してください。完了後、自動的に確認します。"
                }
                AuthManaged => "認証はCodexが管理します。このアプリは認証情報を保存しません。",
                OpenAuthPage => "認証ページを開く",
                StartAuth => "認証を開始",
                Checking => "確認中…",
                CheckAuth => "認証状態を確認",
                AuthCli => "Codex CLIの認証状態を利用します。",
                NoHistory => "履歴なし",
                On => "オン",
                Off => "オフ",
                Connecting => "Codex app-serverへ接続しています…",
                UpdatingUsage => "利用状況を更新しています…",
                CheckingAuthStatus => "認証状態を確認しています…",
                AuthenticatedLoading => "認証済みです。利用量を取得しています…",
                UnauthenticatedStart => "未認証です。認証を開始してください。",
                AuthUrlIssued => "認証URLを発行しました。「認証ページを開く」を押してください。",
                IssuingAuthUrl => "認証URLを発行しています…",
                AuthUrlOpenFailed => "認証URLを開けませんでした。",
                CannotFetchUsage => {
                    "利用状況を取得できません。Codex app-serverへの接続を確認してください。"
                }
                CannotDisplayStatus => "状態を表示できません。",
                QuotaNearlyGone => "残り利用枠はほぼありません。",
                QuotaLow => "残り利用枠が少なくなっています。",
                ResetWithinDay => "リセット前後24時間です。",
                LastUpdated => "最終更新",
                PartialHistoryThreads => {
                    "利用枠は更新しました。履歴とスレッドは前回値を保持しています。"
                }
                PartialHistory => "利用枠は更新しました。履歴は前回値を保持しています。",
                PartialThreads => "利用枠は更新しました。スレッド表示は前回値を保持しています。",
                MainRole => "メイン",
                SubRole => "サブ",
                ParentNotRunning => "親スレッドは現在非実行",
                ParentPrefix => "親",
                CurrentSuffix => "（現在）",
                DeadlinePrefix => "期限",
                EstimatePrefix => "概算",
                SoonReset => "まもなくリセット",
                FixedLimitNone => "固定上限なし",
                QuotaRemaining => "残り利用枠",
                MonthlyQuotaRemaining => "月間残り利用枠",
                UsageLimit => "利用枠",
                DollarMetric => "ドル",
                TokenMetric => "トークン",
            },
            Language::English => english_text(key),
            Language::SimplifiedChinese => chinese_text(key),
            Language::Korean => korean_text(key),
            Language::Spanish => spanish_text(key),
            Language::French => french_text(key),
            Language::German => german_text(key),
            Language::Portuguese => portuguese_text(key),
            Language::Italian => italian_text(key),
            Language::Russian => russian_text(key),
        }
    }

    pub fn font_family(&self) -> &'static str {
        self.text(TextKey::FontFamily)
    }

    pub fn format_elapsed(&self, now: i64, timestamp: Option<i64>) -> String {
        let Some(timestamp) = timestamp else {
            return "—".into();
        };
        if DateTime::<Utc>::from_timestamp(timestamp, 0).is_none() {
            return "—".into();
        }
        let age = now.saturating_sub(timestamp).max(0);
        let (amount, unit) = if age < 60 {
            (age, Unit::Second)
        } else if age < 3_600 {
            let minutes = age / 60;
            let seconds = age % 60;
            return if seconds == 0 {
                self.unit_text(minutes, Unit::Minute)
            } else {
                format!(
                    "{}{}{}",
                    self.unit_text(minutes, Unit::Minute),
                    self.elapsed_separator(),
                    self.unit_text(seconds, Unit::Second)
                )
            };
        } else if age < 86_400 {
            let hours = age / 3_600;
            let minutes = (age % 3_600) / 60;
            return if minutes == 0 {
                self.unit_text(hours, Unit::Hour)
            } else {
                format!(
                    "{}{}{}",
                    self.unit_text(hours, Unit::Hour),
                    self.elapsed_separator(),
                    self.unit_text(minutes, Unit::Minute)
                )
            };
        } else {
            let days = age / 86_400;
            let hours = (age % 86_400) / 3_600;
            return if hours == 0 {
                self.unit_text(days, Unit::Day)
            } else {
                format!(
                    "{}{}{}",
                    self.unit_text(days, Unit::Day),
                    self.elapsed_separator(),
                    self.unit_text(hours, Unit::Hour)
                )
            };
        };
        self.unit_text(amount, unit)
    }

    pub fn format_period_remaining(&self, seconds: i64, kind: PeriodKind) -> String {
        let seconds = seconds.max(0);
        if seconds < 60 {
            return self.text(TextKey::SoonReset).into();
        }
        let (days, hours, minutes) = (
            seconds / 86_400,
            (seconds / 3_600) % 24,
            (seconds / 60) % 60,
        );
        let mut parts = Vec::new();
        if days > 0 {
            parts.push(self.unit_text(days, Unit::Day));
        }
        if hours > 0 {
            parts.push(self.unit_text(hours, Unit::Hour));
        }
        if minutes > 0 {
            parts.push(self.unit_text(minutes, Unit::Minute));
        }
        let duration = parts.join(self.separator());
        match (self.language, kind) {
            (Language::Japanese, PeriodKind::Weekly) => format!("7日間、あと{duration}"),
            (Language::Japanese, PeriodKind::Monthly) => format!("月間、あと{duration}"),
            (Language::SimplifiedChinese, PeriodKind::Weekly) => format!("7天，剩余{duration}"),
            (Language::SimplifiedChinese, PeriodKind::Monthly) => format!("每月，剩余{duration}"),
            (Language::Korean, PeriodKind::Weekly) => format!("7일 기간, {duration} 남음"),
            (Language::Korean, PeriodKind::Monthly) => format!("월간, {duration} 남음"),
            (Language::English, PeriodKind::Weekly) => {
                format!("7-day period, {duration} remaining")
            }
            (Language::English, PeriodKind::Monthly) => format!("Monthly, {duration} remaining"),
            (Language::Spanish, PeriodKind::Weekly) => {
                format!("Periodo de 7 días: quedan {duration}")
            }
            (Language::Spanish, PeriodKind::Monthly) => format!("Mensual: quedan {duration}"),
            (Language::French, PeriodKind::Weekly) => {
                format!("Période de 7 jours : {duration} restantes")
            }
            (Language::French, PeriodKind::Monthly) => format!("Mensuel : {duration} restantes"),
            (Language::German, PeriodKind::Weekly) => format!("7-Tage-Zeitraum, {duration} übrig"),
            (Language::German, PeriodKind::Monthly) => format!("Monatlich, {duration} übrig"),
            (Language::Portuguese, PeriodKind::Weekly) => {
                format!("Período de 7 dias: restam {duration}")
            }
            (Language::Portuguese, PeriodKind::Monthly) => format!("Mensal: restam {duration}"),
            (Language::Italian, PeriodKind::Weekly) => {
                format!("Periodo di 7 giorni: restano {duration}")
            }
            (Language::Italian, PeriodKind::Monthly) => format!("Mensile: restano {duration}"),
            (Language::Russian, PeriodKind::Weekly) => {
                format!("Период 7 дней: осталось {duration}")
            }
            (Language::Russian, PeriodKind::Monthly) => format!("За месяц осталось {duration}"),
        }
    }

    pub fn format_timestamp(&self, timestamp: i64) -> Option<String> {
        let time = DateTime::<Utc>::from_timestamp(timestamp, 0)?;
        Some(
            time.with_timezone(&self.timezone)
                .format("%Y/%m/%d %H:%M:%S %:z")
                .to_string(),
        )
    }

    pub fn format_graph_time(&self, timestamp: i64) -> Option<String> {
        let time = DateTime::<Utc>::from_timestamp(timestamp, 0)?;
        Some(
            time.with_timezone(&self.timezone)
                .format("%m/%d %H:%M")
                .to_string(),
        )
    }

    pub fn format_clock(&self, timestamp: i64) -> Option<String> {
        let time = DateTime::<Utc>::from_timestamp(timestamp, 0)?;
        Some(
            time.with_timezone(&self.timezone)
                .format("%H:%M")
                .to_string(),
        )
    }

    pub fn format_period(&self, start: i64, end: i64) -> Option<String> {
        Some(format!(
            "{}{}{}",
            self.format_timestamp(start)?,
            self.period_separator(),
            self.format_timestamp(end)?
        ))
    }

    pub fn format_deadline_suffix(&self, timestamp: i64) -> Option<String> {
        let timestamp = self.format_timestamp(timestamp)?;
        Some(match self.language {
            Language::Japanese | Language::SimplifiedChinese | Language::Korean => {
                format!("（{} {}）", self.text(TextKey::DeadlinePrefix), timestamp)
            }
            _ => format!(" ({} {})", self.text(TextKey::DeadlinePrefix), timestamp),
        })
    }

    pub fn format_grouped_unsigned(&self, value: u128) -> String {
        group_digits(value.to_string(), self.group_separator())
    }

    pub fn format_grouped_i64(&self, value: i64) -> String {
        if value < 0 {
            format!(
                "-{}",
                self.format_grouped_unsigned(value.unsigned_abs() as u128)
            )
        } else {
            self.format_grouped_unsigned(value as u128)
        }
    }

    pub fn format_dollar(&self, value: f64) -> String {
        let decimal = if value.is_finite() {
            value.max(0.0)
        } else {
            0.0
        };
        let raw = format!("{decimal:.2}");
        let (whole, fraction) = raw.split_once('.').unwrap_or((raw.as_str(), "00"));
        let number = format!(
            "{}{}{}",
            self.format_grouped_unsigned(whole.parse::<u128>().unwrap_or(0)),
            self.decimal_separator(),
            fraction
        );
        format!("${number}")
    }

    pub fn format_thread_count(&self, count: usize) -> String {
        let n = self.format_grouped_unsigned(count as u128);
        match self.language {
            Language::Japanese => format!("{n}件"),
            Language::SimplifiedChinese => format!("{n}条"),
            Language::Korean => format!("{n}개"),
            Language::English => format!("{n} threads"),
            Language::Spanish => format!("{n} hilos"),
            Language::French => format!("{n} fils"),
            Language::German => format!("{n} Threads"),
            Language::Portuguese => format!("{n} threads"),
            Language::Italian => format!("{n} thread"),
            Language::Russian => format!("{n} потоков"),
        }
    }

    pub fn format_token_value(&self, value: u64) -> String {
        let number = self.format_grouped_unsigned(value as u128);
        if matches!(
            self.language,
            Language::Japanese | Language::SimplifiedChinese | Language::Korean
        ) {
            format!("{number}{}", self.text(TextKey::Tokens))
        } else {
            format!("{number} {}", self.text(TextKey::Tokens))
        }
    }

    /// Format the current token total as a percentage of the model's context
    /// window. The caller may pair this value with the localized token limit
    /// so the percentage has an explicit scale.
    pub fn format_context_usage(&self, used_tokens: u64, context_window: u64) -> String {
        if context_window == 0 {
            return "—".to_owned();
        }
        let tenths = (u128::from(used_tokens)
            .saturating_mul(1_000)
            .saturating_add(u128::from(context_window / 2)))
            / u128::from(context_window);
        let tenths = tenths.min(1_000);
        let whole = tenths / 10;
        let fraction = tenths % 10;
        let whole = self.format_grouped_unsigned(whole);
        if fraction == 0 {
            format!("{whole}%")
        } else {
            format!("{whole}{}{fraction}%", self.decimal_separator())
        }
    }

    pub fn format_role(&self, is_subagent: bool, depth: Option<i32>) -> String {
        let base = self.text(if is_subagent {
            TextKey::SubRole
        } else {
            TextKey::MainRole
        });
        if !is_subagent {
            return base.into();
        }
        depth.map_or_else(|| base.into(), |depth| format!("{base} D{}", depth.max(0)))
    }

    pub fn format_parent_title(&self, title: &str) -> String {
        if title.is_empty() {
            return String::new();
        }
        format!("{}: {title}", self.text(TextKey::ParentPrefix))
    }

    pub fn format_estimate(&self, value: f64) -> String {
        format!(
            "{} {}",
            self.text(TextKey::EstimatePrefix),
            self.format_dollar(value)
        )
    }

    pub fn format_last_updated(&self, timestamp: Option<i64>) -> String {
        let time = timestamp
            .and_then(|ts| self.format_clock(ts))
            .unwrap_or_else(|| "—".into());
        format!("{} {}", self.text(TextKey::LastUpdated), time)
    }

    pub fn format_stale_status(&self, timestamp: Option<i64>) -> String {
        let time = timestamp
            .and_then(|ts| self.format_clock(ts))
            .unwrap_or_else(|| "—".into());
        match self.language {
            Language::Japanese => format!("最新情報を取得できません。表示は{time}時点の値です。"),
            Language::English => format!("Unable to fetch the latest data. Showing values from {time}."),
            Language::SimplifiedChinese => format!("无法获取最新信息。显示{time}时的数据。"),
            Language::Korean => format!("최신 정보를 가져올 수 없습니다. {time} 기준 값을 표시합니다."),
            Language::Spanish => format!("No se pudo obtener la información más reciente. Se muestran los valores de {time}."),
            Language::French => format!("Impossible d’obtenir les dernières données. Valeurs de {time}."),
            Language::German => format!("Die neuesten Daten konnten nicht abgerufen werden. Werte von {time}."),
            Language::Portuguese => format!("Não foi possível obter os dados mais recentes. Valores de {time}."),
            Language::Italian => format!("Impossibile ottenere i dati più recenti. Valori delle {time}."),
            Language::Russian => format!("Не удалось получить последние данные. Показаны значения на {time}."),
        }
    }

    fn separator(&self) -> &'static str {
        if self.language == Language::Japanese {
            "と"
        } else {
            ", "
        }
    }
    fn elapsed_separator(&self) -> &'static str {
        if matches!(
            self.language,
            Language::Japanese | Language::SimplifiedChinese | Language::Korean
        ) {
            ""
        } else {
            " "
        }
    }
    fn period_separator(&self) -> &'static str {
        if self.language == Language::Japanese {
            " ～ "
        } else {
            " — "
        }
    }
    fn group_separator(&self) -> char {
        match self.language {
            Language::French | Language::Russian => '\u{202f}',
            Language::German | Language::Spanish | Language::Italian | Language::Portuguese => '.',
            _ => ',',
        }
    }
    fn decimal_separator(&self) -> char {
        match self.language {
            Language::French
            | Language::German
            | Language::Spanish
            | Language::Portuguese
            | Language::Italian
            | Language::Russian => ',',
            _ => '.',
        }
    }
    fn unit_text(&self, value: i64, unit: Unit) -> String {
        let n = self.format_grouped_i64(value);
        match (self.language, unit) {
            (Language::Japanese, Unit::Second) => format!("{n}秒"),
            (Language::Japanese, Unit::Minute) => format!("{n}分"),
            (Language::Japanese, Unit::Hour) => format!("{n}時間"),
            (Language::Japanese, Unit::Day) => format!("{n}日"),
            (Language::SimplifiedChinese, Unit::Second) => format!("{n}秒"),
            (Language::SimplifiedChinese, Unit::Minute) => format!("{n}分钟"),
            (Language::SimplifiedChinese, Unit::Hour) => format!("{n}小时"),
            (Language::SimplifiedChinese, Unit::Day) => format!("{n}天"),
            (Language::Korean, Unit::Second) => format!("{n}초"),
            (Language::Korean, Unit::Minute) => format!("{n}분"),
            (Language::Korean, Unit::Hour) => format!("{n}시간"),
            (Language::Korean, Unit::Day) => format!("{n}일"),
            (Language::English, unit) => format!("{n} {}", english_unit(value, unit)),
            (Language::Spanish, unit) => format!("{n} {}", spanish_unit(value, unit)),
            (Language::French, unit) => format!("{n} {}", french_unit(value, unit)),
            (Language::German, unit) => format!("{n} {}", german_unit(value, unit)),
            (Language::Portuguese, unit) => format!("{n} {}", portuguese_unit(value, unit)),
            (Language::Italian, unit) => format!("{n} {}", italian_unit(value, unit)),
            (Language::Russian, unit) => format!("{n} {}", russian_unit(value, unit)),
        }
    }
}

#[derive(Clone, Copy)]
enum Unit {
    Second,
    Minute,
    Hour,
    Day,
}

fn detect_timezone() -> Tz {
    let configured = env::var("TZ").ok();
    let os_timezone = iana_time_zone::get_timezone().ok();
    timezone_from_names(configured.as_deref(), os_timezone.as_deref())
}

fn timezone_from_names(configured: Option<&str>, os_timezone: Option<&str>) -> Tz {
    let configured = configured.map(str::trim).filter(|value| !value.is_empty());
    let name = configured.or(os_timezone.map(str::trim).filter(|value| !value.is_empty()));
    name.and_then(|name| Tz::from_str(name.strip_prefix(':').unwrap_or(name)).ok())
        .unwrap_or(Tz::UTC)
}

fn group_digits(value: String, separator: char) -> String {
    let mut output = String::with_capacity(value.len() + value.len() / 3);
    for (index, ch) in value.chars().enumerate() {
        if index > 0 && (value.len() - index).is_multiple_of(3) {
            output.push(separator);
        }
        output.push(ch);
    }
    output
}

fn english_unit(value: i64, unit: Unit) -> &'static str {
    match (unit, value == 1) {
        (Unit::Second, true) => "second",
        (Unit::Second, false) => "seconds",
        (Unit::Minute, true) => "minute",
        (Unit::Minute, false) => "minutes",
        (Unit::Hour, true) => "hour",
        (Unit::Hour, false) => "hours",
        (Unit::Day, true) => "day",
        (Unit::Day, false) => "days",
    }
}
fn spanish_unit(value: i64, unit: Unit) -> &'static str {
    match (unit, value == 1) {
        (Unit::Second, true) => "segundo",
        (Unit::Second, false) => "segundos",
        (Unit::Minute, true) => "minuto",
        (Unit::Minute, false) => "minutos",
        (Unit::Hour, true) => "hora",
        (Unit::Hour, false) => "horas",
        (Unit::Day, true) => "día",
        (Unit::Day, false) => "días",
    }
}
fn french_unit(value: i64, unit: Unit) -> &'static str {
    match (unit, value == 1) {
        (Unit::Second, true) => "seconde",
        (Unit::Second, false) => "secondes",
        (Unit::Minute, true) => "minute",
        (Unit::Minute, false) => "minutes",
        (Unit::Hour, true) => "heure",
        (Unit::Hour, false) => "heures",
        (Unit::Day, true) => "jour",
        (Unit::Day, false) => "jours",
    }
}
fn german_unit(value: i64, unit: Unit) -> &'static str {
    match (unit, value == 1) {
        (Unit::Second, true) => "Sekunde",
        (Unit::Second, false) => "Sekunden",
        (Unit::Minute, true) => "Minute",
        (Unit::Minute, false) => "Minuten",
        (Unit::Hour, true) => "Stunde",
        (Unit::Hour, false) => "Stunden",
        (Unit::Day, true) => "Tag",
        (Unit::Day, false) => "Tage",
    }
}
fn portuguese_unit(value: i64, unit: Unit) -> &'static str {
    match (unit, value == 1) {
        (Unit::Second, true) => "segundo",
        (Unit::Second, false) => "segundos",
        (Unit::Minute, true) => "minuto",
        (Unit::Minute, false) => "minutos",
        (Unit::Hour, true) => "hora",
        (Unit::Hour, false) => "horas",
        (Unit::Day, true) => "dia",
        (Unit::Day, false) => "dias",
    }
}
fn italian_unit(value: i64, unit: Unit) -> &'static str {
    match (unit, value == 1) {
        (Unit::Second, true) => "secondo",
        (Unit::Second, false) => "secondi",
        (Unit::Minute, true) => "minuto",
        (Unit::Minute, false) => "minuti",
        (Unit::Hour, true) => "ora",
        (Unit::Hour, false) => "ore",
        (Unit::Day, true) => "giorno",
        (Unit::Day, false) => "giorni",
    }
}
fn russian_unit(value: i64, unit: Unit) -> &'static str {
    match (unit, value % 10 == 1 && value % 100 != 11) {
        (Unit::Second, true) => "секунду",
        (Unit::Second, false) => "секунд",
        (Unit::Minute, true) => "минуту",
        (Unit::Minute, false) => "минут",
        (Unit::Hour, true) => "час",
        (Unit::Hour, false) => "часов",
        (Unit::Day, true) => "день",
        (Unit::Day, false) => "дней",
    }
}

// The non-Japanese catalogs intentionally use one complete match each. This
// keeps missing keys a compile-time review concern instead of a runtime mix.
fn english_text(key: TextKey) -> &'static str {
    basic_text(key, "en")
}
fn chinese_text(key: TextKey) -> &'static str {
    basic_text(key, "zh")
}
fn korean_text(key: TextKey) -> &'static str {
    basic_text(key, "ko")
}
fn spanish_text(key: TextKey) -> &'static str {
    basic_text(key, "es")
}
fn french_text(key: TextKey) -> &'static str {
    basic_text(key, "fr")
}
fn german_text(key: TextKey) -> &'static str {
    basic_text(key, "de")
}
fn portuguese_text(key: TextKey) -> &'static str {
    basic_text(key, "pt")
}
fn italian_text(key: TextKey) -> &'static str {
    basic_text(key, "it")
}
fn russian_text(key: TextKey) -> &'static str {
    basic_text(key, "ru")
}

fn basic_text(key: TextKey, language: &str) -> &'static str {
    use TextKey::*;
    match (language, key) {
        (_, FontFamily) => {
            // The JP subset does not contain every Simplified Chinese code
            // point used by the catalog. The embedded CJK KR face carries the
            // shared Han coverage, so use it for both CJK catalogs while
            // keeping Japanese on the JP face for Japanese glyph forms.
            if language == "ko" || language == "zh" {
                "Noto Sans CJK KR"
            } else {
                "Noto Sans JP"
            }
        }
        ("en", WindowUnauthenticated) => "Account not connected — Plan not set",
        ("en", PlanUnset) => "Plan not set",
        ("en", PlanFree) => "Free",
        ("en", PlanEnterprise) => "Enterprise",
        ("en", PlanEducation) => "Education",
        ("en", UsageStatus) => "Usage",
        ("en", Graph) => "Graph",
        ("en", LegalNotices) => "Legal notices",
        ("en", Running) => "Running",
        ("en", ModelThreads) => "Threads by model",
        ("en", Other) => "Other",
        ("en", Details) => "Details",
        ("en", NoRunningThreads) => "No running threads",
        ("en", LegalCode) => "Codex Info code and documents: GPL-3.0-only",
        ("en", LegalWarranty) => {
            "This software comes without warranty. Redistribution is allowed under GPL-3.0-only."
        }
        ("en", LegalLicense) => "License text: LICENSE",
        ("en", LegalFont) => "Noto Sans JP / Noto Sans CJK KR: OFL-1.1 / Adobe 2014-2021",
        ("en", LegalSchema) => "Codex-generated schema: Apache-2.0 / Copyright 2025 OpenAI",
        ("en", LegalDependencies) => {
            "Slint and Rust dependency crates retain their upstream licenses."
        }
        ("en", LegalDetails) => "Details: THIRD_PARTY_NOTICES.md and LICENSES/",
        ("en", LegalDistribution) => {
            "Include each dependency's LICENSE/NOTICE when distributing binaries."
        }
        ("en", Close) => "Close",
        ("en", ActiveThreads) => "Running threads",
        ("en", Context) => "Context usage",
        ("en", Instruction) => "Instruction",
        ("en", Tokens) => "tokens",
        ("en", Model) => "Model",
        ("en", Input) => "Input",
        ("en", Cached) => "Cached",
        ("en", Output) => "Output",
        ("en", Retry) => "Retry",
        ("en", UsageTrend) => "Usage over time",
        ("en", Remaining) => "Remaining",
        ("en", GraphTokenDescription) => "Hourly token usage (by model) / remaining %",
        ("en", GraphDollarDescription) => "Hourly cumulative spend (by model) / remaining %",
        ("en", NoRecords) => "No records",
        ("en", ConnectAccount) => "Connect Codex account",
        ("en", AuthBrowserInstructions) => {
            "Complete authentication in your browser. It will be checked automatically."
        }
        ("en", AuthManaged) => {
            "Authentication is managed by Codex; this app does not store credentials."
        }
        ("en", OpenAuthPage) => "Open authentication page",
        ("en", StartAuth) => "Start authentication",
        ("en", Checking) => "Checking…",
        ("en", CheckAuth) => "Check authentication",
        ("en", AuthCli) => "Uses the Codex CLI authentication state.",
        ("en", NoHistory) => "No history",
        ("en", On) => "ON",
        ("en", Off) => "OFF",
        ("en", Connecting) => "Connecting to Codex app-server…",
        ("en", UpdatingUsage) => "Updating usage…",
        ("en", CheckingAuthStatus) => "Checking authentication…",
        ("en", AuthenticatedLoading) => "Authenticated. Loading usage…",
        ("en", UnauthenticatedStart) => "Not authenticated. Start authentication.",
        ("en", AuthUrlIssued) => "Authentication URL issued. Select “Open authentication page”.",
        ("en", IssuingAuthUrl) => "Issuing authentication URL…",
        ("en", AuthUrlOpenFailed) => "Authentication URL could not be opened.",
        ("en", CannotFetchUsage) => "Unable to fetch usage. Check the Codex app-server connection.",
        ("en", CannotDisplayStatus) => "Unable to display status.",
        ("en", QuotaNearlyGone) => "Almost no usage remains.",
        ("en", QuotaLow) => "Usage is running low.",
        ("en", ResetWithinDay) => "Within 24 hours of reset.",
        ("en", LastUpdated) => "Last updated",
        ("en", PartialHistoryThreads) => {
            "Usage updated. Previous history and threads are retained."
        }
        ("en", PartialHistory) => "Usage updated. Previous history is retained.",
        ("en", PartialThreads) => "Usage updated. Previous thread display is retained.",
        ("en", MainRole) => "Main",
        ("en", SubRole) => "Sub",
        ("en", ParentNotRunning) => "Parent thread is not running",
        ("en", ParentPrefix) => "Parent",
        ("en", CurrentSuffix) => " (current)",
        ("en", DeadlinePrefix) => "deadline",
        ("en", EstimatePrefix) => "Estimate",
        ("en", SoonReset) => "Resetting soon",
        ("en", FixedLimitNone) => "No fixed limit",
        ("en", QuotaRemaining) => "Remaining usage",
        ("en", MonthlyQuotaRemaining) => "Monthly remaining usage",
        ("en", UsageLimit) => "Usage limit",
        ("en", DollarMetric) => "Dollars",
        ("en", TokenMetric) => "Tokens",
        // Compact catalogs cover every key while keeping the source reviewable.
        ("zh", WindowUnauthenticated) => "账号未连接 — 未设置套餐",
        ("zh", PlanUnset) => "未设置套餐",
        ("zh", PlanFree) => "免费",
        ("zh", PlanEnterprise) => "企业版",
        ("zh", PlanEducation) => "教育版",
        ("zh", UsageStatus) => "使用情况",
        ("zh", Graph) => "图表",
        ("zh", LegalNotices) => "法律声明",
        ("zh", Running) => "运行中",
        ("zh", ModelThreads) => "按模型统计的线程",
        ("zh", Other) => "其他",
        ("zh", Details) => "详情",
        ("zh", NoRunningThreads) => "没有运行中的线程",
        ("zh", LegalCode) => "Codex Info 代码和文档：GPL-3.0-only",
        ("zh", LegalWarranty) => "本软件不提供保证，可按 GPL-3.0-only 条款再分发。",
        ("zh", LegalLicense) => "许可证文本：LICENSE",
        ("zh", LegalFont) => "Noto Sans JP / Noto Sans CJK KR：OFL-1.1 / Adobe 2014-2021",
        ("zh", LegalSchema) => "Codex 生成的架构：Apache-2.0 / Copyright 2025 OpenAI",
        ("zh", LegalDependencies) => "Slint 和 Rust 依赖库保留其上游许可证。",
        ("zh", LegalDetails) => "详情：THIRD_PARTY_NOTICES.md 和 LICENSES/",
        ("zh", LegalDistribution) => "分发二进制文件时请附带各依赖的 LICENSE/NOTICE。",
        ("zh", Close) => "关闭",
        ("zh", ActiveThreads) => "运行中的线程",
        ("zh", Context) => "上下文使用率",
        ("zh", Instruction) => "指令",
        ("zh", Tokens) => "令牌",
        ("zh", Model) => "模型",
        ("zh", Input) => "输入",
        ("zh", Cached) => "缓存",
        ("zh", Output) => "输出",
        ("zh", Retry) => "重试",
        ("zh", UsageTrend) => "使用情况趋势",
        ("zh", Remaining) => "剩余",
        ("zh", GraphTokenDescription) => "按小时令牌用量（按模型）/ 剩余%",
        ("zh", GraphDollarDescription) => "按小时累计美元消耗（按模型）/ 剩余%",
        ("zh", NoRecords) => "没有记录",
        ("zh", ConnectAccount) => "连接 Codex 账号",
        ("zh", AuthBrowserInstructions) => "请在浏览器中完成认证。完成后会自动检查。",
        ("zh", AuthManaged) => "认证由 Codex 管理；本应用不保存凭据。",
        ("zh", OpenAuthPage) => "打开认证页面",
        ("zh", StartAuth) => "开始认证",
        ("zh", Checking) => "检查中…",
        ("zh", CheckAuth) => "检查认证",
        ("zh", AuthCli) => "使用 Codex CLI 的认证状态。",
        ("zh", NoHistory) => "没有历史记录",
        ("zh", On) => "开",
        ("zh", Off) => "关",
        ("zh", Connecting) => "正在连接 Codex app-server…",
        ("zh", UpdatingUsage) => "正在更新使用情况…",
        ("zh", CheckingAuthStatus) => "正在检查认证状态…",
        ("zh", AuthenticatedLoading) => "已认证，正在读取用量…",
        ("zh", UnauthenticatedStart) => "未认证，请开始认证。",
        ("zh", AuthUrlIssued) => "已生成认证 URL，请选择“打开认证页面”。",
        ("zh", IssuingAuthUrl) => "正在生成认证 URL…",
        ("zh", AuthUrlOpenFailed) => "无法打开认证 URL。",
        ("zh", CannotFetchUsage) => "无法获取使用情况，请检查 Codex app-server 连接。",
        ("zh", CannotDisplayStatus) => "无法显示状态。",
        ("zh", QuotaNearlyGone) => "剩余用量几乎为零。",
        ("zh", QuotaLow) => "剩余用量较少。",
        ("zh", ResetWithinDay) => "将在 24 小时内重置。",
        ("zh", LastUpdated) => "最后更新",
        ("zh", PartialHistoryThreads) => "用量已更新，保留之前的历史和线程。",
        ("zh", PartialHistory) => "用量已更新，保留之前的历史。",
        ("zh", PartialThreads) => "用量已更新，保留之前的线程显示。",
        ("zh", MainRole) => "主线程",
        ("zh", SubRole) => "子线程",
        ("zh", ParentNotRunning) => "父线程当前未运行",
        ("zh", ParentPrefix) => "父线程",
        ("zh", CurrentSuffix) => "（当前）",
        ("zh", DeadlinePrefix) => "期限",
        ("zh", EstimatePrefix) => "估算",
        ("zh", SoonReset) => "即将重置",
        ("zh", FixedLimitNone) => "无固定上限",
        ("zh", QuotaRemaining) => "剩余用量",
        ("zh", MonthlyQuotaRemaining) => "每月剩余用量",
        ("zh", UsageLimit) => "用量上限",
        ("zh", DollarMetric) => "美元",
        ("zh", TokenMetric) => "令牌",
        _ => translated_text(language, key),
    }
}

fn translated_text(language: &str, key: TextKey) -> &'static str {
    let index = TextKey::ALL
        .iter()
        .position(|candidate| *candidate == key)
        .expect("all translation keys must be listed");
    let catalog = match language {
        "ko" => KO_CATALOG,
        "es" => ES_CATALOG,
        "fr" => FR_CATALOG,
        "de" => DE_CATALOG,
        "pt" => PT_CATALOG,
        "it" => IT_CATALOG,
        "ru" => RU_CATALOG,
        _ => panic!("unknown translation catalog: {language}"),
    };
    catalog[index]
}

const KO_CATALOG: [&str; 79] = [
    "Noto Sans CJK KR",
    "계정이 연결되지 않음 — 요금제 미설정",
    "요금제 미설정",
    "무료",
    "엔터프라이즈",
    "교육",
    "사용량",
    "그래프",
    "법적 고지",
    "실행 중",
    "모델별 스레드",
    "기타",
    "세부 정보",
    "실행 중인 스레드 없음",
    "Codex Info 코드 및 문서: GPL-3.0-only",
    "이 소프트웨어는 보증 없이 제공되며 GPL-3.0-only로 재배포할 수 있습니다.",
    "라이선스 본문: LICENSE",
    "Noto Sans JP / Noto Sans CJK KR: OFL-1.1 / Adobe 2014-2021",
    "Codex 생성 스키마: Apache-2.0 / Copyright 2025 OpenAI",
    "Slint 및 Rust 의존 크레이트는 각 상위 라이선스를 유지합니다.",
    "세부 정보: THIRD_PARTY_NOTICES.md 및 LICENSES/",
    "바이너리 배포 시 각 의존성의 LICENSE/NOTICE를 포함하세요.",
    "닫기",
    "실행 중인 스레드",
    "컨텍스트 사용률",
    "지시",
    "토큰",
    "모델",
    "입력",
    "캐시",
    "출력",
    "재시도",
    "사용량 추이",
    "남은 양",
    "시간별 토큰 사용량(모델별) / 남은 비율%",
    "시간별 누적 달러 사용량(모델별) / 남은 비율%",
    "기록 없음",
    "Codex 계정 연결",
    "브라우저에서 인증을 완료하세요. 완료 후 자동으로 확인합니다.",
    "인증은 Codex가 관리하며 이 앱은 인증 정보를 저장하지 않습니다.",
    "인증 페이지 열기",
    "인증 시작",
    "확인 중…",
    "인증 상태 확인",
    "Codex CLI 인증 상태를 사용합니다.",
    "기록 없음",
    "켜기",
    "끄기",
    "Codex app-server에 연결 중…",
    "사용량 업데이트 중…",
    "인증 상태 확인 중…",
    "인증됨. 사용량을 불러오는 중…",
    "인증되지 않았습니다. 인증을 시작하세요.",
    "인증 URL이 발급되었습니다. ‘인증 페이지 열기’를 선택하세요.",
    "인증 URL 발급 중…",
    "인증 URL을 열 수 없습니다.",
    "사용량을 가져올 수 없습니다. Codex app-server 연결을 확인하세요.",
    "상태를 표시할 수 없습니다.",
    "남은 사용량이 거의 없습니다.",
    "남은 사용량이 부족합니다.",
    "재설정까지 24시간 이내입니다.",
    "마지막 업데이트",
    "사용량이 업데이트되었습니다. 이전 기록과 스레드를 유지합니다.",
    "사용량이 업데이트되었습니다. 이전 기록을 유지합니다.",
    "사용량이 업데이트되었습니다. 이전 스레드 표시를 유지합니다.",
    "메인",
    "서브",
    "상위 스레드가 실행 중이 아님",
    "상위",
    " (현재)",
    "기한",
    "예상",
    "곧 재설정",
    "고정 한도 없음",
    "남은 사용량",
    "월간 남은 사용량",
    "사용량 한도",
    "달러",
    "토큰",
];

const ES_CATALOG: [&str; 79] = [
    "Noto Sans JP",
    "Cuenta no conectada — plan no establecido",
    "Plan no establecido",
    "Gratis",
    "Empresa",
    "Educación",
    "Uso",
    "Gráfico",
    "Avisos legales",
    "En ejecución",
    "Hilos por modelo",
    "Otros",
    "Detalles",
    "No hay hilos en ejecución",
    "Código y documentos de Codex Info: GPL-3.0-only",
    "Este software se ofrece sin garantía. Se permite redistribuirlo bajo GPL-3.0-only.",
    "Texto de licencia: LICENSE",
    "Noto Sans JP / Noto Sans CJK KR: OFL-1.1 / Adobe 2014-2021",
    "Esquema generado por Codex: Apache-2.0 / Copyright 2025 OpenAI",
    "Slint y las dependencias de Rust conservan sus licencias originales.",
    "Detalles: THIRD_PARTY_NOTICES.md y LICENSES/",
    "Incluye las licencias LICENSE/NOTICE al distribuir binarios.",
    "Cerrar",
    "Hilos en ejecución",
    "Uso del contexto",
    "Instrucción",
    "tokens",
    "Modelo",
    "Entrada",
    "Caché",
    "Salida",
    "Reintentar",
    "Evolución del uso",
    "Restante",
    "Uso de tokens por hora (por modelo) / % restante",
    "Gasto acumulado por hora (por modelo) / % restante",
    "Sin registros",
    "Conectar cuenta de Codex",
    "Completa la autenticación en el navegador. Se comprobará automáticamente.",
    "Codex gestiona la autenticación; esta aplicación no guarda credenciales.",
    "Abrir página de autenticación",
    "Iniciar autenticación",
    "Comprobando…",
    "Comprobar autenticación",
    "Usa el estado de autenticación de Codex CLI.",
    "Sin historial",
    "ACTIVADO",
    "DESACTIVADO",
    "Conectando con Codex app-server…",
    "Actualizando el uso…",
    "Comprobando la autenticación…",
    "Autenticado. Cargando el uso…",
    "No autenticado. Inicia la autenticación.",
    "URL de autenticación emitida. Selecciona «Abrir página de autenticación».",
    "Emitiendo URL de autenticación…",
    "No se pudo abrir la URL de autenticación.",
    "No se pudo obtener el uso. Comprueba la conexión con Codex app-server.",
    "No se puede mostrar el estado.",
    "Queda muy poco uso.",
    "Queda poco uso.",
    "Faltan menos de 24 horas para el reinicio.",
    "Última actualización",
    "Uso actualizado. Se conservan el historial y los hilos anteriores.",
    "Uso actualizado. Se conserva el historial anterior.",
    "Uso actualizado. Se conserva la vista de hilos anterior.",
    "Principal",
    "Sub",
    "El hilo principal no está en ejecución",
    "Principal",
    " (actual)",
    "límite",
    "Estimación",
    "Reinicio inminente",
    "Sin límite fijo",
    "Uso restante",
    "Uso mensual restante",
    "Límite de uso",
    "Dólares",
    "Tokens",
];

const FR_CATALOG: [&str; 79] = [
    "Noto Sans JP",
    "Compte non connecté — forfait non défini",
    "Forfait non défini",
    "Gratuit",
    "Entreprise",
    "Éducation",
    "Utilisation",
    "Graphique",
    "Mentions légales",
    "En cours",
    "Fils par modèle",
    "Autre",
    "Détails",
    "Aucun fil en cours",
    "Code et documents Codex Info : GPL-3.0-only",
    "Ce logiciel est fourni sans garantie. La redistribution est autorisée sous GPL-3.0-only.",
    "Texte de licence : LICENSE",
    "Noto Sans JP / Noto Sans CJK KR: OFL-1.1 / Adobe 2014-2021",
    "Schéma généré par Codex : Apache-2.0 / Copyright 2025 OpenAI",
    "Slint et les dépendances Rust conservent leurs licences amont.",
    "Détails : THIRD_PARTY_NOTICES.md et LICENSES/",
    "Joignez les fichiers LICENSE/NOTICE des dépendances lors de la distribution.",
    "Fermer",
    "Fils en cours",
    "Utilisation du contexte",
    "Instruction",
    "jetons",
    "Modèle",
    "Entrée",
    "Cache",
    "Sortie",
    "Réessayer",
    "Évolution de l’utilisation",
    "Restant",
    "Utilisation horaire des jetons (par modèle) / % restant",
    "Dépense cumulée horaire (par modèle) / % restant",
    "Aucun enregistrement",
    "Connecter le compte Codex",
    "Terminez l’authentification dans le navigateur. Elle sera vérifiée automatiquement.",
    "L’authentification est gérée par Codex ; cette application ne stocke pas les identifiants.",
    "Ouvrir la page d’authentification",
    "Démarrer l’authentification",
    "Vérification…",
    "Vérifier l’authentification",
    "Utilise l’état d’authentification de Codex CLI.",
    "Aucun historique",
    "ACTIVÉ",
    "DÉSACTIVÉ",
    "Connexion à Codex app-server…",
    "Mise à jour de l’utilisation…",
    "Vérification de l’authentification…",
    "Authentifié. Chargement de l’utilisation…",
    "Non authentifié. Démarrez l’authentification.",
    "URL d’authentification créée. Sélectionnez « Ouvrir la page d’authentification ».",
    "Création de l’URL d’authentification…",
    "Impossible d’ouvrir l’URL d’authentification.",
    "Impossible de récupérer l’utilisation. Vérifiez la connexion Codex app-server.",
    "Impossible d’afficher l’état.",
    "Il reste presque aucune utilisation.",
    "L’utilisation restante est faible.",
    "Réinitialisation dans moins de 24 heures.",
    "Dernière mise à jour",
    "Utilisation mise à jour. Historique et fils précédents conservés.",
    "Utilisation mise à jour. Historique précédent conservé.",
    "Utilisation mise à jour. Affichage des fils précédent conservé.",
    "Principal",
    "Sous",
    "Le fil parent n’est pas en cours",
    "Parent",
    " (actuel)",
    "échéance",
    "Estimation",
    "Réinitialisation imminente",
    "Aucune limite fixe",
    "Utilisation restante",
    "Utilisation mensuelle restante",
    "Limite d’utilisation",
    "Dollars",
    "Jetons",
];

const DE_CATALOG: [&str; 79] = [
    "Noto Sans JP", "Konto nicht verbunden — Tarif nicht festgelegt", "Tarif nicht festgelegt", "Kostenlos", "Unternehmen", "Bildung", "Nutzung", "Diagramm", "Rechtliche Hinweise", "Läuft", "Threads nach Modell", "Sonstige", "Details", "Keine laufenden Threads", "Codex-Info-Code und Dokumente: GPL-3.0-only", "Diese Software wird ohne Gewährleistung bereitgestellt. Weitergabe unter GPL-3.0-only ist erlaubt.", "Lizenztext: LICENSE", "Noto Sans JP / Noto Sans CJK KR: OFL-1.1 / Adobe 2014-2021", "Von Codex erzeugtes Schema: Apache-2.0 / Copyright 2025 OpenAI", "Slint und Rust-Abhängigkeiten behalten ihre ursprünglichen Lizenzen.", "Details: THIRD_PARTY_NOTICES.md und LICENSES/", "Beim Verteilen von Binärdateien die LICENSE/NOTICE-Dateien beilegen.", "Schließen", "Laufende Threads", "Kontextnutzung", "Anweisung", "Tokens", "Modell", "Eingabe", "Cache", "Ausgabe", "Erneut versuchen", "Nutzungsverlauf", "Verbleibend", "Stündliche Token-Nutzung (nach Modell) / verbleibend %", "Kumulierte stündliche Ausgaben (nach Modell) / verbleibend %", "Keine Aufzeichnungen", "Codex-Konto verbinden", "Schließe die Authentifizierung im Browser ab. Sie wird automatisch geprüft.", "Die Authentifizierung wird von Codex verwaltet; diese App speichert keine Zugangsdaten.", "Authentifizierungsseite öffnen", "Authentifizierung starten", "Wird geprüft…", "Authentifizierung prüfen", "Verwendet den Authentifizierungsstatus der Codex CLI.", "Kein Verlauf", "AN", "AUS", "Verbindung mit Codex app-server…", "Nutzung wird aktualisiert…", "Authentifizierung wird geprüft…", "Authentifiziert. Nutzung wird geladen…", "Nicht authentifiziert. Authentifizierung starten.", "Authentifizierungs-URL erstellt. «Authentifizierungsseite öffnen» wählen.", "Authentifizierungs-URL wird erstellt…", "Authentifizierungs-URL konnte nicht geöffnet werden.", "Nutzung konnte nicht abgerufen werden. Codex-app-server-Verbindung prüfen.", "Status kann nicht angezeigt werden.", "Fast keine Nutzung mehr verfügbar.", "Nutzung wird knapp.", "Weniger als 24 Stunden bis zum Zurücksetzen.", "Zuletzt aktualisiert", "Nutzung aktualisiert. Vorheriger Verlauf und Threads bleiben erhalten.", "Nutzung aktualisiert. Vorheriger Verlauf bleibt erhalten.", "Nutzung aktualisiert. Vorherige Thread-Anzeige bleibt erhalten.", "Haupt", "Unter", "Übergeordneter Thread läuft nicht", "Übergeordnet", " (aktuell)", "Frist", "Schätzung", "Wird bald zurückgesetzt", "Keine feste Grenze", "Verbleibende Nutzung", "Verbleibende Monatsnutzung", "Nutzungsgrenze", "Dollar", "Token",
];

const PT_CATALOG: [&str; 79] = [
    "Noto Sans JP",
    "Conta não conectada — plano não definido",
    "Plano não definido",
    "Grátis",
    "Empresa",
    "Educação",
    "Uso",
    "Gráfico",
    "Avisos legais",
    "Em execução",
    "Threads por modelo",
    "Outros",
    "Detalhes",
    "Nenhuma thread em execução",
    "Código e documentos do Codex Info: GPL-3.0-only",
    "Este software é fornecido sem garantia. A redistribuição sob GPL-3.0-only é permitida.",
    "Texto da licença: LICENSE",
    "Noto Sans JP / Noto Sans CJK KR: OFL-1.1 / Adobe 2014-2021",
    "Esquema gerado pelo Codex: Apache-2.0 / Copyright 2025 OpenAI",
    "Slint e as dependências Rust mantêm suas licenças originais.",
    "Detalhes: THIRD_PARTY_NOTICES.md e LICENSES/",
    "Inclua LICENSE/NOTICE de cada dependência ao distribuir binários.",
    "Fechar",
    "Threads em execução",
    "Uso do contexto",
    "Instrução",
    "tokens",
    "Modelo",
    "Entrada",
    "Cache",
    "Saída",
    "Tentar novamente",
    "Evolução do uso",
    "Restante",
    "Uso de tokens por hora (por modelo) / % restante",
    "Gasto cumulativo por hora (por modelo) / % restante",
    "Sem registros",
    "Conectar conta Codex",
    "Conclua a autenticação no navegador. Ela será verificada automaticamente.",
    "A autenticação é gerenciada pelo Codex; este app não armazena credenciais.",
    "Abrir página de autenticação",
    "Iniciar autenticação",
    "Verificando…",
    "Verificar autenticação",
    "Usa o estado de autenticação da Codex CLI.",
    "Sem histórico",
    "LIGADO",
    "DESLIGADO",
    "Conectando ao Codex app-server…",
    "Atualizando o uso…",
    "Verificando autenticação…",
    "Autenticado. Carregando uso…",
    "Não autenticado. Inicie a autenticação.",
    "URL de autenticação emitida. Selecione «Abrir página de autenticação».",
    "Emitindo URL de autenticação…",
    "Não foi possível abrir a URL de autenticação.",
    "Não foi possível obter o uso. Verifique a conexão com o Codex app-server.",
    "Não é possível exibir o estado.",
    "Quase não resta uso.",
    "O uso restante está baixo.",
    "Faltam menos de 24 horas para a redefinição.",
    "Última atualização",
    "Uso atualizado. Histórico e threads anteriores foram mantidos.",
    "Uso atualizado. Histórico anterior foi mantido.",
    "Uso atualizado. A visualização anterior das threads foi mantida.",
    "Principal",
    "Sub",
    "A thread pai não está em execução",
    "Pai",
    " (atual)",
    "prazo",
    "Estimativa",
    "Redefinição próxima",
    "Sem limite fixo",
    "Uso restante",
    "Uso mensal restante",
    "Limite de uso",
    "Dólares",
    "Tokens",
];

const IT_CATALOG: [&str; 79] = [
    "Noto Sans JP",
    "Account non collegato — piano non impostato",
    "Piano non impostato",
    "Gratuito",
    "Aziendale",
    "Istruzione",
    "Utilizzo",
    "Grafico",
    "Note legali",
    "In esecuzione",
    "Thread per modello",
    "Altro",
    "Dettagli",
    "Nessun thread in esecuzione",
    "Codice e documenti Codex Info: GPL-3.0-only",
    "Questo software è fornito senza garanzia. La ridistribuzione è consentita con GPL-3.0-only.",
    "Testo della licenza: LICENSE",
    "Noto Sans JP / Noto Sans CJK KR: OFL-1.1 / Adobe 2014-2021",
    "Schema generato da Codex: Apache-2.0 / Copyright 2025 OpenAI",
    "Slint e le dipendenze Rust mantengono le licenze originali.",
    "Dettagli: THIRD_PARTY_NOTICES.md e LICENSES/",
    "Includi LICENSE/NOTICE di ogni dipendenza nella distribuzione dei binari.",
    "Chiudi",
    "Thread in esecuzione",
    "Utilizzo del contesto",
    "Istruzione",
    "token",
    "Modello",
    "Input",
    "Cache",
    "Output",
    "Riprova",
    "Andamento dell’utilizzo",
    "Rimanente",
    "Utilizzo token orario (per modello) / % rimanente",
    "Spesa cumulativa oraria (per modello) / % rimanente",
    "Nessun record",
    "Collega account Codex",
    "Completa l’autenticazione nel browser. Verrà verificata automaticamente.",
    "L’autenticazione è gestita da Codex; l’app non salva credenziali.",
    "Apri pagina di autenticazione",
    "Avvia autenticazione",
    "Verifica in corso…",
    "Verifica autenticazione",
    "Usa lo stato di autenticazione della Codex CLI.",
    "Nessuna cronologia",
    "ATTIVO",
    "DISATTIVO",
    "Connessione a Codex app-server…",
    "Aggiornamento utilizzo…",
    "Verifica autenticazione…",
    "Autenticato. Caricamento utilizzo…",
    "Non autenticato. Avvia l’autenticazione.",
    "URL di autenticazione emesso. Seleziona «Apri pagina di autenticazione».",
    "Emissione URL di autenticazione…",
    "Impossibile aprire l’URL di autenticazione.",
    "Impossibile recuperare l’utilizzo. Controlla la connessione a Codex app-server.",
    "Impossibile mostrare lo stato.",
    "Quasi nessun utilizzo rimanente.",
    "L’utilizzo rimanente è basso.",
    "Meno di 24 ore al ripristino.",
    "Ultimo aggiornamento",
    "Utilizzo aggiornato. Cronologia e thread precedenti conservati.",
    "Utilizzo aggiornato. Cronologia precedente conservata.",
    "Utilizzo aggiornato. Visualizzazione precedente dei thread conservata.",
    "Principale",
    "Secondario",
    "Il thread principale non è in esecuzione",
    "Principale",
    " (attuale)",
    "scadenza",
    "Stima",
    "Ripristino imminente",
    "Nessun limite fisso",
    "Utilizzo rimanente",
    "Utilizzo mensile rimanente",
    "Limite di utilizzo",
    "Dollari",
    "Token",
];

const RU_CATALOG: [&str; 79] = [
    "Noto Sans JP",
    "Аккаунт не подключён — тариф не задан",
    "Тариф не задан",
    "Бесплатный",
    "Корпоративный",
    "Образование",
    "Использование",
    "График",
    "Правовые уведомления",
    "Выполняется",
    "Потоки по модели",
    "Другое",
    "Подробнее",
    "Нет выполняющихся потоков",
    "Код и документы Codex Info: GPL-3.0-only",
    "Программа предоставляется без гарантий. Распространение разрешено по GPL-3.0-only.",
    "Текст лицензии: LICENSE",
    "Noto Sans JP / Noto Sans CJK KR: OFL-1.1 / Adobe 2014-2021",
    "Схема Codex: Apache-2.0 / Copyright 2025 OpenAI",
    "Slint и зависимости Rust сохраняют исходные лицензии.",
    "Подробнее: THIRD_PARTY_NOTICES.md и LICENSES/",
    "При распространении бинарных файлов приложите LICENSE/NOTICE зависимостей.",
    "Закрыть",
    "Выполняющиеся потоки",
    "Использование контекста",
    "Инструкция",
    "токены",
    "Модель",
    "Ввод",
    "Кэш",
    "Вывод",
    "Повторить",
    "Динамика использования",
    "Осталось",
    "Почасовое использование токенов (по модели) / осталось %",
    "Накопленные почасовые расходы (по модели) / осталось %",
    "Нет записей",
    "Подключить аккаунт Codex",
    "Завершите аутентификацию в браузере. Она будет проверена автоматически.",
    "Аутентификацией управляет Codex; приложение не хранит учётные данные.",
    "Открыть страницу аутентификации",
    "Начать аутентификацию",
    "Проверка…",
    "Проверить аутентификацию",
    "Используется состояние аутентификации Codex CLI.",
    "Нет истории",
    "ВКЛ",
    "ВЫКЛ",
    "Подключение к Codex app-server…",
    "Обновление использования…",
    "Проверка аутентификации…",
    "Аутентификация выполнена. Загрузка использования…",
    "Нет аутентификации. Начните аутентификацию.",
    "URL аутентификации создан. Выберите «Открыть страницу аутентификации».",
    "Создание URL аутентификации…",
    "Не удалось открыть URL аутентификации.",
    "Не удалось получить использование. Проверьте подключение к Codex app-server.",
    "Не удалось показать состояние.",
    "Почти не осталось доступного использования.",
    "Доступное использование заканчивается.",
    "До сброса осталось менее 24 часов.",
    "Последнее обновление",
    "Использование обновлено. Предыдущие история и потоки сохранены.",
    "Использование обновлено. Предыдущая история сохранена.",
    "Использование обновлено. Предыдущее отображение потоков сохранено.",
    "Основной",
    "Дочерний",
    "Родительский поток не выполняется",
    "Родитель",
    " (текущий)",
    "срок",
    "Оценка",
    "Скорый сброс",
    "Без фиксированного лимита",
    "Оставшееся использование",
    "Оставшееся месячное использование",
    "Лимит использования",
    "Доллары",
    "Токены",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_precedence_and_fallbacks_are_deterministic() {
        assert_eq!(
            Language::detect_from_values(Some("en_US.UTF-8"), Some("ja_JP.UTF-8"), Some("de_DE")),
            Language::English
        );
        assert_eq!(
            Language::detect_from_values(Some(""), Some("ja_JP.UTF-8"), Some("en_US")),
            Language::Japanese
        );
        assert_eq!(
            Language::detect_from_values(Some("C.UTF-8"), Some("ja_JP.UTF-8"), Some("en_US")),
            Language::English
        );
        assert_eq!(
            Language::detect_from_values(None, None, None),
            Language::English
        );
        assert_eq!(
            Language::detect_from_values(Some("zh_TW.UTF-8"), None, None),
            Language::SimplifiedChinese
        );
        assert_eq!(
            Language::detect_from_values(Some("ko-KR"), None, None),
            Language::Korean
        );
        assert_eq!(
            Language::detect_from_values(Some("ar_SA"), None, None),
            Language::English
        );
    }

    #[test]
    fn every_catalog_key_is_non_empty() {
        for language in Language::ALL {
            let i18n = I18n::from_parts(language, Tz::UTC);
            for key in TextKey::ALL {
                assert!(
                    !i18n.text(*key).trim().is_empty(),
                    "{} {:?}",
                    language.code(),
                    key
                );
            }
        }
        assert_ne!(
            I18n::from_parts(Language::Korean, Tz::UTC).text(TextKey::UsageStatus),
            I18n::from_parts(Language::English, Tz::UTC).text(TextKey::UsageStatus)
        );
        assert_ne!(
            I18n::from_parts(Language::Russian, Tz::UTC).text(TextKey::UsageStatus),
            I18n::from_parts(Language::English, Tz::UTC).text(TextKey::UsageStatus)
        );
        assert_eq!(
            I18n::from_parts(Language::SimplifiedChinese, Tz::UTC).font_family(),
            "Noto Sans CJK KR"
        );
        assert_eq!(
            I18n::from_parts(Language::Japanese, Tz::UTC).font_family(),
            "Noto Sans JP"
        );
    }

    #[test]
    fn legal_font_notice_matches_embedded_font_attribution() {
        for language in Language::ALL {
            let notice = I18n::from_parts(language, Tz::UTC).text(TextKey::LegalFont);
            assert!(notice.contains("OFL-1.1"), "{}: {notice}", language.code());
            assert!(
                notice.contains("Adobe 2014-2021"),
                "{}: {notice}",
                language.code()
            );
            assert!(
                !notice.contains("Noto Sans KR:"),
                "{}: {notice}",
                language.code()
            );
        }
    }

    #[test]
    fn context_usage_is_derived_from_tokens_and_capped_at_full() {
        let english = I18n::from_parts(Language::English, Tz::UTC);
        assert_eq!(english.format_context_usage(0, 100), "0%");
        assert_eq!(english.format_context_usage(50, 100), "50%");
        assert_eq!(english.format_context_usage(1, 3), "33.3%");
        assert_eq!(english.format_context_usage(u64::MAX, 100), "100%");
        assert_eq!(english.format_context_usage(1, 0), "—");

        let french = I18n::from_parts(Language::French, Tz::UTC);
        assert_eq!(french.format_context_usage(1, 3), "33,3%");
    }

    #[test]
    fn elapsed_and_remaining_boundaries_use_utc_seconds() {
        let i18n = I18n::from_parts(Language::English, Tz::UTC);
        assert_eq!(i18n.format_elapsed(100, Some(100)), "0 seconds");
        assert_eq!(i18n.format_elapsed(100, Some(41)), "59 seconds");
        assert_eq!(i18n.format_elapsed(100, Some(40)), "1 minute");
        assert_eq!(i18n.format_elapsed(100, Some(-3_560)), "1 hour 1 minute");
        assert_eq!(
            i18n.format_period_remaining(0, PeriodKind::Weekly),
            "Resetting soon"
        );
        assert_eq!(
            i18n.format_period_remaining(86_400 + 3_600 + 60, PeriodKind::Weekly),
            "7-day period, 1 day, 1 hour, 1 minute remaining"
        );
    }

    #[test]
    fn named_timezone_keeps_dst_in_absolute_labels() {
        let i18n = I18n::from_parts(Language::English, Tz::America__New_York);
        assert!(i18n
            .format_timestamp(1_709_900_000)
            .unwrap()
            .ends_with("-05:00"));
        assert!(i18n
            .format_timestamp(1_715_000_000)
            .unwrap()
            .ends_with("-04:00"));
    }

    #[test]
    fn timezone_configuration_accepts_only_named_iana_ids() {
        assert_eq!(
            timezone_from_names(Some(":Asia/Tokyo"), Some("UTC")),
            Tz::Asia__Tokyo
        );
        assert_eq!(
            timezone_from_names(Some("Etc/GMT-9"), Some("UTC")),
            Tz::from_str("Etc/GMT-9").unwrap()
        );
        assert_eq!(
            timezone_from_names(Some("JST-9"), Some("America/New_York")),
            Tz::UTC
        );
        assert_eq!(
            timezone_from_names(Some("+09:00"), Some("America/New_York")),
            Tz::UTC
        );
        assert_eq!(
            timezone_from_names(Some("  "), Some("Europe/Paris")),
            Tz::Europe__Paris
        );
        assert_eq!(timezone_from_names(None, None), Tz::UTC);
    }

    #[test]
    fn startup_timezone_object_is_immutable_after_environment_resolution() {
        let startup = I18n::from_parts(Language::English, Tz::Asia__Tokyo);
        let before = startup.format_timestamp(0).unwrap();
        let after = startup.format_timestamp(0).unwrap();
        assert_eq!(before, after);
        assert!(before.ends_with("+09:00"));
    }
}
