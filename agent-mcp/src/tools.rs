//! Outils système MCP — administration complète de la machine.

use serde_json::{json, Value};
use std::path::Path;
use tokio::process::Command;

// ═══════════════════════════════════════════════════════════════════
// Registre des outils
// ═══════════════════════════════════════════════════════════════════

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub schema_fn: fn() -> Value,
}

impl ToolDef {
    pub fn schema(&self) -> Value {
        (self.schema_fn)()
    }
}

pub static TOOL_LIST: &[ToolDef] = &[
    // ── Fichiers ─────────────────────────────────────────────
    ToolDef { name: "fs_read", description: "Lire le contenu d'un fichier",
        schema_fn: || json!({"type":"object","properties":{"path":{"type":"string","description":"Chemin absolu du fichier"}},"required":["path"]}) },
    ToolDef { name: "fs_write", description: "Écrire/créer un fichier (crée les dossiers parents)",
        schema_fn: || json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}) },
    ToolDef { name: "fs_append", description: "Ajouter du contenu à la fin d'un fichier",
        schema_fn: || json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}) },
    ToolDef { name: "fs_delete", description: "Supprimer un fichier ou dossier (récursif)",
        schema_fn: || json!({"type":"object","properties":{"path":{"type":"string"},"recursive":{"type":"boolean"}},"required":["path"]}) },
    ToolDef { name: "fs_move", description: "Déplacer/renommer un fichier ou dossier",
        schema_fn: || json!({"type":"object","properties":{"source":{"type":"string"},"destination":{"type":"string"}},"required":["source","destination"]}) },
    ToolDef { name: "fs_copy", description: "Copier un fichier ou dossier",
        schema_fn: || json!({"type":"object","properties":{"source":{"type":"string"},"destination":{"type":"string"}},"required":["source","destination"]}) },
    ToolDef { name: "fs_mkdir", description: "Créer un répertoire (et parents)",
        schema_fn: || json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}) },
    ToolDef { name: "fs_list", description: "Lister le contenu d'un répertoire",
        schema_fn: || json!({"type":"object","properties":{"path":{"type":"string"},"recursive":{"type":"boolean"},"pattern":{"type":"string"}},"required":["path"]}) },
    ToolDef { name: "fs_stat", description: "Informations sur un fichier (taille, permissions, dates)",
        schema_fn: || json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}) },
    ToolDef { name: "fs_chmod", description: "Changer les permissions d'un fichier",
        schema_fn: || json!({"type":"object","properties":{"path":{"type":"string"},"mode":{"type":"string"}},"required":["path","mode"]}) },
    ToolDef { name: "fs_chown", description: "Changer le propriétaire d'un fichier",
        schema_fn: || json!({"type":"object","properties":{"path":{"type":"string"},"owner":{"type":"string","description":"user:group"}},"required":["path","owner"]}) },
    ToolDef { name: "fs_find", description: "Rechercher des fichiers",
        schema_fn: || json!({"type":"object","properties":{"path":{"type":"string"},"name":{"type":"string"},"type":{"type":"string"},"maxdepth":{"type":"integer"}},"required":["path"]}) },
    ToolDef { name: "fs_grep", description: "Rechercher du texte dans des fichiers",
        schema_fn: || json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"},"recursive":{"type":"boolean"},"ignore_case":{"type":"boolean"}},"required":["pattern","path"]}) },
    ToolDef { name: "fs_disk_usage", description: "Espace disque utilisé et disponible",
        schema_fn: || json!({"type":"object","properties":{"path":{"type":"string"}},"required":[]}) },
    // ── Shell ────────────────────────────────────────────────
    ToolDef { name: "shell_exec", description: "Exécuter une commande shell arbitraire",
        schema_fn: || json!({"type":"object","properties":{"command":{"type":"string"},"cwd":{"type":"string"},"timeout":{"type":"integer"}},"required":["command"]}) },
    // ── Packages ─────────────────────────────────────────────
    ToolDef { name: "pkg_install", description: "Installer des paquets (apt/dnf auto-détecté)",
        schema_fn: || json!({"type":"object","properties":{"packages":{"type":"string"}},"required":["packages"]}) },
    ToolDef { name: "pkg_remove", description: "Désinstaller des paquets",
        schema_fn: || json!({"type":"object","properties":{"packages":{"type":"string"}},"required":["packages"]}) },
    ToolDef { name: "pkg_update", description: "Mettre à jour la liste des paquets",
        schema_fn: || json!({"type":"object","properties":{"upgrade":{"type":"boolean"}},"required":[]}) },
    ToolDef { name: "pkg_list", description: "Lister les paquets installés",
        schema_fn: || json!({"type":"object","properties":{"filter":{"type":"string"}},"required":[]}) },
    ToolDef { name: "pkg_info", description: "Informations sur un paquet",
        schema_fn: || json!({"type":"object","properties":{"package":{"type":"string"}},"required":["package"]}) },
    // ── Services systemd ─────────────────────────────────────
    ToolDef { name: "svc_status", description: "Statut d'un service",
        schema_fn: || json!({"type":"object","properties":{"service":{"type":"string"}},"required":["service"]}) },
    ToolDef { name: "svc_start", description: "Démarrer un service",
        schema_fn: || json!({"type":"object","properties":{"service":{"type":"string"}},"required":["service"]}) },
    ToolDef { name: "svc_stop", description: "Arrêter un service",
        schema_fn: || json!({"type":"object","properties":{"service":{"type":"string"}},"required":["service"]}) },
    ToolDef { name: "svc_restart", description: "Redémarrer un service",
        schema_fn: || json!({"type":"object","properties":{"service":{"type":"string"}},"required":["service"]}) },
    ToolDef { name: "svc_enable", description: "Activer un service au boot",
        schema_fn: || json!({"type":"object","properties":{"service":{"type":"string"}},"required":["service"]}) },
    ToolDef { name: "svc_disable", description: "Désactiver un service au boot",
        schema_fn: || json!({"type":"object","properties":{"service":{"type":"string"}},"required":["service"]}) },
    ToolDef { name: "svc_list", description: "Lister les services",
        schema_fn: || json!({"type":"object","properties":{"filter":{"type":"string"}},"required":[]}) },
    ToolDef { name: "svc_logs", description: "Journaux d'un service",
        schema_fn: || json!({"type":"object","properties":{"service":{"type":"string"},"lines":{"type":"integer"}},"required":["service"]}) },
    // ── Cron ─────────────────────────────────────────────────
    ToolDef { name: "cron_list", description: "Lister les crontabs",
        schema_fn: || json!({"type":"object","properties":{"user":{"type":"string"}},"required":[]}) },
    ToolDef { name: "cron_add", description: "Ajouter une entrée crontab",
        schema_fn: || json!({"type":"object","properties":{"schedule":{"type":"string"},"command":{"type":"string"},"user":{"type":"string"},"comment":{"type":"string"}},"required":["schedule","command"]}) },
    ToolDef { name: "cron_remove", description: "Supprimer une entrée crontab",
        schema_fn: || json!({"type":"object","properties":{"pattern":{"type":"string"},"user":{"type":"string"}},"required":["pattern"]}) },
    // ── Réseau ───────────────────────────────────────────────
    ToolDef { name: "net_interfaces", description: "Interfaces réseau et adresses IP",
        schema_fn: || json!({"type":"object","properties":{},"required":[]}) },
    ToolDef { name: "net_ports", description: "Ports en écoute",
        schema_fn: || json!({"type":"object","properties":{},"required":[]}) },
    ToolDef { name: "net_ping", description: "Ping un hôte",
        schema_fn: || json!({"type":"object","properties":{"host":{"type":"string"},"count":{"type":"integer"}},"required":["host"]}) },
    ToolDef { name: "net_dns", description: "Résolution DNS",
        schema_fn: || json!({"type":"object","properties":{"host":{"type":"string"},"type":{"type":"string"}},"required":["host"]}) },
    ToolDef { name: "net_curl", description: "Requête HTTP",
        schema_fn: || json!({"type":"object","properties":{"url":{"type":"string"},"method":{"type":"string"},"data":{"type":"string"},"headers":{"type":"string"}},"required":["url"]}) },
    ToolDef { name: "net_firewall", description: "Gérer le firewall",
        schema_fn: || json!({"type":"object","properties":{"action":{"type":"string","description":"list|allow|deny|delete"},"port":{"type":"string"},"source":{"type":"string"}},"required":["action"]}) },
    // ── Processus ────────────────────────────────────────────
    ToolDef { name: "proc_list", description: "Lister les processus",
        schema_fn: || json!({"type":"object","properties":{"filter":{"type":"string"}},"required":[]}) },
    ToolDef { name: "proc_kill", description: "Tuer un processus",
        schema_fn: || json!({"type":"object","properties":{"target":{"type":"string"},"signal":{"type":"string"}},"required":["target"]}) },
    // ── Système ──────────────────────────────────────────────
    ToolDef { name: "sys_info", description: "Informations système complètes",
        schema_fn: || json!({"type":"object","properties":{},"required":[]}) },
    ToolDef { name: "sys_env", description: "Variables d'environnement",
        schema_fn: || json!({"type":"object","properties":{"name":{"type":"string"},"value":{"type":"string"}},"required":[]}) },
    ToolDef { name: "sys_reboot", description: "Redémarrer la machine",
        schema_fn: || json!({"type":"object","properties":{"delay":{"type":"integer"}},"required":[]}) },
    // ── Utilisateurs ─────────────────────────────────────────
    ToolDef { name: "user_list", description: "Lister les utilisateurs",
        schema_fn: || json!({"type":"object","properties":{},"required":[]}) },
    ToolDef { name: "user_add", description: "Créer un utilisateur",
        schema_fn: || json!({"type":"object","properties":{"username":{"type":"string"},"groups":{"type":"string"},"shell":{"type":"string"},"home":{"type":"string"}},"required":["username"]}) },
    ToolDef { name: "user_delete", description: "Supprimer un utilisateur",
        schema_fn: || json!({"type":"object","properties":{"username":{"type":"string"},"remove_home":{"type":"boolean"}},"required":["username"]}) },
    ToolDef { name: "user_groups", description: "Gérer les groupes d'un utilisateur",
        schema_fn: || json!({"type":"object","properties":{"username":{"type":"string"},"add_groups":{"type":"string"},"remove_groups":{"type":"string"}},"required":["username"]}) },
];

// ═══════════════════════════════════════════════════════════════════
// Exécution
// ═══════════════════════════════════════════════════════════════════

fn s(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
fn i(v: &Value, key: &str, default: i64) -> i64 {
    v.get(key).and_then(|x| x.as_i64()).unwrap_or(default)
}
fn b(v: &Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(|x| x.as_bool()).unwrap_or(default)
}

/// Normalize arguments — LLMs often nest them in "input", "arguments", etc.
fn normalize_args(args: &Value) -> Value {
    if !args.is_object() {
        return args.clone();
    }
    // Unwrap common wrappers
    for wrapper in &["input", "arguments", "params", "parameters"] {
        if let Some(inner) = args.get(*wrapper) {
            if inner.is_object() {
                return inner.clone();
            }
        }
    }
    // Single-key wrapper: {"foo": {"actual": "args"}}
    if let Some(obj) = args.as_object() {
        if obj.len() == 1 {
            let only = obj.values().next().unwrap();
            if only.is_object() {
                return only.clone();
            }
        }
    }
    args.clone()
}

async fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Échec exécution {cmd}: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        Ok(if stdout.is_empty() { stderr } else { stdout })
    } else {
        Err(format!("Exit code {}: {}", output.status.code().unwrap_or(-1),
            if stderr.is_empty() { stdout } else { stderr }))
    }
}

async fn run_sh(cmd: &str) -> Result<String, String> {
    run("bash", &["-c", cmd]).await
}

fn detect_pkg_manager() -> &'static str {
    if Path::new("/usr/bin/apt-get").exists() { "apt" }
    else if Path::new("/usr/bin/dnf").exists() { "dnf" }
    else if Path::new("/usr/bin/yum").exists() { "yum" }
    else { "unknown" }
}

pub async fn execute(name: &str, raw_args: &Value) -> Result<String, String> {
    let a = normalize_args(raw_args);
    let args = &a;
    tracing::debug!("Tool {name} args: {args}");
    match name {
        // ── Fichiers ─────────────────────────────────────
        "fs_read" => {
            let path = s(args, "path");
            tokio::fs::read_to_string(&path).await
                .map_err(|e| format!("Lecture {path}: {e}"))
        }
        "fs_write" => {
            let path = s(args, "path");
            let content = s(args, "content");
            if let Some(parent) = Path::new(&path).parent() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| format!("Création dossier parent: {e}"))?;
            }
            tokio::fs::write(&path, content.as_bytes()).await
                .map_err(|e| format!("Écriture {path}: {e}"))?;
            Ok(format!("Fichier écrit: {path}"))
        }
        "fs_append" => {
            let path = s(args, "path");
            let content = s(args, "content");
            use tokio::io::AsyncWriteExt;
            let mut f = tokio::fs::OpenOptions::new().append(true).create(true).open(&path).await
                .map_err(|e| format!("Ouverture {path}: {e}"))?;
            f.write_all(content.as_bytes()).await
                .map_err(|e| format!("Écriture {path}: {e}"))?;
            Ok(format!("Contenu ajouté à {path}"))
        }
        "fs_delete" => {
            let path = s(args, "path");
            let recursive = b(args, "recursive", false);
            let meta = tokio::fs::metadata(&path).await
                .map_err(|e| format!("Stat {path}: {e}"))?;
            if meta.is_dir() {
                if recursive {
                    tokio::fs::remove_dir_all(&path).await
                } else {
                    tokio::fs::remove_dir(&path).await
                }.map_err(|e| format!("Suppression {path}: {e}"))?;
            } else {
                tokio::fs::remove_file(&path).await
                    .map_err(|e| format!("Suppression {path}: {e}"))?;
            }
            Ok(format!("Supprimé: {path}"))
        }
        "fs_move" => run_sh(&format!("mv '{}' '{}'", s(args, "source"), s(args, "destination"))).await,
        "fs_copy" => run_sh(&format!("cp -a '{}' '{}'", s(args, "source"), s(args, "destination"))).await,
        "fs_mkdir" => {
            let path = s(args, "path");
            tokio::fs::create_dir_all(&path).await
                .map_err(|e| format!("Création {path}: {e}"))?;
            Ok(format!("Répertoire créé: {path}"))
        }
        "fs_list" => {
            let path = s(args, "path");
            let recursive = b(args, "recursive", false);
            let pattern = s(args, "pattern");
            let mut cmd = format!("ls -la '{path}'");
            if recursive { cmd = format!("find '{path}' -ls"); }
            if !pattern.is_empty() { cmd = format!("find '{path}' -name '{pattern}' -ls"); }
            run_sh(&cmd).await
        }
        "fs_stat" => run_sh(&format!("stat '{}'", s(args, "path"))).await,
        "fs_chmod" => run_sh(&format!("chmod {} '{}'", s(args, "mode"), s(args, "path"))).await,
        "fs_chown" => run_sh(&format!("chown {} '{}'", s(args, "owner"), s(args, "path"))).await,
        "fs_find" => {
            let mut cmd = format!("find '{}'", s(args, "path"));
            let name = s(args, "name");
            if !name.is_empty() { cmd.push_str(&format!(" -name '{name}'")); }
            let ftype = s(args, "type");
            if !ftype.is_empty() { cmd.push_str(&format!(" -type {ftype}")); }
            let depth = i(args, "maxdepth", 0);
            if depth > 0 { cmd.push_str(&format!(" -maxdepth {depth}")); }
            run_sh(&cmd).await
        }
        "fs_grep" => {
            let mut cmd = format!("grep");
            if b(args, "recursive", false) { cmd.push_str(" -r"); }
            if b(args, "ignore_case", false) { cmd.push_str(" -i"); }
            cmd.push_str(&format!(" '{}' '{}'", s(args, "pattern"), s(args, "path")));
            run_sh(&cmd).await
        }
        "fs_disk_usage" => {
            let path = if s(args, "path").is_empty() { "/".to_string() } else { s(args, "path") };
            run_sh(&format!("df -h '{path}' && echo '---' && du -sh '{path}'")).await
        }

        // ── Shell ────────────────────────────────────────
        "shell_exec" => {
            let command = s(args, "command");
            let cwd = s(args, "cwd");
            let timeout = i(args, "timeout", 60);
            let cmd = if cwd.is_empty() {
                command.clone()
            } else {
                format!("cd '{}' && {}", cwd, command)
            };
            match tokio::time::timeout(
                std::time::Duration::from_secs(timeout as u64),
                run_sh(&cmd),
            ).await {
                Ok(result) => result,
                Err(_) => Err(format!("Timeout après {timeout}s")),
            }
        }

        // ── Packages ─────────────────────────────────────
        "pkg_install" => {
            let pkgs = s(args, "packages");
            match detect_pkg_manager() {
                "apt" => run_sh(&format!("DEBIAN_FRONTEND=noninteractive apt-get install -y {pkgs}")).await,
                "dnf" => run_sh(&format!("dnf install -y {pkgs}")).await,
                "yum" => run_sh(&format!("yum install -y {pkgs}")).await,
                _ => Err("Gestionnaire de paquets non détecté".into()),
            }
        }
        "pkg_remove" => {
            let pkgs = s(args, "packages");
            match detect_pkg_manager() {
                "apt" => run_sh(&format!("apt-get remove -y {pkgs}")).await,
                "dnf" => run_sh(&format!("dnf remove -y {pkgs}")).await,
                "yum" => run_sh(&format!("yum remove -y {pkgs}")).await,
                _ => Err("Gestionnaire de paquets non détecté".into()),
            }
        }
        "pkg_update" => {
            let upgrade = b(args, "upgrade", false);
            match detect_pkg_manager() {
                "apt" => {
                    let mut c = "apt-get update".to_string();
                    if upgrade { c.push_str(" && apt-get upgrade -y"); }
                    run_sh(&c).await
                }
                "dnf" => if upgrade { run_sh("dnf upgrade -y").await } else { run_sh("dnf check-update").await },
                "yum" => if upgrade { run_sh("yum update -y").await } else { run_sh("yum check-update").await },
                _ => Err("Gestionnaire de paquets non détecté".into()),
            }
        }
        "pkg_list" => {
            let filter = s(args, "filter");
            let base = match detect_pkg_manager() {
                "apt" => "dpkg -l",
                "dnf" | "yum" => "rpm -qa",
                _ => "echo 'Gestionnaire non détecté'",
            };
            if filter.is_empty() { run_sh(base).await }
            else { run_sh(&format!("{base} | grep -i '{filter}'")).await }
        }
        "pkg_info" => {
            let pkg = s(args, "package");
            match detect_pkg_manager() {
                "apt" => run_sh(&format!("apt-cache show {pkg} 2>/dev/null || dpkg -s {pkg}")).await,
                "dnf" => run_sh(&format!("dnf info {pkg}")).await,
                "yum" => run_sh(&format!("yum info {pkg}")).await,
                _ => Err("Non détecté".into()),
            }
        }

        // ── Services systemd ─────────────────────────────
        "svc_status"  => run("systemctl", &["status", &s(args, "service"), "--no-pager"]).await,
        "svc_start"   => run("systemctl", &["start", &s(args, "service")]).await,
        "svc_stop"    => run("systemctl", &["stop", &s(args, "service")]).await,
        "svc_restart" => run("systemctl", &["restart", &s(args, "service")]).await,
        "svc_enable"  => run("systemctl", &["enable", &s(args, "service")]).await,
        "svc_disable" => run("systemctl", &["disable", &s(args, "service")]).await,
        "svc_list" => {
            let filter = s(args, "filter");
            let f = if filter.is_empty() { "active" } else { &filter };
            match f {
                "active" => run_sh("systemctl list-units --type=service --state=running --no-pager").await,
                "failed" => run_sh("systemctl list-units --type=service --state=failed --no-pager").await,
                _ => run_sh("systemctl list-units --type=service --no-pager").await,
            }
        }
        "svc_logs" => {
            let svc = s(args, "service");
            let lines = i(args, "lines", 50);
            run_sh(&format!("journalctl -u {svc} --no-pager -n {lines}")).await
        }

        // ── Cron ─────────────────────────────────────────
        "cron_list" => {
            let user = if s(args, "user").is_empty() { "root".to_string() } else { s(args, "user") };
            run_sh(&format!("crontab -u {user} -l 2>/dev/null || echo 'Pas de crontab pour {user}'")).await
        }
        "cron_add" => {
            let user = if s(args, "user").is_empty() { "root".to_string() } else { s(args, "user") };
            let schedule = s(args, "schedule");
            let command = s(args, "command");
            let comment = s(args, "comment");
            let entry = if comment.is_empty() {
                format!("{schedule} {command}")
            } else {
                format!("# {comment}\n{schedule} {command}")
            };
            run_sh(&format!(
                "(crontab -u {user} -l 2>/dev/null; echo '{entry}') | crontab -u {user} -"
            )).await?;
            Ok(format!("Cron ajouté pour {user}: {schedule} {command}"))
        }
        "cron_remove" => {
            let user = if s(args, "user").is_empty() { "root".to_string() } else { s(args, "user") };
            let pattern = s(args, "pattern");
            run_sh(&format!(
                "crontab -u {user} -l 2>/dev/null | grep -v '{pattern}' | crontab -u {user} -"
            )).await?;
            Ok(format!("Entrées cron correspondant à '{pattern}' supprimées pour {user}"))
        }

        // ── Réseau ───────────────────────────────────────
        "net_interfaces" => run_sh("ip -br addr show 2>/dev/null || ifconfig").await,
        "net_ports" => run_sh("ss -tlnp 2>/dev/null || netstat -tlnp").await,
        "net_ping" => {
            let count = i(args, "count", 4);
            run_sh(&format!("ping -c {count} '{}'", s(args, "host"))).await
        }
        "net_dns" => {
            let host = s(args, "host");
            let dtype = s(args, "type");
            if dtype.is_empty() {
                run_sh(&format!("dig +short '{host}' 2>/dev/null || nslookup '{host}'")).await
            } else {
                run_sh(&format!("dig +short '{host}' {dtype} 2>/dev/null || nslookup -type={dtype} '{host}'")).await
            }
        }
        "net_curl" => {
            let url = s(args, "url");
            let method = if s(args, "method").is_empty() { "GET".to_string() } else { s(args, "method") };
            let data = s(args, "data");
            let headers = s(args, "headers");
            let mut cmd = format!("curl -s -X {method}");
            for h in headers.lines() {
                let h = h.trim();
                if !h.is_empty() { cmd.push_str(&format!(" -H '{h}'")); }
            }
            if !data.is_empty() { cmd.push_str(&format!(" -d '{data}'")); }
            cmd.push_str(&format!(" '{url}'"));
            run_sh(&cmd).await
        }
        "net_firewall" => {
            let action = s(args, "action");
            let port = s(args, "port");
            // Détecte le firewall disponible
            if Path::new("/usr/bin/ufw").exists() {
                match action.as_str() {
                    "list" => run_sh("ufw status verbose").await,
                    "allow" => run_sh(&format!("ufw allow {port}")).await,
                    "deny" => run_sh(&format!("ufw deny {port}")).await,
                    "delete" => run_sh(&format!("ufw delete allow {port}")).await,
                    _ => Err(format!("Action inconnue: {action}")),
                }
            } else if Path::new("/usr/bin/firewall-cmd").exists() {
                match action.as_str() {
                    "list" => run_sh("firewall-cmd --list-all").await,
                    "allow" => run_sh(&format!("firewall-cmd --permanent --add-port={port} && firewall-cmd --reload")).await,
                    "deny" => run_sh(&format!("firewall-cmd --permanent --remove-port={port} && firewall-cmd --reload")).await,
                    "delete" => run_sh(&format!("firewall-cmd --permanent --remove-port={port} && firewall-cmd --reload")).await,
                    _ => Err(format!("Action inconnue: {action}")),
                }
            } else {
                Err("Aucun firewall détecté (ufw/firewalld)".into())
            }
        }

        // ── Processus ────────────────────────────────────
        "proc_list" => {
            let filter = s(args, "filter");
            if filter.is_empty() {
                run_sh("ps aux --sort=-%mem | head -30").await
            } else {
                run_sh(&format!("ps aux | grep -i '{filter}' | grep -v grep")).await
            }
        }
        "proc_kill" => {
            let target = s(args, "target");
            let signal = if s(args, "signal").is_empty() { "TERM".to_string() } else { s(args, "signal") };
            // Essaie par PID d'abord
            if target.chars().all(|c| c.is_ascii_digit()) {
                run_sh(&format!("kill -{signal} {target}")).await
            } else {
                run_sh(&format!("pkill -{signal} '{target}'")).await
            }
        }

        // ── Système ──────────────────────────────────────
        "sys_info" => {
            run_sh(
                "echo '=== Système ===' && uname -a && echo '' && \
                 echo '=== Hostname ===' && hostname -f 2>/dev/null || hostname && echo '' && \
                 echo '=== Uptime ===' && uptime && echo '' && \
                 echo '=== RAM ===' && free -h && echo '' && \
                 echo '=== CPU ===' && nproc && lscpu | grep 'Model name' && echo '' && \
                 echo '=== Disque ===' && df -h / && echo '' && \
                 echo '=== OS ===' && cat /etc/os-release 2>/dev/null | head -5"
            ).await
        }
        "sys_env" => {
            let name = s(args, "name");
            if name.is_empty() {
                run_sh("env | sort").await
            } else {
                let value = s(args, "value");
                if value.is_empty() {
                    run_sh(&format!("echo ${name}")).await
                } else {
                    // Note: ceci ne persiste pas entre les appels
                    Ok(format!("Variable {name}={value} (non persistée — utilisez /etc/environment pour persister)"))
                }
            }
        }
        "sys_reboot" => {
            let delay = i(args, "delay", 0);
            if delay > 0 {
                run_sh(&format!("shutdown -r +{delay}")).await
            } else {
                run_sh("shutdown -r now").await
            }
        }

        // ── Utilisateurs ─────────────────────────────────
        "user_list" => run_sh("getent passwd | awk -F: '$3>=1000{print $1, $3, $6, $7}'").await,
        "user_add" => {
            let username = s(args, "username");
            let mut cmd = format!("useradd");
            let shell = s(args, "shell");
            if !shell.is_empty() { cmd.push_str(&format!(" -s '{shell}'")); } else { cmd.push_str(" -s /bin/bash"); }
            let home = s(args, "home");
            if !home.is_empty() { cmd.push_str(&format!(" -d '{home}'")); }
            let groups = s(args, "groups");
            if !groups.is_empty() { cmd.push_str(&format!(" -G '{groups}'")); }
            cmd.push_str(&format!(" -m '{username}'"));
            run_sh(&cmd).await?;
            Ok(format!("Utilisateur {username} créé"))
        }
        "user_delete" => {
            let username = s(args, "username");
            let flag = if b(args, "remove_home", false) { "-r" } else { "" };
            run_sh(&format!("userdel {flag} '{username}'")).await?;
            Ok(format!("Utilisateur {username} supprimé"))
        }
        "user_groups" => {
            let username = s(args, "username");
            let add = s(args, "add_groups");
            let remove = s(args, "remove_groups");
            let mut result = String::new();
            if !add.is_empty() {
                run_sh(&format!("usermod -aG '{add}' '{username}'")).await?;
                result.push_str(&format!("Groupes ajoutés: {add}\n"));
            }
            if !remove.is_empty() {
                run_sh(&format!("gpasswd -d '{username}' '{remove}'")).await?;
                result.push_str(&format!("Groupes retirés: {remove}\n"));
            }
            result.push_str(&run_sh(&format!("groups '{username}'")).await?);
            Ok(result)
        }

        _ => Err(format!("Outil inconnu: {name}")),
    }
}
