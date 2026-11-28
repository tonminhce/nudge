use chrono::{DateTime, Utc};

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
    more: &[(&str, String)],
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
    let slots = vec![("{{link}}", escape_react(link))];

    RenderedEmail {
        html: fill(html_skeleton, &extend(locale_pair(locale), &slots)),
        text: fill(text_skeleton, &slots),
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

    fn golden_hashes() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            (
                "verification",
                "3fe8a7d83880cabf52db76f3d877d29c439bccd28dc194401499a3eef6049fdf",
                "a6238d298dd2982f375ea0a1f7bd1379d0b42195e0dd9789d6a80d7ff27ece14",
            ),
            (
                "verificationUpdate",
                "33066de4e53f5e3231272d22f11ad27db7b90e2ab1cd095ee266aad2625fda94",
                "76d1042fc3df69a1f6d2e9b917b1c88a17d6ec13579572c92ee3f3e51cbbb17e",
            ),
            (
                "passwordReset",
                "6b4747080b09f157889bf00ec2d6e86e75c25b4ff9922073c68a001d2179d6a8",
                "81e9ff5fbba12a0a9ce32f96240e586f3398c5cc88794f7fe21c86ca06c4d75a",
            ),
            (
                "passwordResetNoPassword",
                "02e4d835e8bf14dd7d8edb28879f2da1f598576b9b2baa1d9d3f0e3bb6787968",
                "5354a911c0de66826cd9860321349164d40ef5f6aaa9c3b87847c97978ae2cdd",
            ),
            (
                "invite",
                "fd09ebb23df8d2690d6d77847bca827ffa7d0219c96290888a848738b19f8343",
                "96e483b9e5981c343bdc597a32e32444eac877d3a4928525c3e7339986986736",
            ),
            (
                "inviteNameOnly",
                "48c37f5ec66321938cd8fa4c8e2184f4502dc50f090af998338a9fec3f1657ad",
                "96e483b9e5981c343bdc597a32e32444eac877d3a4928525c3e7339986986736",
            ),
            (
                "inviteLogoOnly",
                "a1caa5d0310b100a23acaeeaafe7bb95aa4e562e60b1f2a5fc46170deb1a34d9",
                "bb51317126812adb6c81a0c8427f9654d9fb8d6f6daf1211413fc484a308d817",
            ),
            (
                "inviteNoLogo",
                "4df5e93a3eab4bf59886d5173a298083c90587b1009a807b7d7dcc885e201eed",
                "bb51317126812adb6c81a0c8427f9654d9fb8d6f6daf1211413fc484a308d817",
            ),
            (
                "passwordUpdate",
                "54e1f2e3171621853ffb932d819133eac10860870b449512505dbcceba30a097",
                "9d9163754fe27dd242889bb9a5d65032e47826922707e5dd7cf581aa5fd09030",
            ),
            (
                "twoFactorBackupUsed",
                "0a0a625879bf12ba37c9a7dffc54bd74782b0862b13913d20f55e32deef7dfb3",
                "dfc4926fa9c6fa400ace6a8c5b9db966c0b970df1a977b19b8fd3d8d6c660363",
            ),
            (
                "twoFactorCodesRegen",
                "2bd4326354619f3bebff422602a810e9956298e53d7acc173179bdc9616f59c4",
                "04ab12ecc03f1f7e45fecb94adec6bf000b161475a537f6e61ea475c81665431",
            ),
            (
                "twoFactorRecoveryIssued",
                "86509a8a04d3f53842afdffc17dd674ca50024b8467e494e2908ce209a1d0a30",
                "635fab66c80da5276fb49abeb2690a55f832363f96f6365fd3f1079ec9f22951",
            ),
            (
                "twoFactorRemoved",
                "997cee6678a946fd9925e56eb31adbb85aa0be98cefdd92ec86e9a1143ab44e7",
                "36b77049b839cd6ea6f31ab4c2e9631c6e2c8a723c33f957e8fa8ed6c104b998",
            ),
            (
                "twoFactorReset",
                "61d649543513bcae88091398ccef657e556b844fb798b79358a35d5e47215779",
                "9e95ab76246c9757fed7eb6aab5c7c989a55d20a7a6dc374910512d882415281",
            ),
            (
                "serverAdmin",
                "a150fddb0fed1ed7f11bfbfce11d78c3d0f0a0b188ce96604ef2d9e6277d0003",
                "bcb914cb66924ec7221dce6dde2d25fa01e6c7eadbdcb9e82138c81573bd1a62",
            ),
            (
                "approvedDomain",
                "d2af75cb6631d7a23cfc78e05ae478336710ab074656f9b354d2b07f83d2881d",
                "8b3befc26eabf3124b666921ba69f5a202aae12504f682a0a6c42fe5121e79c5",
            ),
            (
                "approvedDomainFull",
                "1d98a0bef95a4259399fb1a774a5d821f55cfaa7c5eeccc0b012d7bf208f7643",
                "8b3befc26eabf3124b666921ba69f5a202aae12504f682a0a6c42fe5121e79c5",
            ),
            (
                "approvedDomainLogoOnly",
                "f44bd5e22ca20d7dc63138a0088388ca4ec6cb292b3cb8d91f54f3d7b82c5c74",
                "7ff82cc57bb6778c6508867b43d66f98243c6f61e8ca6717b364b628f3e6a7fc",
            ),
            (
                "approvedDomainEmpty",
                "636fa7cd3abd7753401c3a13902bfcaccd95132808f29b35e6fd2163c1432052",
                "7ff82cc57bb6778c6508867b43d66f98243c6f61e8ca6717b364b628f3e6a7fc",
            ),
            (
                "warnSuspended",
                "55776cb9e02418943e3cd6283b4c321b293acf63ef1eaf35981ba28dcbb2c33d",
                "a7574d07e4dacb0e619c2e1c84adcb37716e80c35268fa9752ffd9a29a07316f",
            ),
            (
                "cleanSuspended",
                "c57e8489ca89b07ecf7ed52df5ed5bbc645b4fe691038aa0eb7a9b612e5e8ae1",
                "6057b70358ed75f341bbd062dbff00216b9297bdfebd270670b2be48d4665666",
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
        for (key, html_hash, text_hash) in golden_hashes() {
            let rendered = canonical(key);
            assert_eq!(
                sha256_hex(rendered.html.as_bytes()),
                html_hash,
                "html mismatch for {key}"
            );
            assert_eq!(
                sha256_hex(rendered.text.as_bytes()),
                text_hash,
                "text mismatch for {key}"
            );
        }
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
