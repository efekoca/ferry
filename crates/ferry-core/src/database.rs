use crate::docker::{Container, Step};
use crate::toolchain;
use crate::version::PhpVersion;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use walkdir::WalkDir;

const READY_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const SSH_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const SEED_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const LOCAL_ADDRESS: &str = "127.0.0.1";
const ROOT_PASSWORD: &str = "ferry";
const SOCKET_DIR: &str = "/var/run/mysqld";
const DEFAULT_SQLITE: &str = "database/database.sqlite";
const SQLITE_DUMP: &str = ".ferry-dump.sql";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Mysql,
    Postgres,
    Sqlite,
}

impl Engine {
    fn image(self) -> &'static str {
        match self {
            Engine::Mysql => "mysql:8.0",
            Engine::Postgres => "postgres:16",
            Engine::Sqlite => "",
        }
    }

    fn port(self) -> u16 {
        match self {
            Engine::Mysql => 3306,
            Engine::Postgres => 5432,
            Engine::Sqlite => 0,
        }
    }

    pub fn extensions(self) -> [&'static str; 2] {
        match self {
            Engine::Mysql => ["mysqli", "pdo_mysql"],
            Engine::Postgres => ["pgsql", "pdo_pgsql"],
            Engine::Sqlite => ["sqlite3", "pdo_sqlite"],
        }
    }
}

#[derive(Clone)]
pub struct Connection {
    pub engine: Engine,
    pub host: String,
    pub port: u16,
    pub name: String,
    pub user: String,
    pub password: String,
}

impl Connection {
    fn fallback(engine: Engine) -> Self {
        Connection {
            engine,
            host: LOCAL_ADDRESS.into(),
            port: engine.port(),
            name: "ferry".into(),
            user: "ferry".into(),
            password: "ferry".into(),
        }
    }

    /// Where the file sits inside the containers.
    fn sqlite_path(&self) -> String {
        if self.name.starts_with('/') {
            format!("{}/{DEFAULT_SQLITE}", crate::docker::CONTAINER_ROOT)
        } else {
            format!("{}/{}", crate::docker::CONTAINER_ROOT, self.name)
        }
    }

    fn sqlite_copy(&self, project: &Path) -> PathBuf {
        project.join(self.sqlite_path().trim_start_matches(crate::docker::CONTAINER_ROOT).trim_start_matches('/'))
    }

    fn is_local(&self) -> bool {
        self.engine == Engine::Sqlite || ["localhost", "127.0.0.1", "::1", "host.docker.internal"].contains(&self.host.as_str())
    }
}

/// How a comparison gets its data. Every version imports the same data into its own empty database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Dump(PathBuf),
    Live,
    Seed,
    Ssh(Remote),
}

/// A database reached through the person's own `ssh`. Ferry never sees a key or stores a password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    pub host: String,
    pub engine: Engine,
    pub name: String,
    pub user: String,
}

impl Remote {
    fn check(&self) -> Result<(), String> {
        let safe = |text: &str, extra: &str| {
            !text.is_empty()
                && !text.starts_with('-')
                && text.chars().all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c) || extra.contains(c))
        };
        if safe(&self.host, "@") && safe(&self.name, "") && safe(&self.user, "") && self.engine != Engine::Sqlite {
            Ok(())
        } else {
            Err(crate::t!("Sunucu, veritabanı ya da kullanıcı adı geçersiz.", "The server, database or user name isn't valid."))
        }
    }

    /// Reads the password from stdin so it never appears in a process list.
    fn command(&self) -> String {
        let (user, name) = (&self.user, &self.name);
        match self.engine {
            Engine::Postgres => format!("IFS= read -r p; [ -n \"$p\" ] && export PGPASSWORD=\"$p\"; exec pg_dump -h 127.0.0.1 --no-owner --no-privileges -U {user} {name}"),
            _ => format!("IFS= read -r p; [ -n \"$p\" ] && export MYSQL_PWD=\"$p\"; exec mysqldump --single-transaction --quick --no-tablespaces -u {user} {name}"),
        }
    }
}

/// Host names from the person's SSH config, leaving out patterns.
pub fn ssh_hosts() -> Vec<String> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
    let text = std::fs::read_to_string(home.join(".ssh/config")).unwrap_or_default();
    config_hosts(&text)
}

fn config_hosts(text: &str) -> Vec<String> {
    let mut hosts: Vec<String> = text
        .lines()
        .filter_map(|l| l.trim().strip_prefix("Host ").or_else(|| l.trim().strip_prefix("host ")))
        .flat_map(str::split_whitespace)
        .filter(|h| !h.contains(['*', '?', '!']))
        .map(str::to_string)
        .collect();
    hosts.sort();
    hosts.dedup();
    hosts
}

fn dump_ssh(remote: &Remote, password: &str, file: &Path, cancel: &std::sync::atomic::AtomicBool) -> Result<(), String> {
    use std::io::{Read, Write};
    remote.check()?;
    let ssh = toolchain::find_executable("ssh").ok_or_else(|| crate::t!("ssh komutu bulunamadı.", "The ssh command wasn't found."))?;
    let out = std::fs::File::create(file).map_err(|e| e.to_string())?;
    let mut child = toolchain::command(&ssh)
        .args(["-C", "-o", "BatchMode=yes", "-o", "ConnectTimeout=20", "--", &remote.host, &remote.command()])
        .stdin(Stdio::piped())
        .stdout(out)
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = writeln!(stdin, "{password}");
    }
    let mut pipe = child.stderr.take();
    let errors = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(pipe) = pipe.as_mut() {
            let _ = pipe.read_to_string(&mut text);
        }
        text
    });
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            let reason = errors.join().unwrap_or_default().trim().to_string();
            let empty = std::fs::metadata(file).map_or(true, |m| m.len() == 0);
            if status.success() && !empty {
                return Ok(());
            }
            return Err(crate::t!("Sunucudan kopyalanamadı: {reason}", "Couldn't copy from the server: {reason}"));
        }
        if cancel.load(Ordering::Relaxed) || started.elapsed() > SSH_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(crate::t!("Sunucudan kopyalama durduruldu.", "Copying from the server was stopped."));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// What a project offers, without its password.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub engine: Option<Engine>,
    pub host: Option<String>,
    pub name: Option<String>,
    pub user: Option<String>,
    pub live: bool,
    pub seed: Option<&'static str>,
    pub dumps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataChange {
    pub table: String,
    pub source_rows: Option<u64>,
    pub target_rows: Option<u64>,
}

pub type Snapshot = BTreeMap<String, (u64, String)>;

fn unquoted(value: &str) -> String {
    value.trim().trim_matches(|c| c == '"' || c == '\'').to_string()
}

fn env_values(root: &Path) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for name in [".env.example", ".env", ".env.local"] {
        let Ok(text) = std::fs::read_to_string(root.join(name)) else { continue };
        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=').filter(|(k, _)| !k.trim_start().starts_with('#')) {
                values.insert(key.trim().to_string(), unquoted(value));
            }
        }
    }
    values
}

fn engine_named(name: &str) -> Option<Engine> {
    match name {
        "mysql" | "mariadb" | "mysqli" | "pdo_mysql" => Some(Engine::Mysql),
        "pgsql" | "postgres" | "postgresql" | "pdo_pgsql" => Some(Engine::Postgres),
        "sqlite" | "sqlite3" | "pdo_sqlite" => Some(Engine::Sqlite),
        _ => None,
    }
}

fn sqlite_at(path: &str) -> Connection {
    let name = path.trim_start_matches("%kernel.project_dir%/").to_string();
    Connection { engine: Engine::Sqlite, host: String::new(), port: 0, name, user: String::new(), password: String::new() }
}

fn from_url(url: &str) -> Option<Connection> {
    let (scheme, rest) = url.split_once("://")?;
    let engine = engine_named(scheme)?;
    if engine == Engine::Sqlite {
        return Some(sqlite_at(rest.strip_prefix('/').unwrap_or(rest)));
    }
    let (auth, rest) = rest.rsplit_once('@')?;
    let (user, password) = auth.split_once(':').unwrap_or((auth, ""));
    let (address, name) = rest.split_once('/')?;
    let (host, port) = address.split_once(':').unwrap_or((address, ""));
    Some(Connection {
        engine,
        host: host.to_string(),
        port: port.parse().unwrap_or(engine.port()),
        name: name.split('?').next().unwrap_or_default().to_string(),
        user: user.to_string(),
        password: password.to_string(),
    })
}

fn wordpress_value(text: &str, key: &str) -> Option<String> {
    let at = text.find(&format!("'{key}'")).or_else(|| text.find(&format!("\"{key}\"")))?;
    let rest = text[at + key.len() + 2..].trim_start().strip_prefix(',')?.trim_start();
    let quote = rest.chars().next().filter(|c| *c == '\'' || *c == '"')?;
    rest[1..].split_once(quote).map(|(value, _)| value.to_string())
}

pub fn detect(root: &Path) -> Option<Connection> {
    let values = env_values(root);
    if let Some(found) = values.get("DATABASE_URL").and_then(|u| from_url(u)) {
        return Some(found);
    }
    if let Some(engine) = values.get("DB_CONNECTION").and_then(|c| engine_named(c)) {
        if engine == Engine::Sqlite {
            return Some(sqlite_at(values.get("DB_DATABASE").filter(|v| !v.is_empty()).map_or(DEFAULT_SQLITE, |v| v.as_str())));
        }
        let value = |key: &str, default: &str| values.get(key).filter(|v| !v.is_empty()).cloned().unwrap_or_else(|| default.to_string());
        return Some(Connection {
            engine,
            host: value("DB_HOST", LOCAL_ADDRESS),
            port: value("DB_PORT", "").parse().unwrap_or(engine.port()),
            name: value("DB_DATABASE", "ferry"),
            user: value("DB_USERNAME", "ferry"),
            password: values.get("DB_PASSWORD").cloned().unwrap_or_default(),
        });
    }
    let config =
        std::fs::read_to_string(root.join("wp-config.php")).or_else(|_| std::fs::read_to_string(root.join("wp-config-sample.php"))).ok()?;
    let host = wordpress_value(&config, "DB_HOST")?;
    let (host, port) = host.split_once(':').unwrap_or((&host, ""));
    Some(Connection {
        engine: Engine::Mysql,
        host: host.to_string(),
        port: port.parse().unwrap_or(Engine::Mysql.port()),
        name: wordpress_value(&config, "DB_NAME")?,
        user: wordpress_value(&config, "DB_USER")?,
        password: wordpress_value(&config, "DB_PASSWORD").unwrap_or_default(),
    })
}

fn seed_commands(root: &Path) -> Option<(&'static str, &'static str)> {
    if root.join("artisan").is_file() {
        return Some(("laravel", "php artisan migrate --seed --force"));
    }
    let migrations = std::fs::read_dir(root.join("migrations"))
        .is_ok_and(|mut d| d.any(|e| e.is_ok_and(|e| e.path().extension().is_some_and(|x| x == "php"))));
    if root.join("bin/console").is_file() && migrations {
        let migrate = "php bin/console doctrine:migrations:migrate -n";
        let both = "php bin/console doctrine:migrations:migrate -n && php bin/console doctrine:fixtures:load -n";
        return Some(("symfony", if root.join("src/DataFixtures").is_dir() { both } else { migrate }));
    }
    if root.join("bin/console").is_file() && root.join("src/DataFixtures").is_dir() {
        return Some(("symfony", "php bin/console doctrine:schema:create -n && php bin/console doctrine:fixtures:load -n"));
    }
    None
}

pub fn plan(root: &Path) -> Plan {
    let found = detect(root);
    let skipped = |name: &str| name == "vendor" || crate::project::ALWAYS_SKIP.contains(&name);
    let dumps = WalkDir::new(root)
        .max_depth(4)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !skipped(&e.file_name().to_string_lossy()))
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| crate::analyzer::relative_path(root, e.path()))
        .filter(|p| p.ends_with(".sql") || p.ends_with(".sql.gz"))
        .collect();
    Plan {
        engine: found.as_ref().map(|c| c.engine),
        host: found.as_ref().map(|c| c.host.clone()),
        name: found.as_ref().map(|c| c.name.clone()),
        user: found.as_ref().map(|c| c.user.clone()),
        live: found.as_ref().is_some_and(Connection::is_local),
        seed: seed_commands(root).map(|(name, _)| name),
        dumps,
    }
}

fn set_lines(text: &str, values: &[(&str, String)]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        let key = line.split('=').next().unwrap_or_default().trim();
        if !values.iter().any(|(k, _)| *k == key) {
            lines.push(line.to_string());
        }
    }
    lines.extend(values.iter().map(|(k, v)| format!("{k}=\"{v}\"")));
    lines.join("\n") + "\n"
}

fn replace_define(text: &str, key: &str, value: &str) -> String {
    let Some(at) = text.find(&format!("'{key}'")).or_else(|| text.find(&format!("\"{key}\""))) else { return text.to_string() };
    let Some(end) = text[at..].find(';') else { return text.to_string() };
    format!("{}'{key}', '{value}');{}", &text[..at], &text[at + end + 1..])
}

fn wordpress_config(text: &str, connection: &Connection) -> String {
    let mut text = replace_define(text, "DB_HOST", &format!("{LOCAL_ADDRESS}:{}", connection.port));
    if let Some(at) = text.find("<?php") {
        let site = "\ndefine('WP_HOME', 'http://127.0.0.1:8080');\ndefine('WP_SITEURL', 'http://127.0.0.1:8080');\n";
        text.insert_str(at + 5, site);
    }
    text
}

/// Points the temporary copy at the database next to it. The project itself isn't touched.
fn point_copy_at(project: &Path, connection: &Connection) -> std::io::Result<()> {
    let wordpress = project.join("wp-config.php");
    let sample = project.join("wp-config-sample.php");
    if wordpress.is_file() || sample.is_file() {
        let text = std::fs::read_to_string(if wordpress.is_file() { &wordpress } else { &sample })?;
        std::fs::write(&wordpress, wordpress_config(&text, connection))?;
    }
    let example = project.join(".env.example");
    let env = project.join(".env");
    if env.is_file() || example.is_file() {
        let text = std::fs::read_to_string(if env.is_file() { &env } else { &example })?;
        let key = "base64:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string();
        let url = match connection.engine {
            Engine::Sqlite => format!("sqlite:///{}", connection.sqlite_path()),
            engine => {
                let scheme = if engine == Engine::Mysql { "mysql" } else { "postgresql" };
                format!("{scheme}://{}:{}@{LOCAL_ADDRESS}:{}/{}", connection.user, connection.password, connection.port, connection.name)
            }
        };
        let mut values = match connection.engine {
            Engine::Sqlite => vec![("DB_DATABASE", connection.sqlite_path())],
            _ => vec![("DB_HOST", LOCAL_ADDRESS.to_string()), ("DB_PORT", connection.port.to_string())],
        };
        if text.contains("DATABASE_URL") {
            values.push(("DATABASE_URL", url));
        }
        if text.contains("APP_KEY") && env_values(project).get("APP_KEY").is_none_or(String::is_empty) {
            values.push(("APP_KEY", key));
        }
        std::fs::write(&env, set_lines(&text, &values))?;
    }
    Ok(())
}

/// Code that says `localhost` reaches MySQL through this socket, shared with the app's container.
pub(crate) fn socket_mount(running: &Running, connection: &Connection) -> Option<String> {
    (connection.engine == Engine::Mysql).then(|| format!("{}:{SOCKET_DIR}", running.socket))
}

pub(crate) fn environment(connection: &Connection) -> Vec<(String, String)> {
    if connection.engine == Engine::Sqlite {
        return vec![("DB_CONNECTION".to_string(), "sqlite".to_string()), ("DB_DATABASE".to_string(), connection.sqlite_path())];
    }
    let pairs = [
        ("PHP_INI_SCAN_DIR", format!(":{SOCKET_DIR}")),
        ("DB_HOST", LOCAL_ADDRESS.to_string()),
        ("DB_PORT", connection.port.to_string()),
        ("DB_DATABASE", connection.name.clone()),
        ("DB_NAME", connection.name.clone()),
        ("DB_USERNAME", connection.user.clone()),
        ("DB_USER", connection.user.clone()),
        ("DB_PASSWORD", connection.password.clone()),
    ];
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

pub(crate) struct Running<'a> {
    docker: &'a Path,
    pub name: String,
    pub socket: String,
    connection: Connection,
}

impl Drop for Running<'_> {
    fn drop(&mut self) {
        let _ = toolchain::command(self.docker).args(["rm", "-f", &self.name]).stderr(Stdio::null()).stdout(Stdio::null()).status();
        let _ =
            toolchain::command(self.docker).args(["volume", "rm", "-f", &self.socket]).stderr(Stdio::null()).stdout(Stdio::null()).status();
    }
}

impl Running<'_> {
    fn exec(&self, script: &str) -> Option<String> {
        let done = toolchain::command(self.docker).args(["exec", &self.name, "sh", "-c", script]).output().ok()?;
        done.status.success().then(|| String::from_utf8_lossy(&done.stdout).to_string())
    }

    fn client(&self) -> String {
        match self.connection.engine {
            Engine::Mysql => format!(
                "export MYSQL_PWD=\"$MYSQL_ROOT_PASSWORD\"; c='mysql --protocol=tcp -h127.0.0.1 -P{} -uroot -N'; ",
                self.connection.port
            ),
            Engine::Postgres | Engine::Sqlite => format!(
                "export PGPASSWORD=\"$POSTGRES_PASSWORD\"; c='psql -h 127.0.0.1 -p {} -U {} -d {} -At'; ",
                self.connection.port, self.connection.user, self.connection.name
            ),
        }
    }

    fn ready(&self, cancel: &std::sync::atomic::AtomicBool) -> Result<(), String> {
        let check = match self.connection.engine {
            Engine::Mysql => format!("{}$c -e 'select 1'", self.client()),
            Engine::Postgres | Engine::Sqlite => format!("pg_isready -h 127.0.0.1 -p {}", self.connection.port),
        };
        let started = Instant::now();
        while started.elapsed() < READY_TIMEOUT {
            if cancel.load(Ordering::Relaxed) {
                return Err(crate::t!("Karşılaştırma durduruldu.", "The comparison was stopped."));
            }
            if self.exec(&check).is_some() {
                return Ok(());
            }
            let state = toolchain::command(self.docker).args(["inspect", "-f", "{{.State.Running}}", &self.name]).output();
            if state.is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "false") {
                let logs = toolchain::command(self.docker).args(["logs", "--tail", "12", &self.name]).output();
                let text = logs.map(|o| String::from_utf8_lossy(&o.stderr).to_string()).unwrap_or_default();
                let reason = text.trim().to_string();
                return Err(crate::t!(
                    "Veritabanı başlatılamadı ya da veri içe aktarılamadı: {reason}",
                    "The database couldn't start or import the data: {reason}"
                ));
            }
            std::thread::sleep(Duration::from_millis(800));
        }
        Err(crate::t!("Veritabanı zamanında hazır olmadı.", "The database wasn't ready in time."))
    }

    pub fn snapshot(&self) -> Snapshot {
        let name = &self.connection.name;
        let script = match self.connection.engine {
            Engine::Mysql => format!(
                "{}$c -e \"select table_name from information_schema.tables where table_schema='{name}' and table_type='BASE TABLE'\" | while read t; do \
                 r=$($c {name} -e \"select count(*) from \\`$t\\`\"); s=$($c {name} -e \"checksum table \\`$t\\`\" | cut -f2); echo \"$t|$r|$s\"; done",
                self.client()
            ),
            Engine::Postgres | Engine::Sqlite => format!(
                "{}$c -c \"select tablename from pg_tables where schemaname='public'\" | while read t; do \
                 r=$($c -c \"select count(*) from \\\"$t\\\"\"); s=$($c -c \"select md5(coalesce(string_agg(x::text, '|' order by x::text), '')) from \\\"$t\\\" x\"); echo \"$t|$r|$s\"; done",
                self.client()
            ),
        };
        parse_snapshot(&self.exec(&script).unwrap_or_default())
    }

    fn dump_to(&self, file: &Path) -> Result<(), String> {
        let script = match self.connection.engine {
            Engine::Mysql => format!(
                "export MYSQL_PWD=\"$MYSQL_ROOT_PASSWORD\"; mysqldump --protocol=tcp -h127.0.0.1 -P{} -uroot --single-transaction --no-tablespaces {}",
                self.connection.port, self.connection.name
            ),
            Engine::Postgres | Engine::Sqlite => {
                format!("export PGPASSWORD=\"$POSTGRES_PASSWORD\"; pg_dump -h 127.0.0.1 -p {} -U {} {}", self.connection.port, self.connection.user, self.connection.name)
            }
        };
        let out = std::fs::File::create(file).map_err(|e| e.to_string())?;
        let done =
            toolchain::command(self.docker).args(["exec", &self.name, "sh", "-c", &script]).stdout(out).stderr(Stdio::null()).status();
        if done.is_ok_and(|s| s.success()) {
            Ok(())
        } else {
            Err(crate::t!("Test verisi dışa aktarılamadı.", "The test data couldn't be exported."))
        }
    }
}

fn parse_snapshot(text: &str) -> Snapshot {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '|');
            let (table, rows, sum) = (parts.next()?, parts.next()?, parts.next()?);
            Some((table.to_string(), (rows.trim().parse().ok()?, sum.trim().to_string())))
        })
        .collect()
}

pub fn data_changes(source: &Snapshot, target: &Snapshot) -> Vec<DataChange> {
    let mut tables: Vec<&String> = source.keys().chain(target.keys()).collect();
    tables.sort();
    tables.dedup();
    tables
        .into_iter()
        .filter(|t| source.get(*t) != target.get(*t))
        .map(|t| DataChange { table: t.clone(), source_rows: source.get(t).map(|r| r.0), target_rows: target.get(t).map(|r| r.0) })
        .collect()
}

pub(crate) fn start<'a>(
    container: &Container<'a>,
    suffix: &str,
    connection: &Connection,
    dump: Option<&Path>,
) -> Result<Running<'a>, String> {
    let name = format!("{}-db-{suffix}", container.prefix);
    let socket = format!("{name}-socket");
    let engine = connection.engine;
    let mut cmd = toolchain::command(container.docker);
    cmd.args(["run", "-d", "--name", &name, "--network", "none", "--memory", "2g"]);
    let mut pass = |key: &str, value: &str| {
        cmd.arg("-e").arg(key).env(key, value);
    };
    match engine {
        Engine::Mysql => {
            let root = connection.user == "root";
            pass("MYSQL_ROOT_PASSWORD", if root { &connection.password } else { ROOT_PASSWORD });
            pass("MYSQL_ALLOW_EMPTY_PASSWORD", if root && connection.password.is_empty() { "1" } else { "" });
            pass("MYSQL_DATABASE", &connection.name);
            pass("MYSQL_TCP_PORT", &connection.port.to_string());
            if !root {
                pass("MYSQL_USER", &connection.user);
                pass("MYSQL_PASSWORD", &connection.password.replace('\\', "\\\\").replace('\'', "''"));
            }
            cmd.args(["--tmpfs", "/var/lib/mysql:uid=999,gid=999"]);
            cmd.arg("-v").arg(format!("{socket}:{SOCKET_DIR}"));
        }
        Engine::Postgres | Engine::Sqlite => {
            pass("POSTGRES_USER", &connection.user);
            pass("POSTGRES_PASSWORD", &connection.password);
            pass("POSTGRES_DB", &connection.name);
            pass("PGPORT", &connection.port.to_string());
            cmd.args(["--tmpfs", "/var/lib/postgresql/data:uid=999,gid=999"]);
        }
    }
    if let Some(dir) = dump {
        cmd.arg("-v").arg(format!("{}:/docker-entrypoint-initdb.d:ro", dir.display()));
    }
    cmd.arg(engine.image());
    if engine == Engine::Mysql {
        cmd.args(["--default-authentication-plugin=mysql_native_password", "--character-set-server=utf8mb4", "--mysqlx=OFF"]);
    }
    let done = cmd.output().map_err(|e| e.to_string())?;
    let running = Running { docker: container.docker, name, socket, connection: connection.clone() };
    if !done.status.success() {
        let reason = String::from_utf8_lossy(&done.stderr).trim().to_string();
        return Err(crate::t!("Veritabanı başlatılamadı: {reason}", "The database couldn't start: {reason}"));
    }
    running.ready(container.cancel)?;
    if engine == Engine::Mysql {
        running.exec(&format!("printf 'mysqli.default_socket={SOCKET_DIR}/mysqld.sock\\npdo_mysql.default_socket={SOCKET_DIR}/mysqld.sock\\nmysql.default_socket={SOCKET_DIR}/mysqld.sock\\n' > {SOCKET_DIR}/ferry.ini; chmod 755 {SOCKET_DIR}"));
    }
    Ok(running)
}

fn dump_live(docker: &Path, connection: &Connection, file: &Path) -> Result<(), String> {
    if !connection.is_local() {
        let host = &connection.host;
        return Err(crate::t!(
            "{host} bu bilgisayarda değil; canlı kopya yalnızca yerel veritabanlarında çalışır.",
            "{host} isn't on this computer, so a live copy only works with local databases."
        ));
    }
    let out = std::fs::File::create(file).map_err(|e| e.to_string())?;
    let mut cmd = toolchain::command(docker);
    cmd.args(["run", "--rm", "--add-host=host.docker.internal:host-gateway"]);
    let (port, user, name) = (connection.port.to_string(), &connection.user, &connection.name);
    match connection.engine {
        Engine::Mysql => {
            cmd.args(["-e", "MYSQL_PWD"]).env("MYSQL_PWD", &connection.password).arg(Engine::Mysql.image());
            cmd.args(["mysqldump", "-hhost.docker.internal", &format!("-P{port}"), &format!("-u{user}")]);
            cmd.args(["--single-transaction", "--no-tablespaces", "--column-statistics=0", "--set-gtid-purged=OFF", name]);
        }
        Engine::Postgres | Engine::Sqlite => {
            cmd.args(["-e", "PGPASSWORD"]).env("PGPASSWORD", &connection.password).arg(Engine::Postgres.image());
            cmd.args(["pg_dump", "-h", "host.docker.internal", "-p", &port, "-U", user, name]);
        }
    }
    let done = cmd.stdout(out).stderr(Stdio::piped()).output().map_err(|e| e.to_string())?;
    if done.status.success() {
        Ok(())
    } else {
        let reason = String::from_utf8_lossy(&done.stderr).trim().to_string();
        Err(crate::t!("Veritabanı kopyalanamadı: {reason}", "Couldn't copy the database: {reason}"))
    }
}

fn dump_engine(file: &Path) -> Engine {
    let head = std::fs::read(file).map(|b| String::from_utf8_lossy(&b[..b.len().min(8192)]).to_string()).unwrap_or_default();
    if head.contains("PostgreSQL") || head.contains("pg_dump") {
        Engine::Postgres
    } else if head.contains("sqlite_sequence") || head.contains("PRAGMA foreign_keys") {
        Engine::Sqlite
    } else {
        Engine::Mysql
    }
}

/// The engine a run will use, known before the images are prepared.
pub fn engine_for(root: &Path, source: &Source) -> Engine {
    match (detect(root), source) {
        (Some(connection), _) => connection.engine,
        (_, Source::Ssh(remote)) => remote.engine,
        (None, Source::Dump(file)) => dump_engine(file),
        (None, _) => Engine::Mysql,
    }
}

const SQLITE_SNAPSHOT: &str = r#"<?php
$db = new PDO('sqlite:' . $argv[1]);
foreach ($db->query("select name from sqlite_master where type = 'table' and name not like 'sqlite_%'")->fetchAll(PDO::FETCH_NUM) as $table) {
    $rows = $db->query('select * from "' . $table[0] . '"')->fetchAll(PDO::FETCH_NUM);
    sort($rows);
    echo $table[0], '|', count($rows), '|', md5(serialize($rows)), "\n";
}
"#;

const SQLITE_IMPORT: &str = r#"<?php
$text = file_get_contents($argv[2]);
$db = new PDO('sqlite:' . $argv[1]);
$db->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);
$db->exec(substr($text, 0, 2) === "\x1f\x8b" ? gzdecode($text) : $text);
"#;

fn sqlite_script(php: &str, args: &str) -> String {
    format!("cat > /tmp/ferry.php <<'FERRYEOF'\n{php}\nFERRYEOF\nphp /tmp/ferry.php {args}\n")
}

pub(crate) fn sqlite_snapshot(
    container: &Container,
    version: PhpVersion,
    project: &Path,
    image: &str,
    connection: &Connection,
) -> Snapshot {
    let script = sqlite_script(SQLITE_SNAPSHOT, &connection.sqlite_path());
    let name = format!("data-{}", version.compact());
    let run = container.run(&name, project, image, false, &script, Duration::from_secs(120));
    run.map(|done| parse_snapshot(&crate::docker::output(&done))).unwrap_or_default()
}

pub(crate) struct Prepared {
    pub connection: Connection,
    pub dump: Option<PathBuf>,
}

/// Finds the connection and gathers the data once, so both versions start from the same rows.
#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare(
    container: &Container,
    root: &Path,
    project: &Path,
    source: &Source,
    dir: &Path,
    seed_image: &dyn Fn() -> Result<String, String>,
    version: PhpVersion,
    step: &dyn Fn(Step),
) -> Result<Prepared, String> {
    let failed = |e: std::io::Error| e.to_string();
    std::fs::create_dir_all(dir).map_err(failed)?;
    let detected = detect(project);
    let sqlite = detected.as_ref().is_some_and(|c| c.engine == Engine::Sqlite);
    match source {
        Source::Dump(file) if sqlite || dump_engine(file) == Engine::Sqlite => {
            let connection = detected.unwrap_or_else(|| sqlite_at(DEFAULT_SQLITE));
            let copy = connection.sqlite_copy(project);
            std::fs::create_dir_all(copy.parent().unwrap_or(project)).map_err(failed)?;
            let _ = std::fs::remove_file(&copy);
            std::fs::copy(file, project.join(SQLITE_DUMP)).map_err(failed)?;
            point_copy_at(project, &connection).map_err(failed)?;
            step(Step::Database);
            let image = seed_image()?;
            let script = sqlite_script(SQLITE_IMPORT, &format!("{} /app/{SQLITE_DUMP}", connection.sqlite_path()));
            let run = container.run(&format!("import-{}", version.compact()), project, &image, false, &script, SEED_TIMEOUT)?;
            let _ = std::fs::remove_file(project.join(SQLITE_DUMP));
            if !run.status.is_some_and(|s| s.success()) {
                let reason = crate::docker::tail(&crate::docker::output(&run));
                return Err(crate::t!("Döküm içe aktarılamadı: {reason}", "The dump couldn't be imported: {reason}"));
            }
            Ok(Prepared { connection, dump: None })
        }
        Source::Dump(file) => {
            let gz = file.to_string_lossy().ends_with(".gz");
            let connection = detected.unwrap_or_else(|| Connection::fallback(dump_engine(file)));
            std::fs::copy(file, dir.join(if gz { "dump.sql.gz" } else { "dump.sql" })).map_err(failed)?;
            point_copy_at(project, &connection).map_err(failed)?;
            Ok(Prepared { connection, dump: Some(dir.to_path_buf()) })
        }
        Source::Live if sqlite => {
            let connection = detected.ok_or_else(String::new)?;
            let copy = connection.sqlite_copy(project);
            let original = if connection.name.starts_with('/') { PathBuf::from(&connection.name) } else { root.join(&connection.name) };
            if !original.is_file() {
                let shown = original.display().to_string();
                return Err(crate::t!("SQLite dosyası bulunamadı: {shown}", "The SQLite file wasn't found: {shown}"));
            }
            std::fs::create_dir_all(copy.parent().unwrap_or(project)).map_err(failed)?;
            std::fs::copy(original, &copy).map_err(failed)?;
            point_copy_at(project, &connection).map_err(failed)?;
            Ok(Prepared { connection, dump: None })
        }
        Source::Live => {
            let connection = detected
                .ok_or_else(|| crate::t!("Projede veritabanı ayarı bulunamadı.", "No database settings were found in the project."))?;
            step(Step::Database);
            dump_live(container.docker, &connection, &dir.join("dump.sql"))?;
            point_copy_at(project, &connection).map_err(failed)?;
            Ok(Prepared { connection, dump: Some(dir.to_path_buf()) })
        }
        Source::Ssh(remote) => {
            let known = detected.filter(|c| c.engine == remote.engine);
            let password = known.as_ref().filter(|c| c.user == remote.user).map_or(String::new(), |c| c.password.clone());
            let connection = known.unwrap_or_else(|| Connection {
                name: remote.name.clone(),
                user: remote.user.clone(),
                ..Connection::fallback(remote.engine)
            });
            step(Step::Database);
            dump_ssh(remote, &password, &dir.join("dump.sql"), container.cancel)?;
            point_copy_at(project, &connection).map_err(failed)?;
            Ok(Prepared { connection, dump: Some(dir.to_path_buf()) })
        }
        Source::Seed => {
            let (_, command) = seed_commands(project)
                .ok_or_else(|| crate::t!("Bu projede migration ya da seeder bulunamadı.", "This project has no migrations or seeders."))?;
            let connection = detected.unwrap_or_else(|| Connection::fallback(Engine::Mysql));
            point_copy_at(project, &connection).map_err(failed)?;
            step(Step::Database);
            let image = seed_image()?;
            if connection.engine == Engine::Sqlite {
                let copy = connection.sqlite_copy(project);
                std::fs::create_dir_all(copy.parent().unwrap_or(project)).map_err(failed)?;
                std::fs::write(&copy, "").map_err(failed)?;
                container.attach(None, None, environment(&connection));
                let run = container.run(&format!("seed-{}", version.compact()), project, &image, false, command, SEED_TIMEOUT);
                container.detach();
                let run = run?;
                if !run.status.is_some_and(|s| s.success()) {
                    let reason = crate::docker::tail(&crate::docker::output(&run));
                    return Err(crate::t!("Test verisi oluşturulamadı: {reason}", "The test data couldn't be created: {reason}"));
                }
                return Ok(Prepared { connection, dump: None });
            }
            let database = start(container, "seed", &connection, None)?;
            container.attach(Some(&database.name), socket_mount(&database, &connection), environment(&connection));
            let run = container.run(&format!("seed-{}", version.compact()), project, &image, false, command, SEED_TIMEOUT);
            container.detach();
            let run = run?;
            if !run.status.is_some_and(|s| s.success()) {
                let reason = crate::docker::tail(&crate::docker::output(&run));
                return Err(crate::t!("Test verisi oluşturulamadı: {reason}", "The test data couldn't be created: {reason}"));
            }
            database.dump_to(&dir.join("dump.sql"))?;
            Ok(Prepared { connection, dump: Some(dir.to_path_buf()) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laravel_settings_are_read_from_the_env_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".env.example"), "DB_CONNECTION=sqlite\n").unwrap();
        std::fs::write(
            dir.path().join(".env"),
            "DB_CONNECTION=mysql\nDB_HOST=localhost\nDB_DATABASE=\"shop\"\nDB_USERNAME=app\nDB_PASSWORD='s3cret'\n# DB_PORT=1\n",
        )
        .unwrap();
        let found = detect(dir.path()).unwrap();
        assert_eq!((found.engine, found.host.as_str(), found.port, found.name.as_str()), (Engine::Mysql, "localhost", 3306, "shop"));
        assert_eq!((found.user.as_str(), found.password.as_str()), ("app", "s3cret"));
        assert!(found.is_local());
    }

    #[test]
    fn urls_and_wordpress_configs_are_understood() {
        let url = from_url("postgresql://u:p%40ss@db.example.com:5433/app?serverVersion=16").unwrap();
        assert_eq!((url.engine, url.host.as_str(), url.port, url.name.as_str()), (Engine::Postgres, "db.example.com", 5433, "app"));
        assert!(!url.is_local());
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("wp-config.php"), "<?php\ndefine( 'DB_NAME', 'wp' );\ndefine('DB_USER', \"root\");\ndefine( 'DB_PASSWORD', '' );\ndefine( 'DB_HOST', 'localhost:3307' );\n").unwrap();
        let found = detect(dir.path()).unwrap();
        assert_eq!((found.name.as_str(), found.user.as_str(), found.host.as_str(), found.port), ("wp", "root", "localhost", 3307));
    }

    #[test]
    fn the_copy_points_at_the_local_database() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".env.example"), "APP_KEY=\nDB_CONNECTION=mysql\nDB_HOST=db.internal\nDB_PORT=3306\n").unwrap();
        std::fs::write(dir.path().join("wp-config.php"), "<?php\ndefine( 'DB_HOST', 'localhost' );\n").unwrap();
        let connection = Connection { port: 3307, ..Connection::fallback(Engine::Mysql) };
        point_copy_at(dir.path(), &connection).unwrap();
        let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
        assert!(env.contains("DB_HOST=\"127.0.0.1\"") && env.contains("DB_PORT=\"3307\"") && env.contains("APP_KEY=\"base64:"), "{env}");
        assert!(!env.contains("db.internal"));
        let config = std::fs::read_to_string(dir.path().join("wp-config.php")).unwrap();
        assert!(config.contains("'DB_HOST', '127.0.0.1:3307'") && config.contains("WP_HOME"), "{config}");
    }

    #[test]
    fn sqlite_settings_are_understood() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".env"), "DB_CONNECTION=sqlite\n").unwrap();
        let found = detect(dir.path()).unwrap();
        assert_eq!(
            (found.engine, found.name.as_str(), found.sqlite_path().as_str()),
            (Engine::Sqlite, DEFAULT_SQLITE, "/app/database/database.sqlite")
        );
        assert!(found.is_local());
        let symfony = from_url("sqlite:///%kernel.project_dir%/data/database.sqlite").unwrap();
        assert_eq!((symfony.engine, symfony.name.as_str()), (Engine::Sqlite, "data/database.sqlite"));
        assert_eq!(sqlite_at("/home/me/app.sqlite").sqlite_path(), "/app/database/database.sqlite");
        point_copy_at(dir.path(), &found).unwrap();
        let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
        assert!(env.contains("DB_DATABASE=\"/app/database/database.sqlite\""), "{env}");
    }

    #[test]
    fn remote_names_cannot_inject_commands() {
        let remote =
            |host: &str, name: &str, user: &str| Remote { host: host.into(), engine: Engine::Mysql, name: name.into(), user: user.into() };
        assert!(remote("prod", "shop_db", "app").check().is_ok());
        assert!(remote("deploy@prod.example.com", "shop", "app").check().is_ok());
        assert!(remote("-oProxyCommand=x", "shop", "app").check().is_err());
        assert!(remote("prod", "shop; rm -rf /", "app").check().is_err());
        assert!(remote("prod", "shop", "$(id)").check().is_err());
        assert!(remote("prod", "", "app").check().is_err());
        let sqlite = Remote { engine: Engine::Sqlite, ..remote("prod", "shop", "app") };
        assert!(sqlite.check().is_err());
        let command = remote("prod", "shop", "app").command();
        assert!(command.contains("mysqldump") && !command.contains("--password"), "{command}");
    }

    #[test]
    fn ssh_config_hosts_skip_patterns() {
        let text = "Host *\n  AddKeysToAgent yes\nHost prod staging\n  User a\n  Host nested\nhost db-?\nMatch all\n";
        assert_eq!(config_hosts(text), ["nested", "prod", "staging"]);
    }

    #[test]
    fn changed_tables_are_listed() {
        let source = parse_snapshot("users|2|aa\nposts|1|bb\nbad line\n");
        let target = parse_snapshot("users|2|aa\nposts|2|cc\nlogs|1|dd\n");
        let changes = data_changes(&source, &target);
        let names: Vec<_> = changes.iter().map(|c| (c.table.as_str(), c.source_rows, c.target_rows)).collect();
        assert_eq!(names, [("logs", None, Some(1)), ("posts", Some(1), Some(2))]);
    }
}
