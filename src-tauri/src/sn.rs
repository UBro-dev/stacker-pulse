//! Live Stacker News data: feed, active bounties, stacker stats and territory
//! economics.
//! Author: UBro-dev
//! License: MIT (Copyright (c) 2026 UBro-dev)
//!
//! Every query below was written against the Stacker News source
//! (api/typeDefs and api/resolvers, October 2026). All values go in as GraphQL
//! variables, never pasted into the query text, and every request goes through
//! `SnGraphqlClient` — the allowlist, Tor and Airgap rules apply unchanged.

use crate::graphql::SnGraphqlClient;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};

const SN_CLEAR: &str = "https://stacker.news";
const SN_ONION: &str = "http://snsnsnya6h3ot563f3p566wuhfoklkg5f62hokdlaqzcaub3gf4xlxyd.onion";

// ---------------------------------------------------------------------------
// Input validation
// ---------------------------------------------------------------------------

/// Periods understood by SN (`whenToFrom` in lib/time.js).
pub fn check_period(p: &str) -> Result<&'static str, String> {
    match p {
        "day" => Ok("day"),
        "week" => Ok("week"),
        "month" => Ok("month"),
        "year" => Ok("year"),
        "forever" => Ok("forever"),
        _ => Err(format!("unknown period '{p}'")),
    }
}

/// SN sorts: "new", "top", or anything else = hot ranking.
pub fn check_sort(s: &str) -> Result<&'static str, String> {
    match s {
        "hot" => Ok("hot"),
        "new" => Ok("new"),
        "top" => Ok("top"),
        _ => Err(format!("unknown sort '{s}'")),
    }
}

/// Territory and user names on SN are letters, digits and underscores.
pub fn check_name(kind: &str, n: &str) -> Result<String, String> {
    let n = n.trim().trim_start_matches(['@', '~']);
    if !n.is_empty() && n.len() <= 32 && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(n.to_string())
    } else {
        Err(format!("invalid {kind} name '{n}'"))
    }
}

fn s(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}
fn i(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| x.as_i64())
}
/// IDs come back as strings, but accept numbers too.
fn id_of(v: &Value) -> Option<String> {
    match v.get("id")? {
        Value::String(s) if s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty() => {
            Some(s.clone())
        }
        Value::Number(n) => n.as_u64().map(|n| n.to_string()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Feed and bounties
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct FeedItem {
    pub id: String,
    pub title: String,
    pub sats: i64,
    pub comments: i64,
    pub boost: i64,
    pub sub: String,
    pub user: String,
    pub url: String,
    pub external_url: Option<String>,
    pub item_type: String, // BOUNTY | LINK | DISCUSSION
    pub bounty: Option<i64>,
    pub bounty_paid: bool,
    pub created_at: String,
}

pub const FEED_QUERY: &str = r#"query StackerPulseFeed($sub: String, $sort: String, $type: String, $when: String, $limit: Limit!) {
  items(sub: $sub, sort: $sort, type: $type, when: $when, limit: $limit) {
    cursor
    items { id title url sats ncomments boost bounty bountyPaidTo createdAt subName user { name } }
  }
}"#;

pub fn parse_feed(data: &Value, onion: bool) -> Vec<FeedItem> {
    let base = if onion { SN_ONION } else { SN_CLEAR };
    let items = data
        .pointer("/items/items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    items
        .iter()
        .filter_map(|it| {
            let id = id_of(it)?;
            let external = it
                .get("url")
                .and_then(|u| u.as_str())
                .filter(|u| u.starts_with("http"))
                .map(str::to_string);
            let bounty = i(it, "bounty").filter(|b| *b > 0);
            let bounty_paid = it
                .get("bountyPaidTo")
                .map(|v| v.as_array().map(|a| !a.is_empty()).unwrap_or(!v.is_null()))
                .unwrap_or(false);
            Some(FeedItem {
                url: format!("{base}/items/{id}"),
                title: s(it, "title").chars().take(300).collect(),
                sats: i(it, "sats").unwrap_or(0),
                comments: i(it, "ncomments").unwrap_or(0),
                boost: i(it, "boost").unwrap_or(0),
                sub: s(it, "subName"),
                user: it
                    .pointer("/user/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("anon")
                    .to_string(),
                item_type: if bounty.is_some() {
                    "BOUNTY".into()
                } else if external.is_some() {
                    "LINK".into()
                } else {
                    "DISCUSSION".into()
                },
                external_url: external,
                bounty,
                bounty_paid,
                created_at: s(it, "createdAt"),
                id,
            })
        })
        .collect()
}

pub async fn fetch_feed(
    client: &SnGraphqlClient,
    sub: Option<&str>,
    sort: &str,
    when: Option<&str>,
) -> Result<Vec<FeedItem>, String> {
    let sub = match sub.map(str::trim).filter(|x| !x.is_empty() && *x != "all") {
        Some(x) => Some(check_name("territory", x)?),
        None => None,
    };
    let sort = check_sort(sort)?;
    let when = match when {
        Some(w) if sort == "top" => Some(check_period(w)?),
        _ => None,
    };
    let data = client
        .query_data(
            FEED_QUERY,
            json!({ "sub": sub, "sort": sort, "type": Value::Null, "when": when, "limit": 50 }),
        )
        .await?;
    Ok(parse_feed(&data, client.onion_links()))
}

/// Bounties that nobody has been paid for yet ("bounties_active" in SN).
pub async fn fetch_active_bounties(client: &SnGraphqlClient) -> Result<Vec<FeedItem>, String> {
    let data = client
        .query_data(FEED_QUERY, json!({ "sub": Value::Null, "sort": "new", "type": "bounties_active", "when": Value::Null, "limit": 50 }))
        .await?;
    Ok(parse_feed(&data, client.onion_links())
        .into_iter()
        .filter(|f| !f.bounty_paid)
        .collect())
}

// ---------------------------------------------------------------------------
// BTC price and today's rewards pool (shown in the header)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Market {
    pub btc_usd: Option<f64>,
    pub rewards_pool_sats: Option<i64>,
}

pub const MARKET_QUERY: &str =
    r#"query StackerPulseMarket { price(fiatCurrency: "USD") rewards { total time } }"#;

pub fn parse_market(data: &Value) -> Market {
    Market {
        // SN returns -1 when it has no price
        btc_usd: data
            .get("price")
            .and_then(|p| p.as_f64())
            .filter(|p| *p > 0.0),
        rewards_pool_sats: data
            .get("rewards")
            .and_then(|r| r.as_array())
            .and_then(|a| a.first())
            .and_then(|r| r.get("total"))
            .and_then(|t| t.as_i64()),
    }
}

pub async fn fetch_market(client: &SnGraphqlClient) -> Result<Market, String> {
    let data = client.query_data(MARKET_QUERY, json!({})).await?;
    Ok(parse_market(&data))
}

// ---------------------------------------------------------------------------
// Stacker stats
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct StackerStats {
    pub name: String,
    pub member_since: String,
    pub period: String,
    pub items: i64,
    pub territories: i64,
    /// None when the stacker hides their stats ("hide from top users").
    pub stacked: Option<i64>,
    pub spent: Option<i64>,
    pub streak: Option<i64>,
    pub max_streak: Option<i64>,
    pub is_contributor: bool,
    pub has_recv_wallet: bool,
    pub url: String,
}

pub const USER_QUERY: &str = r#"query StackerPulseUser($name: String!, $when: String) {
  user(name: $name) {
    name createdAt
    nitems(when: $when)
    nterritories(when: $when)
    optional { stacked(when: $when) spent(when: $when) streak maxStreak isContributor hasRecvWallet }
  }
}"#;

pub fn parse_user(data: &Value, period: &str, onion: bool) -> Result<StackerStats, String> {
    let u = data
        .get("user")
        .filter(|u| !u.is_null())
        .ok_or("no stacker with that name")?;
    let opt = u.get("optional").cloned().unwrap_or(Value::Null);
    let name = s(u, "name");
    Ok(StackerStats {
        url: format!("{}/{}", if onion { SN_ONION } else { SN_CLEAR }, name),
        member_since: s(u, "createdAt"),
        period: period.to_string(),
        items: i(u, "nitems").unwrap_or(0),
        territories: i(u, "nterritories").unwrap_or(0),
        stacked: i(&opt, "stacked"),
        spent: i(&opt, "spent"),
        streak: i(&opt, "streak"),
        max_streak: i(&opt, "maxStreak"),
        is_contributor: opt
            .get("isContributor")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        has_recv_wallet: opt
            .get("hasRecvWallet")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        name,
    })
}

pub async fn fetch_stacker(
    client: &SnGraphqlClient,
    name: &str,
    period: &str,
) -> Result<StackerStats, String> {
    let name = check_name("stacker", name)?;
    let period = check_period(period)?;
    let data = client
        .query_data(USER_QUERY, json!({ "name": name, "when": period }))
        .await?;
    parse_user(&data, period, client.onion_links())
}

// ---------------------------------------------------------------------------
// Territory economics
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct TerritoryStats {
    pub name: String,
    pub founder: String,
    pub post_cost: i64,
    pub reply_cost: i64,
    pub billing_type: String, // MONTHLY | YEARLY | ONCE
    pub billing_cost: i64,
    pub status: String,
    pub created_at: String,
    pub items: i64,
    /// What the founder actually received in the period (SN TERRITORY_REVENUE).
    pub revenue: i64,
    /// Sats zapped to authors inside the territory in the period.
    pub stacked: i64,
    /// Everything spent inside the territory in the period.
    pub spent: i64,
    pub period_days: f64,
    /// Revenue scaled to 30 days.
    pub revenue_30d: i64,
    /// Plan cost per 30 days (0 for a one-time plan).
    pub cost_30d: i64,
    pub net_30d: i64,
    /// Whether revenue covers the recurring plan; None for one-time plans.
    pub covers_cost: Option<bool>,
    /// One-time plans only: months of current revenue needed to earn the price back.
    pub months_to_recoup: Option<f64>,
    pub url: String,
}

pub const TOP_SUBS_QUERY: &str = r#"query StackerPulseTerritories($when: String, $by: String, $cursor: String) {
  topSubs(when: $when, by: $by, cursor: $cursor, limit: 50) {
    cursor
    subs {
      name createdAt baseCost replyCost billingType billingCost status
      user { name }
      nitems(when: $when)
      optional { stacked(when: $when) spent(when: $when) revenue(when: $when) }
    }
  }
}"#;

fn period_days(period: &str, created_at: &str, now: DateTime<Utc>) -> f64 {
    let since_created = DateTime::parse_from_rfc3339(created_at)
        .map(|c| (now - c.with_timezone(&Utc)).num_seconds() as f64 / 86_400.0)
        .unwrap_or(f64::MAX);
    let window = match period {
        "day" => 1.0,
        "week" => 7.0,
        "month" => 30.0,
        "year" => 365.0,
        // SN's "forever" starts on 2021-05-01
        _ => DateTime::parse_from_rfc3339("2021-05-01T00:00:00Z")
            .map(|d| (now - d.with_timezone(&Utc)).num_seconds() as f64 / 86_400.0)
            .unwrap_or(1.0),
    };
    // A territory younger than the window only earned for its own lifetime.
    window.min(since_created).max(1.0)
}

/// Revenue and plan cost per 30 days. `covers` is None for one-time plans:
/// there is no recurring cost to compare with, so `months` says how long the
/// current revenue needs to earn the price back.
pub fn economics(
    billing_type: &str,
    billing_cost: i64,
    revenue: i64,
    days: f64,
) -> (i64, i64, i64, Option<bool>, Option<f64>) {
    let revenue_30d = ((revenue as f64) * 30.0 / days).round() as i64;
    let cost_30d = match billing_type {
        "MONTHLY" => billing_cost,
        "YEARLY" => ((billing_cost as f64) * 30.0 / 365.0).round() as i64,
        _ => 0,
    };
    let net_30d = revenue_30d.saturating_sub(cost_30d);
    if billing_type == "ONCE" {
        let months = (revenue_30d > 0)
            .then(|| ((billing_cost as f64) / (revenue_30d as f64) * 10.0).round() / 10.0);
        (revenue_30d, cost_30d, net_30d, None, months)
    } else {
        (revenue_30d, cost_30d, net_30d, Some(net_30d >= 0), None)
    }
}

pub fn parse_territories(
    data: &Value,
    period: &str,
    now: DateTime<Utc>,
    onion: bool,
) -> (Vec<TerritoryStats>, Option<String>) {
    let base = if onion { SN_ONION } else { SN_CLEAR };
    let top = data.get("topSubs").cloned().unwrap_or(Value::Null);
    let cursor = top
        .get("cursor")
        .and_then(|c| c.as_str())
        .map(str::to_string);
    let subs = top
        .get("subs")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let list = subs
        .iter()
        .filter_map(|t| {
            let name = s(t, "name");
            if check_name("territory", &name).is_err() {
                return None;
            }
            let opt = t.get("optional").cloned().unwrap_or(Value::Null);
            let created_at = s(t, "createdAt");
            let billing_type = s(t, "billingType");
            let billing_cost = i(t, "billingCost").unwrap_or(0);
            let revenue = i(&opt, "revenue").unwrap_or(0);
            let days = period_days(period, &created_at, now);
            let founder = t
                .pointer("/user/name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let (revenue_30d, cost_30d, net_30d, covers_cost, months_to_recoup) =
                economics(&billing_type, billing_cost, revenue, days);
            Some(TerritoryStats {
                url: format!("{base}/~{name}"),
                founder,
                post_cost: i(t, "baseCost").unwrap_or(0),
                reply_cost: i(t, "replyCost").unwrap_or(0),
                billing_type,
                billing_cost,
                status: s(t, "status"),
                created_at,
                items: i(t, "nitems").unwrap_or(0),
                revenue,
                stacked: i(&opt, "stacked").unwrap_or(0),
                spent: i(&opt, "spent").unwrap_or(0),
                period_days: (days * 10.0).round() / 10.0,
                revenue_30d,
                cost_30d,
                net_30d,
                covers_cost,
                months_to_recoup,
                name,
            })
        })
        .collect();
    (list, cursor)
}

const MAX_TERRITORY_PAGES: usize = 8;

pub async fn fetch_territories(
    client: &SnGraphqlClient,
    period: &str,
) -> Result<Vec<TerritoryStats>, String> {
    let period = check_period(period)?;
    let mut all = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_TERRITORY_PAGES {
        let data = client
            .query_data(
                TOP_SUBS_QUERY,
                json!({ "when": period, "by": "revenue", "cursor": cursor }),
            )
            .await?;
        let (page, next) = parse_territories(&data, period, Utc::now(), client.onion_links());
        let empty = page.is_empty();
        all.extend(page);
        match next {
            Some(c) if !empty => cursor = Some(c),
            _ => break,
        }
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-28T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn validation() {
        assert_eq!(check_name("territory", "~devs").unwrap(), "devs");
        assert_eq!(check_name("stacker", " @k00b ").unwrap(), "k00b");
        for bad in ["", "a b", "x\")", "../x", &"a".repeat(33)] {
            assert!(check_name("stacker", bad).is_err(), "accepted {bad:?}");
        }
        assert!(check_period("forever").is_ok());
        assert!(check_period("recent").is_err());
        assert!(check_sort("new").is_ok());
        assert!(
            check_sort("recent").is_err(),
            "'recent' silently means hot on SN"
        );
    }

    #[test]
    fn queries_use_variables_only() {
        for q in [FEED_QUERY, USER_QUERY, TOP_SUBS_QUERY] {
            assert!(!q.contains("{}"), "no format placeholders in query text");
        }
        assert!(FEED_QUERY.contains("items(sub: $sub, sort: $sort, type: $type"));
    }

    #[test]
    fn feed_parsing() {
        let data = json!({ "items": { "cursor": "x", "items": [
            { "id": "1590001", "title": "Bounty: landing page", "url": null, "sats": 1200, "ncomments": 4,
              "boost": 0, "bounty": 50000, "bountyPaidTo": null, "createdAt": "2026-10-27T09:00:00.000Z",
              "subName": "devs", "user": { "name": "alice" } },
            { "id": "1590002", "title": "A link", "url": "https://example.org/a", "sats": 10, "ncomments": 0,
              "bounty": null, "bountyPaidTo": null, "createdAt": "2026-10-27T10:00:00.000Z", "subName": "bitcoin",
              "user": { "name": "bob" } },
            { "id": "1590003", "title": "Paid", "bounty": 1000, "bountyPaidTo": [42], "user": null },
            { "id": "<b>", "title": "bad id" },
            { "id": 1590004, "title": "numeric id", "url": "javascript:alert(1)" }
        ]}});
        let items = parse_feed(&data, true);
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].item_type, "BOUNTY");
        assert_eq!(items[0].bounty, Some(50000));
        assert!(items[0].url.starts_with(SN_ONION));
        assert_eq!(items[1].item_type, "LINK");
        assert!(items[2].bounty_paid);
        assert_eq!(items[2].user, "anon");
        assert_eq!(items[3].id, "1590004");
        assert_eq!(items[3].external_url, None, "non-http URLs are dropped");
    }

    #[test]
    fn market_parsing() {
        let m = parse_market(
            &json!({ "price": 104250.5, "rewards": [{ "total": 812345, "time": "2026-10-28T00:00:00Z" }] }),
        );
        assert_eq!(
            m,
            Market {
                btc_usd: Some(104250.5),
                rewards_pool_sats: Some(812345)
            }
        );
        let none = parse_market(&json!({ "price": -1, "rewards": [] }));
        assert_eq!(
            none,
            Market {
                btc_usd: None,
                rewards_pool_sats: None
            }
        );
    }

    #[test]
    fn user_parsing_and_hidden_stats() {
        let data = json!({ "user": { "name": "alice", "createdAt": "2023-01-01T00:00:00.000Z", "nitems": 120,
            "nterritories": 1, "optional": { "stacked": 250000, "spent": 90000, "streak": 12, "maxStreak": 99,
            "isContributor": true, "hasRecvWallet": true } } });
        let u = parse_user(&data, "month", false).unwrap();
        assert_eq!(
            (u.items, u.stacked, u.spent),
            (120, Some(250000), Some(90000))
        );
        assert_eq!(u.url, "https://stacker.news/alice");
        let hidden = json!({ "user": { "name": "bob", "createdAt": "2024-01-01T00:00:00Z", "nitems": 3,
            "nterritories": 0, "optional": { "stacked": null, "spent": null, "streak": null } } });
        let h = parse_user(&hidden, "forever", false).unwrap();
        assert_eq!((h.stacked, h.spent), (None, None));
        assert!(parse_user(&json!({ "user": null }), "day", false).is_err());
    }

    #[test]
    fn territory_economics() {
        // month: 60k revenue against a 50k monthly plan
        let (r, c, n, ok, m) = economics("MONTHLY", 50_000, 60_000, 30.0);
        assert_eq!((r, c, n, ok, m), (60_000, 50_000, 10_000, Some(true), None));
        // yearly plan: 500k / 365 * 30 = 41,096 per 30 days
        let (_, c, n, ok, _) = economics("YEARLY", 500_000, 30_000, 30.0);
        assert_eq!((c, n, ok), (41_096, -11_096, Some(false)));
        // one-time plan: 3M at 100k per month -> 30 months
        let (_, c, _, ok, m) = economics("ONCE", 3_000_000, 100_000, 30.0);
        assert_eq!((c, ok, m), (0, None, Some(30.0)));
        let (_, _, _, _, m) = economics("ONCE", 3_000_000, 0, 30.0);
        assert_eq!(m, None, "no revenue, no payback estimate");
        // week of revenue is scaled to 30 days
        let (r, _, _, _, _) = economics("MONTHLY", 50_000, 7_000, 7.0);
        assert_eq!(r, 30_000);
    }

    #[test]
    fn young_territory_uses_its_own_lifetime() {
        // created 10 days ago, asked for a year: revenue covers 10 days, not 365
        assert_eq!(period_days("year", "2026-10-18T00:00:00Z", now()), 10.0);
        assert_eq!(period_days("month", "2020-01-01T00:00:00Z", now()), 30.0);
        assert_eq!(period_days("day", "garbage", now()), 1.0);
    }

    #[test]
    fn territories_parsing() {
        let data = json!({ "topSubs": { "cursor": "next", "subs": [
            { "name": "devs", "createdAt": "2023-12-01T00:00:00.000Z", "baseCost": 100, "replyCost": 1,
              "billingType": "MONTHLY", "billingCost": 50000, "status": "ACTIVE", "user": { "name": "founder1" },
              "nitems": 300, "optional": { "stacked": 900000, "spent": 400000, "revenue": 80000 } },
            { "name": "bad name!", "user": { "name": "x" } }
        ]}});
        let (list, cursor) = parse_territories(&data, "month", now(), false);
        assert_eq!(cursor.as_deref(), Some("next"));
        assert_eq!(list.len(), 1);
        let t = &list[0];
        assert_eq!(
            (t.founder.as_str(), t.revenue, t.net_30d, t.covers_cost),
            ("founder1", 80000, 30000, Some(true))
        );
        assert_eq!(t.url, "https://stacker.news/~devs");
    }
}
