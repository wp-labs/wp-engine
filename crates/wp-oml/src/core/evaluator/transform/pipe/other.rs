use crate::core::prelude::*;
use crate::language::{
    Get, Nth, PathGet, PathType, PiPeOperation, SkipEmpty, SxfGet, UrlGet, UrlType,
};

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use url::{Position, Url};
use wp_model_core::model::{DataField, DataRecord, Value};
impl FieldExtractor for PiPeOperation {
    fn extract_one(
        &self,
        target: &EvaluationTarget,
        src: &mut DataRecordRef<'_>,
        dst: &DataRecord,
    ) -> Option<DataField> {
        if let Some(mut from) = self.from().extract_one(target, src, dst) {
            for pipe in self.items() {
                from = pipe.value_cacu(from);
            }
            return Some(from);
        }
        None
    }
}
impl ValueProcessor for Nth {
    fn value_cacu(&self, in_val: DataField) -> DataField {
        match in_val.get_value() {
            Value::Array(arr) => {
                if let Some(found) = arr.get(self.index) {
                    return found.clone();
                }
                in_val
            }
            _ => in_val,
        }
    }
}
impl ValueProcessor for SkipEmpty {
    fn value_cacu(&self, in_val: DataField) -> DataField {
        match in_val.get_value() {
            Value::Array(x) => {
                if x.is_empty() {
                    return DataField::from_ignore(in_val.get_name());
                }
            }
            Value::Digit(x) => {
                if x.eq(&0) {
                    return DataField::from_ignore(in_val.get_name());
                }
            }
            Value::Float(x) => {
                if x.eq(&0.0) {
                    return DataField::from_ignore(in_val.get_name());
                }
            }
            Value::Chars(x) => {
                if x.is_empty() {
                    return DataField::from_ignore(in_val.get_name());
                }
            }
            Value::Obj(x) => {
                if x.is_empty() {
                    return DataField::from_ignore(in_val.get_name());
                }
            }
            _ => {}
        }
        in_val
    }
}
impl ValueProcessor for Get {
    fn value_cacu(&self, mut in_val: DataField) -> DataField {
        if let Value::Obj(obj) = in_val.get_value_mut() {
            let mut keys: VecDeque<&str> = self.name.split('/').collect();
            while let Some(key) = keys.pop_front() {
                if let Some(val) = obj.get(key) {
                    if !keys.is_empty() {
                        if let Value::Obj(o) = val.get_value() {
                            *obj = o.clone();
                        }
                    } else {
                        return val.clone();
                    }
                }
            }
        }
        in_val
    }
}
impl ValueProcessor for SxfGet {
    fn value_cacu(&self, in_val: DataField) -> DataField {
        match in_val.get_value() {
            Value::Chars(x) => {
                let sxf = parse_log(x);
                match sxf.get(&self.key) {
                    Some(v) => DataField::from_chars(in_val.get_name().to_string(), v.clone()),
                    None => DataField::from_chars(in_val.get_name().to_string(), String::default()),
                }
            }
            _ => in_val,
        }
    }
}

impl ValueProcessor for PathGet {
    fn value_cacu(&self, in_val: DataField) -> DataField {
        match in_val.get_value() {
            Value::Chars(x) => {
                let x = x.replace('\\', "/");
                let path = Path::new(&x);
                let val_str = match &self.key {
                    PathType::Default => x.to_string(),
                    PathType::Path => path
                        .parent()
                        .map(|f| f.to_string_lossy().into_owned())
                        .unwrap_or_else(|| x.to_string()),
                    PathType::FileName => path
                        .file_name()
                        .map(|f| f.to_string_lossy().into_owned())
                        .unwrap_or_else(|| x.to_string()),
                };
                DataField::from_chars(in_val.get_name().to_string(), val_str)
            }
            _ => in_val,
        }
    }
}

impl ValueProcessor for UrlGet {
    fn value_cacu(&self, in_val: DataField) -> DataField {
        match in_val.get_value() {
            Value::Chars(x) => {
                let origin_url = x.clone();
                let val_str = match Url::parse(&origin_url) {
                    Ok(url) => match &self.key {
                        UrlType::Domain => url.domain().unwrap_or(x).to_string(),
                        UrlType::HttpReqHost => {
                            let host = url.host_str().unwrap_or("");
                            let port = url.port().map(|p| format!(":{}", p)).unwrap_or_default();
                            format!("{}{}", host, port)
                        }
                        UrlType::HttpReqUri => url[Position::BeforePath..].to_string(),
                        UrlType::HttpReqPath => url.path().to_string(),
                        UrlType::HttpReqParams => url.query().unwrap_or("").to_string(),
                        UrlType::Default => origin_url.to_string(),
                    },
                    Err(_) => origin_url.to_string(),
                };
                DataField::from_chars(in_val.get_name().to_string(), val_str)
            }
            _ => in_val,
        }
    }
}

use lazy_static::lazy_static;

lazy_static! {
    // 中英文字段映射表
    static ref FIELD_MAPPING: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("url路径", "urlPath");
        m.insert("状态码", "statusCode");
        m.insert("用户名", "username");
        m.insert("密码", "password");
        m.insert("请求体", "requestBody");
        m.insert("请求头", "requestHeaders");
        m.insert("响应头", "responseHeaders");
        m.insert("响应体", "responseBody");
        m.insert("解密账号", "decryptedAccount");
        m.insert("解密密码", "decryptedPassword");
        m.insert("病毒名", "virusName");
        m.insert("文件路径", "filePath");
        m.insert("文件大小", "fileSize");
        m.insert("文件创建时间", "fileCreateTime");
        m.insert("文件MD5", "fileMd5");
        m.insert("referer路径", "refererPath");
        m.insert("描述", "describe");
        m.insert("描述信息", "describeInfo");
        m.insert("检测的引擎", "engine");
        m
    };
}

fn parse_log(input: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let deal_input = input
        .replace("\\r\\n", "\r\n")
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\\", "\\");
    let mut remaining = deal_input.trim();

    // 预定义所有可能的字段标记（按可能出现的顺序）
    const FIELD_MARKERS: [&str; 35] = [
        "疑似账号:",
        "疑似密码:",
        "疑似账号",
        "疑似密码",
        "解密账号:",
        "解密密码:",
        "解密账号",
        "解密密码",
        "url路径:",
        "url路径",
        "状态码:",
        "用户名:",
        "用户名",
        "密码:",
        "密码",
        "请求头:",
        "请求头",
        "请求体:",
        "请求体",
        "响应头:",
        "响应头",
        "响应体:",
        "响应体",
        "referer路径:",
        "referer路径",
        "GRE源IP:",
        "GRE目的IP:",
        "描述:",
        "描述信息:",
        "文件路径",
        "文件大小",
        "病毒名",
        "文件创建时间",
        "文件MD5",
        "检测的引擎",
    ];

    while !remaining.is_empty() {
        // 查找下一个最近的字段标记
        let (marker, pos) = FIELD_MARKERS
            .iter()
            .filter_map(|m| remaining.find(m).map(|p| (m, p)))
            .min_by_key(|(_, p)| *p)
            .unwrap_or((&"", remaining.len()));

        if pos > 0 {
            // 处理字段之间的空白
            remaining = &remaining[pos..];
        }

        let key = marker.trim_end_matches(':');
        let value_start = marker.len();

        // 查找下一个字段的起始位置
        let next_pos = FIELD_MARKERS
            .iter()
            .filter_map(|m| remaining[value_start..].find(m))
            .min()
            .map(|p| p + value_start)
            .unwrap_or(remaining.len());

        let raw_value = &remaining[value_start..next_pos].trim();
        let value = handle_special_cases(key, raw_value);

        map.insert(key.to_string(), value);
        remaining = &remaining[next_pos..];
    }

    // 后处理空值
    for key in FIELD_MARKERS.iter().map(|m| m.trim_end_matches(':')) {
        map.entry(key.to_string())
            .and_modify(|v| {
                if v == "-" {
                    *v = String::new()
                }
            })
            .or_insert_with(String::new);
    }

    // 将中文和英文标准化
    let mut format_map = HashMap::new();
    for (key, value) in map {
        if value.is_empty() {
            continue;
        }
        if let Some(en_key) = FIELD_MAPPING.get(key.as_str()) {
            format_map.insert(en_key.to_string(), value);
        } else {
            format_map.insert(key.to_lowercase().to_string(), value);
        }
    }
    format_map
}

fn handle_special_cases(key: &str, value: &str) -> String {
    match key {
        // 处理包含换行的字段
        "请求头" | "响应头" => value.replace("\\r\\n", "\r\n"),
        // 保留响应体的原始转义
        "响应体" => value.replace("\\\"", "\""),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use crate::core::DataTransformer;
    use crate::parser::oml_parse;
    use orion_error::TestAssert;
    use wp_data_model::cache::FieldQueryCache;
    use wp_model_core::model::{DataField, DataRecord};

    #[test]
    fn test_pipe_sxf_get() {
        let cache = &mut FieldQueryCache::default();
        let data = vec![
            DataField::from_chars(
                "A1",
                "url路径: [www.hniu.cn/zcc/info/1042/info/1038/info/1046/info/1047/info/1045/info/1046/xygk/info/1047/jxky/xygk/xygk/jxky/xyyxk/jxky/index.jsp|http://www.hniu.cn/zcc/info/1042/info/1038/info/1046/info/1047/info/1045/info/1046/xygk/info/1047/jxky/xygk/xygk/jxky/xyyxk/jxky/index.jsp] 状态码: 0 请求头: GET /zcc/info/1042/info/1038/info/1046/info/1047/info/1045/info/1046/xygk/info/1047/jxky/xygk/xygk/jxky/xyyxk/jxky/index.jsp HTTP/1.1\r\nHost: [www.hniu.cn|http://www.hniu.cn/]\r\nX-Forwarded-For: 116.132.136.185, 27.185.201.33, 123.60.254.33\r\nAccept-Encoding: br, gzip, identity\r\nsec-ch-ua-mobile: ?0\r\nsec-ch-ua-platform: \"Windows\"\r\nsec-ch-ua: \"Not(A:Brand\";v=\"99\", \"HeadlessChrome\";v=\"133\", \"Chromium\";v=\"133\"\r\naccept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7\r\nupgrade-insecure-requests: 1\r\nuser-agent: YisouSpider\r\nVia: CHN-HEshijiazhuang-AREACT1-CACHE32, CHN-TJ-GLOBAL1-CACHE32\r\nCdn-Src-Ip: 116.132.136.185\r\nX-Forwarded-Host: [www.hniu.cn|http://www.hniu.cn/]\r\nX-Forwarded-Server: [www.hniu.cn|http://www.hniu.cn/]\r\nConnection: close\r\n\r\n GRE源IP: - GRE目的IP: - ",
            ),
            DataField::from_chars(
                "B2",
                "用户名: 1234567@unicom 密码: xxx 疑似账号: - 疑似密码: - 请求头: GET /drcom/login?callback=dr1003&DDDDD=202315360136%40unicom&upass=721708&0MKKey=123456&R1=0&R2=&R3=0&R6=0&para=00&v6ip=&terminal_type=1&lang=zh-cn&jsVersion=4.2&v=8819&lang=zh HTTP/1.1\r\nHost: 10.253.0.1\r\nConnection: keep-alive\r\nUser-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36 Edg/133.0.0.0\r\nAccept: */*\r\nReferer: http://10.253.0.1/\r\nAccept-Encoding: gzip, deflate\r\nAccept-Language: zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6\r\n\r\n 请求体: - 响应头: HTTP/1.1 200 OK\r\nServer: DrcomServer1.2\r\nAccess-Control-Allow-Origin: *\r\nContent-Type: application/javascript; charset=gbk\r\nCache-Control: no-cache\r\nContent-Length: 396\r\n\r\n 响应体: dr1003({\"result\":1,\"aolno\":2544,\"m46\":0,\"v46ip\":\"10.30.129.80\",\"myv6ip\":\"\",\"sms\":0,\"NID\":\"\",\"olmac\":\"ac198e10d899\",\"ollm\":0,\"olm1\":\"00000800\",\"olm2\":\"0002\",\"olm3\":0,\"olmm\":2,\"olm5\":0,\"gid\":19,\"ispid\":2,\"opip\":\"0.0.0.0\",\"oltime\":2592000,\"olflow\":4294967295,\"lip\":\"\",\"stime\":\"\",\"etime\":\"\",\"uid\":\"202315360136@unicom\",\"UL\":\"http://edge-http.microsoft.com/captiveportal/generate_204\",\"sv\":0}) 解密账号: - 解密密码: - url路径: 10.253.0.1/drcom/login referer路径: http://10.253.0.1/ =",
            ),
            DataField::from_chars(
                "B3",
                r#"用户名teacher16密码newland疑似账号-疑似密码-请求头POST /api/users/login HTTP/1.1\r\nHost: 10.26.3.193\r\nConnection: keep-alive\r\nContent-Length: 45\r\nauthorization: Bearer undefined\r\nCache-Control: no-cache\r\nContent-Security-Policy: upgrade-insecure-requests\r\nUser-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36\r\nAccept: text/html,application/octet-stream,application/json,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8\r\nContent-Type: application/json; charset=UTF-8\r\nOrigin: [http://10.26.3.193|http://10.26.3.193/]\r\nReferer: [http://10.26.3.193/login]\r\nAccept-Encoding: gzip, deflate\r\nAccept-Language: zh-CN,zh;q=0.9\r\n\r\n请求体\{\"username\":\"teacher16\",\"password\":\"newland\"}响应头HTTP/1.1 200 OK\r\nServer: nginx/1.21.5\r\nDate: Thu, 13 Mar 2025 01:47:43 GMT\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: 850\r\nConnection: keep-alive\r\nVary: Origin\r\nAccess-Control-Allow-Origin: [http://localhost:3000|http://localhost:3000/]\r\nAccess-Control-Allow-Credentials: true\r\nAccess-Control-Expose-Headers: WWW-Authenticate,Server-Authorization\r\nCache-Control: max-age=18000\r\n\r\n响应体\{\"errCode\":0,\"message\":\"success\",\"data\":{\"token\":\"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJfaWQiOiI2NzZhNDE1MzA1ZDJlMDAwMTk2MDQ1ZGQiLCJ1c2VybmFtZSI6InRlYWNoZXIxNiIsIm5hbWUiOiJ0ZWFjaGVyMTYiLCJ0eXBlIjoidGVhY2hlciIsImNvZGUiOiIiLCJpYXQiOjE3NDE4MzA0NjN9.5yL3HiuD-pUgqsJwS8aZZhvOtMKZaka3s_clRNVbITw\",\"user\":{\"name\":\"\",\"code\":\"\",\"link\":\"\",\"disable\":0,\"_id\":\"676a415305d2e000196045dd\",\"type\":\"teacher\",\"username\":\"teacher16\",\"createdAt\":\"2024-12-24T05:06:27.770Z\",\"updatedAt\":\"2025-03-12T08:26:42.400Z\",\"loginAt\":\"2025-03-12T06:46:53.453Z\",\"token\":\"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJfaWQiOiI2NzZhNDE1MzA1ZDJlMDAwMTk2MDQ1ZGQiLCJ1c2VybmFtZSI6InRlYWNoZXIxNiIsIm5hbWUiOiJ0ZWFjaGVyMTYiLCJ0eXBlIjoidGVhY2hlciIsImNvZGUiOiIiLCJpYXQiOjE3NDE3NjIwMTN9.Nsi871oFvUPU-jXg6C_TcQpfU1NQmWYm90gQP1nt9wY\",\"lastTime\":\"1741768002399\",\"logOutAt\":\"2025-03-12T07:26:00.109Z\"}}}解密账号-解密密码-url路径10.26.3.193/api/users/loginreferer路径[http://10.26.3.193/login]"#,
            ),
        ];
        let src = DataRecord { items: data };

        let mut conf = r#"
        name : test
        ---
        X : chars =  pipe take(A1) | sxf_get(statusCode);
        Y : chars =  pipe take(B2) | sxf_get(password);
        Z : chars =  pipe take(B3) | sxf_get(password);
         "#;
        let model = oml_parse(&mut conf).unwrap();

        let target = model.transform(src, cache);

        let expect = DataField::from_chars("X".to_string(), "0".to_string());
        assert_eq!(target.field("X"), Some(&expect));
        let expect = DataField::from_chars("Y".to_string(), "xxx".to_string());
        assert_eq!(target.field("Y"), Some(&expect));
        let expect = DataField::from_chars("Z".to_string(), "newland".to_string());
        assert_eq!(target.field("Z"), Some(&expect));
    }

    #[test]
    fn test_pipe_path_get() {
        let cache = &mut FieldQueryCache::default();
        let data = vec![DataField::from_chars(
            "A1",
            "C:\\Users\\dayu\\AppData\\Local\\Temp\\B8A93152-2B59-426D-BE5F-5521D4D2D957\\api-ms-win-core-file-l1-2-1.dll",
        )];
        let src = DataRecord { items: data };

        let mut conf = r#"
        name : test
        ---
        X : chars =  pipe take(A1) | path(name);
         "#;
        let model = oml_parse(&mut conf).unwrap();

        let target = model.transform(src, cache);

        let expect = DataField::from_chars(
            "X".to_string(),
            "api-ms-win-core-file-l1-2-1.dll".to_string(),
        );
        assert_eq!(target.field("X"), Some(&expect));
    }

    #[test]
    fn test_pipe_url_get() {
        let cache = &mut FieldQueryCache::default();
        let data = vec![DataField::from_chars(
            "A1",
            "https://a.b.com:8888/OneCollector/1.0?cors=true&content-type=application/x-json-stream#id1",
        )];
        let src = DataRecord { items: data };

        let mut conf = r#"
        name : test
        ---
        A : chars =  pipe read(A1) | url(domain);
        B : chars =  pipe read(A1) | url(host);
        C : chars =  pipe read(A1) | url(uri);
        D : chars =  pipe read(A1) | url(path);
        E : chars =  pipe read(A1) | url(params);
         "#;
        let model = oml_parse(&mut conf).unwrap();

        let target = model.transform(src, cache);

        let expect = DataField::from_chars("A".to_string(), "a.b.com".to_string());
        assert_eq!(target.field("A"), Some(&expect));
        let expect = DataField::from_chars("B".to_string(), "a.b.com:8888".to_string());
        assert_eq!(target.field("B"), Some(&expect));
        let expect = DataField::from_chars(
            "C".to_string(),
            "/OneCollector/1.0?cors=true&content-type=application/x-json-stream#id1".to_string(),
        );
        assert_eq!(target.field("C"), Some(&expect));
        let expect = DataField::from_chars("D".to_string(), "/OneCollector/1.0".to_string());
        assert_eq!(target.field("D"), Some(&expect));
        let expect = DataField::from_chars(
            "E".to_string(),
            "cors=true&content-type=application/x-json-stream".to_string(),
        );
        assert_eq!(target.field("E"), Some(&expect));
    }

    #[test]
    fn test_pipe_base64() {
        let cache = &mut FieldQueryCache::default();
        let data = vec![
            DataField::from_chars("A1", "hello1"),
            DataField::from_chars(
                "B2",
                "UE9TVCAvYWNjb3VudCBIVFRQLzEuMQ0KSG9zdDogZnRwLXh0by5lbmVyZ3ltb3N0LmNvbTo2MTIyMg0KVXNlci1BZ2VudDogTW96aWxsYS81LjAgKE1hY2ludG9zaDsgSW50ZWwgTWFjIE9TIFggMTBfMTVfNykgQXBwbGVXZWJLaXQvNTM3LjM2IChLSFRNTCwgbGlrZSBHZWNrbykgQ2hyb21lLzEwMS4wLjAuMCBTYWZhcmkvNTM3LjM2DQpDb250ZW50LUxlbmd0aDogMTE0DQpDb25uZWN0aW9uOiBjbG9zZQ0KQ29udGVudC1UeXBlOiBhcHBsaWNhdGlvbi94LXd3dy1mb3JtLXVybGVuY29kZWQNCkFjY2VwdC1FbmNvZGluZzogZ3ppcA0KDQo=",
            ),
            DataField::from_chars(
                "C3",
                "U1NILTIuMC1tb2Rfc2Z0cA0KAAADVAcUUhSdWEFUvYFEugJ7xA68OgAAAT1jdXJ2ZTI1NTE5LXNoYTI1NixjdXJ2ZTI1NTE5LXNoYTI1NkBsaWJzc2gub3JnLGVjZGgtc2hhMi1uaXN0cDUyMSxlY2RoLXNoYTItbmlzdHAzODQsZWNkaC1zaGEyLW5pc3RwMjU2LGRpZmZpZS1oZWxsbWFuLWdyb3VwMTgtc2hhNTEyLGRpZmZpZS1oZWxsbWFuLWdyb3VwMTYtc2hhNTEyLGRpZmZpZS1oZWxsbWFuLWdyb3VwMTQtc2hhMjU2LGRpZmZpZS1oZWxsbWFuLWdyb3VwLWV4Y2hhbmdlLXNoYTI1NixkaWZmaWUtaGVsbG1hbi1ncm91cC1leGNoYW5nZS1zaGExLGRpZmZpZS1oZWxsbWFuLWdyb3VwMTQtc2hhMSxyc2ExMDI0LXNoYTEsZXh0LWluZm8tcwAAAClyc2Etc2hhMi01MTIscnNhLXNoYTItMjU2LHNzaC1yc2Esc3NoLWRzcwAAAF9hZXMyNTYtY3RyLGFlczE5Mi1jdHIsYWVzMTI4LWN0cixhZXMyNTYtY2JjLGFlczE5Mi1jYmMsYWVzMTI4LWNiYyxjYXN0MTI4LWNiYywzZGVzLWN0ciwzZGVzLWNiYwAAAF9hZXMyNTYtY3RyLGFlczE5Mi1jdHIsYWVzMTI4LWN0cixhZXMyNTYtY2JjLGFlczE5Mi1jYmMsYWVzMTI4LWNiYyxjYXN0MTI4LWNiYywzZGVzLWN0ciwzZGVzLWNiYwAAAFtobWFjLXNoYTItMjU2LGhtYWMtc2hhMi01MTIsaG1hYy1zaGExLGhtYWMtc2hhMS05Nix1bWFjLTY0QG9wZW5zc2guY29tLHVtYWMtMTI4QG9wZW5zc2guY29tAAAAW2htYWMtc2hhMi0yNTYsaG1hYy1zaGEyLTUxMixobWFjLXNoYTEsaG1hYy1zaGExLTk2LHVtYWMtNjRAb3BlbnNzaC5jb20sdW1hYy0xMjhAb3BlbnNzaC5jb20AAAAaemxpYkBvcGVuc3NoLmNvbSx6bGliLG5vbmUAAAAaemxpYkBvcGVuc3NoLmNvbSx6bGliLG5vbmUAAAAAAAAAAAAAAAAAXuQ3JWG631Byb3RvY29sIG1pc21hdGNoLgo=",
            ),
        ];
        let src = DataRecord { items: data };

        let mut conf = r#"
        name : test
        ---
        X : chars =  pipe take(A1) | base64_encode | base64_decode() ;
        Y : chars =  pipe take(B2) | base64_decode(Imap) ;
        Z : chars =  pipe take(C3) | base64_decode(Imap) ;
         "#;
        let model = oml_parse(&mut conf).unwrap();

        let target = model.transform(src, cache);

        let expect = DataField::from_chars("X".to_string(), "hello1".to_string());
        assert_eq!(target.field("X"), Some(&expect));

        let expect = DataField::from_chars("Y".to_string(), r#"POST /account HTTP/1.1\r\nHost: ftp-xto.energymost.com:61222\r\nUser-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.0.0 Safari/537.36\r\nContent-Length: 114\r\nConnection: close\r\nContent-Type: application/x-www-form-urlencoded\r\nAccept-Encoding: gzip\r\n\r\n"#.to_string());
        assert_eq!(target.field("Y"), Some(&expect));

        let expect = DataField::from_chars("Z".to_string(), "SSH-2.0-mod_sftp\\r\\n\\x00\\x00\\x03T\\x07\\x14R\\x14\\x9dXAT\\xbd\\x81D\\xba\\x02{\\xc4\\x0e\\xbc:\\x00\\x00\\x01=curve25519-sha256,curve25519-sha256@libssh.org,ecdh-sha2-nistp521,ecdh-sha2-nistp384,ecdh-sha2-nistp256,diffie-hellman-group18-sha512,diffie-hellman-group16-sha512,diffie-hellman-group14-sha256,diffie-hellman-group-exchange-sha256,diffie-hellman-group-exchange-sha1,diffie-hellman-group14-sha1,rsa1024-sha1,ext-info-s\\x00\\x00\\x00)rsa-sha2-512,rsa-sha2-256,ssh-rsa,ssh-dss\\x00\\x00\\x00_aes256-ctr,aes192-ctr,aes128-ctr,aes256-cbc,aes192-cbc,aes128-cbc,cast128-cbc,3des-ctr,3des-cbc\\x00\\x00\\x00_aes256-ctr,aes192-ctr,aes128-ctr,aes256-cbc,aes192-cbc,aes128-cbc,cast128-cbc,3des-ctr,3des-cbc\\x00\\x00\\x00[hmac-sha2-256,hmac-sha2-512,hmac-sha1,hmac-sha1-96,umac-64@openssh.com,umac-128@openssh.com\\x00\\x00\\x00[hmac-sha2-256,hmac-sha2-512,hmac-sha1,hmac-sha1-96,umac-64@openssh.com,umac-128@openssh.com\\x00\\x00\\x00\\x1azlib@openssh.com,zlib,none\\x00\\x00\\x00\\x1azlib@openssh.com,zlib,none\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00^\\xe47%a\\xba\\xdfProtocol mismatch.\\n".to_string());
        assert_eq!(target.field("Z"), Some(&expect));
    }

    #[test]
    fn test_html_escape() {
        let cache = &mut FieldQueryCache::default();
        let data = vec![DataField::from_chars("A1", "<html>")];
        let src = DataRecord { items: data };

        let mut conf = r#"
        name : test
        ---
        X : chars =  pipe take(A1) | html_escape | html_unescape;
         "#;
        let model = oml_parse(&mut conf).assert();

        let target = model.transform(src, cache);

        let expect = DataField::from_chars("X".to_string(), "<html>".to_string());
        assert_eq!(target.field("X"), Some(&expect));
    }

    #[test]
    fn test_str_escape() {
        let cache = &mut FieldQueryCache::default();
        let data = vec![DataField::from_chars("A1", "html\"1_")];
        let src = DataRecord { items: data };

        let mut conf = r#"
        name : test
        ---
        X : chars =  pipe take(A1) | str_escape  ;
         "#;
        let model = oml_parse(&mut conf).assert();

        let target = model.transform(src, cache);

        let expect = DataField::from_chars("X".to_string(), r#"html\"1_"#.to_string());
        assert_eq!(target.field("X"), Some(&expect));
    }

    #[test]
    fn test_json_escape() {
        let cache = &mut FieldQueryCache::default();
        let data = vec![DataField::from_chars("A1", "This is a crab: 🦀")];
        let src = DataRecord { items: data };

        let mut conf = r#"
        name : test
        ---
        X : chars =  pipe take(A1) | json_escape  | json_unescape ;
         "#;
        let model = oml_parse(&mut conf).assert();

        let target = model.transform(src, cache);

        let expect = DataField::from_chars("X".to_string(), "This is a crab: 🦀".to_string());
        assert_eq!(target.field("X"), Some(&expect));
    }

    #[test]
    fn test_pipe_time() {
        let cache = &mut FieldQueryCache::default();
        let data = vec![DataField::from_chars("A1", "<html>")];
        let src = DataRecord { items: data };

        let mut conf = r#"
        name : test
        ---
        Y  =  time(2000-10-10 0:0:0);
        X  =  pipe  read(Y) | Time::to_ts ;
        Z  =  pipe  read(Y) | Time::to_ts_ms ;
        U  =  pipe  read(Y) | Time::to_ts_us ;
         "#;
        let model = oml_parse(&mut conf).assert();
        let target = model.transform(src, cache);
        //let expect = TDOEnum::from_digit("X".to_string(), 971136000);
        let expect = DataField::from_digit("X".to_string(), 971107200);
        assert_eq!(target.field("X"), Some(&expect));
        let expect = DataField::from_digit("Z".to_string(), 971107200000);
        assert_eq!(target.field("Z"), Some(&expect));

        let expect = DataField::from_digit("U".to_string(), 971107200000000);
        assert_eq!(target.field("U"), Some(&expect));
    }

    #[test]
    fn test_pipe_skip() {
        let cache = &mut FieldQueryCache::default();
        let data = vec![
            DataField::from_digit("A1", 0),
            DataField::from_arr("A2", vec![]),
        ];
        let src = DataRecord {
            items: data.clone(),
        };

        let mut conf = r#"
        name : test
        ---
        X  =  collect take(keys: [A1, A2]) ;
        Y  =  pipe  read(A1) | skip_empty ;
        Z  =  pipe  read(A2) | skip_empty ;
         "#;
        let model = oml_parse(&mut conf).assert();
        let target = model.transform(src, cache);
        let expect = DataField::from_arr("X".to_string(), data);
        assert_eq!(target.field("X"), Some(&expect));
        assert_eq!(
            target.field("Y"),
            Some(DataField::from_ignore("Y")).as_ref()
        );
        assert_eq!(
            target.field("Z"),
            Some(DataField::from_ignore("Z")).as_ref()
        );
    }

    #[test]
    fn test_pipe_obj_get() {
        let val = r#"{"items":[{"meta":{"array":"obj"},"name":"current_process","value":{"Array":[{"meta":"obj","name":"obj","value":{"Obj":{"ctime":{"meta":"digit","name":"ctime","value":{"Digit":1676340214}},"desc":{"meta":"chars","name":"desc","value":{"Chars":""}},"md5":{"meta":"chars","name":"md5","value":{"Chars":"d4ed19a8acd9df02123f655fa1e8a8e7"}},"path":{"meta":"chars","name":"path","value":{"Chars":"c:\\\\users\\\\administrator\\\\desktop\\\\domaintool\\\\x64\\\\childproc\\\\test_le9mwv.exe"}},"sign":{"meta":"chars","name":"sign","value":{"Chars":""}},"size":{"meta":"digit","name":"size","value":{"Digit":189446}},"state":{"meta":"digit","name":"state","value":{"Digit":0}},"type":{"meta":"digit","name":"type","value":{"Digit":1}}}}}]}}]}"#;
        let src: DataRecord = serde_json::from_str(val).unwrap();
        let cache = &mut FieldQueryCache::default();

        let mut conf = r#"
        name : test
        ---
        Y  =  pipe read(current_process) | nth(0) | get(current_process/path) ;
         "#;
        let model = oml_parse(&mut conf).assert();
        let target = model.transform(src, cache);
        assert_eq!(
            target.field("Y"),
            Some(DataField::from_chars(
                "Y",
                r#"c:\\users\\administrator\\desktop\\domaintool\\x64\\childproc\\test_le9mwv.exe"#
            ))
            .as_ref()
        );
    }
}
