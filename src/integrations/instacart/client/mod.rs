use {
    http::{HeaderMap, HeaderName, HeaderValue, Method},
    reqwest::Client,
};

static INSTACART_API_URL: &str = "https://www.instacart.com/graphql?operationName=Category&variables=%7B%22retailerInventorySessionToken%22%3A%22v1.6800f77.208882364-17102-04027x17689-1-12-23787-0%22%2C%22pageViewId%22%3A%229420c209-e019-5ece-bd33-780471b15fae%22%2C%22orderBy%22%3A%22MOST_RELEVANT%22%2C%22first%22%3A12%2C%22pageSource%22%3A%22your_items%22%2C%22categoryId%22%3A%22all%22%2C%22shopId%22%3A%2231030%22%7D&extensions=%7B%22persistedQuery%22%3A%7B%22version%22%3A1%2C%22sha256Hash%22%3A%22ba40936148837a6d9827947af32683e3f28f0fbdb8aa048988432463f42cce0d%22%7D%7D";

pub async fn get_frequently_ordered() {
    let headers = HeaderMap::from_iter(
        [
            ("x-client-identifier", HeaderValue::from_static("web")),
            ("accept", HeaderValue::from_static("*/*")),
            (
                "accept-language",
                HeaderValue::from_static("en-US,en;q=0.8"),
            ),
            ("content-type", HeaderValue::from_static("application/json")),
            (
                "sec-ch-ua",
                HeaderValue::from_static(
                    "\"Not A(Brand\";v=\"99\", \"Brave\";v=\"121\", \"Chromium\";v=\"121\"",
                ),
            ),
            ("sec-ch-ua-mobile", HeaderValue::from_static("?0")),
            ("sec-ch-ua-platform", HeaderValue::from_static("\"macOS\"")),
            ("sec-fetch-dest", HeaderValue::from_static("empty")),
            ("sec-fetch-mode", HeaderValue::from_static("cors")),
            ("sec-fetch-site", HeaderValue::from_static("same-origin")),
            ("sec-gpc", HeaderValue::from_static("\"macOS\"")),
            (
                "x-page-view-id",
                HeaderValue::from_static("9420c209-e019-5ece-bd33-780471b15fae"),
            ),
            (
                "referrer",
                HeaderValue::from_static("https://www.instacart.com/store/aldi/buy_it_again"),
            ),
        ]
        .map(|(name, value)| (HeaderName::from_static(name), value)),
    );
    Client::new()
        .request(Method::GET, INSTACART_API_URL)
        .headers(headers)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
}
