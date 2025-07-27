use {
    reqwest::{
        Client,
        cookie::Jar,
        header::{HeaderMap, HeaderValue},
    },
    std::sync::Arc,
};

const LANGUAGE: &str = "en-us";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.0.0 Safari/537.36";
const AMAZON_LOGIN_URL: &str = "https://www.amazon.com/ap/signin";
const ALEXA_URL: &str = "https://alexa.amazon.com";

pub async fn login(_email: &str, _password: &str) -> Result<(), reqwest::Error> {
    let jar = Arc::new(Jar::default());
    let client = Client::builder()
        .cookie_provider(Arc::clone(&jar))
        .build()?;

    client
        .get(ALEXA_URL)
        .header("DNT", "1")
        .header("Upgrade-Insecure-Requests", "1")
        .header("User-Agent", USER_AGENT)
        .header("Accept-Language", LANGUAGE)
        .header("Connection", "keep-alive")
        .header("Accept", "*/*")
        .send()
        .await?;

    let mut login_headers = HeaderMap::new();
    login_headers.insert("DNT", HeaderValue::from_static("1"));
    login_headers.insert("Upgrade-Insecure-Requests", HeaderValue::from_static("1"));
    login_headers.insert("User-Agent", HeaderValue::from_static(USER_AGENT));
    login_headers.insert("Accept-Language", HeaderValue::from_static(LANGUAGE));
    login_headers.insert("Connection", HeaderValue::from_static("keep-alive"));
    login_headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    login_headers.insert("Referer", HeaderValue::from_static(ALEXA_URL));
    login_headers.insert("Accept", HeaderValue::from_static("*/*"));

    let blank_request = client
        .post(AMAZON_LOGIN_URL)
        .headers(login_headers)
        .send()
        .await?;

    print!("{:?}", blank_request);
    Ok(())
}
