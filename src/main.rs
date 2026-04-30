#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]

use regex::Regex;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

#[derive(Debug, Clone)]
struct Device {
    id: u32,
    target: String,
    vendor_product: String,
    name: String,
    port: String,
    connect_type: String,
}

#[allow(clippy::unwrap_used)]
fn device_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"^(?P<id>\d+):\s+(?P<target>allow|block|reject)\s+id\s+(?P<vp>[0-9a-fA-F]{4}:[0-9a-fA-F]{4})\s+serial\s+"[^"]*"\s+name\s+"(?P<name>[^"]*)"\s+hash\s+"[^"]*"\s+parent-hash\s+"[^"]*"\s+via-port\s+"(?P<port>[^"]*)".*with-connect-type\s+"(?P<connect_type>[^"]*)"$"#
        )
        .unwrap()
    })
}

fn parse_devices(output: &str) -> Vec<Device> {
    let re = device_regex();
    output
        .lines()
        .filter_map(|line| {
            let caps = re.captures(line.trim())?;
            Some(Device {
                id: caps["id"].parse().ok()?,
                target: caps["target"].to_string(),
                vendor_product: caps["vp"].to_string(),
                name: caps["name"].to_string(),
                port: caps["port"].to_string(),
                connect_type: caps["connect_type"].to_string(),
            })
        })
        .collect()
}

fn list_devices() -> io::Result<Vec<Device>> {
    let output = Command::new("usbguard").args(["list-devices"]).output()?;

    if !output.status.success() {
        return Err(io::Error::other(format!(
            "usbguard list-devices failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_devices(&stdout))
}

fn run_fuzzel(prompt: &str, items: &[String]) -> io::Result<Option<String>> {
    let mut child = Command::new("fuzzel")
        .args(["--dmenu", "--prompt", prompt])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        for item in items {
            writeln!(stdin, "{item}")?;
        }
    }

    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let selection = stdout.trim();

    if selection.is_empty() {
        Ok(None)
    } else {
        Ok(Some(selection.to_string()))
    }
}

fn format_device(device: &Device) -> String {
    format!(
        "{}: {} ({}) [port: {}, type: {}]",
        device.id, device.name, device.vendor_product, device.port, device.connect_type
    )
}

fn execute_action(device_id: u32, action: &str) -> io::Result<()> {
    let id = device_id.to_string();
    let args: Vec<&str> = match action {
        "Allow" => vec!["allow-device", &id],
        "Allow permanently" => vec!["allow-device", "-p", &id],
        "Block" => vec!["block-device", &id],
        "Reject" => vec!["reject-device", &id],
        _ => return Ok(()),
    };

    let output = Command::new("usbguard").args(&args).output()?;

    if !output.status.success() {
        return Err(io::Error::other(format!(
            "usbguard {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let mut devices = list_devices()?;

    if devices.is_empty() {
        let notice = vec!["No blocked devices".to_string()];
        run_fuzzel("Notice: ", &notice)?;
        return Ok(());
    }

    // Sort: blocked first, then others
    devices.sort_by(|a, b| {
        let a_score = i32::from(a.target != "block");
        let b_score = i32::from(b.target != "block");
        a_score.cmp(&b_score).then_with(|| a.id.cmp(&b.id))
    });

    let has_blocked = devices.iter().any(|d| d.target == "block");

    let mut items: Vec<String> = Vec::new();
    if !has_blocked {
        items.push("No blocked devices".to_string());
    }
    items.extend(devices.iter().map(format_device));

    let Some(selection) = run_fuzzel("Device: ", &items)? else {
        return Ok(());
    };

    if selection == "No blocked devices" {
        return Ok(());
    }

    let device_id: u32 = selection
        .split(':')
        .next()
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid selection"))?;

    let device = devices
        .iter()
        .find(|d| d.id == device_id)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Device not found"))?;

    let actions = vec![
        "Allow".to_string(),
        "Allow permanently".to_string(),
        "Block".to_string(),
        "Reject".to_string(),
        "Cancel".to_string(),
    ];

    let Some(action) = run_fuzzel("Action: ", &actions)? else {
        return Ok(());
    };

    if action == "Cancel" {
        return Ok(());
    }

    execute_action(device.id, &action)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_blocked_device() {
        let output = r#"36: block id a8eb:9e03 serial "35d3c57000b3" name "tbvomwbt" hash "jIRtHA0nbiCtvo9e0zXiMzKsujYavqbQ9X9xxABQHfn=" parent-hash "t0PXjpKuDiQoNAI13HARnDpOB0MpkTDGITMcYavwUao=" via-port "3-1" with-interface { 0e:01:00 0e:02:00 } with-connect-type "hardwired""#;
        let devices = parse_devices(output);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, 36);
        assert_eq!(devices[0].target, "block");
        assert_eq!(devices[0].vendor_product, "a8eb:9e03");
        assert_eq!(devices[0].name, "tbvomwbt");
        assert_eq!(devices[0].port, "3-1");
        assert_eq!(devices[0].connect_type, "hardwired");
    }

    #[test]
    fn parse_multiple_devices() {
        let output = r#"25: allow id ddc0:e786 serial "565e8dd17f35" name "fmisiumi" hash "t0PXjpKuDiQoNAI13HARnDpOB0MpkTDGITMcYavwUao=" parent-hash "3JHXSgZpVxlFEOQDOiZ19ScvE9Dq2wbUlMVD4hxgJTf=" via-port "usb1" with-interface 09:00:00 with-connect-type ""
36: block id a8eb:9e03 serial "35d3c57000b3" name "tbvomwbt" hash "jIRtHA0nbiCtvo9e0zXiMzKsujYavqbQ9X9xxABQHfn=" parent-hash "t0PXjpKuDiQoNAI13HARnDpOB0MpkTDGITMcYavwUao=" via-port "3-1" with-interface { 0e:01:00 } with-connect-type "hardwired"
33: allow id 39b8:2661 serial "16071f740756" name "zhcgmlcq" hash "3JHXSgZpVxlFEOQDOiZ19ScvE9Dq2wbUlMVD4hxgJTf=" parent-hash "jIRtHA0nbiCtvo9e0zXiMzKsujYavqbQ9X9xxABQHfn=" via-port "1-1" with-interface { 11:00:00 03:00:00 } with-connect-type "hotplug""#;
        let devices = parse_devices(output);
        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].id, 25);
        assert_eq!(devices[1].id, 36);
        assert_eq!(devices[2].id, 33);
        assert_eq!(devices[1].target, "block");
        assert_eq!(devices[2].target, "allow");
    }

    #[test]
    fn parse_empty_output() {
        assert!(parse_devices("").is_empty());
    }

    #[test]
    fn parse_malformed_lines_skipped() {
        let output = r#"not a real device line
36: block id a8eb:9e03 serial "35d3c57000b3" name "tbvomwbt" hash "jIRtHA0nbiCtvo9e0zXiMzKsujYavqbQ9X9xxABQHfn=" parent-hash "t0PXjpKuDiQoNAI13HARnDpOB0MpkTDGITMcYavwUao=" via-port "3-1" with-interface { 0e:01:00 } with-connect-type "hardwired"
"#;
        let devices = parse_devices(output);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, 36);
    }

    #[test]
    fn parse_device_with_empty_connect_type() {
        let output = r#"25: allow id ddc0:e786 serial "565e8dd17f35" name "fmisiumi" hash "t0PXjpKuDiQoNAI13HARnDpOB0MpkTDGITMcYavwUao=" parent-hash "3JHXSgZpVxlFEOQDOiZ19ScvE9Dq2wbUlMVD4hxgJTf=" via-port "usb1" with-interface 09:00:00 with-connect-type """#;
        let devices = parse_devices(output);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].connect_type, "");
    }
}
