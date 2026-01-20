use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct HorizonPriceR {
    pub n: i64,
    pub d: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HorizonOffer {
    pub id: String,
    pub paging_token: Option<String>,
    pub seller: String,

    pub selling: serde_json::Value,
    pub buying: serde_json::Value,

    pub amount: String,
    pub price: String,

    pub price_r: Option<HorizonPriceR>,
    pub last_modified_ledger: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HorizonEmbedded<T> {
    pub records: Vec<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HorizonLinks {
    #[serde(rename = "next")]
    pub next: Option<HorizonLink>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HorizonLink {
    pub href: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HorizonPage<T> {
    #[serde(rename = "_embedded")]
    pub embedded: HorizonEmbedded<T>,
    #[serde(rename = "_links")]
    pub links: Option<HorizonLinks>,
}

