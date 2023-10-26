use rand::{thread_rng, Rng};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;

#[no_mangle]
pub fn translate(
    text: &str,
    from: &str,
    to: &str,
    _detect: &str,
    _needs: HashMap<String, String>,
) -> Result<Value, Box<dyn Error>> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0 (Windows NT 6.2) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/28.0.1464.0 Safari/537.36"));
    default_headers.insert("host", HeaderValue::from_str("dict.youdao.com")?);
    default_headers.insert("origin", HeaderValue::from_str("https://fanyi.youdao.com")?);
    default_headers.insert(
        "referer",
        HeaderValue::from_str("https://fanyi.youdao.com/")?,
    );
    default_headers.insert(
        "Cookie",
        HeaderValue::from_str(&format!(
            "OUTFOX_SEARCH_USER_ID={}; OUTFOX_SEARCH_USER_ID_NCOO={}",
            format!(
                "{}@{}.{}.{}.{}",
                thread_rng().gen_range(100000000..=999999999),
                thread_rng().gen_range(1..=255),
                thread_rng().gen_range(1..=255),
                thread_rng().gen_range(1..=255),
                thread_rng().gen_range(1..=255)
            ),
            format!(
                "{}.{}",
                thread_rng().gen_range(100000000..=999999999),
                thread_rng().gen_range(100000000..=999999999)
            )
        ))?,
    );
    let client = reqwest::blocking::ClientBuilder::new()
        .default_headers(default_headers)
        .build()?;
    let key = get_key(&client)?;
    let data = base_body(text, from, to, key);
    let res = client
        .post("https://dict.youdao.com/webtranslate")
        .header("content-type", "application/x-www-form-urlencoded")
        .form(&data)
        .send()?
        .text()?;
    let res = decode_result(res)?;

    if let Some(result) = parse_result(&res, &client) {
        return Ok(result);
    } else {
        return Err(format!("Result parse error: {}", &res.to_string()).into());
    }
}

fn parse_result(res: &Value, client: &Client) -> Option<Value> {
    if let Some(dict_result) = res.as_object()?.get("dictResult") {
        let dict_result = dict_result.as_object()?;
        if let Some(ec) = dict_result.get("ec") {
            let ec = ec.as_object()?;

            let mut pronunciations: Vec<Value> = Vec::new();
            let mut explanations: Vec<Value> = Vec::new();
            let mut associations: Vec<String> = Vec::new();

            if let Some(word) = ec.get("word") {
                let word = word.as_object()?;
                // 变形
                if let Some(wfs) = word.get("wfs") {
                    let wfs = wfs.as_array()?;
                    for wf in wfs {
                        let wf = wf.as_object()?.get("wf")?.as_object()?;
                        let name = wf.get("name")?.as_str()?;
                        let value = wf.get("value")?.as_str()?;
                        associations.push(format!("{}: {}", name, value));
                    }
                }
                // 发音
                if let Some(usphone) = word.get("usphone") {
                    let usphone = usphone.as_str()?;
                    let usspeech = word.get("usspeech")?.as_str()?;
                    if let Ok(voice_res) = client
                        .get(format!(
                            "https://dict.youdao.com/dictvoice?audio={usspeech}"
                        ))
                        .send()
                    {
                        let voice_res = voice_res.bytes().unwrap();
                        pronunciations.push(json!({
                            "region": "US",
                            "symbol": format!("/{}/",usphone),
                            "voice": voice_res.to_vec()
                        }));
                    } else {
                        pronunciations.push(json!({
                            "region": "US",
                            "symbol": format!("/{}/",usphone)
                        }));
                    }
                }
                if let Some(ukphone) = word.get("ukphone") {
                    let ukphone = ukphone.as_str()?;
                    let ukspeech = word.get("ukspeech")?.as_str()?;
                    if let Ok(voice_res) = client
                        .get(format!(
                            "https://dict.youdao.com/dictvoice?audio={ukspeech}"
                        ))
                        .send()
                    {
                        let voice_res = voice_res.bytes().unwrap();
                        pronunciations.push(json!({
                            "region": "UK",
                            "symbol": format!("/{}/",ukphone),
                            "voice": voice_res.to_vec()
                        }));
                    } else {
                        pronunciations.push(json!({
                            "region": "UK",
                            "symbol": format!("/{}/",ukphone)
                        }));
                    }
                }
                if let Some(trs) = word.get("trs") {
                    let trs = trs.as_array()?;
                    for i in trs {
                        let tr = i.as_object()?;
                        let pos = match tr.get("pos") {
                            Some(pos) => pos.as_str()?,
                            None => "",
                        };
                        let tran = tr.get("tran")?.as_str()?;
                        let tran: Vec<&str> = tran.split("；").collect();
                        explanations.push(json!({
                            "trait": pos,
                            "explains": tran
                        }))
                    }
                }
            }
            // 单词类型
            let mut exam_type_str = String::new();
            if let Some(exam_type) = ec.get("exam_type") {
                let exam_type = exam_type.as_array()?;
                for i in exam_type {
                    exam_type_str.push_str(i.as_str()?);
                    exam_type_str.push_str(" ");
                }
            }
            associations.push("".to_string());
            associations.push(exam_type_str.trim().to_string());
            return Some(json!({
                "pronunciations": pronunciations,
                "explanations": explanations,
                "associations": associations
            }));
        }
    }
    let mut result = String::new();
    let translate_result = res.as_object()?.get("translateResult")?.as_array()?;

    for line in translate_result {
        let tgt = line
            .as_array()?
            .get(0)?
            .as_object()?
            .get("tgt")?
            .as_str()?
            .to_string();
        result.push_str(&tgt);
    }
    return Some(Value::String(result));
}

fn decode_result(res: String) -> Result<Value, Box<dyn Error>> {
    use base64::prelude::{Engine as _, BASE64_URL_SAFE};
    use openssl::hash::hash;
    use openssl::hash::MessageDigest;
    use openssl::symm::{Cipher, Crypter, Mode};
    let key = hash(
        MessageDigest::md5(),
        b"ydsecret://query/key/B*RGygVywfNBwpmBaZg*WT7SIOUP2T0C9WHMZN39j^DAdaZhAnxvGcCY6VYFwnHl",
    )?;
    let iv = hash(
        MessageDigest::md5(),
        b"ydsecret://query/iv/C@lZe2YzHtZ2CYgaXKSVfsb7Y4QWHjITPPZ0nQp87fBeJ!Iv6v^6fvi2WN@bYpJ4",
    )?;

    let mut c = Crypter::new(Cipher::aes_128_cbc(), Mode::Decrypt, &key, Some(&iv))?;
    let ciphertext = BASE64_URL_SAFE.decode(res)?;

    let mut decrypted = vec![0; ciphertext.len() + Cipher::aes_128_cbc().block_size()];
    let count = c.update(&ciphertext, &mut decrypted)?;
    let rest = c.finalize(&mut decrypted[count..])?;
    decrypted.truncate(count + rest);

    let decrypted_str = std::str::from_utf8(&decrypted)?;
    println!("{}", decrypted_str);
    Ok(serde_json::from_str(decrypted_str)?)
}

fn sign(t: i64, key: String) -> String {
    use hex::encode;
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(format!(
        "client=fanyideskweb&mysticTime={t}&product=webfanyi&key={key}"
    ));
    let result = hasher.finalize();
    encode(result)
}

fn base_body(text: &str, from: &str, to: &str, key: String) -> Value {
    use chrono::prelude::*;
    let t = Utc::now().timestamp_millis();
    json!({
        "i":text,
        "from":from,
        "to":to,
        "dictResult":true,
        "keyid":"webfanyi",
        "sign": sign(t, key),
        "client": "fanyideskweb",
        "product": "webfanyi",
        "appVersion": "1.0.0",
        "vendor": "web",
        "pointParam": "client,mysticTime,product",
        "mysticTime": t.to_string(),
        "keyfrom": "fanyi.web",
    })
}

fn get_key(_client: &Client) -> Result<String, Box<dyn Error>> {
    Ok("fsdsogkndfokasodnaso".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn try_request() {
        let needs = HashMap::new();
        let result = translate("hello world!", "auto", "it", "ZH", needs).unwrap();
        println!("{result}");
    }
}
