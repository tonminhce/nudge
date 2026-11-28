use chrono::{DateTime, Datelike, Utc};

pub const DEFAULT_WORKSPACE_LOGO: &str =
    "https://twentyhq.github.io/placeholder-images/workspaces/twenty-logo.png";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedEmail {
    pub html: String,
    pub text: String,
}

pub fn escape_react(value: &str) -> String {
    let mut out = String::with_capacity(value.len());

    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(ch),
        }
    }

    out
}

pub fn capitalize(value: &str) -> String {
    let mut chars = value.chars();

    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub fn image_absolute_uri(image_url: &str, base_url: &str) -> String {
    let lower = image_url.to_ascii_lowercase();
    let absolute = ["http:", "https:", "data:", "blob:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
        || image_url.starts_with("//");

    if absolute {
        return image_url.to_string();
    }

    if image_url.starts_with('/') {
        return format!("{base_url}/files{image_url}");
    }

    format!("{base_url}/files/{image_url}")
}

pub fn format_medium_date_utc(value: DateTime<Utc>) -> String {
    format!(
        "{} {}, {}",
        value.format("%b"),
        value.day(),
        value.format("%Y")
    )
}

pub fn format_medium_long_utc(value: DateTime<Utc>) -> String {
    let hour = value.format("%I").to_string();
    let hour = hour.trim_start_matches('0').to_string();

    format!(
        "{} {}, {}, {}{}",
        value.format("%b"),
        value.day(),
        value.format("%Y"),
        hour,
        value.format(":%M:%S %p UTC")
    )
}

pub fn format_duration_long(milliseconds: i64) -> String {
    let abs = milliseconds.unsigned_abs();
    let units = [
        (86_400_000_u64, "day"),
        (3_600_000_u64, "hour"),
        (60_000_u64, "minute"),
        (1_000_u64, "second"),
    ];

    let Some((size, name)) = units.into_iter().find(|(size, _)| abs >= *size) else {
        return format!("{milliseconds} ms");
    };

    let value = (milliseconds as f64 / size as f64).round() as i64;

    if abs >= size * 3 / 2 {
        format!("{value} {name}s")
    } else {
        format!("{value} {name}")
    }
}

fn fill(skeleton: &str, replacements: &[(&str, String)]) -> String {
    let mut out = skeleton.to_string();

    for (token, value) in replacements {
        out = out.replace(token, value);
    }

    out
}

fn locale_pair(locale: &str) -> Vec<(&'static str, String)> {
    vec![("{{locale}}", escape_react(locale))]
}

fn extend(
    base: Vec<(&'static str, String)>,
    more: &[(&'static str, String)],
) -> Vec<(&'static str, String)> {
    let mut out = base;
    out.extend(more.iter().map(|(token, value)| (*token, value.clone())));
    out
}

pub const VERIFICATION_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="333" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Confirm your email address</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">Thanks for registering for an account on Twenty! Before we get started, we just need to confirm that this is you. Click below to verify your email address.</p><br/><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Verify Email</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const VERIFICATION_TEXT: &str = r##"Twenty email


CONFIRM YOUR EMAIL ADDRESS

Thanks for registering for an account on Twenty! Before we get started, we just need to confirm that this is you. Click below to verify your email address.


Verify Email {{link}}



Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const VERIFICATION_UPDATE_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="333" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Confirm your new email address</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">We received a request to change the email address associated with your Twenty account. Click below to confirm this change.</p><br/><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Confirm new email</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const VERIFICATION_UPDATE_TEXT: &str = r##"Twenty email


CONFIRM YOUR NEW EMAIL ADDRESS

We received a request to change the email address associated with your Twenty account. Click below to confirm this change.


Confirm new email {{link}}



Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const PASSWORD_RESET_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="290" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Reset your password 🗝</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">This link is only valid for the next {{duration}}. If the link does not work, you can use the login verification link directly:<br/><a href="{{link}}" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">{{link}}</a></p><br/><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Reset</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const PASSWORD_RESET_TEXT: &str = r##"Twenty email


RESET YOUR PASSWORD 🗝

This link is only valid for the next {{duration}}. If the link does not work, you can use the login verification link directly:
{{link}}


Reset {{link}}



Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const PASSWORD_RESET_NO_PASSWORD_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="290" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Set your password 🗝</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">This link is only valid for the next {{duration}}. If the link does not work, you can use the login verification link directly:<br/><a href="{{link}}" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">{{link}}</a></p><br/><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Set</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const PASSWORD_RESET_NO_PASSWORD_TEXT: &str = r##"Twenty email


SET YOUR PASSWORD 🗝

This link is only valid for the next {{duration}}. If the link does not work, you can use the login verification link directly:
{{link}}


Set {{link}}



Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const INVITE_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="333" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Join your team on Twenty</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">{{sender_name}} (<a href="mailto:{{sender_email}}" style="color:#5e90f2;text-decoration-line:none;text-decoration:underline" target="_blank">{{sender_email}}</a>) has invited you to join a workspace called <b>{{workspace_name}}</b>.<br/></p><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;background:#f1f1f1;border:1px solid #ebebeb;border-radius:8px"><tbody><tr style="width:100%"><td style="padding:24px 48px"><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><img alt="Workspace logo" height="40" src="{{workspace_logo_url}}" style="display:block;outline:none;border:none;text-decoration:none" width="40"/></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><p style="font-size:16px;line-height:24px;border-radius:4px;background:#f1f1f1;padding:4px 8px;font-weight:600;color:#333333;margin-top:16px;margin-bottom:16px">{{workspace_name}}</p></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Accept invite</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a></td></tr></tbody></table></td></tr></tbody></table><h3 style="font-family:Trebuchet MS;font-size:16px;font-weight:600;color:#333333">What is Twenty?</h3><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">It&#x27;s a CRM, a software to help businesses manage their customer data and relationships efficiently.</p><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const INVITE_TEXT: &str = r##"Twenty email


JOIN YOUR TEAM ON TWENTY

{{sender_name}} ({{sender_email}}) has invited you to join a workspace called {{workspace_name}}.


{{workspace_name}}

Accept invite {{link}}


WHAT IS TWENTY?

It's a CRM, a software to help businesses manage their customer data and relationships efficiently.

Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const INVITE_NAME_ONLY_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="333" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Join your team on Twenty</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">{{sender_name}} (<a href="mailto:{{sender_email}}" style="color:#5e90f2;text-decoration-line:none;text-decoration:underline" target="_blank">{{sender_email}}</a>) has invited you to join a workspace called <b>{{workspace_name}}</b>.<br/></p><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;background:#f1f1f1;border:1px solid #ebebeb;border-radius:8px"><tbody><tr style="width:100%"><td style="padding:24px 48px"><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><p style="font-size:16px;line-height:24px;border-radius:4px;background:#f1f1f1;padding:4px 8px;font-weight:600;color:#333333;margin-top:16px;margin-bottom:16px">{{workspace_name}}</p></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Accept invite</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a></td></tr></tbody></table></td></tr></tbody></table><h3 style="font-family:Trebuchet MS;font-size:16px;font-weight:600;color:#333333">What is Twenty?</h3><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">It&#x27;s a CRM, a software to help businesses manage their customer data and relationships efficiently.</p><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const INVITE_NAME_ONLY_TEXT: &str = r##"Twenty email


JOIN YOUR TEAM ON TWENTY

{{sender_name}} ({{sender_email}}) has invited you to join a workspace called {{workspace_name}}.


{{workspace_name}}

Accept invite {{link}}


WHAT IS TWENTY?

It's a CRM, a software to help businesses manage their customer data and relationships efficiently.

Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const INVITE_LOGO_ONLY_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="333" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Join your team on Twenty</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">{{sender_name}} (<a href="mailto:{{sender_email}}" style="color:#5e90f2;text-decoration-line:none;text-decoration:underline" target="_blank">{{sender_email}}</a>) has invited you to join a workspace called <b></b>.<br/></p><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;background:#f1f1f1;border:1px solid #ebebeb;border-radius:8px"><tbody><tr style="width:100%"><td style="padding:24px 48px"><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><img alt="Workspace logo" height="40" src="{{workspace_logo_url}}" style="display:block;outline:none;border:none;text-decoration:none" width="40"/></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Accept invite</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a></td></tr></tbody></table></td></tr></tbody></table><h3 style="font-family:Trebuchet MS;font-size:16px;font-weight:600;color:#333333">What is Twenty?</h3><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">It&#x27;s a CRM, a software to help businesses manage their customer data and relationships efficiently.</p><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const INVITE_LOGO_ONLY_TEXT: &str = r##"Twenty email


JOIN YOUR TEAM ON TWENTY

{{sender_name}} ({{sender_email}}) has invited you to join a workspace called {{workspace_name}}.


Accept invite {{link}}


WHAT IS TWENTY?

It's a CRM, a software to help businesses manage their customer data and relationships efficiently.

Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const INVITE_NO_LOGO_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="333" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Join your team on Twenty</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">{{sender_name}} (<a href="mailto:{{sender_email}}" style="color:#5e90f2;text-decoration-line:none;text-decoration:underline" target="_blank">{{sender_email}}</a>) has invited you to join a workspace called <b>{{workspace_name}}</b>.<br/></p><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;background:#f1f1f1;border:1px solid #ebebeb;border-radius:8px"><tbody><tr style="width:100%"><td style="padding:24px 48px"><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Accept invite</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a></td></tr></tbody></table></td></tr></tbody></table><h3 style="font-family:Trebuchet MS;font-size:16px;font-weight:600;color:#333333">What is Twenty?</h3><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">It&#x27;s a CRM, a software to help businesses manage their customer data and relationships efficiently.</p><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const INVITE_NO_LOGO_TEXT: &str = r##"Twenty email


JOIN YOUR TEAM ON TWENTY

{{sender_name}} ({{sender_email}}) has invited you to join a workspace called {{workspace_name}}.


Accept invite {{link}}


WHAT IS TWENTY?

It's a CRM, a software to help businesses manage their customer data and relationships efficiently.

Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const PASSWORD_UPDATE_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="290" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="{{link}}/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Password updated</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">Dear {{user_name}},<br/><br/>This is a confirmation that password for your account ({{email}}) was successfully changed on {{changed_at_display}}.<br/><br/>If you did not initiate this change, please contact your workspace owner immediately.<br/></p><br/><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Connect to Twenty</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const PASSWORD_UPDATE_TEXT: &str = r##"Twenty email


PASSWORD UPDATED

Dear {{user_name}},

This is a confirmation that password for your account ({{email}}) was successfully changed on {{changed_at_display}}.

If you did not initiate this change, please contact your workspace owner immediately.



Connect to Twenty {{link}}



Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const TWO_FACTOR_BACKUP_USED_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="290" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Backup code used</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">A backup code was used to sign in to {{workspace_display_name}}. Backup codes left: {{remaining_backup_code_count}}.<br/><br/>You can generate new backup codes under Settings, Profile, Two-factor authentication.<br/><br/>If this was not you, change your password and generate new backup codes.<br/></p><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const TWO_FACTOR_BACKUP_USED_TEXT: &str = r##"Twenty email


BACKUP CODE USED

A backup code was used to sign in to {{workspace_display_name}}. Backup codes left: {{remaining_backup_code_count}}.

You can generate new backup codes under Settings, Profile, Two-factor authentication.

If this was not you, change your password and generate new backup codes.




Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const TWO_FACTOR_CODES_REGEN_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="290" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">New backup codes generated</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">New backup codes were generated for your account in {{workspace_display_name}}. Your previous backup codes no longer work.<br/><br/>If this was not you, change your password and generate new backup codes under Settings, Profile, Two-factor authentication.<br/></p><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const TWO_FACTOR_CODES_REGEN_TEXT: &str = r##"Twenty email


NEW BACKUP CODES GENERATED

New backup codes were generated for your account in {{workspace_display_name}}. Your previous backup codes no longer work.

If this was not you, change your password and generate new backup codes under Settings, Profile, Two-factor authentication.




Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const TWO_FACTOR_RECOVERY_ISSUED_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="290" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Two-factor authentication recovery code issued</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">{{actor_name}} generated a recovery code that lets you sign in to {{workspace_display_name}} without your authenticator app.<br/><br/>The code can be used once and expires on {{expires_at_display}}. Using it removes your current authenticator for this workspace and signs you out of your other sessions there.<br/><br/>If you did not ask for this, contact your workspace administrator and change your password.<br/></p><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const TWO_FACTOR_RECOVERY_ISSUED_TEXT: &str = r##"Twenty email


TWO-FACTOR AUTHENTICATION RECOVERY CODE ISSUED

{{actor_name}} generated a recovery code that lets you sign in to {{workspace_display_name}} without your authenticator app.

The code can be used once and expires on {{expires_at_display}}. Using it removes your current authenticator for this workspace and signs you out of your other sessions there.

If you did not ask for this, contact your workspace administrator and change your password.




Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const TWO_FACTOR_REMOVED_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="290" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Two-factor authentication removed</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">The authenticator for your account in {{workspace_display_name}} was removed from Settings. Your backup codes for this workspace no longer work.<br/><br/>Set up two-factor authentication again under Settings, Profile.<br/><br/>If this was not you, contact your workspace administrator and change your password.<br/></p><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const TWO_FACTOR_REMOVED_TEXT: &str = r##"Twenty email


TWO-FACTOR AUTHENTICATION REMOVED

The authenticator for your account in {{workspace_display_name}} was removed from Settings. Your backup codes for this workspace no longer work.

Set up two-factor authentication again under Settings, Profile.

If this was not you, contact your workspace administrator and change your password.




Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const TWO_FACTOR_RESET_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="290" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Two-factor authentication reset</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">A recovery code was used to sign in to {{workspace_display_name}}. Your authenticator for this workspace was removed and your other sessions there were signed out.<br/><br/>Set up two-factor authentication again under Settings, Profile.<br/><br/>If this was not you, contact your workspace administrator and change your password.<br/></p><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const TWO_FACTOR_RESET_TEXT: &str = r##"Twenty email


TWO-FACTOR AUTHENTICATION RESET

A recovery code was used to sign in to {{workspace_display_name}}. Your authenticator for this workspace was removed and your other sessions there were signed out.

Set up two-factor authentication again under Settings, Profile.

If this was not you, contact your workspace administrator and change your password.




Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const SERVER_ADMIN_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="290" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Server administrator access changed</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">{{actor_name}} updated server administrator access for {{target_name}} ({{target_email}}).<br/><br/>Full admin panel access: {{admin_panel_access}}<br/>Impersonation: {{impersonation_access}}<br/><br/>If you did not expect this change, review your server administrators immediately.<br/></p><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const SERVER_ADMIN_TEXT: &str = r##"Twenty email


SERVER ADMINISTRATOR ACCESS CHANGED

{{actor_name}} updated server administrator access for {{target_name}} ({{target_email}}).

Full admin panel access: {{admin_panel_access}}
Impersonation: {{impersonation_access}}

If you did not expect this change, review your server administrators immediately.




Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const APPROVED_DOMAIN_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="333" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Validate domain</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">{{sender_name}} (<a href="mailto:{{sender_email}}" style="color:#5e90f2;text-decoration-line:none;text-decoration:underline" target="_blank">{{sender_email}}</a>): Please validate this domain to allow users with <b>@{{domain}}</b> email addresses to join your workspace without requiring an invitation.<br/></p><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;background:#f1f1f1;border:1px solid #ebebeb;border-radius:8px"><tbody><tr style="width:100%"><td style="padding:24px 48px"><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><img alt="{{workspace_name}}" height="40" src="https://twentyhq.github.io/placeholder-images/workspaces/twenty-logo.png" style="display:block;outline:none;border:none;text-decoration:none" width="40"/></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><p style="font-size:16px;line-height:24px;border-radius:4px;background:#f1f1f1;padding:4px 8px;font-weight:600;color:#333333;margin-top:16px;margin-bottom:16px">{{workspace_name}}</p></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Validate domain</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a></td></tr></tbody></table></td></tr></tbody></table><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const APPROVED_DOMAIN_TEXT: &str = r##"Twenty email


VALIDATE DOMAIN

{{sender_name}} ({{sender_email}}): Please validate this domain to allow users with @{{domain}} email addresses to join your workspace without requiring an invitation.


{{workspace_name}}

Validate domain {{link}}


Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const APPROVED_DOMAIN_FULL_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="333" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Validate domain</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">{{sender_name}} (<a href="mailto:{{sender_email}}" style="color:#5e90f2;text-decoration-line:none;text-decoration:underline" target="_blank">{{sender_email}}</a>): Please validate this domain to allow users with <b>@{{domain}}</b> email addresses to join your workspace without requiring an invitation.<br/></p><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;background:#f1f1f1;border:1px solid #ebebeb;border-radius:8px"><tbody><tr style="width:100%"><td style="padding:24px 48px"><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><img alt="{{workspace_name}}" height="40" src="{{workspace_logo_url}}" style="display:block;outline:none;border:none;text-decoration:none" width="40"/></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><p style="font-size:16px;line-height:24px;border-radius:4px;background:#f1f1f1;padding:4px 8px;font-weight:600;color:#333333;margin-top:16px;margin-bottom:16px">{{workspace_name}}</p></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Validate domain</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a></td></tr></tbody></table></td></tr></tbody></table><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const APPROVED_DOMAIN_FULL_TEXT: &str = r##"Twenty email


VALIDATE DOMAIN

{{sender_name}} ({{sender_email}}): Please validate this domain to allow users with @{{domain}} email addresses to join your workspace without requiring an invitation.


{{workspace_name}}

Validate domain {{link}}


Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const APPROVED_DOMAIN_LOGO_ONLY_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="333" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Validate domain</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">{{sender_name}} (<a href="mailto:{{sender_email}}" style="color:#5e90f2;text-decoration-line:none;text-decoration:underline" target="_blank">{{sender_email}}</a>): Please validate this domain to allow users with <b>@{{domain}}</b> email addresses to join your workspace without requiring an invitation.<br/></p><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;background:#f1f1f1;border:1px solid #ebebeb;border-radius:8px"><tbody><tr style="width:100%"><td style="padding:24px 48px"><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><img alt="Workspace logo" height="40" src="{{workspace_logo_url}}" style="display:block;outline:none;border:none;text-decoration:none" width="40"/></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Validate domain</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a></td></tr></tbody></table></td></tr></tbody></table><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const APPROVED_DOMAIN_LOGO_ONLY_TEXT: &str = r##"Twenty email


VALIDATE DOMAIN

{{sender_name}} ({{sender_email}}): Please validate this domain to allow users with @{{domain}} email addresses to join your workspace without requiring an invitation.


Validate domain {{link}}


Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const APPROVED_DOMAIN_EMPTY_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="333" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Validate domain</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">{{sender_name}} (<a href="mailto:{{sender_email}}" style="color:#5e90f2;text-decoration-line:none;text-decoration:underline" target="_blank">{{sender_email}}</a>): Please validate this domain to allow users with <b>@{{domain}}</b> email addresses to join your workspace without requiring an invitation.<br/></p><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;background:#f1f1f1;border:1px solid #ebebeb;border-radius:8px"><tbody><tr style="width:100%"><td style="padding:24px 48px"><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><img alt="Workspace logo" height="40" src="https://twentyhq.github.io/placeholder-images/workspaces/twenty-logo.png" style="display:block;outline:none;border:none;text-decoration:none" width="40"/></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"></td></tr></tbody></table><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td align="center" data-id="__react-email-column"><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Validate domain</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a></td></tr></tbody></table></td></tr></tbody></table><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const APPROVED_DOMAIN_EMPTY_TEXT: &str = r##"Twenty email


VALIDATE DOMAIN

{{sender_name}} ({{sender_email}}): Please validate this domain to allow users with @{{domain}} email addresses to join your workspace without requiring an invitation.


Validate domain {{link}}


Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const WARN_SUSPENDED_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="333" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Your workspace is paused</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">Hi {{user_name}},<br/><br/>Good news first: your workspace <b>{{workspace_display_name}}</b> is only paused — none of your data is gone.<br/><br/>Reactivate it within the next {{remaining_days}} {{day_or_days}} and you&#x27;ll pick up exactly where you left off, with every record, view and setting right where you left it.<br/><br/>After that, the workspace and all of its data will be permanently deleted — and we won&#x27;t be able to bring it back.</p><br/><a href="{{link}}" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Reactivate workspace</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const WARN_SUSPENDED_TEXT: &str = r##"Twenty email


YOUR WORKSPACE IS PAUSED

Hi {{user_name}},

Good news first: your workspace {{workspace_display_name}} is only paused — none of your data is gone.

Reactivate it within the next {{remaining_days}} {{day_or_days}} and you'll pick up exactly where you left off, with every record, view and setting right where you left it.

After that, the workspace and all of its data will be permanently deleted — and we won't be able to bring it back.


Reactivate workspace {{link}}



Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;
pub const CLEAN_SUSPENDED_HTML: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"><html dir="ltr" lang="{{locale}}"><head><meta content="text/html; charset=UTF-8" http-equiv="Content-Type"/><meta name="x-apple-disable-message-reformatting"/><title>Twenty email</title><style>
    @font-face {
      font-family: 'Trebuchet MS';
      font-style: normal;
      font-weight: 400;
      mso-font-alt: 'sans-serif';
      
    }

    * {
      font-family: 'Trebuchet MS', sans-serif;
    }
  </style></head><!--$--><!--html--><!--head--><table align="center" width="333" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em"><tbody><tr style="width:100%"><td><img alt="Twenty logo" height="40" src="https://app.twenty.com/images/icons/windows11/Square150x150Logo.scale-100.png" style="display:block;outline:none;border:none;text-decoration:none;margin-bottom:40px" width="40"/><h1 style="font-family:Trebuchet MS;font-size:24px;font-weight:600;color:#333333">Your workspace has been deleted</h1><p style="font-size:13px;line-height:20px;font-family:Trebuchet MS;font-weight:400;color:#666666;margin-top:16px;margin-bottom:16px">Hi {{user_name}},<br/><br/>Your workspace <b>{{workspace_display_name}}</b> has now been permanently deleted — it was paused {{days_since_inactive}} days ago and wasn&#x27;t reactivated in time.<br/><br/>Its data has been removed and can no longer be recovered.<br/><br/>If you&#x27;d ever like to give Twenty another try, you can start a fresh workspace in minutes — we&#x27;d love to have you back.</p><br/><a href="https://app.twenty.com/" style="line-height:100%;text-decoration:none;display:inline-flex;max-width:100%;mso-padding-alt:0px;padding:8px 32px;border-radius:8px;border:1px solid rgba(0, 0, 0, 0.04);background:#333333;box-shadow:0px 2px 4px 0px rgba(0, 0, 0, 0.04), 0px 0px 4px 0px rgba(0, 0, 0, 0.08);color:#ffffff;font-size:13px;font-weight:600;width:auto;padding-top:8px;padding-right:32px;padding-bottom:8px;padding-left:32px" target="_blank"><span><!--[if mso]><i style="mso-font-width:400%;mso-text-raise:12px" hidden>&#8202;&#8202;&#8202;&#8202;</i><![endif]--></span><span style="max-width:100%;display:inline-block;line-height:120%;mso-padding-alt:0px;mso-text-raise:6px">Start a new workspace</span><span><!--[if mso]><i style="mso-font-width:400%" hidden>&#8202;&#8202;&#8202;&#8202;&#8203;</i><![endif]--></span></a><br/><br/><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation" style="max-width:37.5em;margin-top:12px"><tbody><tr style="width:100%"><td><table align="center" width="100%" border="0" cellPadding="0" cellSpacing="0" role="presentation"><tbody style="width:100%"><tr style="width:100%"><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Website</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://github.com/twentyhq/twenty" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Github</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/getting-started/introduction" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">User guide</a></p></td><td data-id="__react-email-column"><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0"><a href="https://docs.twenty.com/" style="color:#b3b3b3;text-decoration-line:none;text-decoration:underline" target="_blank">Developers</a></p></td></tr></tbody></table><p style="font-size:12px;line-height:20px;font-weight:400;color:#b3b3b3;margin:0 0 12px 0;margin-top:0;margin-right:0;margin-bottom:12px;margin-left:0">Twenty.com, Public Benefit Corporation<br/>San Francisco / Paris</p></td></tr></tbody></table></td></tr></tbody></table><!--/$--></html>"##;
pub const CLEAN_SUSPENDED_TEXT: &str = r##"Twenty email


YOUR WORKSPACE HAS BEEN DELETED

Hi {{user_name}},

Your workspace {{workspace_display_name}} has now been permanently deleted — it was paused {{days_since_inactive}} days ago and wasn't reactivated in time.

Its data has been removed and can no longer be recovered.

If you'd ever like to give Twenty another try, you can start a fresh workspace in minutes — we'd love to have you back.


Start a new workspace https://app.twenty.com/



Website https://twenty.com/

Github https://github.com/twentyhq/twenty

User guide https://docs.twenty.com/getting-started/introduction

Developers https://docs.twenty.com/

Twenty.com, Public Benefit Corporation
San Francisco / Paris"##;

pub fn send_email_verification_link_email(
    link: &str,
    locale: &str,
    is_email_update: bool,
) -> RenderedEmail {
    let (html_skeleton, text_skeleton) = if is_email_update {
        (VERIFICATION_UPDATE_HTML, VERIFICATION_UPDATE_TEXT)
    } else {
        (VERIFICATION_HTML, VERIFICATION_TEXT)
    };
    let html_slots = vec![("{{link}}", escape_react(link))];
    let text_slots = vec![("{{link}}", link.to_string())];

    RenderedEmail {
        html: fill(html_skeleton, &extend(locale_pair(locale), &html_slots)),
        text: fill(text_skeleton, &text_slots),
    }
}

pub fn password_reset_link_email(
    duration: &str,
    has_password: bool,
    link: &str,
    locale: &str,
) -> RenderedEmail {
    let (html_skeleton, text_skeleton) = if has_password {
        (PASSWORD_RESET_HTML, PASSWORD_RESET_TEXT)
    } else {
        (
            PASSWORD_RESET_NO_PASSWORD_HTML,
            PASSWORD_RESET_NO_PASSWORD_TEXT,
        )
    };
    let slots = vec![
        ("{{link}}", escape_react(link)),
        ("{{duration}}", escape_react(duration)),
    ];

    RenderedEmail {
        html: fill(html_skeleton, &extend(locale_pair(locale), &slots)),
        text: fill(
            text_skeleton,
            &[
                ("{{link}}", link.to_string()),
                ("{{duration}}", duration.to_string()),
            ],
        ),
    }
}

pub fn send_invite_link_email(
    link: &str,
    workspace_name: Option<&str>,
    workspace_logo: Option<&str>,
    sender_email: &str,
    sender_first_name: &str,
    server_url: &str,
    locale: &str,
) -> RenderedEmail {
    let (html_skeleton, text_skeleton) = match (workspace_logo, workspace_name) {
        (Some(_), Some(_)) => (INVITE_HTML, INVITE_TEXT),
        (Some(_), None) => (INVITE_LOGO_ONLY_HTML, INVITE_LOGO_ONLY_TEXT),
        (None, Some(_)) => (INVITE_NAME_ONLY_HTML, INVITE_NAME_ONLY_TEXT),
        (None, None) => (INVITE_NO_LOGO_HTML, INVITE_NO_LOGO_TEXT),
    };
    let name = workspace_name.unwrap_or("");
    let logo_url = workspace_logo
        .map(|logo| image_absolute_uri(logo, server_url))
        .unwrap_or_default();
    let sender_name = capitalize(sender_first_name);
    let html_slots = vec![
        ("{{workspace_logo_url}}", escape_react(&logo_url)),
        ("{{link}}", escape_react(link)),
        ("{{sender_email}}", escape_react(sender_email)),
        ("{{workspace_name}}", escape_react(name)),
        ("{{sender_name}}", escape_react(&sender_name)),
    ];
    let text_slots = vec![
        ("{{link}}", link.to_string()),
        ("{{sender_email}}", sender_email.to_string()),
        ("{{workspace_name}}", name.to_string()),
        ("{{sender_name}}", sender_name.clone()),
    ];

    RenderedEmail {
        html: fill(html_skeleton, &extend(locale_pair(locale), &html_slots)),
        text: fill(text_skeleton, &text_slots),
    }
}

pub fn password_update_notify_email(
    user_name: &str,
    email: &str,
    link: &str,
    locale: &str,
    changed_at: DateTime<Utc>,
) -> RenderedEmail {
    let changed_at_display = format_medium_date_utc(changed_at);
    let slots = vec![
        ("{{link}}", escape_react(link)),
        ("{{email}}", escape_react(email)),
        ("{{changed_at_display}}", escape_react(&changed_at_display)),
        ("{{user_name}}", escape_react(user_name)),
    ];
    let text_slots = vec![
        ("{{link}}", link.to_string()),
        ("{{email}}", email.to_string()),
        ("{{changed_at_display}}", changed_at_display),
        ("{{user_name}}", user_name.to_string()),
    ];

    RenderedEmail {
        html: fill(PASSWORD_UPDATE_HTML, &extend(locale_pair(locale), &slots)),
        text: fill(PASSWORD_UPDATE_TEXT, &text_slots),
    }
}

pub fn two_factor_authentication_backup_code_used_email(
    workspace_display_name: &str,
    remaining_backup_code_count: u32,
    locale: &str,
) -> RenderedEmail {
    let count = remaining_backup_code_count.to_string();
    let name = escape_react(workspace_display_name);
    let slots = vec![
        ("{{remaining_backup_code_count}}", count.clone()),
        ("{{workspace_display_name}}", name),
    ];
    let text_slots = vec![
        ("{{remaining_backup_code_count}}", count),
        (
            "{{workspace_display_name}}",
            workspace_display_name.to_string(),
        ),
    ];

    RenderedEmail {
        html: fill(
            TWO_FACTOR_BACKUP_USED_HTML,
            &extend(locale_pair(locale), &slots),
        ),
        text: fill(TWO_FACTOR_BACKUP_USED_TEXT, &text_slots),
    }
}

pub fn two_factor_authentication_backup_codes_regenerated_email(
    workspace_display_name: &str,
    locale: &str,
) -> RenderedEmail {
    let slots = vec![(
        "{{workspace_display_name}}",
        escape_react(workspace_display_name),
    )];
    let text_slots = vec![(
        "{{workspace_display_name}}",
        workspace_display_name.to_string(),
    )];

    RenderedEmail {
        html: fill(
            TWO_FACTOR_CODES_REGEN_HTML,
            &extend(locale_pair(locale), &slots),
        ),
        text: fill(TWO_FACTOR_CODES_REGEN_TEXT, &text_slots),
    }
}

pub fn two_factor_authentication_recovery_code_issued_email(
    actor_name: &str,
    workspace_display_name: &str,
    expires_at: DateTime<Utc>,
    locale: &str,
) -> RenderedEmail {
    let expires_at_display = format_medium_long_utc(expires_at);
    let slots = vec![
        ("{{expires_at_display}}", escape_react(&expires_at_display)),
        ("{{actor_name}}", escape_react(actor_name)),
        (
            "{{workspace_display_name}}",
            escape_react(workspace_display_name),
        ),
    ];
    let text_slots = vec![
        ("{{expires_at_display}}", expires_at_display),
        ("{{actor_name}}", actor_name.to_string()),
        (
            "{{workspace_display_name}}",
            workspace_display_name.to_string(),
        ),
    ];

    RenderedEmail {
        html: fill(
            TWO_FACTOR_RECOVERY_ISSUED_HTML,
            &extend(locale_pair(locale), &slots),
        ),
        text: fill(TWO_FACTOR_RECOVERY_ISSUED_TEXT, &text_slots),
    }
}

pub fn two_factor_authentication_removed_email(
    workspace_display_name: &str,
    locale: &str,
) -> RenderedEmail {
    let slots = vec![(
        "{{workspace_display_name}}",
        escape_react(workspace_display_name),
    )];
    let text_slots = vec![(
        "{{workspace_display_name}}",
        workspace_display_name.to_string(),
    )];

    RenderedEmail {
        html: fill(
            TWO_FACTOR_REMOVED_HTML,
            &extend(locale_pair(locale), &slots),
        ),
        text: fill(TWO_FACTOR_REMOVED_TEXT, &text_slots),
    }
}

pub fn two_factor_authentication_reset_email(
    workspace_display_name: &str,
    locale: &str,
) -> RenderedEmail {
    let slots = vec![(
        "{{workspace_display_name}}",
        escape_react(workspace_display_name),
    )];
    let text_slots = vec![(
        "{{workspace_display_name}}",
        workspace_display_name.to_string(),
    )];

    RenderedEmail {
        html: fill(TWO_FACTOR_RESET_HTML, &extend(locale_pair(locale), &slots)),
        text: fill(TWO_FACTOR_RESET_TEXT, &text_slots),
    }
}

pub fn server_admin_access_changed_email(
    actor_name: &str,
    target_name: &str,
    target_email: &str,
    can_access_full_admin_panel: bool,
    can_impersonate: bool,
    locale: &str,
) -> RenderedEmail {
    let admin_panel_access = if can_access_full_admin_panel {
        "Enabled"
    } else {
        "Disabled"
    };
    let impersonation_access = if can_impersonate {
        "Enabled"
    } else {
        "Disabled"
    };
    let slots = vec![
        ("{{target_email}}", escape_react(target_email)),
        ("{{target_name}}", escape_react(target_name)),
        ("{{actor_name}}", escape_react(actor_name)),
        ("{{admin_panel_access}}", admin_panel_access.to_string()),
        ("{{impersonation_access}}", impersonation_access.to_string()),
    ];
    let text_slots = vec![
        ("{{target_email}}", target_email.to_string()),
        ("{{target_name}}", target_name.to_string()),
        ("{{actor_name}}", actor_name.to_string()),
        ("{{admin_panel_access}}", admin_panel_access.to_string()),
        ("{{impersonation_access}}", impersonation_access.to_string()),
    ];

    RenderedEmail {
        html: fill(SERVER_ADMIN_HTML, &extend(locale_pair(locale), &slots)),
        text: fill(SERVER_ADMIN_TEXT, &text_slots),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "keeps the same one-slot-per-argument shape as the sibling template functions here"
)]
pub fn send_approved_access_domain_validation(
    link: &str,
    domain: &str,
    workspace_name: Option<&str>,
    workspace_logo: Option<&str>,
    sender_email: &str,
    sender_first_name: &str,
    server_url: &str,
    locale: &str,
) -> RenderedEmail {
    let (html_skeleton, text_skeleton) = match (workspace_logo, workspace_name) {
        (Some(_), Some(_)) => (APPROVED_DOMAIN_FULL_HTML, APPROVED_DOMAIN_FULL_TEXT),
        (Some(_), None) => (
            APPROVED_DOMAIN_LOGO_ONLY_HTML,
            APPROVED_DOMAIN_LOGO_ONLY_TEXT,
        ),
        (None, Some(_)) => (APPROVED_DOMAIN_HTML, APPROVED_DOMAIN_TEXT),
        (None, None) => (APPROVED_DOMAIN_EMPTY_HTML, APPROVED_DOMAIN_EMPTY_TEXT),
    };
    let name = workspace_name.unwrap_or("");
    let logo_url = workspace_logo
        .map(|logo| image_absolute_uri(logo, server_url))
        .unwrap_or_else(|| DEFAULT_WORKSPACE_LOGO.to_string());
    let sender_name = capitalize(sender_first_name);
    let html_slots = vec![
        ("{{workspace_logo_url}}", escape_react(&logo_url)),
        ("{{link}}", escape_react(link)),
        ("{{sender_email}}", escape_react(sender_email)),
        ("{{workspace_name}}", escape_react(name)),
        ("{{domain}}", escape_react(domain)),
        ("{{sender_name}}", escape_react(&sender_name)),
    ];
    let text_slots = vec![
        ("{{link}}", link.to_string()),
        ("{{sender_email}}", sender_email.to_string()),
        ("{{workspace_name}}", name.to_string()),
        ("{{domain}}", domain.to_string()),
        ("{{sender_name}}", sender_name),
    ];

    RenderedEmail {
        html: fill(html_skeleton, &extend(locale_pair(locale), &html_slots)),
        text: fill(text_skeleton, &text_slots),
    }
}

pub fn warn_suspended_workspace_email(
    days_since_inactive: i64,
    inactive_days_before_delete: i64,
    user_name: &str,
    workspace_display_name: &str,
    link: &str,
    locale: &str,
) -> RenderedEmail {
    let days_left = inactive_days_before_delete - days_since_inactive;
    let day_or_days = if days_left > 1 { "days" } else { "day" };
    let remaining_days = if days_left > 0 { days_left } else { 0 }.to_string();
    let slots = vec![
        ("{{link}}", escape_react(link)),
        (
            "{{workspace_display_name}}",
            escape_react(workspace_display_name),
        ),
        ("{{user_name}}", escape_react(user_name)),
        ("{{remaining_days}}", remaining_days.clone()),
        ("{{day_or_days}}", day_or_days.to_string()),
    ];
    let text_slots = vec![
        ("{{link}}", link.to_string()),
        (
            "{{workspace_display_name}}",
            workspace_display_name.to_string(),
        ),
        ("{{user_name}}", user_name.to_string()),
        ("{{remaining_days}}", remaining_days),
        ("{{day_or_days}}", day_or_days.to_string()),
    ];

    RenderedEmail {
        html: fill(WARN_SUSPENDED_HTML, &extend(locale_pair(locale), &slots)),
        text: fill(WARN_SUSPENDED_TEXT, &text_slots),
    }
}

pub fn clean_suspended_workspace_email(
    days_since_inactive: i64,
    user_name: &str,
    workspace_display_name: &str,
    locale: &str,
) -> RenderedEmail {
    let days = days_since_inactive.to_string();
    let slots = vec![
        ("{{days_since_inactive}}", days.clone()),
        ("{{user_name}}", escape_react(user_name)),
        (
            "{{workspace_display_name}}",
            escape_react(workspace_display_name),
        ),
    ];
    let text_slots = vec![
        ("{{days_since_inactive}}", days),
        ("{{user_name}}", user_name.to_string()),
        (
            "{{workspace_display_name}}",
            workspace_display_name.to_string(),
        ),
    ];

    RenderedEmail {
        html: fill(CLEAN_SUSPENDED_HTML, &extend(locale_pair(locale), &slots)),
        text: fill(CLEAN_SUSPENDED_TEXT, &text_slots),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn sha256_hex(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    #[test]
    fn duration_long_formatting_matches_the_ms_package() {
        let measured = [
            (86_400_000_i64, "1 day"),
            (3_600_000, "1 hour"),
            (1_800_000, "30 minutes"),
            (90_000, "2 minutes"),
            (45_000, "45 seconds"),
            (86_461_000, "1 day"),
            (90_061_000, "1 day"),
            (1_000, "1 second"),
            (59_999, "60 seconds"),
            (60_000, "1 minute"),
            (86_399_000, "24 hours"),
            (172_800_000, "2 days"),
            (365_000_000, "4 days"),
            (300_000, "5 minutes"),
            (89_999, "1 minute"),
            (120_960_000, "1 day"),
            (129_600_000, "2 days"),
            (1_499_999, "25 minutes"),
            (999, "999 ms"),
            (1, "1 ms"),
            (0, "0 ms"),
            (-45_000, "-45 seconds"),
        ];

        for (milliseconds, expected) in measured {
            assert_eq!(
                format_duration_long(milliseconds),
                expected,
                "ms({milliseconds}, {{ long: true }})"
            );
        }
    }

    fn golden_hashes() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            (
                "verification",
                "61b0d29ac9aa8f09d3333432a3585b085a0becb10b22eb047d94b75d262af2c9",
                "43d2b8f53f8159872f65dfa7e3980213a633de75ba8f1bcac581838d40558673",
            ),
            (
                "verificationUpdate",
                "d1f1dcd5c3ce1a8b0ab48af52f721f9f5ccdaf3ebad8ceb25ab2da92af892969",
                "fd4d297187913a1a494207495224fc104d74c30055e4192224b83f31958a9219",
            ),
            (
                "passwordReset",
                "949dce3875a2b9893d4dab2ff7cc2873914b63617046467aa01efd30622b6a75",
                "6a0ded7bf6c318bf6886f1aa1872d43d32150cc8c5c24e66e4c71c25c9d36a80",
            ),
            (
                "passwordResetNoPassword",
                "9dc49e2032b2629d8fb1637c3efe8a2cd9d564b0365854945521313e07aadfd2",
                "c0a8d8aab132ecc262e76476c246333395d7b656619ec13e251080b785d58152",
            ),
            (
                "invite",
                "b000b60a96318e433315e920364bbf6890b37ef6f545d3de6d026dec7575687c",
                "6c1c05697ba4578d83378e731394b5068c6099d9fab332fa119d62a972b2d5ba",
            ),
            (
                "inviteNameOnly",
                "4a7287b0726b7e3024778f7bc46ae43bbcb3c7fbb9d8a3f3d129101e9b54f102",
                "6c1c05697ba4578d83378e731394b5068c6099d9fab332fa119d62a972b2d5ba",
            ),
            (
                "inviteLogoOnly",
                "e7fae2a7a902c800010664a5c93e8be5fd711748d7edab7ceac9a86e71f2d699",
                "5e76899bc55e702b3bf53532590cbbb2f586cf31e78f672325e0b741abde067b",
            ),
            (
                "inviteNoLogo",
                "7787e44aa8acd0465d0171f682185977ec95e1258d879426be7b30161b191d3b",
                "a911a22ebd46512de95148911c28e0bc09816618aea907e9bef80f392107180f",
            ),
            (
                "passwordUpdate",
                "a2253b9472387fc04f698c2fcab7574f58e5dc7d925adadeb5c2a889fa64b8cf",
                "ca19fd711fa427f64727d7b0f436647cca2731d1bacfd889a855c67a22ef5f4a",
            ),
            (
                "twoFactorBackupUsed",
                "2a6b6f1658cdd29e8c53baf3c4d3c4c1240ba847f07089101ddef21edc99ffb1",
                "fe4f45c843f3b3efe4237579684a07ae57bece8b6625ebe528dc53fe8de0152c",
            ),
            (
                "twoFactorCodesRegen",
                "f6de3a662b263ccf91602c82dc1238c7788655e8e1cedfcd8ae2e15f0efbbc60",
                "7255b40fe6fcf03da1c71b80214d65ab4d22c581b1297118ffb3b4b269909701",
            ),
            (
                "twoFactorRecoveryIssued",
                "b3b8097ef46b2b536315635b0564f8d2d846ea9a244eecc0059576827e6e94f3",
                "d4f398b0b00178b5ef5352b990d7b41f54d2a8a81d8b55d827c007c13c9e9335",
            ),
            (
                "twoFactorRemoved",
                "9f8982b08de3b1d1dfe69988b667f5b26ce080979e738534f9c75cca501e92ec",
                "1466d47132cbe3c02386df06eaa8208fe26c6df87a372a223f0ba9051b5c3e1f",
            ),
            (
                "twoFactorReset",
                "ed9402f7406a4661ed914db4b518840cab013d64a9298e55747529a82a19f992",
                "485420c692e0a0e6954037a10dc536fed69b81ea0d049bd87b5b953a4ef8db50",
            ),
            (
                "serverAdmin",
                "bf28c28a19501e68dabde3091be3a6aee1c5c5027230d66d4413da3a84727180",
                "8a0fe7a49df156f7bb55f5718d4acb347ecdf78aa15b2314c985371ea9e56521",
            ),
            (
                "approvedDomain",
                "7e08b77d9bb8053de9d3b6c90d419b793a708de04c20a3b92f589dd4d9736692",
                "ac541192337be759c02c41915a6052d2d47c98b669c0aa6e0ae8439b74283e34",
            ),
            (
                "approvedDomainFull",
                "20b0cbfa1b49d9cfc319b4a62176660cab2d6826289b27d477b5ec4a83f9261a",
                "ac541192337be759c02c41915a6052d2d47c98b669c0aa6e0ae8439b74283e34",
            ),
            (
                "approvedDomainLogoOnly",
                "3289485411fc788331f3eb49911c91e1b71c1c56c0b66f4903e815f4275a1465",
                "f64e344f6cabd430d8d7c859a18efef83ad18b386934ca612b2cbade065246be",
            ),
            (
                "approvedDomainEmpty",
                "ae0453adb5d2e638ab09fb2023670dc5a1f077edb6b2d0609ef4b2e532138a60",
                "f64e344f6cabd430d8d7c859a18efef83ad18b386934ca612b2cbade065246be",
            ),
            (
                "warnSuspended",
                "92306180fcd3d4adec722e136383bd8b35986535450bb288a6034d5ba864ad21",
                "81e6d44d5f99ed95a973c34618697a3284f4dc38e67b3a191d00bfd1572f4f77",
            ),
            (
                "cleanSuspended",
                "4784f29adf74b3354e772547a5423ead1d12ce8891b120372997ebd5312e8e88",
                "33430b3aeb7b3bdd4a6444c11de08719fe6376dc400294ef15c6bf7ab9a30151",
            ),
        ]
    }

    fn canonical(key: &str) -> RenderedEmail {
        match key {
            "verification" => send_email_verification_link_email(
                "https://app.twenty.com/verify-email?token=abc123&email=test%40example.com",
                "en",
                false,
            ),
            "verificationUpdate" => send_email_verification_link_email(
                "https://app.twenty.com/verify-email?token=abc123",
                "en",
                true,
            ),
            "passwordReset" => password_reset_link_email(
                "24 hours",
                true,
                "https://app.twenty.com/reset-password?token=abc123",
                "en",
            ),
            "passwordResetNoPassword" => password_reset_link_email(
                "1 hour",
                false,
                "https://acme.localhost:3000/reset-password?token=xyz",
                "en",
            ),
            "invite" => send_invite_link_email(
                "https://app.twenty.com/invite/abc",
                Some("Acme Inc."),
                Some("workspaces/7c9e6679/logo.png"),
                "john.doe@example.com",
                "John",
                "https://app.twenty.com",
                "en",
            ),
            "inviteNameOnly" => send_invite_link_email(
                "https://app.twenty.com/invite/abc",
                Some("Acme Inc."),
                None,
                "john.doe@example.com",
                "John",
                "https://app.twenty.com",
                "en",
            ),
            "inviteLogoOnly" => send_invite_link_email(
                "https://app.twenty.com/invite/abc",
                None,
                Some("workspaces/7c9e6679/logo.png"),
                "john.doe@example.com",
                "John",
                "https://app.twenty.com",
                "en",
            ),
            "inviteNoLogo" => send_invite_link_email(
                "https://app.twenty.com/invite/abc",
                None,
                None,
                "jane@example.com",
                "Jane",
                "https://app.twenty.com",
                "en",
            ),
            "passwordUpdate" => password_update_notify_email(
                "John Doe",
                "john.doe@example.com",
                "https://app.twenty.com",
                "en",
                chrono::DateTime::parse_from_rfc3339("2026-11-28T00:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            "twoFactorBackupUsed" => {
                two_factor_authentication_backup_code_used_email("Acme", 9, "en")
            }
            "twoFactorCodesRegen" => {
                two_factor_authentication_backup_codes_regenerated_email("Acme", "en")
            }
            "twoFactorRecoveryIssued" => two_factor_authentication_recovery_code_issued_email(
                "John Doe",
                "Acme",
                chrono::DateTime::parse_from_rfc3339("2026-07-02T10:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
                "en",
            ),
            "twoFactorRemoved" => two_factor_authentication_removed_email("Acme", "en"),
            "twoFactorReset" => two_factor_authentication_reset_email("Acme", "en"),
            "serverAdmin" => server_admin_access_changed_email(
                "John Doe",
                "Jane Smith",
                "jane.smith@example.com",
                true,
                false,
                "en",
            ),
            "approvedDomain" => send_approved_access_domain_validation(
                "https://app.twenty.com/validate-domain",
                "example.com",
                Some("Acme Inc."),
                None,
                "john.doe@example.com",
                "John",
                "https://app.twenty.com",
                "en",
            ),
            "approvedDomainFull" => send_approved_access_domain_validation(
                "https://app.twenty.com/validate-domain",
                "example.com",
                Some("Acme Inc."),
                Some("workspaces/7c9e6679/logo.png"),
                "john.doe@example.com",
                "John",
                "https://app.twenty.com",
                "en",
            ),
            "approvedDomainLogoOnly" => send_approved_access_domain_validation(
                "https://app.twenty.com/validate-domain",
                "example.com",
                None,
                Some("workspaces/7c9e6679/logo.png"),
                "john.doe@example.com",
                "John",
                "https://app.twenty.com",
                "en",
            ),
            "approvedDomainEmpty" => send_approved_access_domain_validation(
                "https://app.twenty.com/validate-domain",
                "example.com",
                None,
                None,
                "john.doe@example.com",
                "John",
                "https://app.twenty.com",
                "en",
            ),
            "warnSuspended" => warn_suspended_workspace_email(
                10,
                14,
                "John Doe",
                "Acme Inc.",
                "https://acme.twenty.com/settings/billing",
                "en",
            ),
            "cleanSuspended" => {
                clean_suspended_workspace_email(14, "John Doe", "My Workspace", "en")
            }
            other => panic!("unexpected canonical variant {other}"),
        }
    }

    #[test]
    fn canonical_renders_match_upstream_sha256() {
        let mut failures = Vec::new();

        for (key, html_hash, text_hash) in golden_hashes() {
            let rendered = canonical(key);

            if sha256_hex(rendered.html.as_bytes()) != html_hash {
                failures.push(format!("{key} html"));
            }
            if sha256_hex(rendered.text.as_bytes()) != text_hash {
                failures.push(format!("{key} text"));
            }
        }

        assert!(
            failures.is_empty(),
            "canonical renders drifted from the upstream emails package: {}",
            failures.join(", ")
        );
    }

    #[test]
    fn dynamic_values_are_react_escaped_in_html() {
        let rendered = send_invite_link_email(
            "https://app.twenty.com/invite/abc?x=1&y=2",
            Some("<b>Q&</b>"),
            Some("workspaces/1/logo.png"),
            "a.b@example.com",
            "jane",
            "https://app.twenty.com",
            "en",
        );

        assert!(rendered.html.contains("&lt;b&gt;Q&amp;&lt;/b&gt;"));
        assert!(rendered.html.contains("invite/abc?x=1&amp;y=2"));
        assert!(rendered.html.contains("Jane ("));
        assert!(rendered.text.contains("<b>Q&</b>"));
        assert!(rendered.text.contains("Jane ("));
    }

    #[test]
    fn capitalize_matches_upstream_for_empty_and_odd_case() {
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("jane"), "Jane");
        assert_eq!(capitalize("Üsa"), "Üsa");
        assert_eq!(capitalize("ABC"), "ABC");
    }

    #[test]
    fn image_absolute_uri_matches_upstream_branches() {
        assert_eq!(
            image_absolute_uri("workspaces/7c/logo.png", "https://app.twenty.com"),
            "https://app.twenty.com/files/workspaces/7c/logo.png"
        );
        assert_eq!(
            image_absolute_uri("/workspaces/7c/logo.png", "https://app.twenty.com"),
            "https://app.twenty.com/files/workspaces/7c/logo.png"
        );
        assert_eq!(
            image_absolute_uri("HTTPS://cdn/x.png", "https://app.twenty.com"),
            "HTTPS://cdn/x.png"
        );
        assert_eq!(
            image_absolute_uri("data:image/png;base64,aaa", "https://app.twenty.com"),
            "data:image/png;base64,aaa"
        );
        assert_eq!(
            image_absolute_uri("//cdn/x.png", "https://a"),
            "//cdn/x.png"
        );
    }

    #[test]
    fn warn_suspended_pluralization_and_clamp() {
        let one_day = warn_suspended_workspace_email(13, 14, "N", "W", "https://l", "en");
        assert!(one_day.text.contains("next 1 day and"));
        let zero = warn_suspended_workspace_email(14, 14, "N", "W", "https://l", "en");
        assert!(zero.text.contains("next 0 day and"));
        let negative = warn_suspended_workspace_email(20, 14, "N", "W", "https://l", "en");
        assert!(negative.text.contains("next 0 day and"));
    }

    #[test]
    fn server_admin_word_switch() {
        let rendered =
            server_admin_access_changed_email("A", "T", "t@e.example", false, true, "en");
        assert!(rendered.text.contains("Full admin panel access: Disabled"));
        assert!(rendered.text.contains("Impersonation: Enabled"));
    }

    #[test]
    fn approved_domain_falls_back_to_default_workspace_logo() {
        let rendered = send_approved_access_domain_validation(
            "https://app.twenty.com/validate-domain",
            "example.com",
            Some("Acme Inc."),
            None,
            "john.doe@example.com",
            "John",
            "https://app.twenty.com",
            "en",
        );
        assert!(rendered.html.contains(DEFAULT_WORKSPACE_LOGO));
    }
}
