//! Транспорт HTTP: соединение, TLS и разбор ответа.
//!
//! Выделен из [`super::http`], потому что «как достучаться до сервера» и
//! «какие функции видит программа» — разные вопросы, и первый заметно сложнее.
//!
//! Главное здесь — поддержка HTTPS. Раньше её не было, и на любой адрес
//! `https://` библиотека отвечала отказом: в вебе, где обычный `http://`
//! почти не встречается, это означало, что пользоваться ею нельзя.
//! Шифрование даёт `rustls` с корневыми сертификатами `webpki-roots` —
//! обе зависимости уже были в проекте и до сих пор ни разу не использовались.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use once_cell::sync::Lazy;

/// Сколько ждать ответа сервера.
const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// Сколько ждать отправки запроса.
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
/// Предел переходов по перенаправлениям.
///
/// Цепочка длиннее почти всегда означает не «сервер так устроен», а петлю.
const MAX_REDIRECTS: usize = 5;
/// Предел размера ответа — 32 МиБ.
///
/// Без него ошибочный адрес (поток видео, огромный файл) съел бы всю память
/// учебной машины молча.
const MAX_BODY: usize = 32 * 1024 * 1024;

/// Разобранный адрес.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Url {
    pub host: String,
    pub port: u16,
    pub path: String,
    pub https: bool,
}

impl Url {
    /// Собирает адрес обратно — нужно для сообщений и перенаправлений.
    fn origin(&self) -> String {
        let scheme = if self.https { "https" } else { "http" };
        let default_port = if self.https { 443 } else { 80 };
        if self.port == default_port {
            format!("{scheme}://{}", self.host)
        } else {
            format!("{scheme}://{}:{}", self.host, self.port)
        }
    }
}

/// Разбирает адрес вида `https://узел:порт/путь`.
///
/// Схема необязательна: `example.org/page` понимается как `http://`.
pub(crate) fn parse_url(url: &str) -> Result<Url, String> {
    let trimmed = url.trim();
    let (https, rest) = if let Some(rest) = trimmed.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        (false, rest)
    } else {
        (false, trimmed)
    };

    if rest.is_empty() {
        return Err("Адрес пуст".to_string());
    }

    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };

    // Порт отделяется последним двоеточием, но только если после него цифры:
    // иначе адрес IPv6 распался бы по своим же двоеточиям.
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => {
            let port = p
                .parse::<u16>()
                .map_err(|_| format!("Неверный порт в адресе: {url}"))?;
            (h.to_string(), port)
        }
        _ => (authority.to_string(), if https { 443 } else { 80 }),
    };

    if host.is_empty() {
        return Err(format!("В адресе не указан узел: {url}"));
    }

    Ok(Url {
        host,
        port,
        path: path.to_string(),
        https,
    })
}

// ---------------------------------------------------------------------------
//                                СОЕДИНЕНИЕ
// ---------------------------------------------------------------------------

/// Настройки TLS готовятся один раз: разбор корневых сертификатов —
/// заметная работа, повторять её на каждый запрос незачем.
static TLS_CONFIG: Lazy<Arc<rustls::ClientConfig>> = Lazy::new(|| {
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    Arc::new(
        rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
});

/// Соединение — обычное или зашифрованное.
enum Connection {
    Plain(TcpStream),
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>),
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            Self::Tls(s) => s.read(buf),
        }
    }
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(buf),
            Self::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
    }
}

fn connect(url: &Url) -> Result<Connection, String> {
    let address = format!("{}:{}", url.host, url.port);
    let tcp = TcpStream::connect(&address)
        .map_err(|e| format!("Не удалось подключиться к {address}: {e}"))?;
    tcp.set_read_timeout(Some(READ_TIMEOUT))
        .and_then(|()| tcp.set_write_timeout(Some(WRITE_TIMEOUT)))
        .map_err(|e| format!("Не удалось задать таймаут: {e}"))?;

    if !url.https {
        return Ok(Connection::Plain(tcp));
    }

    let server_name = rustls::pki_types::ServerName::try_from(url.host.clone())
        .map_err(|_| format!("Имя узла не годится для TLS: {}", url.host))?;
    let session = rustls::ClientConnection::new(Arc::clone(&TLS_CONFIG), server_name)
        .map_err(|e| format!("Не удалось установить защищённое соединение: {e}"))?;

    Ok(Connection::Tls(Box::new(rustls::StreamOwned::new(
        session, tcp,
    ))))
}

// ---------------------------------------------------------------------------
//                                  ОТВЕТ
// ---------------------------------------------------------------------------

/// Ответ сервера.
#[derive(Debug, Clone)]
pub(crate) struct Response {
    pub status: u16,
    pub reason: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Response {
    /// Значение заголовка без учёта регистра имени.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn is_redirect(&self) -> bool {
        matches!(self.status, 301 | 302 | 303 | 307 | 308)
    }
}

/// Выполняет запрос, переходя по перенаправлениям.
pub(crate) fn request(
    method: &str,
    url: &str,
    body: Option<&str>,
    headers: &[(String, String)],
) -> Result<Response, String> {
    let mut target = parse_url(url)?;
    let mut method = method.to_string();
    let mut body = body.map(str::to_string);

    for _ in 0..=MAX_REDIRECTS {
        let response = request_once(&method, &target, body.as_deref(), headers)?;
        if !response.is_redirect() {
            return Ok(response);
        }

        let Some(location) = response.header("Location") else {
            // Перенаправление без адреса — отвечать нечем; отдаём как есть.
            return Ok(response);
        };
        target = resolve_redirect(&target, location)?;

        // 303 и «переход после POST» по обычаю превращаются в GET: тело
        // относилось к прежнему адресу, и посылать его повторно неверно.
        if response.status == 303 || (response.status == 302 && method != "GET") {
            method = "GET".to_string();
            body = None;
        }
    }

    Err(format!(
        "Слишком много перенаправлений (больше {MAX_REDIRECTS}) при запросе {url}"
    ))
}

/// Разрешает адрес перенаправления относительно текущего.
fn resolve_redirect(current: &Url, location: &str) -> Result<Url, String> {
    let location = location.trim();
    if location.starts_with("http://") || location.starts_with("https://") {
        return parse_url(location);
    }
    if let Some(path) = location.strip_prefix('/') {
        return parse_url(&format!("{}/{}", current.origin(), path));
    }
    // Относительный путь считается от каталога текущего.
    let base = match current.path.rfind('/') {
        Some(idx) => &current.path[..idx],
        None => "",
    };
    parse_url(&format!("{}{}/{}", current.origin(), base, location))
}

fn request_once(
    method: &str,
    url: &Url,
    body: Option<&str>,
    headers: &[(String, String)],
) -> Result<Response, String> {
    let mut connection = connect(url)?;

    let mut request = format!(
        "{method} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        url.path, url.host
    );
    // Часть серверов отвечает отказом на запрос без User-Agent.
    if !headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("User-Agent"))
    {
        request.push_str("User-Agent: Kumir3\r\n");
    }
    for (key, value) in headers {
        request.push_str(&format!("{key}: {value}\r\n"));
    }
    if let Some(payload) = body {
        request.push_str(&format!("Content-Length: {}\r\n", payload.len()));
        if !headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Content-Type"))
        {
            request.push_str("Content-Type: application/x-www-form-urlencoded\r\n");
        }
    }
    request.push_str("\r\n");
    if let Some(payload) = body {
        request.push_str(payload);
    }

    connection
        .write_all(request.as_bytes())
        .and_then(|()| connection.flush())
        .map_err(|e| format!("Не удалось отправить запрос: {e}"))?;

    let raw = read_all(&mut connection)?;
    parse_response(&raw)
}

fn read_all(connection: &mut Connection) -> Result<Vec<u8>, String> {
    let mut collected = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match connection.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                collected.extend_from_slice(&chunk[..n]);
                if collected.len() > MAX_BODY {
                    return Err(format!(
                        "Ответ больше {} МиБ — чтение прервано",
                        MAX_BODY / (1024 * 1024)
                    ));
                }
            }
            // Закрытие TLS-соединения без прощания — обычное дело у серверов;
            // уже прочитанное при этом верно, поэтому это не ошибка.
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(format!("Ошибка чтения ответа: {e}")),
        }
    }
    Ok(collected)
}

/// Разбирает ответ на статус, заголовки и тело.
pub(crate) fn parse_response(raw: &[u8]) -> Result<Response, String> {
    let text = String::from_utf8_lossy(raw);
    let (head, body) = match text.find("\r\n\r\n") {
        Some(idx) => (&text[..idx], &text[idx + 4..]),
        None => return Err("Ответ сервера оборван: нет заголовков".to_string()),
    };

    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let mut parts = status_line.splitn(3, ' ');
    let _version = parts.next().unwrap_or_default();
    let status = parts
        .next()
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| format!("Не разобрать строку состояния: «{status_line}»"))?;
    let reason = parts.next().unwrap_or_default().trim().to_string();

    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect();

    let chunked = headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("Transfer-Encoding") && v.contains("chunked"));

    let body = if chunked {
        decode_chunked(body)?
    } else {
        body.to_string()
    };

    Ok(Response {
        status,
        reason,
        headers,
        body,
    })
}

/// Склеивает тело, переданное кусками (`Transfer-Encoding: chunked`).
///
/// Без этого тело приходило вперемешку с длинами кусков в шестнадцатеричном
/// виде, и таким ответом нельзя было пользоваться — а так отвечает
/// большинство современных серверов.
fn decode_chunked(body: &str) -> Result<String, String> {
    let mut rest = body;
    let mut out = String::new();

    // Отсутствие разделителя означает оборванный ответ: цикл заканчивается,
    // и наружу уходит то, что успели собрать.
    while let Some(line_end) = rest.find("\r\n") {
        // Длину могут сопровождать расширения через `;` — они не нужны.
        let size_text = rest[..line_end].split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| format!("Неверная длина куска ответа: «{size_text}»"))?;
        if size == 0 {
            break;
        }

        let start = line_end + 2;
        let end = start + size;
        if end > rest.len() {
            out.push_str(&rest[start..]);
            break;
        }
        out.push_str(&rest[start..end]);
        // За куском идёт свой перевод строки.
        rest = rest.get(end + 2..).unwrap_or("");
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn razbor_adresa() {
        let url = parse_url("https://example.org/путь?a=1").unwrap();
        assert_eq!(url.host, "example.org");
        assert_eq!(url.port, 443, "у https порт по умолчанию 443");
        assert_eq!(url.path, "/путь?a=1");
        assert!(url.https);

        let url = parse_url("example.org").unwrap();
        assert_eq!(url.port, 80, "без схемы — обычный http");
        assert_eq!(url.path, "/", "путь по умолчанию — корень");
        assert!(!url.https);

        let url = parse_url("http://localhost:8080/api").unwrap();
        assert_eq!((url.host.as_str(), url.port), ("localhost", 8080));
    }

    #[test]
    fn pustoj_adres_soobshchaetsya() {
        assert!(parse_url("").is_err());
        assert!(parse_url("http://").is_err());
    }

    /// Собирает ответ из строк: в HTTP они разделяются именно CR+LF.
    fn raw_response(lines: &[&str], body: &str) -> Vec<u8> {
        let mut text = lines.join("\r\n");
        text.push_str("\r\n\r\n");
        text.push_str(body);
        text.into_bytes()
    }

    #[test]
    fn razbor_otveta_s_zagolovkami() {
        let raw = raw_response(
            &[
                "HTTP/1.1 404 Not Found",
                "Content-Type: text/plain",
                "X-Test: 1",
            ],
            "нет такой страницы",
        );
        let response = parse_response(&raw).unwrap();
        assert_eq!(response.status, 404);
        assert_eq!(response.reason, "Not Found");
        assert_eq!(response.body, "нет такой страницы");
        assert_eq!(response.header("content-type"), Some("text/plain"));
        assert_eq!(response.header("нет-такого"), None);
    }

    #[test]
    fn telo_iz_kuskov_skleivaetsya() {
        // Длина каждого куска — в байтах, а русская буква занимает два.
        let raw = raw_response(
            &["HTTP/1.1 200 OK", "Transfer-Encoding: chunked"],
            "6\r\nПри\r\n6\r\nвет\r\n0\r\n\r\n",
        );
        let response = parse_response(&raw).unwrap();
        assert_eq!(response.body, "Привет");
    }

    #[test]
    fn rasshireniya_kuskov_ne_meshayut() {
        let body = "3;имя=значение\r\nabc\r\n0\r\n\r\n";
        assert_eq!(decode_chunked(body).unwrap(), "abc");
    }

    #[test]
    fn perenapravlenie_razreshaetsya_otnositelno_tekushchego() {
        let current = parse_url("https://example.org/a/b/c").unwrap();

        let absolute = resolve_redirect(&current, "https://other.org/x").unwrap();
        assert_eq!(absolute.host, "other.org");

        let from_root = resolve_redirect(&current, "/x").unwrap();
        assert_eq!(
            (from_root.host.as_str(), from_root.path.as_str()),
            ("example.org", "/x")
        );

        let relative = resolve_redirect(&current, "d").unwrap();
        assert_eq!(relative.path, "/a/b/d", "относительный путь — от каталога");
    }

    #[test]
    fn nestandartnyj_port_sohranyaetsya_pri_perenapravlenii() {
        let current = parse_url("http://localhost:8080/a").unwrap();
        let next = resolve_redirect(&current, "/b").unwrap();
        assert_eq!(
            (next.host.as_str(), next.port, next.path.as_str()),
            ("localhost", 8080, "/b")
        );
    }
}
