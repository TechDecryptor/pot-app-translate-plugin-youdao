async function translate(text, from, to, options) {
    const { utils } = options;
    const { tauriFetch: fetch, CryptoJS } = utils;

    const URL = "https://dict.youdao.com/webtranslate";

    const mysticTime = Date.now().toString();

    const form = new FormData();
    form.append("i", text);
    form.append("from", from);
    form.append("to", to);
    form.append("dictResult", true);
    form.append("keyid", "webfanyi");
    form.append("sign", CryptoJS.MD5(`client=fanyideskweb&mysticTime=${mysticTime}&product=webfanyi&key=Vy4EQ1uwPkUoqvcP1nIu6WiAjxFeA3Y3`).toString(CryptoJS.enc.Hex));
    form.append("client", "fanyideskweb");
    form.append("product", "webfanyi");
    form.append("appVersion", "1.0.0");
    form.append("vendor", "web");
    form.append("pointParam", "client,mysticTime,product");
    form.append("mysticTime", mysticTime);
    form.append("keyfrom", "fanyi.web");

    const headers = {
        'user-agent': 'Mozilla/5.0 (Windows NT 6.2) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/28.0.1464.0 Safari/537.36',
        "host": "dict.youdao.com",
        "origin": "https://fanyi.youdao.com",
        "referer": "https://fanyi.youdao.com/",
        "cookie": `OUTFOX_SEARCH_USER_ID=${Math.floor(Math.random() * 100000000)}@${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}; OUTFOX_SEARCH_USER_ID_NCOO=${Math.floor(Math.random() * 100000000)}.${Math.floor(Math.random() * 100000000)}`
    };

    let res = await fetch(URL, {
        method: 'POST',
        headers: {
            ...headers, "content-type": "application/x-www-form-urlencoded"
        },
        body: {
            type: 'Form',
            payload: form
        },
        responseType: 2
    });

    function decryptResult(result) {
        let aesKey = "ydsecret://query/key/B*RGygVywfNBwpmBaZg*WT7SIOUP2T0C9WHMZN39j^DAdaZhAnxvGcCY6VYFwnHl"
        let aesIv = "ydsecret://query/iv/C@lZe2YzHtZ2CYgaXKSVfsb7Y4QWHjITPPZ0nQp87fBeJ!Iv6v^6fvi2WN@bYpJ4"

        const key = CryptoJS.MD5(aesKey);
        const iv = CryptoJS.MD5(aesIv);
        var res = CryptoJS.enc.Base64.parse(result.replace(/-/g, '+').replace(/_/g, '/')).toString(CryptoJS.enc.Base64);

        const decrypted = CryptoJS.AES.decrypt(res, key, {
            iv: iv,
            mode: CryptoJS.mode.CBC,
            padding: CryptoJS.pad.Pkcs7
        });

        const decryptedText = decrypted.toString(CryptoJS.enc.Utf8);
        return JSON.parse(decryptedText.trim());
    }


    if (res.ok) {
        let result = res.data;
        let json = decryptResult(result);
        if (json.dictResult && json.dictResult.ec) {
            let ec = json.dictResult.ec;
            let pronunciations = [];
            let explanations = [];
            let associations = [];
            if (ec.word) {
                if (ec.word.wfs) {
                    for (let wf of ec.word.wfs) {
                        associations.push(`${wf.wf.name}: ${wf.wf.value}`);
                    }
                }
                if (ec.word.usphone) {
                    let usspeech = ec.word.usspeech;
                    let speechRes = await fetch(`https://dict.youdao.com/dictvoice?audio=${usspeech}`, {
                        method: 'GET',
                        headers: headers,
                        responseType: 3
                    })
                    if (speechRes.ok) {
                        let speechData = speechRes.data;

                        pronunciations.push({
                            "region": "US",
                            "symbol": `/${ec.word.usphone}/`,
                            "voice": speechData
                        });
                    } else {
                        pronunciations.push({
                            "region": "US",
                            "symbol": `/${ec.word.usphone}/`
                        });
                    }
                }
                if (ec.word.ukphone) {
                    let ukspeech = ec.word.ukspeech;
                    let speechRes = await fetch(`https://dict.youdao.com/dictvoice?audio=${ukspeech}`, {
                        method: 'GET',
                        headers: headers,
                        responseType: 3
                    })
                    if (speechRes.ok) {
                        let speechData = speechRes.data;

                        pronunciations.push({
                            "region": "UK",
                            "symbol": `/${ec.word.ukphone}/`,
                            "voice": speechData
                        });
                    } else {
                        pronunciations.push({
                            "region": "UK",
                            "symbol": `/${ec.word.ukphone}/`
                        });
                    }

                }
                if (ec.word.trs) {
                    for (let tr of ec.word.trs) {
                        let pos = tr.pos;
                        let tran = tr.tran.split("；");
                        explanations.push({
                            "trait": pos,
                            "explains": tran
                        })
                    }
                }
            }
            if (ec.exam_type) {
                associations.push("");
                associations.push(ec.exam_type.join(" "));
            }
            return {
                "pronunciations": pronunciations,
                "explanations": explanations,
                "associations": associations
            };
        } else if (json.translateResult) {
            let target = "";
            for (let pass of json.translateResult) {
                for (let line of pass) {
                    target += line.tgt + "\n";
                }
            }
            return target;
        } else {
            throw JSON.stringify(result);
        }
    } else {
        throw `Http Request Error\nHttp Status: ${res.status}\n${JSON.stringify(res.data)}`;
    }
}
