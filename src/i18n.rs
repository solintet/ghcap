use serde_json::Value;
use std::{env, fs, path::PathBuf};
const EN:&str=include_str!("../locales/en.json");
const JA:&str=include_str!("../locales/ja.json");
const ZH:&str=include_str!("../locales/zh-CN.json");
const KO:&str=include_str!("../locales/ko.json");
pub fn detect_language()->String{for v in [env::var("LC_ALL").ok(),env::var("LANGUAGE").ok(),env::var("LANG").ok()].into_iter().flatten(){let l=v.to_lowercase();if l.starts_with("ja"){return "ja".into()}if l.starts_with("zh"){return "zh-CN".into()}if l.starts_with("ko"){return "ko".into()}if l.starts_with("en"){return "en".into()}}"en".into()}
fn bundled(l:&str)->&'static str{match l{"ja"=>JA,"zh-CN"=>ZH,"ko"=>KO,_=>EN}}
fn parse(s:&str)->Value{serde_json::from_str(s).unwrap_or_else(|_|serde_json::from_str(EN).expect("bundled English locale is valid JSON"))}
pub fn locale(l:&str)->Value{let p=dirs::config_dir().unwrap_or_else(||PathBuf::from(".")).join("ghcap/locales").join(format!("{l}.json"));fs::read_to_string(p).map(|s|parse(&s)).unwrap_or_else(|_|parse(bundled(l)))}
pub fn english()->Value{parse(EN)}
pub fn text(l:&str,k:&str)->String{locale(l).get(k).and_then(Value::as_str).map(str::to_owned).or_else(||english().get(k).and_then(Value::as_str).map(str::to_owned)).unwrap_or_else(||k.into())}
pub fn lang_name(l:&str)->String{text(l,&format!("language.{l}"))}
pub fn available()->&'static [&'static str]{&["en","ja","zh-CN","ko"]}
