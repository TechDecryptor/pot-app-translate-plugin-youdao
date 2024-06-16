use aes::cipher::{BlockDecryptMut, KeyIvInit};
use base64::prelude::{Engine as _, BASE64_URL_SAFE};
use cbc::cipher::block_padding::Pkcs7;
use md5::{Digest, Md5};
use rand::{thread_rng, Rng};
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde_json;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

#[no_mangle]
pub fn translate(
    text: &str,
    from: &str,
    to: &str,
    _detect: &str,
    _needs: HashMap<String, String>,
) -> Result<Value, Box<dyn Error>> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert(header::USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 6.2) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/28.0.1464.0 Safari/537.36"));
    default_headers.insert(header::HOST, HeaderValue::from_static("dict.youdao.com"));
    default_headers.insert(
        header::ORIGIN,
        HeaderValue::from_static("https://fanyi.youdao.com"),
    );
    default_headers.insert(
        header::REFERER,
        HeaderValue::from_static("https://fanyi.youdao.com/"),
    );
    default_headers.insert(
        header::COOKIE,
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

    let data = base_body(text, from, to);
    let res = client
        .post("https://dict.youdao.com/webtranslate")
        .header("content-type", "application/x-www-form-urlencoded")
        .form(&data)
        .send()?
        .text()?;

    let res = decode_result(res)?;

    let res = res.as_object().ok_or("Result is not an object")?;

    let mut result = String::new();
    let translate_result = res.get("translateResult").ok_or("No translateResult")?;
    let translate_result = translate_result
        .as_array()
        .ok_or("translateResult is not an array")?;

    for p in translate_result {
        let p = p.as_array().ok_or("translateResult item is not an array")?;
        for l in p {
            let tgt = l
                .as_object()
                .ok_or("translateResult item is not an object")?;
            let tgt = tgt.get("tgt").ok_or("No tgt")?;
            let tgt = tgt.as_str().ok_or("tgt is not a string")?;

            result.push_str(tgt);
        }
    }
    Ok(Value::String(result))
}

fn decode_result(res: String) -> Result<Value, Box<dyn Error>> {
    let mut key_hasher = Md5::new();
    key_hasher.update(
        b"ydsecret://query/key/B*RGygVywfNBwpmBaZg*WT7SIOUP2T0C9WHMZN39j^DAdaZhAnxvGcCY6VYFwnHl",
    );
    let key = key_hasher.finalize();

    let mut iv_hasher = Md5::new();
    iv_hasher.update(
        b"ydsecret://query/iv/C@lZe2YzHtZ2CYgaXKSVfsb7Y4QWHjITPPZ0nQp87fBeJ!Iv6v^6fvi2WN@bYpJ4",
    );
    let iv = iv_hasher.finalize();

    // 只使用前 16 个字节（128 位）作为 AES 密钥和 IV
    let key = &key[..16];
    let iv = &iv[..16];

    let mut ciphertext = BASE64_URL_SAFE.decode(res)?;
    // 创建 AES 解密器
    let decryptor = Aes128CbcDec::new_from_slices(key, iv).unwrap();

    // 解密
    let decrypted: &mut [u8] = &mut ciphertext;
    // let mut decrypted: &mut [u8] = ciphertext.clone();
    let len = decryptor
        .decrypt_padded_mut::<Pkcs7>(decrypted)
        .unwrap()
        .len();

    let decrypted_str = std::str::from_utf8(&decrypted[..len])?;
    // return Ok(Value::String(decrypted_str.to_string()));
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

fn base_body(text: &str, from: &str, to: &str) -> Value {
    use chrono::prelude::*;
    let t = Utc::now().timestamp_millis();
    json!({
        "i":text,
        "from":from,
        "to":to,
        "dictResult":true,
        "keyid":"webfanyi",
        "sign": sign(t, "fsdsogkndfokasodnaso".to_string()),
        "client": "fanyideskweb",
        "product": "webfanyi",
        "appVersion": "1.0.0",
        "vendor": "web",
        "pointParam": "client,mysticTime,product",
        "mysticTime": t.to_string(),
        "keyfrom": "fanyi.web",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn try_request() {
        let needs = HashMap::new();
        let result = translate("hello world!", "auto", "it", "ZH", needs);
        println!("{result:?}");
    }
}
