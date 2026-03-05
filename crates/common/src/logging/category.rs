use super::colors;

#[derive(Debug, Clone, Copy)]
pub enum Category {
    System,
    Db,
    Net,
    Auth,
    Conn,
    Data,
    Boot,
    Work,
    Alert,
    Cfg,
    Retry,
    Notify,
    Rest,
    Backpressure,
    Registry,
    Unknown,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "SYSTEM",
            Self::Db => "DB",
            Self::Net => "NET",
            Self::Auth => "AUTH",
            Self::Conn => "CONN",
            Self::Data => "DATA",
            Self::Boot => "BOOT",
            Self::Work => "WORK",
            Self::Alert => "ALERT",
            Self::Cfg => "CFG",
            Self::Retry => "RETRY",
            Self::Notify => "NOTIFY",
            Self::Rest => "REST",
            Self::Backpressure => "BKPRES",
            Self::Registry => "REGISTRY",
            Self::Unknown => "LOG",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::System => colors::BRIGHT_MAGENTA,
            Self::Db => colors::CYAN,
            Self::Net => colors::BLUE,
            Self::Auth => colors::YELLOW,
            Self::Conn => colors::GREEN,
            Self::Data => colors::BRIGHT_WHITE,
            Self::Boot => colors::BRIGHT_MAGENTA,
            Self::Work => colors::BRIGHT_CYAN,
            Self::Alert => colors::BRIGHT_RED,
            Self::Cfg => colors::WHITE,
            Self::Retry => colors::YELLOW,
            Self::Notify => colors::BRIGHT_BLUE,
            Self::Rest => colors::BRIGHT_GREEN,
            Self::Backpressure => colors::RED,
            Self::Registry => colors::MAGENTA,
            Self::Unknown => colors::GRAY,
        }
    }

    pub fn from_target(target: &str) -> Self {
        let t = target.to_lowercase();

        if t == "system" || t == "sys" {
            return Self::System;
        }
        if t == "db" || t == "database" {
            return Self::Db;
        }
        if t == "net" || t == "network" || t == "grpc" || t == "nats" {
            return Self::Net;
        }
        if t == "auth" {
            return Self::Auth;
        }
        if t == "conn" || t == "connection" || t == "session" {
            return Self::Conn;
        }
        if t == "data" || t == "ingest" {
            return Self::Data;
        }
        if t == "boot" || t == "bootstrap" {
            return Self::Boot;
        }
        if t == "work" || t == "worker" {
            return Self::Work;
        }
        if t == "alert" {
            return Self::Alert;
        }
        if t == "cfg" || t == "config" {
            return Self::Cfg;
        }
        if t == "retry" {
            return Self::Retry;
        }
        if t == "notify" || t == "notification" {
            return Self::Notify;
        }
        if t == "rest" || t == "api" {
            return Self::Rest;
        }
        if t == "backpressure" {
            return Self::Backpressure;
        }
        if t == "registry" {
            return Self::Registry;
        }

        if t.contains("migration") || t.contains("database") {
            return Self::Db;
        }
        if t.contains("stream") || t.contains("grpc") || t.contains("nats") || t.contains("broker")
        {
            return Self::Net;
        }
        if t.contains("auth") || t.contains("provisioning") || t.contains("token") {
            return Self::Auth;
        }
        if t.contains("session") || t.contains("watchdog") || t.contains("presence") {
            return Self::Conn;
        }
        if t.contains("registry") || t.contains("heartbeat") {
            return Self::Registry;
        }
        if t.contains("metric") || t.contains("ingest") || t.contains("consumer") {
            return Self::Data;
        }
        if t.contains("bootstrap") {
            return Self::Boot;
        }
        if t.contains("worker") {
            return Self::Work;
        }
        if t.contains("alert") {
            return Self::Alert;
        }
        if t.contains("notif") {
            return Self::Notify;
        }
        if t.contains("retry") {
            return Self::Retry;
        }
        if t.contains("backpressure") || t.contains("circuit") {
            return Self::Backpressure;
        }
        if t.contains("rest") {
            return Self::Rest;
        }
        if t.contains("config") {
            return Self::Cfg;
        }

        Self::Unknown
    }
}
